//! Analíticas de visitas por sección, resolución de carriles de Aris por rol
//! y acciones sociales seguras ejecutadas desde el núcleo de Aris.

use anyhow::Result;
use serde::Serialize;

use crate::models::User;
use super::Db;

#[derive(Debug, Clone, Serialize)]
pub struct UserArisLanes {
    pub chat: bool,
    pub quick_drawer: bool,
    pub playground: bool,
    pub studio: bool,
    pub social: bool,
    pub analytics: bool,
    pub admin_ops: bool,
}

impl UserArisLanes {
    pub fn any_chat(&self) -> bool {
        self.chat || self.quick_drawer
    }

    pub fn active_labels(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.chat { out.push("Chat (/aris)"); }
        if self.quick_drawer { out.push("Asistente rápido"); }
        if self.playground { out.push("Playground código"); }
        if self.studio { out.push("Estudio creativo"); }
        if self.social { out.push("DMs a amigos"); }
        if self.analytics { out.push("Analíticas y uso"); }
        if self.admin_ops { out.push("Ops admin"); }
        out
    }

    pub fn locked_keys(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if !self.chat { out.push("aris.lane.chat"); }
        if !self.quick_drawer { out.push("aris.lane.quick_drawer"); }
        if !self.playground { out.push("aris.lane.playground"); }
        if !self.studio { out.push("aris.lane.studio"); }
        if !self.social { out.push("aris.lane.social"); }
        if !self.analytics { out.push("aris.lane.analytics"); }
        if !self.admin_ops { out.push("aris.lane.admin_ops"); }
        out
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WebVisitStat {
    pub slug: String,
    pub visits: i64,
    pub unique_users: i64,
}

impl Db {
    pub async fn user_aris_lanes(&self, user: &User) -> UserArisLanes {
        UserArisLanes {
            chat: self.user_has_perm(user, "aris.lane.chat").await,
            quick_drawer: self.user_has_perm(user, "aris.lane.quick_drawer").await,
            playground: self.user_has_perm(user, "aris.lane.playground").await,
            studio: self.user_has_perm(user, "aris.lane.studio").await,
            social: self.user_has_perm(user, "aris.lane.social").await,
            analytics: self.user_has_perm(user, "aris.lane.analytics").await,
            admin_ops: self.user_has_perm(user, "aris.lane.admin_ops").await,
        }
    }

    pub async fn record_web_visit(
        &self,
        slug: &str,
        user_id: Option<i64>,
        role_key: &str,
    ) -> Result<()> {
        let clean_slug = slug.trim();
        if clean_slug.is_empty() {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO web_visits (slug, user_id, role_key) VALUES (?, ?, ?)",
        )
        .bind(clean_slug)
        .bind(user_id)
        .bind(role_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn total_visits_in_days(&self, days: i64) -> Result<i64> {
        let window = format!("-{} days", days.clamp(1, 365));
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM web_visits WHERE created_at >= datetime('now', ?)",
        )
        .bind(window)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn top_webs_in_days(&self, days: i64, limit: i64) -> Result<Vec<WebVisitStat>> {
        let window = format!("-{} days", days.clamp(1, 365));
        let rows = sqlx::query_as::<_, WebVisitStat>(
            "SELECT slug,
                    COUNT(*) AS visits,
                    COUNT(DISTINCT COALESCE(CAST(user_id AS TEXT), 'guest')) AS unique_users
             FROM web_visits
             WHERE created_at >= datetime('now', ?)
             GROUP BY slug
             ORDER BY visits DESC, slug ASC
             LIMIT ?",
        )
        .bind(window)
        .bind(limit.clamp(1, 50))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn send_dm_via_aris(
        &self,
        sender: &User,
        target_username: &str,
        body: &str,
    ) -> std::result::Result<(i64, User, i64), String> {
        let clean_user = target_username.trim().trim_start_matches('@');
        let clean_body = body.trim();
        if clean_user.is_empty() || clean_body.is_empty() {
            return Err("Falta destinatario o contenido del mensaje.".to_string());
        }

        let target = self
            .find_user_by_username(clean_user)
            .await
            .map_err(|_| "Error consultando el usuario destino.".to_string())?
            .ok_or_else(|| format!("No existe el usuario @{}.", clean_user))?;

        if target.id == sender.id {
            return Err("No puedes enviarte un DM a ti mismo.".to_string());
        }

        let friends = self
            .are_friends(sender.id, target.id)
            .await
            .unwrap_or(false);
        if !friends {
            return Err(format!(
                "@{} no está en tu lista de amigos aceptados.",
                target.username
            ));
        }

        let conv_id = match self.find_dm(sender.id, target.id).await {
            Ok(Some(c)) => c.id,
            _ => {
                let cid = self
                    .create_conversation("dm", None, sender.id)
                    .await
                    .map_err(|_| "No se pudo abrir la conversación DM.".to_string())?;
                let _ = self.add_conversation_member(cid, sender.id).await;
                let _ = self.add_conversation_member(cid, target.id).await;
                cid
            }
        };

        let msg_id = self
            .send_message(conv_id, sender.id, clean_body)
            .await
            .map_err(|_| "No se pudo guardar el mensaje DM.".to_string())?;

        Ok((conv_id, target, msg_id))
    }

    pub async fn build_aris_core_snapshot(
        &self,
        user: &User,
        lanes: &UserArisLanes,
        days: i64,
    ) -> String {
        let (_, role_display) = self.role_display(user).await;
        let active_str = {
            let list = lanes.active_labels();
            if list.is_empty() {
                "Ninguno".to_string()
            } else {
                list.join(", ")
            }
        };
        let locked_str = {
            let list = lanes.locked_keys();
            if list.is_empty() {
                "Ninguno".to_string()
            } else {
                list.join(", ")
            }
        };

        let mut parts = vec![
            format!(
                "Usuario actual: {} (@{}) | Rol: {}",
                user.display(),
                user.username,
                role_display
            ),
            format!("Carriles activos para este rol: {}", active_str),
            format!("Carriles bloqueados para este rol: {}", locked_str),
        ];

        if lanes.analytics {
            let registered: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM users WHERE status = 'active'",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));

            let online_count = self
                .list_online_user_ids()
                .await
                .map(|v| v.len())
                .unwrap_or(0);
            let total_visits = self.total_visits_in_days(days).await.unwrap_or(0);
            let top = self.top_webs_in_days(days, 10).await.unwrap_or_default();

            let mut ranking = String::new();
            if top.is_empty() {
                ranking.push_str("sin visitas registradas en esa ventana");
            } else {
                for (idx, row) in top.iter().enumerate() {
                    if idx > 0 {
                        ranking.push_str("; ");
                    }
                    ranking.push_str(&format!(
                        "#{} /{} ({} visitas, {} usuarios)",
                        idx + 1,
                        row.slug,
                        row.visits,
                        row.unique_users
                    ));
                }
            }

            parts.push(format!(
                "ANALÍTICA EN VIVO (últimos {} días): usuarios registrados activos={}, sesiones activas recientes={}, visitas totales={} | Ranking secciones: {}",
                days, registered.0, online_count, total_visits, ranking
            ));
        }

        if lanes.social {
            let friend_ids = self.list_friends(user.id).await.unwrap_or_default();
            let mut names = Vec::new();
            for fid in friend_ids.iter().take(25) {
                if let Ok(Some(fu)) = self.find_user_by_id(*fid).await {
                    names.push(format!("@{}", fu.username));
                }
            }
            let unread = self.count_total_unread_for_user(user.id).await.unwrap_or(0);
            let friends_desc = if names.is_empty() {
                "ninguno".to_string()
            } else {
                names.join(", ")
            };
            parts.push(format!(
                "SOCIAL DEL USUARIO: amigos aceptados ({}) = [{}] | mensajes sin leer = {}",
                friend_ids.len(),
                friends_desc,
                unread
            ));
        }

        if lanes.admin_ops {
            let open_tickets: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM tickets WHERE status != 'closed'",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
            let active_keys: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM api_keys WHERE is_active = 1",
            )
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
            let audit_entries = self.list_audit_entries(5, 0).await.unwrap_or_default();
            let audit_summary = if audit_entries.is_empty() {
                "sin registros".to_string()
            } else {
                audit_entries
                    .iter()
                    .map(|a| format!("{} ({})", a.action, a.detail.as_deref().unwrap_or("-")))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            parts.push(format!(
                "OPS ADMIN: tickets abiertos={}, API keys activas={} | Última auditoría: {}",
                open_tickets.0, active_keys.0, audit_summary
            ));
        }

        parts.join("\n")
    }
}

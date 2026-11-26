#![allow(unused_imports)]
use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

use crate::models::{AuditEntry, BanInfo, Community, ConfigEntry, Conversation, ConversationRank, Document, IpBan, LinkedDevice, LinkCode, Message, PushSubscription, Role, SessionInfo, SuspensionInfo, Title, User, UserIpHistory, Web};

use super::Db;
use super::{hash_password, verify_password, random_hex, random_base32, random_numeric};

impl Db {
    pub async fn role_has_perm(&self, role_id: i64, perm: &str) -> Result<bool> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM role_perms WHERE role_id = ? AND perm = ?")
                .bind(role_id)
                .bind(perm)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.is_some())
    }

    /// Devuelve el id del rol identificado por su role_key (p.ej. "invitado").
    /// None si no existe (la migración que lo siembra no corrió).
    pub async fn role_id_by_key(&self, role_key: &str) -> Result<Option<i64>> {
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM roles WHERE role_key = ?")
                .bind(role_key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(id,)| id))
    }

    pub async fn get_role(&self, id: i64) -> Result<Option<Role>> {
        let role = sqlx::query_as::<_, Role>(
            "SELECT id, name, role_key, color, position, description, is_system, is_protected
             FROM roles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(role)
    }

    pub async fn list_roles(&self) -> Result<Vec<Role>> {
        let roles = sqlx::query_as::<_, Role>(
            "SELECT id, name, role_key, color, position, description, is_system, is_protected
             FROM roles ORDER BY position DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(roles)
    }

    pub async fn create_role(
        &self,
        name: &str,
        color: Option<&str>,
        position: i64,
        description: Option<&str>,
    ) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "INSERT INTO roles (name, role_key, color, position, description, is_system, is_protected)
             VALUES (?, '', ?, ?, ?, 0, 0)",
        )
        .bind(name)
        .bind(color)
        .bind(position)
        .bind(description)
        .execute(&mut *tx)
        .await?;
        let id = result.last_insert_rowid();
        let role_key = format!("custom_{}", id);
        sqlx::query("UPDATE roles SET role_key = ? WHERE id = ?")
            .bind(&role_key)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn update_role(
        &self,
        id: i64,
        name: &str,
        color: Option<&str>,
        position: i64,
        description: Option<&str>,
    ) -> Result<()> {
        let role = self
            .get_role(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("rol no encontrado"))?;
        let mut tx = self.pool.begin().await?;
        if role.is_protected() {
            sqlx::query("UPDATE roles SET color = ?, description = ? WHERE id = ?")
                .bind(color)
                .bind(description)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                "UPDATE roles SET name = ?, color = ?, position = ?, description = ? WHERE id = ?",
            )
            .bind(name)
            .bind(color)
            .bind(position)
            .bind(description)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_role(&self, id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM roles WHERE id = ? AND is_system = 0")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            anyhow::bail!("rol no encontrado o es del sistema (no se puede borrar)");
        }
        Ok(())
    }

    pub async fn set_role_perms(&self, role_id: i64, perms: &[String]) -> Result<()> {
        use crate::models::PERMISSIONS;
        let valid: std::collections::HashSet<&str> =
            PERMISSIONS.iter().map(|p| p.key).collect();
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM role_perms WHERE role_id = ?")
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        for p in perms {
            let p = p.trim();
            if valid.contains(p) {
                sqlx::query("INSERT OR IGNORE INTO role_perms (role_id, perm) VALUES (?, ?)")
                    .bind(role_id)
                    .bind(p)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_role_web_perms(&self, role_id: i64, web_ids: &[i64]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM role_web_perms WHERE role_id = ?")
            .bind(role_id)
            .execute(&mut *tx)
            .await?;
        for wid in web_ids {
            sqlx::query("INSERT OR IGNORE INTO role_web_perms (role_id, web_id) VALUES (?, ?)")
                .bind(role_id)
                .bind(wid)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn role_perms(&self, role_id: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT perm FROM role_perms WHERE role_id = ? ORDER BY perm")
                .bind(role_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    pub async fn role_web_perms(&self, role_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT web_id FROM role_web_perms WHERE role_id = ? ORDER BY web_id")
                .bind(role_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(w,)| w).collect())
    }

    pub async fn count_users_with_role(&self, role_id: i64) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE role_id = ?")
            .bind(role_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    pub async fn reassign_users_role(&self, from_role_id: i64, to_role_id: i64) -> Result<i64> {
        let result = sqlx::query("UPDATE users SET role_id = ? WHERE role_id = ?")
            .bind(to_role_id)
            .bind(from_role_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn assign_user_role(&self, user_id: i64, role_id: i64) -> Result<()> {
        let role = self
            .get_role(role_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("rol no encontrado"))?;
        if role.role_key == "owner" {
            anyhow::bail!("no se puede asignar el rol owner");
        }
        let legacy_role = match role.role_key.as_str() {
            "user" | "admin" | "staff" => role.role_key.clone(),
            // Rol custom: derivar legacy según posición para no romper el
            // código que lee users.role esperando un valor del enum viejo.
            _ => {
                if role.position >= 500 {
                    "admin".to_string()
                } else if role.position >= 200 {
                    "staff".to_string()
                } else {
                    "user".to_string()
                }
            }
        };
        sqlx::query("UPDATE users SET role_id = ?, role = ? WHERE id = ?")
            .bind(role_id)
            .bind(&legacy_role)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn role_display(&self, user: &User) -> (String, String) {
        // Owner: shortcut sin query (lo identificamos por users.role).
        if user.role == "owner" {
            return ("owner".to_string(), "Owner".to_string());
        }
        // Si tiene role_id y el rol es custom, mostrar su nombre real.
        if let Some(rid) = user.role_id {
            if let Ok(Some(r)) = self.get_role(rid).await {
                let is_system = r.role_key == "user"
                    || r.role_key == "admin"
                    || r.role_key == "staff"
                    || r.role_key == "owner";
                if !is_system {
                    // Custom: css_key = role_key real (custom_N) para que matchee
                    // con /role-colors.css (que genera .role-custom_N{...}).
                    // Antes devolvíamos el legacy por posición ("admin"/"staff"/
                    // "user"), lo que hacía que el color custom configurado en
                    // /admin/roles nunca se aplicase al badge: la regla CSS
                    // .role-custom_N existía pero el badge usaba .role-admin.
                    // El display_name sigue siendo el nombre real del rol.
                    return (r.role_key.clone(), r.name.clone());
                }
                // Sistema: css_key y display = role_key capitalizado.
                let key = r.role_key.clone();
                let disp = match key.as_str() {
                    "user" => "User".to_string(),
                    "admin" => "Admin".to_string(),
                    "staff" => "Staff".to_string(),
                    "owner" => "Owner".to_string(),
                    _ => key.clone(),
                };
                return (key, disp);
            }
        }
        // Sin role_id o rol no encontrado: fallback a users.role legado.
        let key = user.role.clone();
        let disp = match key.as_str() {
            "owner" => "Owner".to_string(),
            "admin" => "Admin".to_string(),
            "staff" => "Staff".to_string(),
            _ => "User".to_string(),
        };
        (key, disp)
    }

    pub async fn user_has_perm(&self, user: &User, perm: &str) -> bool {
        // Owner: bypass total. Lo identificamos por users.role (TEXT legado,
        // sincronizado con role_key en assign_user_role). Esto evita un
        // query extra para los owners.
        if user.role == "owner" {
            return true;
        }
        // Si el usuario tiene role_id (post-migración 031), consultar role_perms.
        if let Some(rid) = user.role_id {
            self.role_has_perm(rid, perm).await.unwrap_or(false)
        } else {
            // Fallback para DB sin migrar: baseline hardcoded.
            crate::models::baseline_has_perm(&user.role, perm)
        }
    }

}

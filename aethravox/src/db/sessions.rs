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
    pub async fn create_session(
        &self,
        user_id: i64,
        remember: bool,
    ) -> Result<(String, DateTime<Utc>)> {
        let token = random_hex(32);
        // La sesión NO caduca sola: duran 1 año haya o no "Recuérdame".
        // Solo se cierra al pulsar "Cerrar sesión" (/logout borra la fila
        // y la cookie). El flag `remember` se conserva por compat con el
        // formulario y la columna, pero ya no cambia el TTL.
        let ttl = Duration::days(365);
        let expires = Utc::now() + ttl;
        let rem = if remember { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO sessions (token, user_id, expires_at, remember) VALUES (?, ?, ?, ?)",
        )
        .bind(&token)
        .bind(user_id)
        .bind(expires)
        .bind(rem)
        .execute(&self.pool)
        .await?;

        Ok((token, expires))
    }

    pub async fn get_session_user(&self, token: &str) -> Result<Option<User>> {
        // Trae expires_at + is_primary + link_expires_at para poder aplicar la
        // caducidad propia de las sesiones vinculadas (3 días rolling desde la
        // última actividad). Las primary solo miran expires_at.
        let row: Option<(i64, DateTime<Utc>, i64, Option<DateTime<Utc>>)> =
            sqlx::query_as(
                "SELECT user_id, expires_at, is_primary, link_expires_at \
                 FROM sessions WHERE token = ?",
            )
            .bind(token)
            .fetch_optional(&self.pool)
            .await?;

        let now = Utc::now();
        match row {
            Some((user_id, expires, is_primary, link_expires_at)) => {
                let primary_expired = expires <= now;
                // Para vinculadas, link_expires_at es la fecha real de corte
                // (rolling 3 días). Si ya pasó, se trata como expirada.
                let linked_expired = is_primary == 0
                    && link_expires_at.map(|t| t <= now).unwrap_or(true);
                if primary_expired || linked_expired {
                    let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
                        .bind(token)
                        .execute(&self.pool)
                        .await;
                    return Ok(None);
                }
                // Toca last_seen_at (siempre) y, para vinculadas, extiende
                // link_expires_at a now + 3 días (rolling: mientras uses la
                // sesión vinculada no caduca; si pasas 3 días sin tocarla, sí).
                if is_primary == 0 {
                    let new_link_exp = now + Duration::days(3);
                    let _ = sqlx::query(
                        "UPDATE sessions SET last_seen_at = ?, link_expires_at = ? WHERE token = ?",
                    )
                    .bind(now)
                    .bind(new_link_exp)
                    .bind(token)
                    .execute(&self.pool)
                    .await;
                } else {
                    let _ = sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE token = ?")
                        .bind(now)
                        .bind(token)
                        .execute(&self.pool)
                        .await;
                }
                self.find_user_by_id(user_id).await
            }
            None => Ok(None),
        }
    }

    pub async fn delete_session(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_user_sessions(&self, user_id: i64) -> Result<Vec<SessionInfo>> {
        let rows = sqlx::query_as::<_, SessionInfo>(
            "SELECT token, expires_at, remember, created_at, last_seen_at,
                    device_name, is_primary, linked_by_user_id, link_expires_at
             FROM sessions WHERE user_id = ? ORDER BY last_seen_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn revoke_all_user_sessions(&self, user_id: i64) -> Result<u64> {
        let res = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }

    pub async fn revoke_session_by_token(&self, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

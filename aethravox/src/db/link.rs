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
    pub async fn create_linked_session(
        &self,
        user_id: i64,
        linked_by_user_id: i64,
        device_name: Option<&str>,
    ) -> Result<(String, DateTime<Utc>)> {
        let token = random_hex(32);
        let now = Utc::now();
        // Las vinculadas no "recuerdan" en el sentido clásico (no esperamos
        // 30 días); su ventana real es link_expires_at (3 días rolling).
        // expires_at = now + 30 días como tope holgado (no queremos que la
        // caduque antes que link_expires_at, que es el que manda).
        let expires = now + Duration::days(30);
        let link_exp = now + Duration::days(3);
        sqlx::query(
            "INSERT INTO sessions \
             (token, user_id, expires_at, remember, device_name, is_primary, \
              linked_by_user_id, link_expires_at) \
             VALUES (?, ?, ?, 0, ?, 0, ?, ?)",
        )
        .bind(&token)
        .bind(user_id)
        .bind(expires)
        .bind(device_name)
        .bind(linked_by_user_id)
        .bind(link_exp)
        .execute(&self.pool)
        .await?;
        Ok((token, link_exp))
    }

    pub async fn revoke_other_primary_sessions(
        &self,
        user_id: i64,
        except_token: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM sessions \
             WHERE user_id = ? AND token != ? AND is_primary = 1",
        )
        .bind(user_id)
        .bind(except_token)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn create_link_code(
        &self,
        user_id: i64,
    ) -> Result<(String, String, DateTime<Utc>)> {
        // Token URL-safe de 16 chars (base32 sin padding: A-Z2-7). Suficiente
        // entropía (80 bits) para que no se pueda adivinar el QR; corto para
        // que el QR sea pequeño.
        let code = random_base32(16);
        let numeric = random_numeric(6);
        let expires = Utc::now() + Duration::seconds(60);
        sqlx::query(
            "INSERT INTO link_codes (code, numeric_code, user_id, expires_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&code)
        .bind(&numeric)
        .bind(user_id)
        .bind(expires)
        .execute(&self.pool)
        .await?;
        Ok((code, numeric, expires))
    }

    pub async fn find_active_link_code(&self, code_or_numeric: &str) -> Result<Option<LinkCode>> {
        let trimmed = code_or_numeric.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        // Buscar por code exacto; si no, por numeric_code (sin guiones ni
        // espacios — el frontend normaliza antes de enviar).
        let row = sqlx::query_as::<_, LinkCode>(
            "SELECT code, numeric_code, user_id, expires_at FROM link_codes \
             WHERE (code = ? OR numeric_code = ?) AND used_at IS NULL AND expires_at > ? \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(trimmed)
        .bind(trimmed)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn consume_link_code(&self, code: &str, used_by_token: &str) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE link_codes SET used_at = ?, used_by_token = ? \
             WHERE code = ? AND used_at IS NULL",
        )
        .bind(Utc::now())
        .bind(used_by_token)
        .bind(code)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list_linked_devices(&self, user_id: i64) -> Result<Vec<LinkedDevice>> {
        let rows = sqlx::query_as::<_, LinkedDevice>(
            "SELECT token, device_name, created_at, last_seen_at, link_expires_at \
             FROM sessions WHERE user_id = ? AND is_primary = 0 \
             ORDER BY last_seen_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn count_linked_devices(&self, user_id: i64) -> Result<i64> {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sessions WHERE user_id = ? AND is_primary = 0",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    pub async fn revoke_linked_device(
        &self,
        owner_user_id: i64,
        token: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            "DELETE FROM sessions WHERE token = ? AND user_id = ? AND is_primary = 0",
        )
        .bind(token)
        .bind(owner_user_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn cleanup_link_codes(&self) -> Result<()> {
        let cutoff = Utc::now() - Duration::hours(24);
        sqlx::query("DELETE FROM link_codes WHERE expires_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

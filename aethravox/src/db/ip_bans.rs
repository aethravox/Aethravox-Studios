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
    pub async fn ban_ip(
        &self,
        ip: &str,
        reason: &str,
        banned_by: Option<i64>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO ip_bans (ip, reason, banned_by, expires_at) VALUES (?, ?, ?, ?)",
        )
        .bind(ip)
        .bind(reason)
        .bind(banned_by)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn is_ip_banned(&self, ip: &str) -> Result<Option<IpBan>> {
        // Marcar como "expirado" los bans temporales cuya expires_at ya pasó.
        // Lo hacemos con un UPDATE silencioso para que las siguientes queries los ignoren.
        let _ = sqlx::query(
            "UPDATE ip_bans SET revoked_at = ?
             WHERE revoked_at IS NULL AND expires_at IS NOT NULL AND expires_at <= ?",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await;

        let row = sqlx::query_as::<_, IpBan>(
            "SELECT id, ip, reason, banned_by, created_at, expires_at, revoked_at
             FROM ip_bans
             WHERE ip = ? AND revoked_at IS NULL
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(ip)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_active_ip_bans(&self) -> Result<Vec<IpBan>> {
        let rows = sqlx::query_as::<_, IpBan>(
            "SELECT id, ip, reason, banned_by, created_at, expires_at, revoked_at
             FROM ip_bans
             WHERE revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > ?)
             ORDER BY created_at DESC",
        )
        .bind(Utc::now())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_ip_bans_history(&self, limit: i64) -> Result<Vec<IpBan>> {
        let rows = sqlx::query_as::<_, IpBan>(
            "SELECT id, ip, reason, banned_by, created_at, expires_at, revoked_at
             FROM ip_bans
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn revoke_ip_ban(&self, ban_id: i64) -> Result<()> {
        sqlx::query("UPDATE ip_bans SET revoked_at = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(ban_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_login_ip(&self, user_id: i64, ip: &str) -> Result<bool> {
        // 1. ¿Ya existe esta IP para este usuario?
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM user_ip_history WHERE user_id = ? AND ip = ?",
        )
        .bind(user_id)
        .bind(ip)
        .fetch_optional(&self.pool)
        .await?;

        let is_new = existing.is_none();

        if is_new {
            // 2a. Insert nueva fila.
            sqlx::query(
                "INSERT INTO user_ip_history (user_id, ip, first_seen, last_seen, times_used)
                 VALUES (?, ?, ?, ?, 1)",
            )
            .bind(user_id)
            .bind(ip)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;
        } else {
            // 2b. Update existing: last_seen + times_used += 1.
            sqlx::query(
                "UPDATE user_ip_history SET last_seen = ?, times_used = times_used + 1
                 WHERE user_id = ? AND ip = ?",
            )
            .bind(Utc::now())
            .bind(user_id)
            .bind(ip)
            .execute(&self.pool)
            .await?;
        }

        // 3. Actualizar users.last_ip para acceso rápido en admin.
        sqlx::query("UPDATE users SET last_ip = ? WHERE id = ?")
            .bind(ip)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(is_new)
    }

    pub async fn list_user_ips(&self, user_id: i64) -> Result<Vec<UserIpHistory>> {
        let rows = sqlx::query_as::<_, UserIpHistory>(
            "SELECT id, user_id, ip, first_seen, last_seen, times_used
             FROM user_ip_history
             WHERE user_id = ?
             ORDER BY last_seen DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

}

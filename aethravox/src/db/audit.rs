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
    pub async fn audit(
        &self,
        user_id: Option<i64>,
        action: &str,
        detail: Option<&str>,
        ip: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO audit_log (user_id, action, detail, ip) VALUES (?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(action)
        .bind(detail)
        .bind(ip)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_audit_entries(&self, limit: i64, offset: i64) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as::<_, AuditEntry>(
            "SELECT id, user_id, action, detail, ip, created_at
             FROM audit_log
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn count_audit_entries(&self) -> Result<i64> {
        let (c,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&self.pool)
            .await?;
        Ok(c)
    }

}

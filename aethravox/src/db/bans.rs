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
    pub async fn create_ban(
        &self,
        user_id: i64,
        banned_by: i64,
        reason: Option<&str>,
        permanent: bool,
        duration_hours: Option<i64>,
    ) -> Result<i64> {
        let expires_at = if permanent {
            None
        } else if let Some(h) = duration_hours.filter(|&h| h > 0) {
            Some(Utc::now() + Duration::hours(h))
        } else {
            Some(Utc::now() + Duration::hours(24))
        };

        let res = sqlx::query(
            "INSERT INTO user_bans (user_id, banned_by, reason, expires_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(banned_by)
        .bind(reason)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn create_suspension(
        &self,
        user_id: i64,
        suspended_by: i64,
        reason: Option<&str>,
        duration_hours: i64,
    ) -> Result<i64> {
        let expires_at = if duration_hours > 0 {
            Utc::now() + Duration::hours(duration_hours)
        } else {
            Utc::now() + Duration::hours(1)
        };
        let res = sqlx::query(
            "INSERT INTO user_suspensions (user_id, suspended_by, reason, expires_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(suspended_by)
        .bind(reason)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn is_user_banned(&self, user_id: i64) -> Result<Option<BanInfo>> {
        let row = sqlx::query_as::<_, BanInfo>(
            "SELECT id, user_id, banned_by, reason, created_at, expires_at, revoked_at
             FROM user_bans
             WHERE user_id = ? AND revoked_at IS NULL
               AND (expires_at IS NULL OR expires_at > ?)
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn is_user_suspended(&self, user_id: i64) -> Result<Option<SuspensionInfo>> {
        let row = sqlx::query_as::<_, SuspensionInfo>(
            "SELECT id, user_id, suspended_by, reason, expires_at, created_at, revoked_at
             FROM user_suspensions
             WHERE user_id = ? AND revoked_at IS NULL AND expires_at > ?
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

}

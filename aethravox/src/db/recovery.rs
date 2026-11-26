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
    pub async fn create_recovery_token(&self, user_id: i64) -> Result<String> {
        let token = random_hex(32);
        let expires = Utc::now() + Duration::hours(1);
        sqlx::query("INSERT INTO recovery_tokens (token, user_id, expires_at) VALUES (?, ?, ?)")
            .bind(&token)
            .bind(user_id)
            .bind(expires)
            .execute(&self.pool)
            .await?;
        Ok(token)
    }

    pub async fn consume_recovery_token(&self, token: &str) -> Result<Option<i64>> {
        let row: Option<(i64, DateTime<Utc>, i64)> = sqlx::query_as(
            "SELECT user_id, expires_at, used FROM recovery_tokens WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((user_id, expires, used)) if used == 0 && expires > Utc::now() => {
                sqlx::query("UPDATE recovery_tokens SET used = 1 WHERE token = ?")
                    .bind(token)
                    .execute(&self.pool)
                    .await?;
                Ok(Some(user_id))
            }
            _ => Ok(None),
        }
    }

}

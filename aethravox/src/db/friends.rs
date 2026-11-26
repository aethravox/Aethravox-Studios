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
    pub async fn send_friend_request(&self, user_id: i64, friend_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO friends (user_id, friend_id, status) VALUES (?, ?, 'pending')")
            .bind(user_id)
            .bind(friend_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn accept_friend_request(&self, accepter_id: i64, requester_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE friends SET status = 'accepted', accepted_at = ?
             WHERE user_id = ? AND friend_id = ? AND status = 'pending'",
        )
        .bind(Utc::now())
        .bind(requester_id)
        .bind(accepter_id)
        .execute(&self.pool)
        .await?;

        // Fila inversa para bidireccionalidad (INSERT OR IGNORE evita duplicados).
        sqlx::query(
            "INSERT OR IGNORE INTO friends (user_id, friend_id, status, accepted_at)
             VALUES (?, ?, 'accepted', ?)",
        )
        .bind(accepter_id)
        .bind(requester_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn reject_friend_request(&self, receiver_id: i64, requester_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM friends WHERE user_id = ? AND friend_id = ? AND status = 'pending'")
            .bind(requester_id)
            .bind(receiver_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_friend(&self, user_id: i64, friend_id: i64) -> Result<()> {
        sqlx::query(
            "DELETE FROM friends
             WHERE (user_id = ? AND friend_id = ?)
                OR (user_id = ? AND friend_id = ?)",
        )
        .bind(user_id)
        .bind(friend_id)
        .bind(friend_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_friends(&self, user_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT friend_id FROM friends WHERE user_id = ? AND status = 'accepted'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn list_pending_requests(&self, user_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT user_id FROM friends WHERE friend_id = ? AND status = 'pending'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn are_friends(&self, user_id: i64, other_id: i64) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM friends
             WHERE user_id = ? AND friend_id = ? AND status = 'accepted' LIMIT 1",
        )
        .bind(user_id)
        .bind(other_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

}

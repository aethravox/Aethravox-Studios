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
    pub async fn create_community(&self, name: &str, description: Option<&str>, icon: Option<&str>, owner_id: i64) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO communities (name, description, icon, owner_id) VALUES (?, ?, ?, ?)"
        )
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(owner_id)
        .execute(&self.pool)
        .await?;
        let id = res.last_insert_rowid();
        // Owner auto-joins
        let _ = self.join_community(id, owner_id).await;
        Ok(id)
    }

    pub async fn join_community(&self, community_id: i64, user_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO community_members (community_id, user_id) VALUES (?, ?)")
            .bind(community_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn leave_community(&self, community_id: i64, user_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM community_members WHERE community_id = ? AND user_id = ?")
            .bind(community_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn is_community_member(&self, community_id: i64, user_id: i64) -> bool {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM community_members WHERE community_id = ? AND user_id = ?"
        )
        .bind(community_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();
        row.is_some()
    }

    pub async fn list_community_members(&self, community_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT user_id FROM community_members WHERE community_id = ?"
        )
        .bind(community_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn list_user_communities(&self, user_id: i64) -> Result<Vec<Community>> {
        let rows = sqlx::query_as::<_, Community>(
            "SELECT c.id, c.name, c.description, c.icon, c.image_url, c.owner_id, c.created_at, c.is_active FROM communities c
             JOIN community_members m ON m.community_id = c.id
             WHERE m.user_id = ? AND c.is_active = 1
             ORDER BY c.name ASC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Lista TODAS las comunidades activas (sin filtro de membresía). Lo usa
    /// el invitado (sin sesión) en /communities: como no es miembro de ninguna,
    /// list_user_communities(0) devolvería vacío. El invitado ve todas las
    /// públicas (activas) en modo solo lectura; unirse requiere cuenta
    /// (/api/communities/:id/join está tras require_auth).
    pub async fn list_all_communities(&self) -> Result<Vec<Community>> {
        let rows = sqlx::query_as::<_, Community>(
            "SELECT c.id, c.name, c.description, c.icon, c.image_url, c.owner_id, c.created_at, c.is_active FROM communities c
             WHERE c.is_active = 1
             ORDER BY c.name ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_community(&self, id: i64) -> Result<Option<Community>> {
        let row = sqlx::query_as::<_, Community>(
            "SELECT * FROM communities WHERE id = ? AND is_active = 1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn deactivate_community(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE communities SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_community_conversations(&self, community_id: i64) -> Result<Vec<Conversation>> {
        let rows = sqlx::query_as::<_, Conversation>(
            "SELECT id, type, name, created_by, created_at, description, image_url, owner_id, community_id
             FROM conversations
             WHERE community_id = ?
             ORDER BY created_at ASC"
        )
        .bind(community_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn create_community_group(&self, community_id: i64, name: &str, description: Option<&str>, created_by: i64) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO conversations (type, name, description, created_by, owner_id, community_id)
             VALUES ('group', ?, ?, ?, ?, ?)"
        )
        .bind(name)
        .bind(description)
        .bind(created_by)
        .bind(created_by)
        .bind(community_id)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

}

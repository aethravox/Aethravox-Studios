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
    pub async fn list_all_webs(&self) -> Result<Vec<Web>> {
        let rows = sqlx::query_as::<_, Web>(
            "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
             FROM webs ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_webs_for_role(&self, role: &str) -> Result<Vec<Web>> {
        // Solo secciones ACTIVAS (is_active=1) aparecen en el hub.
        // El invitado ('invitado') NO puede tener filas en web_role_perms: su
        // CHECK restringe role a owner/admin/staff/usuario. Su visibilidad vive
        // en role_web_perms (sembrada en migración 036), así que la leemos vía
        // role_key (JOIN roles). Los system roles siguen usando web_role_perms.
        if role == "invitado" {
            let rows = sqlx::query_as::<_, Web>(
                "SELECT w.id, w.slug, w.name, w.description, w.icon, w.url, w.is_public, w.created_at, w.tag, w.tag_message, w.tag_opens_at, w.is_active, w.is_custom, w.html_content
                 FROM webs w
                 JOIN role_web_perms rwp ON rwp.web_id = w.id
                 JOIN roles r ON r.id = rwp.role_id
                 WHERE r.role_key = 'invitado' AND w.is_active = 1
                 ORDER BY w.id ASC",
            )
            .fetch_all(&self.pool)
            .await?;
            return Ok(rows);
        }
        let rows = sqlx::query_as::<_, Web>(
            "SELECT w.id, w.slug, w.name, w.description, w.icon, w.url, w.is_public, w.created_at, w.tag, w.tag_message, w.tag_opens_at, w.is_active, w.is_custom, w.html_content
             FROM webs w
             JOIN web_role_perms p ON p.web_id = w.id
             WHERE p.role = ? AND w.is_active = 1
             ORDER BY w.id ASC",
        )
        .bind(role)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_webs_for_user(&self, role: &str) -> Result<Vec<Web>> {
        if role == "owner" {
            return sqlx::query_as::<_, Web>(
                "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
                 FROM webs WHERE is_active = 1 ORDER BY id ASC",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(Into::into);
        }
        self.list_webs_for_role(role).await
    }

    pub async fn list_webs_for_user_obj(&self, user: &User) -> Result<Vec<Web>> {
        if user.role == "owner" {
            return self.list_webs_for_user("owner").await;
        }
        if user.id == 0 {
            return self.list_webs_for_role("invitado").await;
        }
        if let Some(rid) = user.role_id {
            let meta: Option<(String, i64)> = sqlx::query_as(
                "SELECT r.role_key,
                        (SELECT COUNT(*) FROM role_web_perms rwp WHERE rwp.role_id = r.id)
                 FROM roles r WHERE r.id = ?",
            )
            .bind(rid)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((role_key, cnt)) = meta {
                if role_key.starts_with("custom_") || cnt > 0 {
                    let rows = sqlx::query_as::<_, Web>(
                        "SELECT w.id, w.slug, w.name, w.description, w.icon, w.url, w.is_public, w.created_at, w.tag, w.tag_message, w.tag_opens_at, w.is_active, w.is_custom, w.html_content
                         FROM webs w
                         JOIN role_web_perms rwp ON rwp.web_id = w.id
                         WHERE rwp.role_id = ? AND w.is_active = 1
                         ORDER BY w.id ASC",
                    )
                    .bind(rid)
                    .fetch_all(&self.pool)
                    .await?;
                    return Ok(rows);
                }
            }
        }
        self.list_webs_for_role(&user.role).await
    }

    pub async fn user_obj_can_access_web(&self, user: &User, web_id: i64) -> Result<bool> {
        if user.role == "owner" {
            return Ok(true);
        }
        if user.id == 0 {
            let row: Option<(i64,)> = sqlx::query_as(
                "SELECT 1 FROM role_web_perms rwp
                 JOIN roles r ON r.id = rwp.role_id
                 WHERE r.role_key = 'invitado' AND rwp.web_id = ? LIMIT 1",
            )
            .bind(web_id)
            .fetch_optional(&self.pool)
            .await?;
            return Ok(row.is_some());
        }
        if let Some(rid) = user.role_id {
            let meta: Option<(String, i64)> = sqlx::query_as(
                "SELECT r.role_key,
                        (SELECT COUNT(*) FROM role_web_perms rwp WHERE rwp.role_id = r.id)
                 FROM roles r WHERE r.id = ?",
            )
            .bind(rid)
            .fetch_optional(&self.pool)
            .await?;
            if let Some((role_key, cnt)) = meta {
                if role_key.starts_with("custom_") || cnt > 0 {
                    let row: Option<(i64,)> = sqlx::query_as(
                        "SELECT 1 FROM role_web_perms WHERE role_id = ? AND web_id = ? LIMIT 1",
                    )
                    .bind(rid)
                    .bind(web_id)
                    .fetch_optional(&self.pool)
                    .await?;
                    return Ok(row.is_some());
                }
            }
        }
        self.user_can_access_web(&user.role, web_id).await
    }

    pub async fn create_web(
        &self,
        slug: &str,
        name: &str,
        description: Option<&str>,
        icon: Option<&str>,
        url: &str,
        is_public: bool,
        is_custom: bool,
        html_content: Option<&str>,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO webs (slug, name, description, icon, url, is_public, is_custom, html_content)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(slug)
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(url)
        .bind(if is_public { 1 } else { 0 })
        .bind(if is_custom { 1 } else { 0 })
        .bind(html_content)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn delete_web(&self, web_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM webs WHERE id = ?")
            .bind(web_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_web_by_id(&self, web_id: i64) -> Result<Option<Web>> {
        let row = sqlx::query_as::<_, Web>(
            "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
             FROM webs WHERE id = ?",
        )
        .bind(web_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_web_active(&self, web_id: i64, active: bool) -> Result<()> {
        sqlx::query("UPDATE webs SET is_active = ? WHERE id = ?")
            .bind(if active { 1 } else { 0 })
            .bind(web_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_web_by_slug(&self, slug: &str) -> Result<Option<Web>> {
        let row = sqlx::query_as::<_, Web>(
            "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
             FROM webs
             WHERE slug = ? AND is_active = 1
             LIMIT 1",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn user_can_access_web(&self, role: &str, web_id: i64) -> Result<bool> {
        if role == "owner" {
            return Ok(true);
        }
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1
             FROM webs w
             LEFT JOIN web_role_perms rp ON rp.web_id = w.id AND rp.role = ?
             WHERE w.id = ? AND rp.web_id IS NOT NULL
             LIMIT 1",
        )
        .bind(role)
        .bind(web_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    pub async fn update_web_html(&self, web_id: i64, html: &str) -> Result<()> {
        sqlx::query("UPDATE webs SET html_content = ? WHERE id = ?")
            .bind(html)
            .bind(web_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_web_perms(&self, web_id: i64, roles: &[&str]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM web_role_perms WHERE web_id = ?")
            .bind(web_id)
            .execute(&mut *tx)
            .await?;
        for role in roles {
            sqlx::query("INSERT INTO web_role_perms (web_id, role) VALUES (?, ?)")
                .bind(web_id)
                .bind(role)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn web_perms(&self, web_id: i64) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT role FROM web_role_perms WHERE web_id = ?")
            .bind(web_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|(r,)| r).collect())
    }

}

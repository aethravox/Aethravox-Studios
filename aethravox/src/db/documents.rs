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
    pub async fn create_document(
        &self,
        title: &str,
        description: Option<&str>,
        file_name: &str,
        file_path: &str,
        file_type: &str,
        file_size: i64,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO documents (title, description, file_name, file_path, file_type, file_size)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(title)
        .bind(description)
        .bind(file_name)
        .bind(file_path)
        .bind(file_type)
        .bind(file_size)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn list_documents(&self) -> Result<Vec<Document>> {
        let rows = sqlx::query_as::<_, Document>(
            "SELECT id, title, description, file_name, file_path, file_type, file_size,
                    preview_path, preview_status, created_at, updated_at, downloaded
             FROM documents ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_document(&self, id: i64) -> Result<Option<Document>> {
        let row = sqlx::query_as::<_, Document>(
            "SELECT id, title, description, file_name, file_path, file_type, file_size,
                    preview_path, preview_status, created_at, updated_at, downloaded
             FROM documents WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_document(
        &self,
        id: i64,
        title: &str,
        description: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE documents SET title = ?, description = ?, updated_at = ? WHERE id = ?",
        )
        .bind(title)
        .bind(description)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_document_preview(
        &self,
        id: i64,
        preview_path: Option<&str>,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE documents SET preview_path = ?, preview_status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(preview_path)
        .bind(status)
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn increment_document_downloads(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE documents SET downloaded = downloaded + 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_document(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_document_by_uuid(&self, uuid: &str) -> Result<Option<Document>> {
        let row = sqlx::query_as::<_, Document>(
            "SELECT id, title, description, file_name, file_path, file_type, file_size,
                    preview_path, preview_status, created_at, updated_at, downloaded,
                    public_uuid, public_enabled, public_can_download
             FROM documents WHERE public_uuid = ? AND public_enabled = 1",
        )
        .bind(uuid)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn record_document_view(&self, doc_id: i64, viewer_id: Option<i64>, ip: Option<&str>) -> Result<()> {
        sqlx::query("INSERT INTO document_views (document_id, viewer_id, viewer_ip) VALUES (?, ?, ?)")
            .bind(doc_id)
            .bind(viewer_id)
            .bind(ip)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

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
    pub async fn list_titles(&self) -> Result<Vec<Title>> {
        let rows = sqlx::query_as::<_, Title>(
            "SELECT id, name, color, effect, description, created_at
             FROM titles ORDER BY name COLLATE NOCASE ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_title(&self, id: i64) -> Result<Option<Title>> {
        let row = sqlx::query_as::<_, Title>(
            "SELECT id, name, color, effect, description, created_at
             FROM titles WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn create_title(
        &self,
        name: &str,
        color: Option<&str>,
        effect: &str,
        description: Option<&str>,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO titles (name, color, effect, description) VALUES (?, ?, ?, ?)",
        )
        .bind(name)
        .bind(color)
        .bind(effect)
        .bind(description)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn update_title(
        &self,
        id: i64,
        name: &str,
        color: Option<&str>,
        effect: &str,
        description: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE titles SET name = ?, color = ?, effect = ?, description = ? WHERE id = ?",
        )
        .bind(name)
        .bind(color)
        .bind(effect)
        .bind(description)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_title(&self, id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE users SET primary_title_id = NULL WHERE primary_title_id = ?")
            .bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_1 = NULL WHERE visible_title_1 = ?")
            .bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_2 = NULL WHERE visible_title_2 = ?")
            .bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_3 = NULL WHERE visible_title_3 = ?")
            .bind(id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM titles WHERE id = ?")
            .bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_user_titles(&self, user_id: i64) -> Result<Vec<Title>> {
        let rows = sqlx::query_as::<_, Title>(
            "SELECT t.id, t.name, t.color, t.effect, t.description, t.created_at
             FROM titles t
             JOIN user_titles ut ON ut.title_id = t.id
             WHERE ut.user_id = ?
             ORDER BY t.name COLLATE NOCASE ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn user_has_title(&self, user_id: i64, title_id: i64) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM user_titles WHERE user_id = ? AND title_id = ?",
        )
        .bind(user_id)
        .bind(title_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    pub async fn assign_title(&self, user_id: i64, title_id: i64, granted_by: i64) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO user_titles (user_id, title_id, granted_by) VALUES (?, ?, ?)",
        )
        .bind(user_id)
        .bind(title_id)
        .bind(granted_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_title(&self, user_id: i64, title_id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM user_titles WHERE user_id = ? AND title_id = ?")
            .bind(user_id).bind(title_id).execute(&mut *tx).await?;
        // Si era el principal o visible, limpiar esas referencias.
        sqlx::query("UPDATE users SET primary_title_id = NULL WHERE id = ? AND primary_title_id = ?")
            .bind(user_id).bind(title_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_1 = NULL WHERE id = ? AND visible_title_1 = ?")
            .bind(user_id).bind(title_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_2 = NULL WHERE id = ? AND visible_title_2 = ?")
            .bind(user_id).bind(title_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET visible_title_3 = NULL WHERE id = ? AND visible_title_3 = ?")
            .bind(user_id).bind(title_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_primary_title(&self, user_id: i64) -> Result<Option<Title>> {
        let row = sqlx::query_as::<_, Title>(
            "SELECT t.id, t.name, t.color, t.effect, t.description, t.created_at
             FROM titles t
             JOIN users u ON u.primary_title_id = t.id
             JOIN user_titles ut ON ut.user_id = u.id AND ut.title_id = t.id
             WHERE u.id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_user_title_selection(
        &self,
        user_id: i64,
        primary: Option<i64>,
        v1: Option<i64>,
        v2: Option<i64>,
        v3: Option<i64>,
    ) -> Result<()> {
        // Validar que todos los IDs no-None estén en user_titles del usuario.
        for tid in [primary, v1, v2, v3].into_iter().flatten() {
            if !self.user_has_title(user_id, tid).await? {
                return Err(anyhow::anyhow!("título no asignado al usuario"));
            }
        }
        // El principal debe ser uno de los visibles (si hay visibles).
        if let Some(p) = primary {
            let in_visible = [v1, v2, v3].into_iter().flatten().any(|x| x == p);
            if !in_visible && (v1.is_some() || v2.is_some() || v3.is_some()) {
                return Err(anyhow::anyhow!("el título principal debe estar entre los visibles"));
            }
        }
        sqlx::query(
            "UPDATE users SET primary_title_id = ?, visible_title_1 = ?, visible_title_2 = ?, visible_title_3 = ? WHERE id = ?",
        )
        .bind(primary).bind(v1).bind(v2).bind(v3).bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

}

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
    pub async fn create_conversation(&self, conv_type: &str, name: Option<&str>, created_by: i64) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO conversations (type, name, created_by, owner_id) VALUES (?, ?, ?, ?)"
        )
        .bind(conv_type)
        .bind(name)
        .bind(created_by)
        .bind(created_by)  // owner_id = created_by por defecto
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn create_group(&self, name: &str, description: Option<&str>, created_by: i64) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO conversations (type, name, description, created_by, owner_id) VALUES ('group', ?, ?, ?, ?)"
        )
        .bind(name)
        .bind(description)
        .bind(created_by)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn add_conversation_member(&self, conv_id: i64, user_id: i64) -> Result<()> {
        sqlx::query("INSERT OR IGNORE INTO conversation_members (conversation_id, user_id) VALUES (?, ?)")
            .bind(conv_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn is_conversation_member(&self, conv_id: i64, user_id: i64) -> bool {
        let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM conversation_members WHERE conversation_id = ? AND user_id = ?")
            .bind(conv_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();
        row.is_some()
    }

    pub async fn get_conversation(&self, id: i64) -> Result<Option<Conversation>> {
        let row = sqlx::query_as::<_, Conversation>("SELECT id, type, name, created_by, created_at, description, image_url, owner_id, community_id FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn list_user_conversations(&self, user_id: i64) -> Result<Vec<Conversation>> {
        // Solo conversaciones que tengan AL MENOS un mensaje enviado.
        // Ordenadas por el último mensaje (más reciente primero), tipo WhatsApp.
        // Una conversación recién creada (DM o grupo) SIN mensajes NO aparece aquí
        // hasta que se envíe el primer mensaje.
        let rows = sqlx::query_as::<_, Conversation>(
            "SELECT c.id, c.type, c.name, c.created_by, c.created_at, c.description, c.image_url, c.owner_id, c.community_id
             FROM conversations c
             JOIN conversation_members cm ON cm.conversation_id = c.id
             WHERE cm.user_id = ?
               AND EXISTS (SELECT 1 FROM messages m WHERE m.conversation_id = c.id)
             ORDER BY (SELECT MAX(m.created_at) FROM messages m WHERE m.conversation_id = c.id) DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_conversation_members(&self, conv_id: i64) -> Result<Vec<i64>> {
        let rows: Vec<(i64,)> = sqlx::query_as("SELECT user_id FROM conversation_members WHERE conversation_id = ?")
            .bind(conv_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn send_message(&self, conv_id: i64, sender_id: i64, body: &str) -> Result<i64> {
        let res = sqlx::query("INSERT INTO messages (conversation_id, sender_id, body) VALUES (?, ?, ?)")
            .bind(conv_id)
            .bind(sender_id)
            .bind(body)
            .execute(&self.pool)
            .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn list_messages(&self, conv_id: i64, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query_as::<_, Message>(
            "SELECT id, conversation_id, sender_id, body, created_at, status, edited_at, deleted_at FROM messages
             WHERE conversation_id = ? ORDER BY created_at ASC LIMIT ?"
        )
        .bind(conv_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_last_message(&self, conv_id: i64) -> Result<Option<Message>> {
        let row = sqlx::query_as::<_, Message>(
            "SELECT id, conversation_id, sender_id, body, created_at, status, edited_at, deleted_at FROM messages
             WHERE conversation_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(conv_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_message(&self, msg_id: i64) -> Result<Option<Message>> {
        let row = sqlx::query_as::<_, Message>(
            "SELECT id, conversation_id, sender_id, body, created_at, status, edited_at, deleted_at FROM messages
             WHERE id = ?"
        )
        .bind(msg_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn edit_message(&self, msg_id: i64, sender_id: i64, new_body: &str) -> Result<bool> {
        // Validar en la DB: solo autor + dentro de 15 min + no borrado.
        let res = sqlx::query(
            "UPDATE messages
             SET body = ?, edited_at = ?
             WHERE id = ?
               AND sender_id = ?
               AND deleted_at IS NULL
               AND created_at >= datetime('now', '-15 minutes')"
            )
            .bind(new_body)
            .bind(Utc::now())
            .bind(msg_id)
            .bind(sender_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn soft_delete_message(&self, msg_id: i64, sender_id: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE messages
             SET deleted_at = ?, body = ''
             WHERE id = ?
               AND sender_id = ?
               AND deleted_at IS NULL
               AND created_at >= datetime('now', '-30 minutes')"
            )
            .bind(Utc::now())
            .bind(msg_id)
            .bind(sender_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn soft_delete_message_admin(&self, msg_id: i64) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE messages
             SET deleted_at = ?, body = ''
             WHERE id = ? AND deleted_at IS NULL"
            )
            .bind(Utc::now())
            .bind(msg_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn mark_conversation_read(&self, conv_id: i64, user_id: i64) -> Result<()> {
        // Marcar last_read_at del miembro
        sqlx::query("UPDATE conversation_members SET last_read_at = ? WHERE conversation_id = ? AND user_id = ?")
            .bind(Utc::now())
            .bind(conv_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        // Marcar como 'read' todos los mensajes que otros me enviaron en esta conversación
        sqlx::query("UPDATE messages SET status = 'read' WHERE conversation_id = ? AND sender_id != ? AND status != 'read'")
            .bind(conv_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_messages_delivered(&self, conv_id: i64) -> Result<()> {
        sqlx::query("UPDATE messages SET status = 'delivered' WHERE conversation_id = ? AND status = 'sent'")
            .bind(conv_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_unread_in_conversation(&self, conv_id: i64, user_id: i64) -> Result<i64> {
        let member: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT last_read_at FROM conversation_members WHERE conversation_id = ? AND user_id = ?"
        )
        .bind(conv_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let last_read = member.and_then(|(lr,)| lr).unwrap_or_else(|| Utc::now() - Duration::days(365));

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ? AND created_at > ? AND sender_id != ?"
        )
        .bind(conv_id)
        .bind(last_read)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_total_unread_for_user(&self, user_id: i64) -> Result<i64> {
        // Si el usuario nunca marcó como leído (last_read_at NULL), tomamos una
        // fecha muy antigua para que cuenten todos los mensajes previos.
        let epoch = Utc::now() - Duration::days(365 * 50);
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages m
             JOIN conversation_members cm
               ON cm.conversation_id = m.conversation_id AND cm.user_id = ?
             WHERE m.sender_id != ?
               AND m.created_at > COALESCE(cm.last_read_at, ?)"
        )
        .bind(user_id)
        .bind(user_id)
        .bind(epoch)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn count_community_unread_for_user(&self, user_id: i64) -> Result<i64> {
        let epoch = Utc::now() - Duration::days(365 * 50);
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM messages m
             JOIN conversation_members cm
               ON cm.conversation_id = m.conversation_id AND cm.user_id = ?
             JOIN conversations c ON c.id = m.conversation_id
             WHERE m.sender_id != ?
               AND c.community_id IS NOT NULL
               AND m.created_at > COALESCE(cm.last_read_at, ?)"
        )
        .bind(user_id)
        .bind(user_id)
        .bind(epoch)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn find_dm(&self, user1: i64, user2: i64) -> Result<Option<Conversation>> {
        let row = sqlx::query_as::<_, Conversation>(
            "SELECT c.id, c.type, c.name, c.created_by, c.created_at, c.description, c.image_url, c.owner_id, c.community_id FROM conversations c
             WHERE c.type = 'dm'
             AND (SELECT COUNT(*) FROM conversation_members WHERE conversation_id = c.id) = 2
             AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = ?)
             AND EXISTS (SELECT 1 FROM conversation_members WHERE conversation_id = c.id AND user_id = ?)
             LIMIT 1"
        )
        .bind(user1)
        .bind(user2)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn delete_conversation(&self, conv_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(conv_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_online_user_ids(&self) -> Result<Vec<i64>> {
        let cutoff = Utc::now() - Duration::minutes(2);
        let now = Utc::now();
        let rows: Vec<(i64,)> = sqlx::query_as(
            "SELECT DISTINCT u.id
             FROM users u
             JOIN sessions s ON s.user_id = u.id
             WHERE u.status = 'active'
               AND s.expires_at > ?
               AND s.last_seen_at > ?
             ORDER BY u.id ASC",
        )
        .bind(now)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

}

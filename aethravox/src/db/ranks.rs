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
    pub async fn create_default_ranks(&self, conv_id: i64, owner_user_id: i64) -> Result<()> {
        // Owner: todos los permisos
        let owner_rank = sqlx::query(
            "INSERT INTO conversation_ranks (conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership)
             VALUES (?, 'Owner', '#e0a308', 100, 1, 1,1,1,1,1,1,1,1,1)"
        )
        .bind(conv_id)
        .execute(&self.pool)
        .await?;
        let owner_rank_id = owner_rank.last_insert_rowid();

        // Admin: casi todos excepto transfer_ownership
        sqlx::query(
            "INSERT INTO conversation_ranks (conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership)
             VALUES (?, 'Admin', '#00d4ff', 50, 0, 1,1,1,1,1,1,1,1,0)"
        )
        .bind(conv_id)
        .execute(&self.pool)
        .await?;

        // Miembro: básico
        sqlx::query(
            "INSERT INTO conversation_ranks (conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership)
             VALUES (?, 'Miembro', '#888888', 10, 0, 1,0,0,0,0,0,0,0,0)"
        )
        .bind(conv_id)
        .execute(&self.pool)
        .await?;

        // Asignar Owner al creador
        sqlx::query("UPDATE conversation_members SET rank_id = ? WHERE conversation_id = ? AND user_id = ?")
            .bind(owner_rank_id)
            .bind(conv_id)
            .bind(owner_user_id)
            .execute(&self.pool)
            .await?;

        // Asignar Miembro a los demás miembros
        let miembro_rank: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM conversation_ranks WHERE conversation_id = ? AND name = 'Miembro'"
        )
        .bind(conv_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some((miembro_id,)) = miembro_rank {
            sqlx::query("UPDATE conversation_members SET rank_id = ? WHERE conversation_id = ? AND rank_id IS NULL")
                .bind(miembro_id)
                .bind(conv_id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    pub async fn list_conversation_ranks(&self, conv_id: i64) -> Result<Vec<ConversationRank>> {
        let rows = sqlx::query_as::<_, ConversationRank>(
            "SELECT id, conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership
             FROM conversation_ranks WHERE conversation_id = ? ORDER BY position DESC"
        )
        .bind(conv_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_member_rank(&self, conv_id: i64, user_id: i64) -> Result<Option<ConversationRank>> {
        // Primero obtener rank_id del miembro
        let rank_id: Option<(Option<i64>,)> = sqlx::query_as(
            "SELECT rank_id FROM conversation_members WHERE conversation_id = ? AND user_id = ?"
        )
        .bind(conv_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((Some(rid),)) = rank_id else { return Ok(None); };

        let row = sqlx::query_as::<_, ConversationRank>(
            "SELECT id, conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership
             FROM conversation_ranks WHERE id = ?"
        )
        .bind(rid)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_member_rank(&self, conv_id: i64, user_id: i64, rank_id: i64) -> Result<()> {
        sqlx::query("UPDATE conversation_members SET rank_id = ? WHERE conversation_id = ? AND user_id = ?")
            .bind(rank_id)
            .bind(conv_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_rank(&self, conv_id: i64, name: &str, color: Option<&str>,
        can_send: bool, can_add: bool, can_edit_info: bool, can_delete: bool,
        can_remove: bool, can_manage_ranks: bool, can_assign: bool, can_pin: bool, can_transfer: bool,
    ) -> Result<i64> {
        let res = sqlx::query(
            "INSERT INTO conversation_ranks (conversation_id, name, color, position, is_owner_rank,
             can_send_messages, can_add_members, can_edit_group_info, can_delete_messages,
             can_remove_members, can_manage_ranks, can_assign_ranks, can_pin_messages, can_transfer_ownership)
             VALUES (?, ?, ?, 0, 0, ?,?,?,?,?,?,?,?,?)"
        )
        .bind(conv_id)
        .bind(name)
        .bind(color)
        .bind(if can_send {1} else {0})
        .bind(if can_add {1} else {0})
        .bind(if can_edit_info {1} else {0})
        .bind(if can_delete {1} else {0})
        .bind(if can_remove {1} else {0})
        .bind(if can_manage_ranks {1} else {0})
        .bind(if can_assign {1} else {0})
        .bind(if can_pin {1} else {0})
        .bind(if can_transfer {1} else {0})
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn update_rank(&self, rank_id: i64, name: &str, color: Option<&str>,
        can_send: bool, can_add: bool, can_edit_info: bool, can_delete: bool,
        can_remove: bool, can_manage_ranks: bool, can_assign: bool, can_pin: bool, can_transfer: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE conversation_ranks SET name=?, color=?, can_send_messages=?, can_add_members=?,
             can_edit_group_info=?, can_delete_messages=?, can_remove_members=?, can_manage_ranks=?,
             can_assign_ranks=?, can_pin_messages=?, can_transfer_ownership=? WHERE id=?"
        )
        .bind(name)
        .bind(color)
        .bind(if can_send {1} else {0})
        .bind(if can_add {1} else {0})
        .bind(if can_edit_info {1} else {0})
        .bind(if can_delete {1} else {0})
        .bind(if can_remove {1} else {0})
        .bind(if can_manage_ranks {1} else {0})
        .bind(if can_assign {1} else {0})
        .bind(if can_pin {1} else {0})
        .bind(if can_transfer {1} else {0})
        .bind(rank_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_rank(&self, rank_id: i64) -> Result<bool> {
        let in_use: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM conversation_members WHERE rank_id = ?"
        )
        .bind(rank_id)
        .fetch_one(&self.pool)
        .await?;
        if in_use.0 > 0 { return Ok(false); }
        sqlx::query("DELETE FROM conversation_ranks WHERE id = ? AND is_owner_rank = 0")
            .bind(rank_id)
            .execute(&self.pool)
            .await?;
        Ok(true)
    }

    pub async fn transfer_ownership(&self, conv_id: i64, new_owner_id: i64) -> Result<()> {
        let owner_rank: (i64,) = sqlx::query_as(
            "SELECT id FROM conversation_ranks WHERE conversation_id = ? AND is_owner_rank = 1"
        )
        .bind(conv_id)
        .fetch_one(&self.pool)
        .await?;
        // Asignar owner_rank al nuevo owner
        sqlx::query("UPDATE conversation_members SET rank_id = ? WHERE conversation_id = ? AND user_id = ?")
            .bind(owner_rank.0)
            .bind(conv_id)
            .bind(new_owner_id)
            .execute(&self.pool)
            .await?;
        // Actualizar conversations.owner_id
        sqlx::query("UPDATE conversations SET owner_id = ? WHERE id = ?")
            .bind(new_owner_id)
            .bind(conv_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

}

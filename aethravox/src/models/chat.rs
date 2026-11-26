#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct Conversation {
    pub id: i64,
    #[sqlx(rename = "type")]
    pub conv_type: String,
    pub name: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    /// Descripción del grupo (NULL para DMs).
    pub description: Option<String>,
    /// URL de la imagen de perfil del grupo (NULL si no tiene).
    pub image_url: Option<String>,
    /// ID del owner actual del grupo (puede cambiar si se transfiere).
    pub owner_id: Option<i64>,
    /// ID de la comunidad a la que pertenece (NULL si es independiente).
    pub community_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConversationMember {
    pub conversation_id: i64,
    pub user_id: i64,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
    pub rank_id: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConversationRank {
    pub id: i64,
    pub conversation_id: i64,
    pub name: String,
    pub color: Option<String>,
    pub position: i64,
    pub is_owner_rank: i64,
    pub can_send_messages: i64,
    pub can_add_members: i64,
    pub can_edit_group_info: i64,
    pub can_delete_messages: i64,
    pub can_remove_members: i64,
    pub can_manage_ranks: i64,
    pub can_assign_ranks: i64,
    pub can_pin_messages: i64,
    pub can_transfer_ownership: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub sender_id: i64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub status: String,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct NewDmForm {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct NewGroupForm {
    pub name: String,
    pub members: String, // comma-separated usernames
    #[serde(default)]
    pub description: Option<String>,
}


#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct Community {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub image_url: Option<String>,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
    pub is_active: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityMember {
    pub community_id: i64,
    pub user_id: i64,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommunityForm {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}


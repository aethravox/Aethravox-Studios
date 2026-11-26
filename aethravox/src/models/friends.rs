#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct Friend {
    pub id: i64,
    pub user_id: i64,
    pub friend_id: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct FriendRequestForm {
    pub username: String,
}


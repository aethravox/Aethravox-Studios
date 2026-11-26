#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct AuditEntry {
    pub id: i64,
    pub user_id: Option<i64>,
    pub action: String,
    pub detail: Option<String>,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}


#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct BanInfo {
    pub id: i64,
    pub user_id: i64,
    pub banned_by: i64,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SuspensionInfo {
    pub id: i64,
    pub user_id: i64,
    pub suspended_by: i64,
    pub reason: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct BanForm {
    pub reason: Option<String>,
    #[serde(default)]
    pub permanent: Option<String>,
    pub duration_value: Option<i64>,
    #[serde(default)]
    pub duration_unit: Option<String>,    // minutes | hours | days | weeks | months
}

#[derive(Debug, Deserialize)]
pub struct SuspendForm {
    pub reason: Option<String>,
    pub duration_value: i64,
    #[serde(default)]
    pub duration_unit: Option<String>,    // minutes | hours | days | weeks | months
}


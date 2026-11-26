#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigForm {
    pub studio_name: Option<String>,
    pub foundation_date: Option<String>,
    #[serde(default)]
    pub allow_public_register: Option<String>,
    pub default_role: Option<String>,
    #[serde(default)]
    pub maintenance_mode: Option<String>,
}


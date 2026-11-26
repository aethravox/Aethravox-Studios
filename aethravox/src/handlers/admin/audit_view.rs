#![allow(unused_imports)]
// Web 2 — Panel de administración (Owner + Admin).

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use tower_cookies::Cookies;

use crate::{
    auth_mw::AuthUser,
    models::{
        ActionOnUser, AdminCreateUserForm, BanForm, ChangeRoleForm, CreateWebForm,
        SuspendForm, UpdateConfigForm, User, WebPermsForm,
    },
    AppState,
};

// ===== Idea 1: re-autenticación para /admin/* =====



pub fn parse_local_datetime_to_utc(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Formato esperado: "2026-11-15T18:00"
    use chrono::NaiveDateTime;
    let ndt = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").ok()?;
    Some(chrono::TimeZone::from_utc_datetime(&chrono::Utc, &ndt))
}

// ===== /admin/audit =====
#[derive(Deserialize)]
pub struct AuditPageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

pub async fn audit(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Query(q): Query<AuditPageQuery>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "audit.view").await {
        return Redirect::to("/admin").into_response();
    }
    let per_page = 50_i64;
    let page = q.page.max(1);
    let offset = (page - 1) * per_page;
    let total = state.db.count_audit_entries().await.unwrap_or(0);
    let entries = state
        .db
        .list_audit_entries(per_page, offset)
        .await
        .unwrap_or_default();

    let mut user_names: HashMap<i64, String> = HashMap::new();
    for e in &entries {
        if let Some(uid) = e.user_id {
            if !user_names.contains_key(&uid) {
                if let Ok(Some(u)) = state.db.find_user_by_id(uid).await {
                    user_names.insert(uid, u.username);
                }
            }
        }
    }

    AdminAuditTemplate {
        actor_username: actor.username.clone(),
        entries: entries
            .iter()
            .map(|e| AuditRow {
                id: e.id,
                user: e
                    .user_id
                    .and_then(|uid| user_names.get(&uid).cloned())
                    .unwrap_or_else(|| "—".into()),
                action: e.action.clone(),
                detail: e.detail.clone().unwrap_or_default(),
                ip: e.ip.clone().unwrap_or_else(|| "—".into()),
                at: e.created_at.format("%d/%m/%Y %H:%M").to_string(),
            })
            .collect(),
        page,
        total_pages: ((total + per_page - 1) / per_page).max(1),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/system/admin_audit.html")]
pub struct AdminAuditTemplate {
    pub actor_username: String,
    pub entries: Vec<AuditRow>,
    pub page: i64,
    pub total_pages: i64,
}

pub struct AuditRow {
    pub id: i64,
    pub user: String,
    pub action: String,
    pub detail: String,
    pub ip: String,
    pub at: String,
}

// ===== /admin/config =====

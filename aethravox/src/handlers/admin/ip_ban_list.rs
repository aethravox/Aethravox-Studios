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



pub async fn ip_bans(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "ip_bans.view").await {
        return Redirect::to("/admin").into_response();
    }

    let active = state.db.list_active_ip_bans().await.unwrap_or_default();
    let history = state.db.list_ip_bans_history(50).await.unwrap_or_default();

    // Mapa de admin_id → username para mostrar quién baneó.
    let mut admin_names: HashMap<i64, String> = HashMap::new();
    for ban in active.iter().chain(history.iter()) {
        if let Some(uid) = ban.banned_by {
            if !admin_names.contains_key(&uid) {
                if let Ok(Some(u)) = state.db.find_user_by_id(uid).await {
                    admin_names.insert(uid, u.username);
                }
            }
        }
    }

    let active_rows: Vec<IpBanRow> = active.iter().map(|b| IpBanRow {
        id: b.id,
        ip: b.ip.clone(),
        reason: b.reason.clone(),
        banned_by_username: b.banned_by.and_then(|id| admin_names.get(&id).cloned()),
        created_at: b.created_at.format("%d/%m/%Y %H:%M").to_string(),
        expires_at: b.expires_at.map(|dt| dt.format("%d/%m/%Y %H:%M").to_string()).unwrap_or_else(|| "Permanente".into()),
        is_permanent: b.expires_at.is_none(),
    }).collect();

    let history_rows: Vec<IpBanRow> = history.iter().map(|b| IpBanRow {
        id: b.id,
        ip: b.ip.clone(),
        reason: b.reason.clone(),
        banned_by_username: b.banned_by.and_then(|id| admin_names.get(&id).cloned()),
        created_at: b.created_at.format("%d/%m/%Y %H:%M").to_string(),
        expires_at: b.expires_at.map(|dt| dt.format("%d/%m/%Y %H:%M").to_string()).unwrap_or_else(|| "Permanente".into()),
        is_permanent: b.expires_at.is_none(),
    }).collect();

    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    IpBansTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        active_count: active_rows.len() as i64,
        active: active_rows,
        history: history_rows,
    }.into_response()
}

#[derive(Template)]
#[template(path = "admin/system/admin_ip_bans.html")]
pub struct IpBansTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub active_count: i64,
    pub active: Vec<IpBanRow>,
    pub history: Vec<IpBanRow>,
}

pub struct IpBanRow {
    pub id: i64,
    pub ip: String,
    pub reason: String,
    pub banned_by_username: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub is_permanent: bool,
}

#[derive(Debug, Deserialize)]
pub struct IpBanForm {
    pub ip: String,
    pub reason: String,
    /// "permanent" o "duration". Si "duration", se requiere duration_value + duration_unit.
    pub ban_type: String,
    pub duration_value: Option<i64>,
    pub duration_unit: Option<String>,  // "minutes" | "hours" | "days" | "weeks" | "months"
}

/// POST /admin/ip_bans/create — Crea un nuevo ban de IP.
pub async fn create_ip_ban(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<IpBanForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "ip_bans.create").await {
        return Redirect::to("/admin").into_response();
    }

    let ip = form.ip.trim().to_string();
    let reason = form.reason.trim().to_string();
    if ip.is_empty() || reason.is_empty() {
        return Redirect::to("/admin/ip_bans?err=empty").into_response();
    }

    let expires_at = match form.ban_type.as_str() {
        "permanent" => None,
        "duration" => {
            let value = form.duration_value.unwrap_or(0);
            let unit = form.duration_unit.unwrap_or_else(|| "hours".into());
            if value <= 0 {
                return Redirect::to("/admin/ip_bans?err=duration").into_response();
            }
            let duration = match unit.as_str() {
                "minutes" => chrono::Duration::minutes(value),
                "hours"   => chrono::Duration::hours(value),
                "days"    => chrono::Duration::days(value),
                "weeks"   => chrono::Duration::days(value * 7),
                "months"  => chrono::Duration::days(value * 30),
                _ => return Redirect::to("/admin/ip_bans?err=unit").into_response(),
            };
            Some(Utc::now() + duration)
        }
        _ => return Redirect::to("/admin/ip_bans?err=type").into_response(),
    };

    let _ = state.db.ban_ip(&ip, &reason, Some(actor.id), expires_at).await;
    let _ = state.db.audit(
        Some(actor.id),
        "ip_ban_created",
        Some(&format!("ip={} reason={} expires={:?}", ip, reason, expires_at)),
        None,
    ).await;

    Redirect::to("/admin/ip_bans?ok=created").into_response()
}

/// POST /admin/ip_bans/:id/revoke — Levanta un ban antes de que expire.
pub async fn revoke_ip_ban(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(ban_id): Path<i64>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "ip_bans.revoke").await {
        return Redirect::to("/admin").into_response();
    }
    let _ = state.db.revoke_ip_ban(ban_id).await;
    let _ = state.db.audit(
        Some(actor.id),
        "ip_ban_revoked",
        Some(&format!("ban_id={}", ban_id)),
        None,
    ).await;
    Redirect::to("/admin/ip_bans?ok=revoked").into_response()
}

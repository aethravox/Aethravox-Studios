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



pub async fn config_get(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "config.edit").await {
        return Redirect::to("/admin").into_response();
    }
    let entries = state.db.list_config().await.unwrap_or_default();
    let map: HashMap<String, String> = entries
        .iter()
        .map(|c| (c.key.clone(), c.value.clone()))
        .collect();

    AdminConfigTemplate {
        actor_username: actor.username.clone(),
        studio_name: map.get("studio_name").cloned().unwrap_or_default(),
        foundation_date: map.get("foundation_date").cloned().unwrap_or_default(),
        allow_public_register: map
            .get("allow_public_register")
            .map(|s| s == "1")
            .unwrap_or(true),
        default_role: map
            .get("default_role")
            .cloned()
            .unwrap_or_else(|| "usuario".into()),
        maintenance_mode: map
            .get("maintenance_mode")
            .map(|s| s == "1")
            .unwrap_or(false),
        info: None,
    }
    .into_response()
}

pub async fn config_post(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<UpdateConfigForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "config.edit").await {
        return Redirect::to("/admin").into_response();
    }

    let studio_name = form
        .studio_name
        .as_deref()
        .unwrap_or("Aethravox Studios")
        .trim();
    let _ = state
        .db
        .set_config("studio_name", studio_name, actor.id)
        .await;
    if let Some(fd) = form.foundation_date.as_deref() {
        let _ = state.db.set_config("foundation_date", fd.trim(), actor.id).await;
    }
    let _ = state
        .db
        .set_config(
            "allow_public_register",
            if form.allow_public_register.is_some() { "1" } else { "0" },
            actor.id,
        )
        .await;
    if let Some(dr) = form.default_role.as_deref() {
        if crate::models::is_role(dr) {
            let _ = state.db.set_config("default_role", dr, actor.id).await;
        }
    }
    let _ = state
        .db
        .set_config(
            "maintenance_mode",
            if form.maintenance_mode.is_some() { "1" } else { "0" },
            actor.id,
        )
        .await;

    let _ = state
        .db
        .audit(Some(actor.id), "config_update", Some("system_config"), None)
        .await;

    let entries = state.db.list_config().await.unwrap_or_default();
    let map: HashMap<String, String> = entries
        .iter()
        .map(|c| (c.key.clone(), c.value.clone()))
        .collect();
    AdminConfigTemplate {
        actor_username: actor.username.clone(),
        studio_name: map.get("studio_name").cloned().unwrap_or_default(),
        foundation_date: map.get("foundation_date").cloned().unwrap_or_default(),
        allow_public_register: map
            .get("allow_public_register")
            .map(|s| s == "1")
            .unwrap_or(true),
        default_role: map
            .get("default_role")
            .cloned()
            .unwrap_or_else(|| "usuario".into()),
        maintenance_mode: map
            .get("maintenance_mode")
            .map(|s| s == "1")
            .unwrap_or(false),
        info: Some("Configuración guardada.".into()),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/system/admin_config.html")]
pub struct AdminConfigTemplate {
    pub actor_username: String,
    pub studio_name: String,
    pub foundation_date: String,
    pub allow_public_register: bool,
    pub default_role: String,
    pub maintenance_mode: bool,
    pub info: Option<String>,
}

pub fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b;
        if c.is_ascii_alphanumeric() || c == b'/' || c == b'-' || c == b'_' || c == b'.' {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{:02X}", c));
        }
    }
    out
}

// ===== IP BANS (Web 7 — Anti-abuso) =====

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



pub async fn index(State(state): State<AppState>, AuthUser(user): AuthUser) -> impl IntoResponse {
    if !state.db.user_has_perm(&user, "admin.access").await {
        return Redirect::to("/hub").into_response();
    }

    let (owner_count, admin_count, staff_count, user_count) =
        state.db.count_users_by_role().await.unwrap_or((0, 0, 0, 0));
    let total_users = owner_count + admin_count + staff_count + user_count;
    let webs = state.db.list_all_webs().await.unwrap_or_default();
    let audit_total = state.db.count_audit_entries().await.unwrap_or(0);
    let visits_7d = state.db.total_visits_in_days(7).await.unwrap_or(0);
    let top_webs_7d = state.db.top_webs_in_days(7, 5).await.unwrap_or_default();

    let (role_css_key, role_display) = state.db.role_display(&user).await;
    AdminIndexTemplate {
        username: user.username.clone(),
        role: user.role.clone(),
        role_display,
        role_css_key,
        total_users,
        owner_count,
        admin_count,
        staff_count,
        user_count,
        web_count: webs.len() as i64,
        audit_total,
        visits_7d,
        top_webs_7d,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/admin_index.html")]
pub struct AdminIndexTemplate {
    pub username: String,
    pub role: String,
    pub role_display: String,
    pub role_css_key: String,
    pub total_users: i64,
    pub owner_count: i64,
    pub admin_count: i64,
    pub staff_count: i64,
    pub user_count: i64,
    pub web_count: i64,
    pub audit_total: i64,
    pub visits_7d: i64,
    pub top_webs_7d: Vec<crate::db::analytics::WebVisitStat>,
}

// ===== /admin/users =====

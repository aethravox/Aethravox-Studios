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



pub async fn users(State(state): State<AppState>, AuthUser(actor): AuthUser) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "users.view").await {
        return Redirect::to("/hub").into_response();
    }

    let users = state.db.list_users().await.unwrap_or_default();
    // IDs de usuarios con sesión activa vista en los últimos 2 min
    // (misma definición que list_online_user_ids en db.rs).
    let online_ids = state.db.list_online_user_ids().await.unwrap_or_default();
    // Roles configurables para el dropdown de asignación de rol.
    let roles = state.db.list_roles().await.unwrap_or_default();
    let actor_id = actor.id;
    let actor_role = actor.role.clone();
    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;

    AdminUsersTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor_role.clone(),
        actor_role_display,
        actor_role_css_key,
        actor_id,
        online_count: online_ids.len() as i64,
        roles: roles
            .iter()
            .map(|r| RoleOption {
                id: r.id,
                name: r.name.clone(),
                role_key: r.role_key.clone(),
                position: r.position,
                is_owner: r.is_owner(),
                selected: false, // se setea por usuario abajo
            })
            .collect(),
        users: users
            .iter()
            .map(|u| {
                let role_options: Vec<RoleOption> = roles
                    .iter()
                    .map(|r| RoleOption {
                        id: r.id,
                        name: r.name.clone(),
                        role_key: r.role_key.clone(),
                        position: r.position,
                        is_owner: r.is_owner(),
                        selected: u.role_id == Some(r.id),
                    })
                    .collect();
                // Resolver (css_key, display_name) para el badge sin un query
                // extra por usuario: buscamos el rol del usuario en `roles`
                // (ya cargados). Si es custom, css_key = role_key real (custom_N)
                // para que matchee con /role-colors.css; display = nombre real.
                let (role_css_key, role_display_name) = if u.role == "owner" {
                    ("owner".to_string(), "Owner".to_string())
                } else if let Some(rid) = u.role_id {
                    if let Some(r) = roles.iter().find(|r| r.id == rid) {
                        let is_system = r.role_key == "user"
                            || r.role_key == "admin"
                            || r.role_key == "staff"
                            || r.role_key == "owner";
                        if !is_system {
                            (r.role_key.clone(), r.name.clone())
                        } else {
                            let disp = match r.role_key.as_str() {
                                "user" => "User".to_string(),
                                "admin" => "Admin".to_string(),
                                "staff" => "Staff".to_string(),
                                other => other.to_string(),
                            };
                            (r.role_key.clone(), disp)
                        }
                    } else {
                        (u.role.clone(), u.role.clone())
                    }
                } else {
                    (u.role.clone(), u.role.clone())
                };
                UserRow {
                    id: u.id,
                    username: u.username.clone(),
                    display: u.display(),
                    role: role_display_name,
                    role_css_key,
                    status: u.status.clone(),
                    email: u.email.clone(),
                    created_at: u.created_at.format("%d/%m/%Y").to_string(),
                    last_login: u
                        .last_login_at
                        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
                        .unwrap_or_else(|| "—".into()),
                    is_self: u.id == actor_id,
                    is_online: online_ids.contains(&u.id),
                    last_ip: u.last_ip.clone(),
                    role_id: u.role_id,
                    role_options,
                }
            })
            .collect(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/users/admin_users.html")]
pub struct AdminUsersTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub actor_id: i64,
    pub online_count: i64,
    pub users: Vec<UserRow>,
    /// Todos los roles configurables (para el dropdown de asignación de rol).
    pub roles: Vec<RoleOption>,
}

/// Opción de rol configurable para el dropdown de un usuario.
pub struct RoleOption {
    pub id: i64,
    pub name: String,
    pub role_key: String,
    pub position: i64,
    pub is_owner: bool,
    pub selected: bool,
}

pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub role: String,
    /// Clave CSS para el badge (role-{{ role_css_key }}). Para roles custom es
    /// el legacy derivado (admin/staff/user) para que coja el color correcto;
    /// `role` lleva el nombre display (p.ej. "Moderador").
    pub role_css_key: String,
    pub status: String,
    pub email: String,
    pub created_at: String,
    pub last_login: String,
    pub is_self: bool,
    pub is_online: bool,
    pub last_ip: Option<String>,
    /// ID del rol configurables del usuario (post-migración 031).
    pub role_id: Option<i64>,
    /// Opciones de rol para el dropdown, con `selected` precomputado.
    pub role_options: Vec<RoleOption>,
}

// ===== /admin/users/new =====
pub async fn new_user_get(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "users.create").await {
        return Redirect::to("/admin/users").into_response();
    }
    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    AdminNewUserTemplate {
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        error: None,
        username: String::new(),
        email: String::new(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/users/admin_new_user.html")]
pub struct AdminNewUserTemplate {
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub error: Option<String>,
    pub username: String,
    pub email: String,
}

pub async fn new_user_post(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<AdminCreateUserForm>,
) -> impl IntoResponse {
    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    let target = User {
        id: 0,
        username: form.username.clone(),
        email: form.email.clone(),
        password_hash: String::new(),
        role: form.role.clone(),
        avatar_url: None,
        created_at: Utc::now(),
        last_login_at: None,
        total_seconds: 0,
        display_name: None,
        bio: None,
        status: "active".into(),
        last_ip: None,
        role_id: None,
        primary_title_id: None,
        visible_title_1: None,
        visible_title_2: None,
        visible_title_3: None,
    };
    if !state.db.user_has_perm(&actor, "users.create").await
        || !actor.can_act_on(&target, ActionOnUser::CreateUser) {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some(format!("No puedes crear usuarios con rol '{}'.", form.role)),
            username: form.username.clone(),
            email: form.email.clone(),
        }
        .into_response();
    }

    if let Err(e) = crate::models::validate_username(&form.username) {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some(e),
            username: form.username,
            email: form.email,
        }
        .into_response();
    }
    if let Err(e) = crate::models::validate_email(&form.email) {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some(e),
            username: form.username,
            email: form.email,
        }
        .into_response();
    }
    if let Err(e) = crate::models::validate_password(&form.password) {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some(e),
            username: form.username,
            email: form.email,
        }
        .into_response();
    }

    if state
        .db
        .find_user_by_username(&form.username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some("Ese nombre de usuario ya está en uso.".into()),
            username: form.username,
            email: form.email,
        }
        .into_response();
    }
    if state
        .db
        .find_user_by_email(&form.email)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return AdminNewUserTemplate {
            actor_role: actor.role.clone(),
            actor_role_display,
            actor_role_css_key,
            error: Some("Ese email ya está registrado.".into()),
            username: form.username,
            email: form.email,
        }
        .into_response();
    }

    match state
        .db
        .create_user(&form.username, &form.email, &form.password)
        .await
    {
        Ok(uid) => {
            if form.role != "usuario" {
                let _ = state.db.set_user_role(uid, &form.role).await;
            }
            let detail = format!("{} ({})", form.username, form.role);
            let _ = state
                .db
                .audit(Some(actor.id), "user_create", Some(&detail), None)
                .await;
            Redirect::to("/admin/users?ok=created").into_response()
        }
        Err(e) => {
            tracing::error!("Error creando usuario: {e}");
            AdminNewUserTemplate {
                actor_role: actor.role.clone(),
                actor_role_display,
                actor_role_css_key,
                error: Some("Error interno al crear la cuenta.".into()),
                username: form.username,
                email: form.email,
            }
            .into_response()
        }
    }
}

// ===== POST /admin/users/:id/role =====

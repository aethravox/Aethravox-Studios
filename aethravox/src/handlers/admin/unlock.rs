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



#[derive(Debug, Deserialize)]
pub struct UnlockQuery {
    #[serde(default)]
    pub next: Option<String>,
}

/// GET /admin/unlock — Formulario para re-meter la contraseña.
/// Solo usuarios logueados con rol owner/admin/staff pueden verlo.
pub async fn unlock_get(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<UnlockQuery>,
) -> impl IntoResponse {
    // El acceso al panel = permiso "admin.access". Owner por bypass;
    // admin/staff por baseline; un usuario común solo si su rol se lo concede.
    if !state.db.user_has_perm(&user, "admin.access").await {
        return Redirect::to("/hub").into_response();
    }
    let next_safe = sanitize_next(&q.next, "/admin");
    let (role_css_key, role_display) = state.db.role_display(&user).await;
    AdminUnlockTemplate {
        username: user.username.clone(),
        role: user.role.clone(),
        role_display,
        role_css_key,
        next: next_safe,
        error: None,
    }
    .into_response()
}

/// POST /admin/unlock — Verifica la contraseña, setea la cookie de desbloqueo.
pub async fn unlock_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    cookies: Cookies,
    Form(form): Form<UnlockForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&user, "admin.access").await {
        return Redirect::to("/hub").into_response();
    }
    let next_safe = sanitize_next(&form.next, "/admin");
    let (role_css_key, role_display) = state.db.role_display(&user).await;

    // Verificar contraseña actual contra el hash del user.
    if !crate::db::verify_password(&form.password, &user.password_hash) {
        let _ = state
            .db
            .audit(Some(user.id), "admin_unlock_failed", None, None)
            .await;
        return AdminUnlockTemplate {
            username: user.username.clone(),
            role: user.role.clone(),
            role_display,
            role_css_key,
            next: next_safe,
            error: Some("Contraseña incorrecta.".into()),
        }
        .into_response();
    }

    // Generar token aleatorio de 32 bytes hex.
    let token = crate::db::random_hex(32);
    let now_epoch = Utc::now().timestamp();
    let ttl_secs = state.cfg.admin_unlock_ttl_minutes * 60;
    let expires_at = now_epoch + ttl_secs;

    // Guardar en memoria (Arc<Mutex<HashMap>>).
    {
        let mut unlocks = state.admin_unlocks.lock().await;
        unlocks.insert(
            token.clone(),
            crate::AdminUnlock {
                user_id: user.id,
                expires_at,
            },
        );
    }

    // Setear cookie con el builder pattern (mismo estilo que auth::set_session_cookie).
    // HttpOnly + SameSite=Lax; Secure si production. Path=/admin para que el
    // navegador solo la envíe a rutas de admin.
    let ttl_duration = time::Duration::seconds(ttl_secs);
    let mut builder = tower_cookies::Cookie::build((
        state.cfg.admin_unlock_cookie_name.clone(),
        token,
    ))
    .path("/admin")
    .http_only(true)
    .same_site(tower_cookies::cookie::SameSite::Lax)
    .max_age(ttl_duration);
    if !state.cfg.cookie_domain.is_empty() {
        builder = builder.domain(state.cfg.cookie_domain.clone());
    }
    if state.cfg.production {
        builder = builder.secure(true);
    }
    cookies.add(builder.build());

    let _ = state
        .db
        .audit(Some(user.id), "admin_unlock_ok", None, None)
        .await;

    Redirect::to(&next_safe).into_response()
}

#[derive(Debug, Deserialize)]
pub struct UnlockForm {
    pub password: String,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/system/admin_unlock.html")]
pub struct AdminUnlockTemplate {
    pub username: String,
    pub role: String,
    pub role_display: String,
    pub role_css_key: String,
    pub next: String,
    pub error: Option<String>,
}

/// Sanitiza el `next` para evitar open redirect: debe empezar con `/admin`
/// (no es un URL absoluto ni javascript: ni //otrodominio).
fn sanitize_next(next: &Option<String>, default: &str) -> String {
    match next {
        Some(s) => {
            let s = s.trim();
            if s.starts_with("/admin") && !s.starts_with("//") {
                s.to_string()
            } else {
                default.to_string()
            }
        }
        None => default.to_string(),
    }
}

// ===== /admin =====

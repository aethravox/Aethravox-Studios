//! aexOS — Entorno de escritorio web interactivo de Aethravox Studios.
//!
//! Cada usuario de aexOS corresponde a una cuenta real de Aethravox Studios.
//! Al entrar al sistema desde otra cuenta, un segundo usuario puede vincular e
//! iniciar su sesión dentro de aexOS ingresando su usuario y contraseña (solo se
//! guarda el nombre/referencia del usuario, jamás su contraseña). Mientras esté
//! abierta, todas las apps embebidas ejecutan bajo esa cuenta; al salir de
//! aexOS se vuelve a la cuenta principal sin pedir contraseña.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use tower_cookies::Cookies;

use crate::{
    auth_mw::{clear_aexos_session_cookie, set_aexos_session_cookie, AuthUser, AEXOS_COOKIE_NAME},
    AppState,
};

pub async fn aexos_page(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
) -> impl IntoResponse {
    // Determinar usuario activo en aexOS (si hay sesión de perfil abierta)
    let active_user = if let Some(c) = cookies.get(AEXOS_COOKIE_NAME) {
        state
            .db
            .get_aexos_session_user(primary_user.id, c.value())
            .await
            .unwrap_or_else(|| primary_user.clone())
    } else {
        primary_user.clone()
    };

    let can_access_admin = state.db.user_has_perm(&active_user, "admin.access").await;
    let (_, role_display) = state.db.role_display(&active_user).await;
    let _ = state.db.ensure_primary_aexos_prefs(&primary_user).await;

    crate::templates::AexosTemplate {
        username: primary_user.username.clone(),
        display: primary_user.display(),
        role_display,
        avatar: primary_user.avatar(),
        can_access_admin,
    }
}

#[derive(Deserialize)]
pub struct CreateAexosProfilePayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SwitchAexosProfilePayload {
    pub user_id: i64,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveAexosPrefsPayload {
    pub profile_user_id: i64,
    pub wallpaper: String,
    pub show_desktop_icons: bool,
    pub pinned_apps_json: String,
    pub files_json: String,
    pub notes_text: String,
}

async fn build_profiles_json(
    state: &AppState,
    primary_user: &crate::models::User,
    active_user: &crate::models::User,
) -> serde_json::Value {
    let rows = state
        .db
        .list_aexos_accounts(primary_user)
        .await
        .unwrap_or_default();

    let mut list = Vec::with_capacity(rows.len());
    for r in rows {
        let user_opt = state.db.find_user_by_id(r.user_id).await.ok().flatten();
        let can_admin = match &user_opt {
            Some(u) => state.db.user_has_perm(u, "admin.access").await,
            None => false,
        };
        let avatar = match &user_opt {
            Some(u) => u.avatar(),
            None => primary_user.avatar(),
        };
        let (_, role_display) = match &user_opt {
            Some(u) => state.db.role_display(u).await,
            None => ("usuario".into(), "Usuario".into()),
        };
        list.push(json!({
            "user_id": r.user_id,
            "is_primary": r.user_id == primary_user.id,
            "username": r.username,
            "display_name": r.display_name.unwrap_or_else(|| r.username.clone()),
            "avatar": avatar,
            "role": r.role,
            "role_display": role_display,
            "can_admin": can_admin,
            "wallpaper": r.wallpaper,
            "show_desktop_icons": r.show_desktop_icons != 0,
            "pinned_apps_json": r.pinned_apps_json,
            "files_json": r.files_json,
            "notes_text": r.notes_text,
        }));
    }

    let active_can_admin = state.db.user_has_perm(active_user, "admin.access").await;
    json!({
        "ok": true,
        "primary_user_id": primary_user.id,
        "active_user_id": active_user.id,
        "active_username": active_user.username,
        "active_can_admin": active_can_admin,
        "profiles": list,
    })
}

/// GET /api/aexos/profiles
pub async fn api_list_profiles(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
) -> Response {
    let active_user = if let Some(c) = cookies.get(AEXOS_COOKIE_NAME) {
        state
            .db
            .get_aexos_session_user(primary_user.id, c.value())
            .await
            .unwrap_or_else(|| primary_user.clone())
    } else {
        primary_user.clone()
    };
    Json(build_profiles_json(&state, &primary_user, &active_user).await).into_response()
}

/// POST /api/aexos/profiles — Verifica usuario + contraseña de otra cuenta real de Aethravox Studios,
/// guarda únicamente la referencia del usuario (sin contraseña) y abre su sesión dentro de aexOS.
pub async fn api_create_profile(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
    Json(payload): Json<CreateAexosProfilePayload>,
) -> Response {
    let linked = match state
        .db
        .link_existing_aexos_account(&primary_user, &payload.username, &payload.password)
        .await
    {
        Ok(row) => row,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let _ = state
        .db
        .audit(
            Some(primary_user.id),
            "aexos_account_link",
            Some(&format!("@{} (id={})", linked.username, linked.user_id)),
            None,
        )
        .await;

    // Al verificar credenciales, abrir directamente la cuenta dentro de aexOS
    let active_user = match state
        .db
        .create_aexos_session(primary_user.id, linked.user_id)
        .await
    {
        Ok(token) => {
            set_aexos_session_cookie(&state, &cookies, &token);
            state
                .db
                .find_user_by_id(linked.user_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| primary_user.clone())
        }
        Err(_) => primary_user.clone(),
    };

    let mut body = build_profiles_json(&state, &primary_user, &active_user).await;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("created_user_id".into(), json!(linked.user_id));
    }
    Json(body).into_response()
}

/// POST /api/aexos/switch — Verifica la contraseña de la cuenta seleccionada (principal o vinculada)
/// y cambia la sesión activa dentro de aexOS.
pub async fn api_switch_profile(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
    Json(payload): Json<SwitchAexosProfilePayload>,
) -> Response {
    if payload.user_id == primary_user.id {
        let pw = payload.password.as_deref().unwrap_or("");
        if pw.is_empty() || !crate::db::verify_password(pw, &primary_user.password_hash) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "ok": false,
                    "error": format!("Contraseña incorrecta para @{}.", primary_user.username)
                })),
            )
                .into_response();
        }
        if let Some(c) = cookies.get(AEXOS_COOKIE_NAME) {
            let _ = state.db.delete_aexos_session_by_token(c.value()).await;
        }
        clear_aexos_session_cookie(&state, &cookies);
        return Json(build_profiles_json(&state, &primary_user, &primary_user).await).into_response();
    }

    match state
        .db
        .verify_and_create_aexos_session(
            primary_user.id,
            payload.user_id,
            payload.password.as_deref(),
        )
        .await
    {
        Ok(token) => {
            set_aexos_session_cookie(&state, &cookies, &token);
            let active_user = state
                .db
                .find_user_by_id(payload.user_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| primary_user.clone());
            Json(build_profiles_json(&state, &primary_user, &active_user).await).into_response()
        }
        Err(e) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// POST /api/aexos/profiles/:id/delete — Quita una cuenta vinculada de la lista de aexOS
/// (nunca elimina la cuenta real de Aethravox Studios).
pub async fn api_delete_profile(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
    Path(target_id): Path<i64>,
) -> Response {
    match state
        .db
        .delete_aexos_account(primary_user.id, target_id)
        .await
    {
        Ok(()) => {
            let active_user = if let Some(c) = cookies.get(AEXOS_COOKIE_NAME) {
                match state
                    .db
                    .get_aexos_session_user(primary_user.id, c.value())
                    .await
                {
                    Some(u) => u,
                    None => {
                        clear_aexos_session_cookie(&state, &cookies);
                        primary_user.clone()
                    }
                }
            } else {
                primary_user.clone()
            };
            Json(build_profiles_json(&state, &primary_user, &active_user).await).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// POST /api/aexos/prefs — Guarda fondo, iconos, apps fijadas, archivos y notas del perfil activo
pub async fn api_save_prefs(
    State(state): State<AppState>,
    AuthUser(primary_user): AuthUser,
    Json(payload): Json<SaveAexosPrefsPayload>,
) -> Response {
    match state
        .db
        .save_aexos_prefs(
            primary_user.id,
            payload.profile_user_id,
            &payload.wallpaper,
            payload.show_desktop_icons,
            &payload.pinned_apps_json,
            &payload.files_json,
            &payload.notes_text,
        )
        .await
    {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// POST /api/aexos/exit — Al cerrar/salir de aexOS, limpia la sesión de perfil sin pedir contraseña
pub async fn api_exit_aexos(
    State(state): State<AppState>,
    cookies: Cookies,
    AuthUser(primary_user): AuthUser,
) -> Response {
    let _ = state
        .db
        .delete_aexos_sessions_for_owner(primary_user.id)
        .await;
    clear_aexos_session_cookie(&state, &cookies);
    Json(json!({ "ok": true })).into_response()
}

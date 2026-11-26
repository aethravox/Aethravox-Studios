// Web 5 — Aethravox Amigos
// Sistema de amistad con solicitudes pendientes, aceptar/rechazar y búsqueda de usuarios.
use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{auth_mw::AuthUser, models::FriendRequestForm, AppState};

// ===== GET /friends — Página principal de amigos =====
pub async fn friends_page(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    // Cargar amigos aceptados con su info de usuario.
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let online_ids = state.db.list_online_user_ids().await.unwrap_or_default();
    let mut friends: Vec<FriendView> = Vec::new();
    for fid in friend_ids {
        if let Ok(Some(f)) = state.db.find_user_by_id(fid).await {
            // Badge de rol real (custom o de sistema): pedimos el (css_key, display)
            // a la DB en vez de leer users.role legado, que para roles custom queda
            // degradado a "admin"/"staff"/"user" por posición. Así un rol custom
            // "Moderador" muestra "Moderador", no "admin".
            let (role_css_key, role_display) = state.db.role_display(&f).await;
            friends.push(FriendView {
                id: f.id,
                username: f.username.clone(),
                display: f.display(),
                avatar: f.avatar(),
                online: online_ids.contains(&f.id),
                role_css_key,
                role_display,
            });
        }
    }
    // Ordenar: online primero, luego alfabético.
    friends.sort_by(|a, b| {
        b.online.cmp(&a.online).then_with(|| a.display.to_lowercase().cmp(&b.display.to_lowercase()))
    });

    // Cargar solicitudes pendientes recibidas.
    let requester_ids = state.db.list_pending_requests(user.id).await.unwrap_or_default();
    let mut pending: Vec<PendingView> = Vec::new();
    for rid in requester_ids {
        if let Ok(Some(r)) = state.db.find_user_by_id(rid).await {
            pending.push(PendingView {
                id: r.id,
                username: r.username.clone(),
                display: r.display(),
                avatar: r.avatar(),
            });
        }
    }

    // Badge de rol del propio usuario (puede ser custom).
    let (user_role_css_key, user_role_display) = state.db.role_display(&user).await;

    FriendsTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role_css_key: user_role_css_key,
        role_display: user_role_display,
        friends_count: friends.len() as i64,
        pending_count: pending.len() as i64,
        friends,
        pending,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/friends/friends.html")]
pub struct FriendsTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role_css_key: String,
    pub role_display: String,
    pub friends_count: i64,
    pub pending_count: i64,
    pub friends: Vec<FriendView>,
    pub pending: Vec<PendingView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendView {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub online: bool,
    /// Clave CSS del rol (p.ej. "custom_3") para matchear con /role-colors.css.
    pub role_css_key: String,
    /// Nombre a mostrar del rol (p.ej. "Moderador"), puede ser custom.
    pub role_display: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PendingView {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub avatar: String,
}

// ===== API JSON (para frontend dinámico, opcional) =====

#[derive(Serialize)]
struct FriendList {
    id: i64,
    username: String,
    display: String,
    avatar: String,
    online: bool,
}

#[derive(Serialize)]
struct PendingRequest {
    id: i64,
    username: String,
    display: String,
    avatar: String,
}

/// GET /api/friends/list — Lista de amigos aceptados.
pub async fn api_friends_list(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let online_ids = state.db.list_online_user_ids().await.unwrap_or_default();
    let mut result = Vec::new();
    for fid in friend_ids {
        if let Ok(Some(f)) = state.db.find_user_by_id(fid).await {
            result.push(FriendList {
                id: f.id,
                username: f.username.clone(),
                display: f.display(),
                avatar: f.avatar(),
                online: online_ids.contains(&f.id),
            });
        }
    }
    Json(result).into_response()
}

/// GET /api/friends/pending — Solicitudes pendientes recibidas.
pub async fn api_pending_requests(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let requester_ids = state.db.list_pending_requests(user.id).await.unwrap_or_default();
    let mut result = Vec::new();
    for rid in requester_ids {
        if let Ok(Some(r)) = state.db.find_user_by_id(rid).await {
            result.push(PendingRequest {
                id: r.id,
                username: r.username.clone(),
                display: r.display(),
                avatar: r.avatar(),
            });
        }
    }
    Json(result).into_response()
}

/// POST /api/friends/request — Enviar solicitud de amistad por username.
pub async fn api_send_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<FriendRequestForm>,
) -> Response {
    let target = match state.db.find_user_by_username(&form.username).await {
        Ok(Some(u)) => u,
        _ => return Json(json!({"error":"Usuario no encontrado"})).into_response(),
    };

    if target.id == user.id {
        return Json(json!({"error":"No puedes enviarte solicitud a ti mismo"})).into_response();
    }
    if state.db.are_friends(user.id, target.id).await.unwrap_or(false) {
        return Json(json!({"error":"Ya son amigos"})).into_response();
    }

    let _ = state.db.send_friend_request(user.id, target.id).await;
    Json(json!({"ok":true,"target":{"id":target.id,"username":target.username}})).into_response()
}

/// POST /api/friends/accept/:id — Aceptar solicitud pendiente de `requester_id`.
pub async fn api_accept_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(requester_id): Path<i64>,
) -> Response {
    let _ = state.db.accept_friend_request(user.id, requester_id).await;
    Json(json!({"ok":true})).into_response()
}

/// POST /api/friends/reject/:id — Rechazar solicitud pendiente.
pub async fn api_reject_request(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(requester_id): Path<i64>,
) -> Response {
    let _ = state.db.reject_friend_request(user.id, requester_id).await;
    Json(json!({"ok":true})).into_response()
}

/// POST /api/friends/remove/:id — Eliminar amigo.
pub async fn api_remove_friend(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(friend_id): Path<i64>,
) -> Response {
    let _ = state.db.remove_friend(user.id, friend_id).await;
    Json(json!({"ok":true})).into_response()
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// GET /api/friends/search?q=... — Buscar usuarios para añadir.
/// Excluye: yo mismo, usuarios baneados/suspendidos, mis amigos actuales,
/// y usuarios a quienes ya envié solicitud pendiente.
pub async fn api_search_users(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<SearchQuery>,
) -> Response {
    let all = state.db.list_users().await.unwrap_or_default();
    let query = q.q.to_lowercase();
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let pending = state.db.list_pending_requests(user.id).await.unwrap_or_default();

    // Necesitamos llamar a role_display (async) por cada usuario, así que no
    // podemos usar .map() con una closure síncrona. Iteramos a mano.
    let mut result = Vec::new();
    for u in &all {
        if u.id == user.id || u.status != "active" {
            continue;
        }
        if !u.username.to_lowercase().contains(&query)
            && !u.display().to_lowercase().contains(&query)
        {
            continue;
        }
        if friend_ids.contains(&u.id) || pending.contains(&u.id) {
            continue;
        }
        let (role_css_key, role_display) = state.db.role_display(u).await;
        result.push(json!({
            "id": u.id,
            "username": u.username,
            "display": u.display(),
            "avatar": u.avatar(),
            "role_css_key": role_css_key,
            "role_display": role_display,
        }));
    }
    Json(result).into_response()
}

// ===== Form-based fallback (no-JS) =====
// Si el navegador del usuario no tiene JS o el fetch falla, los botones de la
// plantilla friends.html hacen POST a estas rutas y recargan la página.
// El redirect lleva ?toast=...&user=... para que la UI muestre un toast de éxito.

/// POST /friends/send — Enviar solicitud por formulario tradicional.
pub async fn send_request_form(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<FriendRequestForm>,
) -> Response {
    if let Ok(Some(target)) = state.db.find_user_by_username(&form.username).await {
        if target.id != user.id
            && !state.db.are_friends(user.id, target.id).await.unwrap_or(false)
        {
            let _ = state.db.send_friend_request(user.id, target.id).await;
            // Mensaje de éxito con username (URL-encoded automáticamente por axum).
            return Redirect::to(&format!(
                "/friends?toast=send&user={}",
                urlencoding(&target.username)
            )).into_response();
        }
        // Ya son amigos o es a sí mismo: redirigir silenciosamente.
    }
    Redirect::to("/friends").into_response()
}

/// POST /friends/accept/:id — Aceptar solicitud (form fallback).
pub async fn accept_request_form(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(requester_id): Path<i64>,
) -> Response {
    let username = state
        .db
        .find_user_by_id(requester_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "desconocido".into());
    let _ = state.db.accept_friend_request(user.id, requester_id).await;
    Redirect::to(&format!("/friends?toast=accept&user={}", urlencoding(&username))).into_response()
}

/// POST /friends/reject/:id — Rechazar solicitud (form fallback).
pub async fn reject_request_form(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(requester_id): Path<i64>,
) -> Response {
    let username = state
        .db
        .find_user_by_id(requester_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "desconocido".into());
    let _ = state.db.reject_friend_request(user.id, requester_id).await;
    Redirect::to(&format!("/friends?toast=reject&user={}", urlencoding(&username))).into_response()
}

/// POST /friends/remove/:id — Eliminar amigo (form fallback).
pub async fn remove_friend_form(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(friend_id): Path<i64>,
) -> Response {
    let username = state
        .db
        .find_user_by_id(friend_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "desconocido".into());
    let _ = state.db.remove_friend(user.id, friend_id).await;
    Redirect::to(&format!("/friends?toast=remove&user={}", urlencoding(&username))).into_response()
}

/// Helper local para URL-encodear usernames (carácteres ASCII válidos pasan
/// sin escape; el resto se codifica como %XX). Reutilizable por los handlers
/// de arriba para construir URLs de redirect seguras.
fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b;
        if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.' {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{:02X}", c));
        }
    }
    out
}

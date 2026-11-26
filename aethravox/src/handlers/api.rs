use axum::{extract::{Path, State}, response::{IntoResponse, Response}, Json};
use serde_json::json;

use crate::{auth_mw::AuthUser, AppState};

pub async fn hub_stats(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let (o, a, s, u) = state.db.count_users_by_role().await.unwrap_or((0,0,0,0));
    // Badges del hub: mensajes sin leer en chat (todas las conversaciones) y
    // en comunidades (solo grupos que pertenecen a una comunidad). El JS del hub
    // los vuelca sobre la tarjeta de Chat y la de Comunidades cada 5s.
    let chat_unread = state.db.count_total_unread_for_user(user.id).await.unwrap_or(0);
    let community_unread = state.db.count_community_unread_for_user(user.id).await.unwrap_or(0);
    Json(json!({
        "owner_count": o,
        "admin_count": a,
        "staff_count": s,
        "user_count": u,
        "total_users": o + a + s + u,
        "online_count": state.db.list_online_user_ids().await.unwrap_or_default().len() as i64,
        "doc_count": state.db.list_documents().await.unwrap_or_default().len() as i64,
        "chat_unread": chat_unread,
        "community_unread": community_unread,
    })).into_response()
}

pub async fn docs_list_api(State(_): State<AppState>, AuthUser(_): AuthUser) -> Response { Json(json!([])).into_response() }
pub async fn doc_views(State(_): State<AppState>, AuthUser(_): AuthUser, Path(_id): Path<i64>) -> Response { Json(json!({"views":0,"downloads":0})).into_response() }
pub async fn users_online(State(state): State<AppState>, AuthUser(_): AuthUser) -> Response {
    let ids = state.db.list_online_user_ids().await.unwrap_or_default();
    Json(json!({"online_ids":ids,"online_count":ids.len() as i64})).into_response()
}
pub async fn list_tags(State(_): State<AppState>, AuthUser(_): AuthUser) -> Response { Json(json!([])).into_response() }

// ===== /api/me — datos mínimos del usuario logueado =====
// Lo usa el sidebar global (base.html) para pintar la foto de perfil real en
// el icono de Perfil + el FAB/drawer de cuenta, sin que cada handler tenga que
// pasar `avatar` al template (hay ~40 páginas autenticadas; tocar todas sería
// inviable). Devuelve { avatar, username, display, role_css_key, role_display }.
// Requiere AuthUser (401 si no hay sesión).
//
// NO se cachea en sessionStorage: el usuario puede cambiar su avatar/rol y el
// sidebar seguiría mostrando el viejo hasta cerrar la pestaña. El fetch va con
// credenciales y el SW no cachea /api/me, así que cada navegación pide la foto
// real (es una query barata: 1 SELECT + 1 JOIN opcional con roles).
pub async fn me(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let (role_css_key, role_display) = state.db.role_display(&user).await;
    Json(json!({
        "avatar": user.avatar(),
        "username": user.username,
        "display": user.display(),
        "role_css_key": role_css_key,
        "role_display": role_display,
    })).into_response()
}

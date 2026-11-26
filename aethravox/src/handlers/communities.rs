// Fase 5 — Comunidades
use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};

use crate::{auth_mw::AuthUser, models::CreateCommunityForm, AppState};

// ===== GET /communities =====
pub async fn communities_page(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let is_guest = user.id == 0;
    // El invitado no es miembro de ninguna comunidad → list_user_communities(0)
    // devolvería vacío. Le mostramos TODAS las activas (públicas) en modo solo
    // lectura; unirse requiere cuenta (/api/communities/:id/join → require_auth).
    let communities = if is_guest {
        state.db.list_all_communities().await.unwrap_or_default()
    } else {
        state.db.list_user_communities(user.id).await.unwrap_or_default()
    };
    CommunitiesTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: user.role.clone(),
        is_guest,
        communities: communities.iter().map(|c| CommunityRow {
            id: c.id,
            name: c.name.clone(),
            description: c.description.clone().unwrap_or_default(),
            icon: c.icon.clone().unwrap_or_else(|| "🏘️".into()),
            member_count: 0,
        }).collect(),
    }.into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/communities/communities.html")]
pub struct CommunitiesTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    /// true si el viewer es invitado sin sesión (User sintético id=0).
    pub is_guest: bool,
    pub communities: Vec<CommunityRow>,
}

pub struct CommunityRow {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub member_count: i64,
}

// ===== GET /community/:id =====
pub async fn community_view(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    let is_guest = user.id == 0;
    // El invitado puede VER cualquier comunidad activa (solo lectura); los
    // usuarios logueados deben ser miembros. Unirse/invitar/escribir requiere
    // cuenta (APIs tras require_auth).
    if !is_guest && !state.db.is_community_member(id, user.id).await {
        return Redirect::to("/communities").into_response();
    }
    let community = state.db.get_community(id).await.ok().flatten();
    let Some(community) = community else {
        return Redirect::to("/communities").into_response();
    };
    let groups = state.db.list_community_conversations(id).await.unwrap_or_default();
    CommunityViewTemplate {
        community_id: community.id,
        community_name: community.name.clone(),
        community_description: community.description.clone().unwrap_or_default(),
        community_icon: community.icon.clone().unwrap_or_else(|| "🏘️".into()),
        username: user.username.clone(),
        role: user.role.clone(),
        avatar: user.avatar(),
        is_guest,
        groups: groups.iter().map(|g| GroupRow {
            id: g.id,
            name: g.name.clone().unwrap_or_else(|| "Grupo".into()),
            description: g.description.clone().unwrap_or_default(),
        }).collect(),
    }.into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/communities/community_view.html")]
pub struct CommunityViewTemplate {
    pub community_id: i64,
    pub community_name: String,
    pub community_description: String,
    pub community_icon: String,
    pub username: String,
    pub role: String,
    pub avatar: String,
    /// true si el viewer es invitado sin sesión. Oculta acciones de
    /// unirse/invitar/escribir (sus APIs están tras require_auth).
    pub is_guest: bool,
    pub groups: Vec<GroupRow>,
}

pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub description: String,
}

// ===== POST /api/communities/create =====
pub async fn api_create_community(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<CreateCommunityForm>,
) -> Response {
    let name = form.name.trim();
    if name.is_empty() {
        return Redirect::to("/communities?err=empty").into_response();
    }
    let id = state.db.create_community(
        name,
        form.description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
        form.icon.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()),
        user.id,
    ).await.unwrap_or(0);
    let _ = state.db.audit(Some(user.id), "community_created", Some(&format!("id={} name={}", id, name)), None).await;
    Redirect::to(&format!("/community/{}", id)).into_response()
}

// ===== POST /api/communities/:id/join =====
pub async fn api_join_community(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let _ = state.db.join_community(id, user.id).await;
    Redirect::to(&format!("/community/{}", id)).into_response()
}

// ===== POST /api/communities/:id/leave =====
pub async fn api_leave_community(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let _ = state.db.leave_community(id, user.id).await;
    Redirect::to("/communities").into_response()
}

// ===== Invitar a la comunidad seleccionando de una lista =====
// El usuario no escribe nombres: ve una lista de sus amigos que aún no son
// miembros de la comunidad, marca a los que quiere invitar y los envía. La
// lista lleva un buscador (input de texto) para filtrar rápido — ahí sí se
// escribe, pero solo para buscar, no para identificar a la persona.

/// GET /api/communities/:id/inviteable — amigos del usuario que NO son miembros
/// aún de la comunidad. JSON: [{id, username, display, avatar}].
pub async fn api_community_inviteable(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let mut out = Vec::new();
    for fid in friend_ids {
        if state.db.is_community_member(id, fid).await {
            continue; // ya es miembro, no lo mostramos
        }
        if let Ok(Some(f)) = state.db.find_user_by_id(fid).await {
            out.push(serde_json::json!({
                "id": f.id,
                "username": f.username,
                "display": f.display(),
                "avatar": f.avatar(),
            }));
        }
    }
    Json(out).into_response()
}

/// GET /api/communities/:id/members — miembros de la comunidad (para elegir a
/// quién añadir a un grupo dentro de ella). JSON: [{id, username, display, avatar}].
pub async fn api_community_members(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if !state.db.is_community_member(id, user.id).await {
        return Json(serde_json::json!({"error":"no_miembro"})).into_response();
    }
    let member_ids = state.db.list_community_members(id).await.unwrap_or_default();
    let mut out = Vec::new();
    for mid in member_ids {
        if mid == user.id {
            continue;
        }
        if let Ok(Some(m)) = state.db.find_user_by_id(mid).await {
            out.push(serde_json::json!({
                "id": m.id,
                "username": m.username,
                "display": m.display(),
                "avatar": m.avatar(),
            }));
        }
    }
    Json(out).into_response()
}

#[derive(Debug, serde::Deserialize)]
pub struct InviteCommunityForm {
    /// IDs de usuario separados por coma (seleccionados de la lista).
    #[serde(default)]
    pub members: Option<String>,
}

/// POST /api/communities/:id/invite — invitar (añadir) a la comunidad a los
/// usuarios seleccionados. Acepta form-encoded members=1,2,3.
pub async fn api_invite_community(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<InviteCommunityForm>,
) -> Response {
    // Solo un miembro puede invitar (si no, cualquiera podría inundar la
    // comunidad). El dueño siempre puede.
    let is_owner = state.db.get_community(id).await.ok().flatten()
        .map(|c| c.owner_id == user.id).unwrap_or(false);
    if !is_owner && !state.db.is_community_member(id, user.id).await {
        return Redirect::to(&format!("/community/{}?err=notmember", id)).into_response();
    }
    let mut added = 0i64;
    if let Some(members) = form.members.as_deref() {
        for tok in members.split(',') {
            let tok = tok.trim();
            if tok.is_empty() { continue; }
            if let Ok(uid) = tok.parse::<i64>() {
                // Solo invitamos a amigos del usuario (no a cualquiera): evita
                // que un miembro añada a desconocidos a la comunidad.
                let is_friend = state.db.list_friends(user.id).await.unwrap_or_default().contains(&uid);
                if is_friend && !state.db.is_community_member(id, uid).await {
                    let _ = state.db.join_community(id, uid).await;
                    added += 1;
                }
            }
        }
    }
    let _ = state.db.audit(Some(user.id), "community_invite", Some(&format!("cid={} added={}", id, added)), None).await;
    Redirect::to(&format!("/community/{}?ok=invited&n={}", id, added)).into_response()
}

// ===== POST /api/communities/:id/group/create =====
#[derive(Debug, serde::Deserialize)]
pub struct CreateCommunityGroupForm {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// IDs de usuario separados por coma (seleccionados de la lista de miembros
    /// de la comunidad). Antes eran usernames escritos a mano.
    #[serde(default)]
    pub members: Option<String>,
}
pub async fn api_create_community_group(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<CreateCommunityGroupForm>,
) -> Response {
    if !state.db.is_community_member(id, user.id).await {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    let name = form.name.trim();
    if name.is_empty() {
        return Json(serde_json::json!({"error": "Nombre requerido"})).into_response();
    }
    let desc = form.description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let conv_id = state.db.create_community_group(id, name, desc, user.id).await.unwrap_or(0);
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    let _ = state.db.create_default_ranks(conv_id, user.id).await;
    // Añadir miembros seleccionados de la lista (IDs). Solo se añaden si son
    // miembros de la comunidad (las conversaciones-grupo viven dentro de ella).
    if let Some(members) = form.members.as_deref() {
        for tok in members.split(',') {
            let tok = tok.trim();
            if tok.is_empty() { continue; }
            if let Ok(uid) = tok.parse::<i64>() {
                if state.db.is_community_member(id, uid).await {
                    let _ = state.db.add_conversation_member(conv_id, uid).await;
                }
            }
        }
    }
    Redirect::to(&format!("/chat/group/{}/settings", conv_id)).into_response()
}

// ===== Moderación de comunidades (permiso `communities.moderate`) =====
//
// Desactiva una comunidad. Lo puede hacer el dueño de la comunidad, o
// cualquier usuario con el permiso del hub `communities.moderate` (rol o
// rango configurables). No borra filas: marca is_active=0 (soft delete).
// Esto evita que un admin se equivoque y pierda datos.

pub async fn api_community_moderate_delete(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let community = match state.db.get_community(id).await {
        Ok(Some(c)) => c,
        _ => return Json(serde_json::json!({"error": "Comunidad no encontrada"})).into_response(),
    };
    // (1) Owner de la comunidad, o (2) permiso del hub communities.moderate.
    let is_owner = community.owner_id == user.id;
    let can_moderate = state.db.user_has_perm(&user, "communities.moderate").await;
    if !is_owner && !can_moderate {
        return Json(serde_json::json!({"error": "No tienes permiso para moderar esta comunidad"})).into_response();
    }
    match state.db.deactivate_community(id).await {
        Ok(_) => {
            let _ = state.db.audit(
                Some(user.id),
                "community_deactivate",
                Some(&format!("id={},name={}", id, community.name)),
                None,
            ).await;
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(e) => {
            tracing::warn!("community_deactivate error: {e}");
            Json(serde_json::json!({"error": "No se pudo desactivar la comunidad"})).into_response()
        }
    }
}

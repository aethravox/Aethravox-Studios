#![allow(unused_imports)]
// Web 2 — Aethravox Chat (estilo WhatsApp con WebSocket)
use askama::Template;
use futures::stream::StreamExt;
use futures::sink::SinkExt;
use axum::{
    extract::{
        ws::{self, WebSocketUpgrade, Message as WsMessage},
        Form, Multipart, Path, State,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::{
    auth_mw::AuthUser,
    models::{NewDmForm, NewGroupForm},
    AppState,
};

// ===== WebSocket connections registry =====
pub type WsConnections = Arc<Mutex<HashMap<i64, mpsc::UnboundedSender<String>>>>;

// ===== GET /chat — Chat page (Askama template, responsive) =====


pub async fn api_new_dm(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<NewDmForm>,
) -> Response {
    let target = state.db.find_user_by_username(&form.username).await.ok().flatten();
    let Some(target) = target else {
        return Json(serde_json::json!({"error": "Usuario no encontrado"})).into_response();
    };
    if target.id == user.id {
        return Json(serde_json::json!({"error": "No puedes crear DM contigo mismo"})).into_response();
    }

    // Find or create DM
    if let Some(existing) = state.db.find_dm(user.id, target.id).await.ok().flatten() {
        return Json(serde_json::json!({"id": existing.id})).into_response();
    }

    let conv_id = state.db.create_conversation("dm", None, user.id).await.unwrap_or(0);
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    let _ = state.db.add_conversation_member(conv_id, target.id).await;
    Json(serde_json::json!({"id": conv_id})).into_response()
}

// ===== API: Create group =====
pub async fn api_new_group(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<NewGroupForm>,
) -> Response {
    let name = form.name.trim();
    if name.is_empty() {
        return Json(serde_json::json!({"error": "Nombre requerido"})).into_response();
    }

    let description = form.description.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let conv_id = state.db.create_group(name, description, user.id).await.unwrap_or(0);
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    // Crear rangos predeterminados (Owner, Admin, Miembro) y asignar Owner al creador
    let _ = state.db.create_default_ranks(conv_id, user.id).await;

    // Add members
    for username in form.members.split(',') {
        let username = username.trim();
        if username.is_empty() { continue; }
        if let Some(u) = state.db.find_user_by_username(username).await.ok().flatten() {
            let _ = state.db.add_conversation_member(conv_id, u.id).await;
        }
    }

    Json(serde_json::json!({"id": conv_id})).into_response()
}

// ===== API: List all users (for new chat modal) =====
#[derive(Serialize)]
struct UserList {
    username: String,
}

pub async fn api_users(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let users = state.db.list_users().await.unwrap_or_default();
    let result: Vec<UserList> = users
        .iter()
        .filter(|u| u.id != user.id && u.status == "active")
        .map(|u| UserList { username: u.username.clone() })
        .collect();
    Json(result).into_response()
}

// ===== POST /api/chat/:id/upload — Subir imagen al chat =====
// Acepta multipart con campo "image". Valida que sea imagen y < 5MB.
// Guarda en data/chat_images/<hex>.ext, crea mensaje con body="[IMG]/chat/images/...",
// y broadcasta via WS a todos los miembros de la conversación.

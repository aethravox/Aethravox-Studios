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


pub async fn chat_page(AuthUser(user): AuthUser) -> Response {
    ChatTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: user.role.clone(),
        // ICE servers (STUN + opcional TURN) para las llamadas/videollamadas
        // P2P del chat. Igual que en /transmitir/enviar.
        ice_servers_json: crate::handlers::transmit::build_ice_servers_json(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/chat/chat.html")]
pub struct ChatTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    pub ice_servers_json: String,
}

// ===== GET /chat/ws — WebSocket upgrade =====

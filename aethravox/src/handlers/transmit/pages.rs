#![allow(unused_imports)]
// Aethravox Studios — Transmitir (Fase 7)
// Screen sharing P2P con WebRTC. El servidor SOLO hace señalización
// (intercambio SDP + ICE). El video viaja P2P entre emisor y receptor,
// no pasa por el server. STUN público de Google para NAT traversal en
// la mayoría de redes; TURN opcional via .env para redes restrictivas.

use askama::Template;
use axum::{
    extract::{
        ws::{self, WebSocketUpgrade, Message as WsMessage},
        State,
    },
    response::{IntoResponse, Redirect, Response},
};
use futures::stream::StreamExt;
use futures::sink::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::auth_mw::AuthUser;
use crate::AppState;

// ====== Tipos del modelo de salas (en memoria) ======

/// Una sala de transmisión. Creada por un emisor con nombre+password.
/// Varios receptores pueden unirse con ese nombre+password.



// ====== Tipos del modelo de salas (en memoria) ======

/// Una sala de transmisión. Creada por un emisor con nombre+password.
/// Varios receptores pueden unirse con ese nombre+password.
pub struct TransmitRoom {
    /// Nombre de la sala, ya en minúsculas. Clave en el HashMap.
    pub name: String,
    /// Hash de la password (SHA-256 hex). No guardamos la password original.
    pub password_hash: String,
    /// user_id del emisor (para info/auditoría).
    pub owner_id: i64,
    /// Nombre visible del emisor (display name).
    pub owner_display: String,
    /// connection_id del emisor (cada conexión WS tiene un ID único).
    pub sender_conn_id: i64,
    /// Canal para enviar mensajes al emisor via WS.
    pub sender_tx: mpsc::UnboundedSender<String>,
    /// Receptores conectados: connection_id → (user_id, display, tx).
    pub receivers: HashMap<i64, (i64, String, mpsc::UnboundedSender<String>)>,
    /// Receptor que tiene el control remoto activo (None = nadie).
    /// Solo uno a la vez. Cambia cuando el emisor lo revoca, cuando el
    /// receptor se va o suelta el control voluntariamente.
    pub current_controller: Option<i64>,
    /// Cola de conn_ids que pidieron control y están esperando aprobación.
    /// Se limpia cuando se aprueba (solo se saca al de adelante) o se rechaza.
    pub control_request_queue: Vec<i64>,
}

impl TransmitRoom {
    /// Broadcast a TODOS los receptores + el emisor (para eventos tipo
    /// "sala cerrada"). Excluye opcionalmente un conn_id (ej: el que se va).
    async fn broadcast(&self, msg: &str, exclude: Option<i64>) {
        for (conn_id, (_, _, tx)) in &self.receivers {
            if Some(*conn_id) == exclude { continue; }
            let _ = tx.send(msg.to_string());
        }
        if Some(self.sender_conn_id) != exclude {
            let _ = self.sender_tx.send(msg.to_string());
        }
    }
}

// ====== Handlers de páginas (Askama) ======

pub async fn panel_page(AuthUser(user): AuthUser) -> Response {
    PanelTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: user.role.clone(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/transmitir/transmit.html")]
pub struct PanelTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
}

pub async fn sender_page(AuthUser(user): AuthUser) -> Response {
    SenderTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: user.role.clone(),
        // STUN/TURN config para pasárselo al JS como JSON.
        ice_servers_json: build_ice_servers_json(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/transmitir/transmit_send.html")]
pub struct SenderTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    pub ice_servers_json: String,
}

pub async fn receiver_page(AuthUser(user): AuthUser) -> Response {
    ReceiverTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: user.role.clone(),
        ice_servers_json: build_ice_servers_json(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "comunicacion/transmitir/transmit_receive.html")]
pub struct ReceiverTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    pub ice_servers_json: String,
}

// ====== STUN / TURN config ======

/// Construye la lista de servidores ICE para WebRTC.
/// STUN público de Google siempre. TURN opcional si está configurado en .env.
/// Formato del .env:
///   AETHRAVOX_TURN_URL=turn:turn.example.com:3478
///   AETHRAVOX_TURN_USER=usuario
///   AETHRAVOX_TURN_PASS=secreto
pub fn build_ice_servers_json() -> String {
    let mut servers: Vec<serde_json::Value> = vec![
        serde_json::json!({ "urls": "stun:stun.l.google.com:19302" }),
        serde_json::json!({ "urls": "stun:stun1.l.google.com:19302" }),
    ];

    let turn_url = std::env::var("AETHRAVOX_TURN_URL").ok().filter(|s| !s.is_empty());
    if let Some(url) = turn_url {
        let user = std::env::var("AETHRAVOX_TURN_USER").unwrap_or_default();
        let pass = std::env::var("AETHRAVOX_TURN_PASS").unwrap_or_default();
        servers.push(serde_json::json!({
            "urls": url,
            "username": user,
            "credential": pass,
        }));
    }

    serde_json::to_string(&servers).unwrap_or_else(|_| "[]".into())
}

// ====== WebSocket de señalización ======

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

mod pages;
pub use self::pages::*;
mod websocket;
pub use self::websocket::*;
mod ws_tail;
pub use self::ws_tail::*;

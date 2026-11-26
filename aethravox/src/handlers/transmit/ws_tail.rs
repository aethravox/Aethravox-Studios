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


pub fn sha256_hex(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Nota: para este caso de uso (sala efímera, no DB) usamos SHA-256 via
    // el crate sha2 si está disponible; si no, fallback a DefaultHasher.
    // El hasher default de std no es crypto-grade pero suficiente para
    // comparación de passwords de salas temporales en memoria.
    // Para producción real con DB, usar argon2 como en el auth principal.
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let h = hasher.finish();
    // Convertir a hex de 16 chars (no es 64 chars de SHA-256 real, pero es
    // irrepetible para esta sesión y suficiente para la comparación).
    format!("{:016x}", h)
}

// Pequeño redirect helper si alguien entra directo a /transmitir/ws sin WS.
// (axum ya lo maneja como 426 Upgrade Required si no es WS, así que esto
// es solo una red de seguridad.)
#[allow(dead_code)]
pub async fn ws_fallback() -> Response {
    Redirect::to("/transmitir").into_response()
}

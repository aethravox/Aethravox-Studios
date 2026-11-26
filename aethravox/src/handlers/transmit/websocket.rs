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

use super::ws_tail::sha256_hex;
use super::pages::TransmitRoom;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, user.id, user.display()))
}

async fn handle_socket(socket: ws::WebSocket, state: AppState, user_id: i64, user_display: String) {
    let (sender, mut receiver) = socket.split();

    // Cada conexión WS recibe un conn_id único (incremental, atómico).
    let conn_id = state.transmit_conn_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Canal para enviar mensajes a ESTE cliente (emisor o receptor).
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Para limpiar la sala al desconectarse: registramos en qué sala está
    // y en qué rol (sender o receiver). Si es None, no está en ninguna sala.
    // Variable compartida con el cleanup task → Arc<Mutex<Option<...>>>>
    let room_state: Arc<Mutex<Option<RoomMembership>>> = Arc::new(Mutex::new(None));

    let mut sender = sender;

    // Tarea 1: enviar mensajes del canal al WS.
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Tarea 2: recibir mensajes del WS y procesar.
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            WsMessage::Text(text) => {
                handle_ws_text(
                    &state,
                    conn_id,
                    user_id,
                    &user_display,
                    &tx,
                    &room_state,
                    &text,
                )
                .await;
            }
            WsMessage::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup: si estaba en una sala, salir y notificar al otro extremo.
    cleanup_membership(&state, conn_id, user_id, &user_display, &room_state).await;

    send_task.abort();
}

#[derive(Clone)]
enum RoomMembership {
    Sender { room_name: String },
    Receiver { room_name: String },
}

// ====== Lógica de señalización ======

async fn handle_ws_text(
    state: &AppState,
    conn_id: i64,
    user_id: i64,
    user_display: &str,
    tx: &mpsc::UnboundedSender<String>,
    room_state: &Arc<Mutex<Option<RoomMembership>>>,
    text: &str,
) {
    let Ok(data) = serde_json::from_str::<serde_json::Value>(text) else { return };
    let msg_type = data["type"].as_str().unwrap_or("");

    match msg_type {
        // ===== Emisor crea una sala =====
        "create_room" => {
            let name = data["name"].as_str().unwrap_or("").trim().to_lowercase();
            let password = data["password"].as_str().unwrap_or("").to_string();
            if name.is_empty() || password.is_empty() || name.len() > 64 || password.len() > 128 {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "Nombre y contraseña requeridos (máx 64/128 chars)"
                }).to_string());
                return;
            }
            let mut rooms = state.transmit_rooms.lock().await;
            if rooms.contains_key(&name) {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "Ya existe una sala con ese nombre. Elegí otro."
                }).to_string());
                return;
            }
            rooms.insert(name.clone(), TransmitRoom {
                name: name.clone(),
                password_hash: sha256_hex(&password),
                owner_id: user_id,
                owner_display: user_display.to_string(),
                sender_conn_id: conn_id,
                sender_tx: tx.clone(),
                receivers: HashMap::new(),
                current_controller: None,
                control_request_queue: Vec::new(),
            });
            *room_state.lock().await = Some(RoomMembership::Sender { room_name: name.clone() });
            let _ = tx.send(serde_json::json!({
                "type": "room_created", "name": name
            }).to_string());
            tracing::info!("[transmitir] Sala '{name}' creada por user {user_id} ({user_display})");
        }

        // ===== Receptor se une a una sala =====
        "join_room" => {
            let name = data["name"].as_str().unwrap_or("").trim().to_lowercase();
            let password = data["password"].as_str().unwrap_or("").to_string();
            if name.is_empty() || password.is_empty() {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "Nombre y contraseña requeridos"
                }).to_string());
                return;
            }
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&name) else {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "No existe una sala con ese nombre"
                }).to_string());
                return;
            };
            // Validar password
            if room.password_hash != sha256_hex(&password) {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "Contraseña incorrecta"
                }).to_string());
                return;
            }
            // Máximo 30 receptores por sala (límite razonable)
            if room.receivers.len() >= 30 {
                let _ = tx.send(serde_json::json!({
                    "type": "error", "message": "La sala está llena (máx 30 receptores)"
                }).to_string());
                return;
            }
            // Registrar al receptor
            room.receivers.insert(conn_id, (user_id, user_display.to_string(), tx.clone()));
            *room_state.lock().await = Some(RoomMembership::Receiver { room_name: name.clone() });

            // Notificar al receptor: conexión OK + info del emisor.
            let _ = tx.send(serde_json::json!({
                "type": "room_joined",
                "name": name,
                "sender_display": room.owner_display,
                "sender_conn_id": room.sender_conn_id
            }).to_string());

            // Notificar al emisor: un nuevo receptor se conectó → crear PeerConnection + ofrecer.
            let _ = room.sender_tx.send(serde_json::json!({
                "type": "peer_joined",
                "peer_id": conn_id,
                "peer_display": user_display
            }).to_string());

            tracing::info!("[transmitir] Receptor {conn_id} (user {user_id}) se unió a '{name}'");
        }

        // ===== Señalización P2P: offer / answer / ice_candidate =====
        // El emisor envía offer/ICE a un receptor específico (peer_id).
        // El receptor envía answer/ICE al emisor (sender_conn_id).
        "offer" | "answer" | "ice_candidate" => {
            let to = data["to"].as_i64().unwrap_or(0);
            if to == 0 {
                let _ = tx.send(serde_json::json!({"type":"error","message":"Falta 'to'"}).to_string());
                return;
            }
            let rooms = state.transmit_rooms.lock().await;
            // Buscar la sala que contenga a este conn_id (sea como emisor o receptor).
            // Para evitar escanear todas, podríamos haber guardado room_name en room_state,
            // pero como las salas son pocas en un servidor típico, el lineal es suficiente.
            let target_tx: Option<mpsc::UnboundedSender<String>> = rooms.values()
                .find_map(|room| {
                    if room.sender_conn_id == to {
                        Some(room.sender_tx.clone())
                    } else if let Some((_, _, rx_tx)) = room.receivers.get(&to) {
                        Some(rx_tx.clone())
                    } else {
                        None
                    }
                });
            drop(rooms);
            let Some(target_tx) = target_tx else {
                // El destino no existe (se fue). Notificar al caller.
                let _ = tx.send(serde_json::json!({
                    "type": "peer_left", "peer_id": to
                }).to_string());
                return;
            };
            // Reenviar el mensaje al destino, añadiendo "from" para que el receptor sepa
            // qué PeerConnection corresponde.
            let mut forward = data.clone();
            forward["from"] = serde_json::json!(conn_id);
            let _ = target_tx.send(forward.to_string());
        }

        // ===== El emisor detiene la transmisión =====
        "stop_room" => {
            let membership = room_state.lock().await.clone();
            if let Some(RoomMembership::Sender { room_name }) = membership {
                let mut rooms = state.transmit_rooms.lock().await;
                if let Some(room) = rooms.remove(&room_name) {
                    // Notificar a todos los receptores que la sala cerró.
                    for (_, (_, _, rx_tx)) in &room.receivers {
                        let _ = rx_tx.send(serde_json::json!({
                            "type": "room_closed", "name": room_name
                        }).to_string());
                    }
                    tracing::info!("[transmitir] Sala '{room_name}' cerrada por el emisor");
                }
                *room_state.lock().await = None;
            }
        }

        // ====== Control remoto (Fase 7.1) ======
        // Receptor pide control → notificar al emisor.
        "control_request" => {
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Receiver { room_name }) = membership else { return };
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_name) else { return };
            // Validar que el receptor esté efectivamente conectado (defensivo).
            if !room.receivers.contains_key(&conn_id) { return; }
            // Evitar duplicados en la cola.
            if !room.control_request_queue.contains(&conn_id) {
                room.control_request_queue.push(conn_id);
            }
            // Notificar al emisor con el display del receptor.
            let display = room.receivers.get(&conn_id).map(|(_, d, _)| d.clone()).unwrap_or_default();
            let _ = room.sender_tx.send(serde_json::json!({
                "type": "control_requested",
                "peer_id": conn_id,
                "peer_display": display,
            }).to_string());
            // Confirmar al receptor que su solicitud está pendiente.
            let _ = tx.send(serde_json::json!({
                "type": "control_pending"
            }).to_string());
        }

        // Emisor aprueba a un receptor específico → ese receptor pasa a ser controller.
        "control_grant" => {
            let peer_id = data["peer_id"].as_i64().unwrap_or(0);
            if peer_id == 0 { return; }
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Sender { room_name }) = membership else { return };
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_name) else { return };
            // Validar que el receptor esté conectado.
            let Some(rx) = room.receivers.get(&peer_id) else { return };
            // Si había un controller previo, se le revoca silenciosamente
            // (el emisor debería haber hecho revoke antes, pero defensive).
            let prev_controller = room.current_controller.replace(peer_id);
            // Sacar al nuevo controller de la cola.
            room.control_request_queue.retain(|&id| id != peer_id);
            // Notificar al nuevo controller.
            let rx_tx = rx.2.clone();
            let rx_display = rx.1.clone();
            let _ = rx_tx.send(serde_json::json!({
                "type": "control_granted"
            }).to_string());
            // Notificar a TODOS los demás receptores (incluso al previo) el nuevo controller.
            let notify = serde_json::json!({
                "type": "controller_changed",
                "controller_id": peer_id,
                "controller_display": rx_display,
            }).to_string();
            for (cid, (_, _, tx2)) in &room.receivers {
                if *cid != peer_id {
                    let _ = tx2.send(notify.clone());
                }
            }
            // Confirmar al emisor el cambio (eco) → el frontend actualiza su panel.
            let _ = room.sender_tx.send(serde_json::json!({
                "type": "controller_changed",
                "controller_id": peer_id,
                "controller_display": rx_display,
            }).to_string());
            let _ = prev_controller; // explicit drop para que el linter no se queje
        }

        // Emisor rechaza a un receptor específico → notificar al receptor.
        "control_deny" => {
            let peer_id = data["peer_id"].as_i64().unwrap_or(0);
            if peer_id == 0 { return; }
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Sender { room_name }) = membership else { return };
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_name) else { return };
            room.control_request_queue.retain(|&id| id != peer_id);
            if let Some((_, _, rx_tx)) = room.receivers.get(&peer_id) {
                let _ = rx_tx.send(serde_json::json!({
                    "type": "control_denied"
                }).to_string());
            }
        }

        // Emisor revoca el control del controller actual.
        "control_revoke" => {
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Sender { room_name }) = membership else { return };
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_name) else { return };
            if let Some(controller_id) = room.current_controller.take() {
                if let Some((_, _, rx_tx)) = room.receivers.get(&controller_id) {
                    let _ = rx_tx.send(serde_json::json!({
                        "type": "control_revoked"
                    }).to_string());
                }
                // Notificar a todos + emisor el cambio.
                let notify = serde_json::json!({
                    "type": "controller_changed",
                    "controller_id": null,
                    "controller_display": "",
                }).to_string();
                for (_, (_, _, tx2)) in &room.receivers {
                    let _ = tx2.send(notify.clone());
                }
                let _ = room.sender_tx.send(notify);
            }
        }

        // Receptor con control lo suelta voluntariamente.
        "control_release" => {
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Receiver { room_name }) = membership else { return };
            let mut rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get_mut(&room_name) else { return };
            // Solo el controller actual puede soltar.
            if room.current_controller == Some(conn_id) {
                room.current_controller = None;
                // Notificar al emisor.
                let _ = room.sender_tx.send(serde_json::json!({
                    "type": "controller_changed",
                    "controller_id": null,
                    "controller_display": "",
                }).to_string());
                // Notificar a los demás receptores también.
                let notify = serde_json::json!({
                    "type": "controller_changed",
                    "controller_id": null,
                    "controller_display": "",
                }).to_string();
                for (cid, (_, _, tx2)) in &room.receivers {
                    if *cid != conn_id {
                        let _ = tx2.send(notify.clone());
                    }
                }
            }
        }

        // Receptor (controller) envía un input event (mouse move/click/scroll).
        // El server SOLO lo reenvía al emisor si es el controller actual.
        "control_input" => {
            let membership = room_state.lock().await.clone();
            let Some(RoomMembership::Receiver { room_name }) = membership else { return };
            let rooms = state.transmit_rooms.lock().await;
            let Some(room) = rooms.get(&room_name) else { return };
            // Validación crítica: solo el controller actual puede mandar inputs.
            if room.current_controller != Some(conn_id) { return; }
            // Reenviar el mensaje al emisor, añadiendo from para identificación.
            let mut forward = data.clone();
            forward["from"] = serde_json::json!(conn_id);
            let _ = room.sender_tx.send(forward.to_string());
        }

        _ => {}
    }
}

/// Limpieza cuando una conexión WS se cierra (página cerrada, refresh, red caída).
/// Si era emisor → cerrar la sala entera. Si era receptor → notificar al emisor
/// para que cierre ese PeerConnection específico.
async fn cleanup_membership(
    state: &AppState,
    conn_id: i64,
    _user_id: i64,
    user_display: &str,
    room_state: &Arc<Mutex<Option<RoomMembership>>>,
) {
    let membership = room_state.lock().await.take();
    let Some(membership) = membership else { return };

    match membership {
        RoomMembership::Sender { room_name } => {
            let mut rooms = state.transmit_rooms.lock().await;
            if let Some(room) = rooms.remove(&room_name) {
                for (_, (_, _, rx_tx)) in &room.receivers {
                    let _ = rx_tx.send(serde_json::json!({
                        "type": "room_closed", "name": room_name
                    }).to_string());
                }
                tracing::info!("[transmitir] Sala '{room_name}' cerrada (emisor se desconectó)");
            }
        }
        RoomMembership::Receiver { room_name } => {
            let mut rooms = state.transmit_rooms.lock().await;
            if let Some(room) = rooms.get_mut(&room_name) {
                // Sacar al receptor de la lista
                if room.receivers.remove(&conn_id).is_some() {
                    // Si este receptor era el controller actual → revocar.
                    let was_controller = room.current_controller == Some(conn_id);
                    if was_controller {
                        room.current_controller = None;
                    }
                    // Sacarlo de la cola de requests pendientes.
                    room.control_request_queue.retain(|&id| id != conn_id);

                    // Notificar al emisor para que cierre ese PeerConnection.
                    let _ = room.sender_tx.send(serde_json::json!({
                        "type": "peer_left", "peer_id": conn_id
                    }).to_string());

                    // Si era el controller → notificar a todos que ya no hay controller.
                    if was_controller {
                        let notify = serde_json::json!({
                            "type": "controller_changed",
                            "controller_id": null,
                            "controller_display": "",
                        }).to_string();
                        let _ = room.sender_tx.send(notify.clone());
                        for (_, (_, _, tx2)) in &room.receivers {
                            let _ = tx2.send(notify.clone());
                        }
                    }

                    tracing::info!(
                        "[transmitir] Receptor {conn_id} ({user_display}) salió de '{room_name}'{}",
                        if was_controller { " (era controller — revocado)" } else { "" }
                    );
                }
            }
        }
    }
}

// ====== Util: SHA-256 hex de un string (para password de salas) ======

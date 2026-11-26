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


pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, user.id))
}

async fn handle_socket(socket: ws::WebSocket, state: AppState, user_id: i64) {
    let (sender, mut receiver) = socket.split();

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Store connection
    state.ws_connections.lock().await.insert(user_id, tx.clone());

    // Notify user is online
    broadcast_user_status(&state, user_id, true).await;

    let mut sender = sender;
    // Spawn task to forward channel messages to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(WsMessage::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Receive messages from client
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            WsMessage::Text(text) => {
                handle_ws_text(&state, user_id, &text).await;
            }
            WsMessage::Close(_) => break,
            _ => {}
        }
    }

    // Clean up
    state.ws_connections.lock().await.remove(&user_id);
    broadcast_user_status(&state, user_id, false).await;
    send_task.abort();
}

async fn broadcast_user_status(state: &AppState, user_id: i64, online: bool) {
    let msg = serde_json::json!({
        "type": "user_status",
        "user_id": user_id,
        "online": online
    }).to_string();
    let conns = state.ws_connections.lock().await;
    for (_, tx) in conns.iter() {
        let _ = tx.send(msg.clone());
    }
}

async fn handle_ws_text(state: &AppState, user_id: i64, text: &str) {
    let Ok(data) = serde_json::from_str::<serde_json::Value>(text) else { return };
    let msg_type = data["type"].as_str().unwrap_or("");

    match msg_type {
        "message" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            let body = data["body"].as_str().unwrap_or("").to_string();
            if conv_id == 0 || body.trim().is_empty() { return; }

            if !state.db.is_conversation_member(conv_id, user_id).await { return; }

            // Permission check: ¿el usuario tiene permiso de enviar mensajes?
            // Solo para grupos (DMs siempre pueden). Si no tiene rango, default permitir.
            if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user_id).await {
                if rank.can_send_messages == 0 {
                    // No tiene permiso → enviar error solo a ese usuario
                    if let Some(tx) = state.ws_connections.lock().await.get(&user_id) {
                        let _ = tx.send(serde_json::json!({
                            "type": "error",
                            "message": "No tienes permiso para enviar mensajes en este grupo"
                        }).to_string());
                    }
                    return;
                }
            }

            let msg_id = state.db.send_message(conv_id, user_id, &body).await.unwrap_or(0);
            if msg_id == 0 { return; }

            let sender = state.db.find_user_by_id(user_id).await.ok().flatten();
            let sender_name = sender.as_ref().map(|u| u.display()).unwrap_or_default();
            let sender_avatar = sender.as_ref().map(|u| u.avatar()).unwrap_or_default();

            let created_at = chrono::Utc::now().format("%H:%M").to_string();
            let created_iso = chrono::Utc::now().to_rfc3339();

            // Si algún otro miembro está en línea → marcar como 'delivered'
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let other_online = {
                let conns = state.ws_connections.lock().await;
                members.iter().any(|&m| m != user_id && conns.contains_key(&m))
            };
            let status = if other_online { "delivered" } else { "sent" };
            if other_online {
                let _ = state.db.mark_messages_delivered(conv_id).await;
            }

            let conns = state.ws_connections.lock().await;
            // Miembros SIN WS activo (excluyendo al sender): a ellos no llega
            // el mensaje en tiempo real → les disparamos un Web Push para que
            // se enteren aunque tengan la pestaña cerrada.
            let mut offline_members: Vec<i64> = Vec::new();
            for member_id in members {
                if let Some(tx) = conns.get(&member_id) {
                    let msg_for_user = if member_id == user_id {
                        serde_json::json!({
                            "type": "message",
                            "id": msg_id,
                            "conversation_id": conv_id,
                            "sender_id": user_id,
                            "sender_name": sender_name,
                            "sender_avatar": sender_avatar,
                            "body": body,
                            "time": created_at,
                            "created_iso": created_iso,
                            "is_self": true,
                            "status": status
                        }).to_string()
                    } else {
                        serde_json::json!({
                            "type": "message",
                            "id": msg_id,
                            "conversation_id": conv_id,
                            "sender_id": user_id,
                            "sender_name": sender_name,
                            "sender_avatar": sender_avatar,
                            "body": body,
                            "time": created_at,
                            "created_iso": created_iso,
                            "status": status
                        }).to_string()
                    };
                    let _ = tx.send(msg_for_user);
                } else if member_id != user_id {
                    // Sin WS → candidato a push.
                    offline_members.push(member_id);
                }
            }
            drop(conns);  // liberar el lock antes de tareas async de push

            // Web Push a los miembros offline. Preview del body legible (igual
            // que en el frontend): los prefijos [IMAGE]/[VOICE]/[FILE] se
            // traducen a texto, y se trunca a 120 chars.
            if !offline_members.is_empty() {
                // Clonamos el Arc<Option<PushManager>> (barato) para moverlo
                // a la tarea; PushManager en sí no es Clone (isahc no lo es),
                // pero el Arc nos da una ref compartida al cliente reutilizable.
                let push_arc = state.push.clone();
                let db = state.db.clone();
                let body_preview = if body.starts_with("[IMAGE]") {
                    "📷 Imagen".to_string()
                } else if body.starts_with("[VOICE]") {
                    "🎙️ Nota de voz".to_string()
                } else if body.starts_with("[FILE]") {
                    "📎 Archivo".to_string()
                } else {
                    let b: String = body.chars().take(120).collect();
                    if body.chars().count() > 120 { b + "…" } else { b }
                };
                let payload = serde_json::json!({
                    "title": sender_name,
                    "body": body_preview,
                    "tag": format!("msg-{}", conv_id),
                    "type": "message",
                    "conversation_id": conv_id,
                    "url": "/chat",
                    "requireInteraction": false
                });
                tokio::spawn(async move {
                    if let Some(pm) = push_arc.as_ref() {
                        for uid in offline_members {
                            pm.send_to_user(&db, uid, &payload).await;
                        }
                    }
                });
            }
        }
        "typing" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let sender = state.db.find_user_by_id(user_id).await.ok().flatten();
            let sender_name = sender.as_ref().map(|u| u.display()).unwrap_or_default();
            // Broadcast "typing" a todos los miembros excepto al remitente
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let conns = state.ws_connections.lock().await;
            for member_id in members {
                if member_id != user_id {
                    if let Some(tx) = conns.get(&member_id) {
                        let _ = tx.send(serde_json::json!({
                            "type": "typing",
                            "conversation_id": conv_id,
                            "sender_id": user_id,
                            "sender_name": sender_name
                        }).to_string());
                    }
                }
            }
        }
        "read" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id > 0 {
                let _ = state.db.mark_conversation_read(conv_id, user_id).await;
            }
        }

        // ===== Señalización de llamadas P2P (WebRTC) =====
        // El servidor solo enruta los mensajes entre los miembros de la conv.
        // El audio/vídeo viaja P2P (DTLS-SRTP), como en Transmitir.
        //
        // call_start: el usuario inicia una llamada en una conv DM.
        //   payload: { conversation_id, call_type: "voice"|"video" }
        //   → server envía { type:"call_incoming", from_user_id, from_name,
        //     from_avatar, conversation_id, call_type, call_id } al OTRO miembro.
        //   → server responde al caller con { type:"call_ringing", call_id }.
        "call_start" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            let call_type = data["call_type"].as_str().unwrap_or("voice").to_string();
            if conv_id == 0 { return; }
            if !state.db.is_conversation_member(conv_id, user_id).await { return; }
            // Solo DMs admiten llamadas por ahora (1:1). Para grupos habría que
            // hacer mesh / SFU — fuera de scope de esta iteración.
            let conv = state.db.get_conversation(conv_id).await.ok().flatten();
            let Some(conv) = conv else { return };
            if conv.conv_type != "dm" {
                if let Some(tx) = state.ws_connections.lock().await.get(&user_id) {
                    let _ = tx.send(serde_json::json!({
                        "type": "error",
                        "message": "Las llamadas solo están disponibles en mensajes directos (DM) por ahora."
                    }).to_string());
                }
                return;
            }
            // Encontrar al otro miembro
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let callee = members.into_iter().find(|&m| m != user_id);
            let Some(callee_id) = callee else { return };

            // call_id aleatorio para esta sesión de señalización
            let call_id = crate::db::random_hex(8);
            let caller = state.db.find_user_by_id(user_id).await.ok().flatten();
            let from_name = caller.as_ref().map(|u| u.display()).unwrap_or_default();
            let from_avatar = caller.as_ref().map(|u| u.avatar()).unwrap_or_default();

            // ¿El callee está en línea? Si no, avisar al caller.
            let callee_online = state.ws_connections.lock().await.contains_key(&callee_id);
            if !callee_online {
                if let Some(tx) = state.ws_connections.lock().await.get(&user_id) {
                    let _ = tx.send(serde_json::json!({
                        "type": "call_unavailable",
                        "call_id": call_id,
                        "conversation_id": conv_id,
                        "reason": "offline"
                    }).to_string());
                }
                // Web Push de "llamada perdida" al callee: aunque no pueda
                // contestar en tiempo real (sin WS no hay señalización
                // WebRTC), al menos se entera de que le llamaron y puede
                // devolver la llamada al abrir la app. requireInteraction=false
                // (no es urgente bloquear; el caller ya vio "offline").
                let push_arc = state.push.clone();
                let db = state.db.clone();
                let payload = serde_json::json!({
                    "title": from_name.clone(),
                    "body": "📞 Llamada perdida",
                    "tag": format!("missed-call-{}", call_id),
                    "type": "call",
                    "conversation_id": conv_id,
                    "url": "/chat",
                    "requireInteraction": false
                });
                let callee_id_cap = callee_id;
                tokio::spawn(async move {
                    if let Some(pm) = push_arc.as_ref() {
                        pm.send_to_user(&db, callee_id_cap, &payload).await;
                    }
                });
                return;
            }

            // Ringing al caller
            if let Some(tx) = state.ws_connections.lock().await.get(&user_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_ringing",
                    "call_id": call_id,
                    "conversation_id": conv_id,
                    "callee_id": callee_id
                }).to_string());
            }
            // Incoming al callee
            if let Some(tx) = state.ws_connections.lock().await.get(&callee_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_incoming",
                    "call_id": call_id,
                    "conversation_id": conv_id,
                    "from_user_id": user_id,
                    "from_name": from_name,
                    "from_avatar": from_avatar,
                    "call_type": call_type
                }).to_string());
            }
        }
        // call_accept: el callee acepta la llamada entrante.
        //   payload: { call_id, conversation_id }
        //   → server avisa al caller { type:"call_accepted", call_id } para que
        //     empiece a crear la PeerConnection y enviar el SDP offer.
        "call_accept" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let caller_id = members.into_iter().find(|&m| m != user_id);
            let Some(caller_id) = caller_id else { return };
            if let Some(tx) = state.ws_connections.lock().await.get(&caller_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_accepted",
                    "call_id": data["call_id"],
                    "conversation_id": conv_id,
                    "callee_id": user_id
                }).to_string());
            }
        }
        // call_reject: el callee rechaza.
        //   → server avisa al caller { type:"call_rejected", call_id, reason }.
        "call_reject" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let caller_id = members.into_iter().find(|&m| m != user_id);
            let Some(caller_id) = caller_id else { return };
            if let Some(tx) = state.ws_connections.lock().await.get(&caller_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_rejected",
                    "call_id": data["call_id"],
                    "conversation_id": conv_id,
                    "reason": data["reason"].as_str().unwrap_or("declined")
                }).to_string());
            }
        }
        // call_sdp: intercambio de offer/answer entre los dos peers.
        //   payload: { call_id, conversation_id, sdp_type: "offer"|"answer", sdp }
        //   → server lo reenvía al otro miembro tal cual.
        "call_sdp" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let other_id = members.into_iter().find(|&m| m != user_id);
            let Some(other_id) = other_id else { return };
            if let Some(tx) = state.ws_connections.lock().await.get(&other_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_sdp",
                    "call_id": data["call_id"],
                    "conversation_id": conv_id,
                    "from_user_id": user_id,
                    "sdp_type": data["sdp_type"],
                    "sdp": data["sdp"]
                }).to_string());
            }
        }
        // call_ice: intercambio de ICE candidates.
        //   payload: { call_id, conversation_id, candidate }
        "call_ice" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let other_id = members.into_iter().find(|&m| m != user_id);
            let Some(other_id) = other_id else { return };
            if let Some(tx) = state.ws_connections.lock().await.get(&other_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_ice",
                    "call_id": data["call_id"],
                    "conversation_id": conv_id,
                    "from_user_id": user_id,
                    "candidate": data["candidate"]
                }).to_string());
            }
        }
        // call_end: cualquiera cuelga.
        //   → server avisa al otro { type:"call_end", call_id }.
        "call_end" => {
            let conv_id = data["conversation_id"].as_i64().unwrap_or(0);
            if conv_id == 0 || !state.db.is_conversation_member(conv_id, user_id).await { return; }
            let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
            let other_id = members.into_iter().find(|&m| m != user_id);
            let Some(other_id) = other_id else { return };
            if let Some(tx) = state.ws_connections.lock().await.get(&other_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "call_end",
                    "call_id": data["call_id"],
                    "conversation_id": conv_id,
                    "from_user_id": user_id
                }).to_string());
            }
        }
        _ => {}
    }
}

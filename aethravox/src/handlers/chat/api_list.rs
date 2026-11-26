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



// ===== API: List conversations =====
#[derive(Serialize)]
struct ConvList {
    id: i64,
    conv_type: String,
    name: String,
    description: Option<String>,
    image_url: Option<String>,
    last_message: String,
    last_message_at: String,
    unread_count: i64,
}

pub async fn api_conversations(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let convs = state.db.list_user_conversations(user.id).await.unwrap_or_default();
    let mut result = Vec::new();

    for c in convs {
        let last_msg = state.db.get_last_message(c.id).await.ok().flatten();
        let unread = state.db.count_unread_in_conversation(c.id, user.id).await.unwrap_or(0);

        // For DMs, get the other user's name + avatar. For groups, use conversation data.
        let (name, image_url) = if c.conv_type == "dm" {
            let members = state.db.get_conversation_members(c.id).await.unwrap_or_default();
            let other_id = members.into_iter().find(|&m| m != user.id).unwrap_or(0);
            let other = state.db.find_user_by_id(other_id).await.ok().flatten();
            let avatar = other.as_ref().map(|u| u.avatar());
            let display_name = other.map(|u| u.display()).unwrap_or_else(|| "Usuario".into());
            (display_name, avatar)
        } else {
            (c.name.clone().unwrap_or_else(|| "Grupo".into()), c.image_url.clone())
        };

        result.push(ConvList {
            id: c.id,
            conv_type: c.conv_type,
            name,
            description: c.description.clone(),
            image_url,
            last_message: last_msg.as_ref().map(|m| m.body.clone()).unwrap_or_default(),
            last_message_at: last_msg.as_ref().map(|m| m.created_at.format("%H:%M").to_string()).unwrap_or_default(),
            unread_count: unread,
        });
    }

    Json(result).into_response()
}

// ===== API: Listar comunidades del usuario con sus grupos (árbol para el sidebar del chat)
// Devuelve [{ id, name, icon, description, groups: [{ id, name, description, unread }] }]
// Esto permite al chat mostrar comunidades → grupos como canales de Discord/Slack.
#[derive(Serialize)]
struct CommunityGroupListItem {
    id: i64,
    name: String,
    description: String,
    unread: i64,
}
#[derive(Serialize)]
struct CommunityListItem {
    id: i64,
    name: String,
    icon: String,
    description: String,
    groups: Vec<CommunityGroupListItem>,
}

pub async fn api_chat_communities(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let comms = state.db.list_user_communities(user.id).await.unwrap_or_default();
    let mut result = Vec::with_capacity(comms.len());
    for c in comms {
        let groups_db = state.db.list_community_conversations(c.id).await.unwrap_or_default();
        let mut groups = Vec::with_capacity(groups_db.len());
        for g in groups_db {
            let unread = state.db.count_unread_in_conversation(g.id, user.id).await.unwrap_or(0);
            groups.push(CommunityGroupListItem {
                id: g.id,
                name: g.name.clone().unwrap_or_else(|| "Grupo".into()),
                description: g.description.clone().unwrap_or_default(),
                unread,
            });
        }
        result.push(CommunityListItem {
            id: c.id,
            name: c.name,
            icon: c.icon.unwrap_or_else(|| "🏘️".into()),
            description: c.description.unwrap_or_default(),
            groups,
        });
    }
    Json(result).into_response()
}

// ===== API: Get messages =====
#[derive(Serialize)]
struct MsgList {
    id: i64,
    sender_id: i64,
    sender_name: String,
    sender_avatar: String,
    body: String,
    time: String,
    /// Timestamp ISO 8601 (UTC) del envío. El frontend lo usa para decidir
    /// si todavía se puede editar (dentro de 15 min) y para ordenar.
    created_iso: String,
    is_self: bool,
    status: String,
    edited: bool,
    deleted: bool,
}

pub async fn api_messages(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if !state.db.is_conversation_member(id, user.id).await {
        return Json(Vec::<MsgList>::new()).into_response();
    }

    let msgs = state.db.list_messages(id, 200).await.unwrap_or_default();

    // Pre-fetch en batch los nombres reales de todos los senders distintos.
    // Evita mostrar "User 3" en vez de "Andru". Cachea en HashMap para no
    // repetir lookups si el mismo usuario envió varios mensajes.
    let sender_ids: std::collections::HashSet<i64> =
        msgs.iter().map(|m| m.sender_id).collect();
    let mut sender_info: HashMap<i64, (String, String)> = HashMap::new();
    for sid in sender_ids {
        if let Ok(Some(u)) = state.db.find_user_by_id(sid).await {
            sender_info.insert(sid, (u.display(), u.avatar()));
        }
    }

    let result: Vec<MsgList> = msgs.iter().map(|m| {
        let info = sender_info.get(&m.sender_id);
        // Si el mensaje está borrado → el body en la DB ya viene vacío.
        // El frontend lo renderiza como "Mensaje eliminado".
        let display_body = if m.deleted_at.is_some() {
            String::from("")
        } else {
            m.body.clone()
        };
        MsgList {
            id: m.id,
            sender_id: m.sender_id,
            sender_name: info.map(|(n,_)| n.clone()).unwrap_or_else(|| format!("User {}", m.sender_id)),
            sender_avatar: info.map(|(_,a)| a.clone()).unwrap_or_default(),
            body: display_body,
            time: m.created_at.format("%H:%M").to_string(),
            created_iso: m.created_at.to_rfc3339(),
            is_self: m.sender_id == user.id,
            status: m.status.clone(),
            edited: m.edited_at.is_some(),
            deleted: m.deleted_at.is_some(),
        }
    }).collect();

    Json(result).into_response()
}

// ===== API: Mark read =====
pub async fn api_mark_read(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let _ = state.db.mark_conversation_read(id, user.id).await;

    // Notificar a todos los miembros de la conversación que los mensajes fueron leídos.
    // Esto permite que el sender vea el cambio "entregado → leído" en tiempo real
    // sin tener que recargar la página.
    let members = state.db.get_conversation_members(id).await.unwrap_or_default();
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if member_id != user.id {
            if let Some(tx) = conns.get(&member_id) {
                let _ = tx.send(serde_json::json!({
                    "type": "status_update",
                    "conversation_id": id,
                    "reader_id": user.id,
                    "status": "read"
                }).to_string());
            }
        }
    }

    Json(serde_json::json!({"ok": true})).into_response()
}

// ===== API: Edit message (autor, dentro de 30 min) =====
#[derive(Debug, serde::Deserialize)]
pub struct EditMsgForm {
    pub body: String,
}
pub async fn api_edit_message(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(msg_id): Path<i64>,
    Form(form): Form<EditMsgForm>,
) -> Response {
    let new_body = form.body.trim();
    if new_body.is_empty() {
        return Json(serde_json::json!({"error": "El mensaje no puede estar vacío"})).into_response();
    }
    if new_body.chars().count() > 4000 {
        return Json(serde_json::json!({"error": "Mensaje demasiado largo (máx 4000 caracteres)"})).into_response();
    }

    // Intentar editar (la DB valida autor + 15 min + no borrado).
    let ok = state.db.edit_message(msg_id, user.id, new_body).await.unwrap_or(false);
    if !ok {
        return Json(serde_json::json!({"error": "No se puede editar: solo puedes editar tus propios mensajes dentro de los 15 minutos posteriores al envío"})).into_response();
    }

    // Broadcast a todos los miembros de la conversación: "mensaje editado".
    let msg = match state.db.get_message(msg_id).await {
        Ok(Some(m)) => m,
        _ => return Json(serde_json::json!({"error": "Mensaje no encontrado"})).into_response(),
    };
    let conv_id = msg.conversation_id;
    let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
    let edited_time = msg.edited_at.as_ref()
        .and_then(|d| Some(d.format("%H:%M").to_string()))
        .unwrap_or_else(|| chrono::Utc::now().format("%H:%M").to_string());
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if let Some(tx) = conns.get(&member_id) {
            let _ = tx.send(serde_json::json!({
                "type": "message_edited",
                "id": msg_id,
                "conversation_id": conv_id,
                "body": new_body,
                "edited_at": edited_time,
            }).to_string());
        }
    }

    Json(serde_json::json!({"ok": true, "id": msg_id, "body": new_body})).into_response()
}

// ===== API: Delete message (autor 30 min / admin grupo con can_delete_messages) =====
pub async fn api_delete_message(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(msg_id): Path<i64>,
) -> Response {
    // Cargar el mensaje para saber a qué conversación pertenece.
    let msg = match state.db.get_message(msg_id).await {
        Ok(Some(m)) => m,
        _ => return Json(serde_json::json!({"error": "Mensaje no encontrado"})).into_response(),
    };
    let conv_id = msg.conversation_id;

    // ¿Es el autor?
    let is_author = msg.sender_id == user.id;
    // ¿Es miembro de la conversación?
    if !state.db.is_conversation_member(conv_id, user.id).await {
        return Json(serde_json::json!({"error": "No eres miembro de esta conversación"})).into_response();
    }

    let ok = if is_author {
        // Autor: puede borrar dentro de 30 min.
        state.db.soft_delete_message(msg_id, user.id).await.unwrap_or(false)
    } else {
        // No es autor: (a) rango del grupo con can_delete_messages, o
        // (b) permiso del hub `chat.moderate` (rol configurable).
        let rank = state.db.get_member_rank(conv_id, user.id).await.ok().flatten();
        let group_can_delete = rank.as_ref().map(|r| r.can_delete_messages != 0 || r.is_owner_rank != 0).unwrap_or(false);
        let hub_can_moderate = state.db.user_has_perm(&user, "chat.moderate").await;
        if !group_can_delete && !hub_can_moderate {
            return Json(serde_json::json!({"error": "No tienes permiso para borrar mensajes ajenos"})).into_response();
        }
        if !group_can_delete && hub_can_moderate {
            let _ = state.db.audit(Some(user.id), "chat_msg_delete_mod",
                Some(&format!("msg={},conv={}", msg_id, conv_id)), None).await;
        }
        state.db.soft_delete_message_admin(msg_id).await.unwrap_or(false)
    };

    if !ok {
        return Json(serde_json::json!({"error": "No se pudo borrar (¿ya estaba borrado o pasaron 30 min?)"})).into_response();
    }

    // Broadcast a todos los miembros: "mensaje borrado".
    let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if let Some(tx) = conns.get(&member_id) {
            let _ = tx.send(serde_json::json!({
                "type": "message_deleted",
                "id": msg_id,
                "conversation_id": conv_id,
            }).to_string());
        }
    }

    Json(serde_json::json!({"ok": true, "id": msg_id})).into_response()
}

// ===== API: Borrado múltiple (multi-select).
// Recibe { ids: [1,2,3...] } en JSON. Aplica las mismas reglas que el borrado
// individual: autor dentro de 30 min, o admin de grupo con can_delete_messages
// (sin límite de tiempo). Devuelve { ok: [ids_borrados], denied: [ids_rechazados] }.
#[derive(Debug, serde::Deserialize)]
pub struct BulkDeleteForm {
    #[serde(default)]
    pub ids: Vec<i64>,
}
pub async fn api_bulk_delete_messages(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(form): Json<BulkDeleteForm>,
) -> Response {
    if form.ids.is_empty() {
        return Json(serde_json::json!({"error": "No se enviaron IDs"})).into_response();
    }
    if form.ids.len() > 200 {
        return Json(serde_json::json!({"error": "Máximo 200 mensajes por operación"})).into_response();
    }

    let mut ok: Vec<i64> = Vec::new();
    let mut denied: Vec<i64> = Vec::new();
    // Agrupamos los IDs por conversación para un único broadcast al final.
    let mut broadcast_convs: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();

    for &msg_id in &form.ids {
        let msg = match state.db.get_message(msg_id).await {
            Ok(Some(m)) => m,
            _ => { denied.push(msg_id); continue; }
        };
        let conv_id = msg.conversation_id;
        let is_author = msg.sender_id == user.id;
        // Debe ser miembro de la conversación para borrar algo en ella.
        if !state.db.is_conversation_member(conv_id, user.id).await {
            denied.push(msg_id);
            continue;
        }
        let success = if is_author {
            state.db.soft_delete_message(msg_id, user.id).await.unwrap_or(false)
        } else {
            // (a) Rango del grupo con can_delete_messages, o
            // (b) permiso del hub `chat.moderate` (rol configurable).
            let rank = state.db.get_member_rank(conv_id, user.id).await.ok().flatten();
            let group_can_delete = rank.as_ref()
                .map(|r| r.can_delete_messages != 0 || r.is_owner_rank != 0)
                .unwrap_or(false);
            let hub_can_moderate = state.db.user_has_perm(&user, "chat.moderate").await;
            if !group_can_delete && !hub_can_moderate {
                denied.push(msg_id);
                continue;
            }
            state.db.soft_delete_message_admin(msg_id).await.unwrap_or(false)
        };
        if success {
            ok.push(msg_id);
            broadcast_convs.entry(conv_id).or_default().push(msg_id);
        } else {
            denied.push(msg_id);
        }
    }

    // Broadcast "message_deleted" por cada mensaje borrado, a los miembros de su conv.
    let conns = state.ws_connections.lock().await;
    for (conv_id, ids) in &broadcast_convs {
        let members = state.db.get_conversation_members(*conv_id).await.unwrap_or_default();
        for member_id in members {
            if let Some(tx) = conns.get(&member_id) {
                for &id in ids {
                    let _ = tx.send(serde_json::json!({
                        "type": "message_deleted",
                        "id": id,
                        "conversation_id": conv_id,
                    }).to_string());
                }
            }
        }
    }

    Json(serde_json::json!({
        "ok": ok,
        "denied": denied,
        "ok_count": ok.len(),
        "denied_count": denied.len(),
    })).into_response()
}
pub async fn api_message_info(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(msg_id): Path<i64>,
) -> Response {
    let msg = match state.db.get_message(msg_id).await {
        Ok(Some(m)) => m,
        _ => return Json(serde_json::json!({"error": "Mensaje no encontrado"})).into_response(),
    };

    // Solo miembros de la conversación pueden ver info.
    if !state.db.is_conversation_member(msg.conversation_id, user.id).await {
        return Json(serde_json::json!({"error": "Sin permiso"})).into_response();
    }

    // El body puede venir vacío si está borrado; lo filtramos.
    let body = if msg.deleted_at.is_some() { String::new() } else { msg.body.clone() };

    Json(serde_json::json!({
        "id": msg.id,
        "conversation_id": msg.conversation_id,
        "sender_id": msg.sender_id,
        "body": body,
        "created_at": msg.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        "status": msg.status,
        "edited_at": msg.edited_at.as_ref().map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        "deleted_at": msg.deleted_at.as_ref().map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
    })).into_response()
}

// ===== API: Create DM =====

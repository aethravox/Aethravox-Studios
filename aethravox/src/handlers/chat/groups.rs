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


use askama::Template as AskamaTemplate;

#[derive(AskamaTemplate)]
#[template(path = "comunicacion/chat/group_settings.html")]
pub struct GroupSettingsTemplate {
    pub conv_id: i64,
    pub group_name: String,
    pub group_description: String,
    pub username: String,
    pub role: String,
    pub avatar: String,
}

pub async fn group_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
) -> Response {
    if !state.db.is_conversation_member(conv_id, user.id).await {
        return axum::response::Redirect::to("/chat").into_response();
    }
    let conv = state.db.get_conversation(conv_id).await.ok().flatten();
    let Some(conv) = conv else {
        return axum::response::Redirect::to("/chat").into_response();
    };
    GroupSettingsTemplate {
        conv_id,
        group_name: conv.name.unwrap_or_else(|| "Grupo".into()),
        group_description: conv.description.unwrap_or_default(),
        username: user.username.clone(),
        role: user.role.clone(),
        avatar: user.avatar(),
    }.into_response()
}

// API: List ranks
pub async fn api_list_ranks(State(state): State<AppState>, AuthUser(_user): AuthUser, Path(conv_id): Path<i64>) -> Response {
    let ranks = state.db.list_conversation_ranks(conv_id).await.unwrap_or_default();
    let result: Vec<_> = ranks.iter().map(|r| {
        serde_json::json!({
            "id": r.id,
            "name": r.name,
            "color": r.color,
            "position": r.position,
            "is_owner_rank": r.is_owner_rank != 0,
            "can_send_messages": r.can_send_messages != 0,
            "can_add_members": r.can_add_members != 0,
            "can_edit_group_info": r.can_edit_group_info != 0,
            "can_delete_messages": r.can_delete_messages != 0,
            "can_remove_members": r.can_remove_members != 0,
            "can_manage_ranks": r.can_manage_ranks != 0,
            "can_assign_ranks": r.can_assign_ranks != 0,
            "can_pin_messages": r.can_pin_messages != 0,
            "can_transfer_ownership": r.can_transfer_ownership != 0,
        })
    }).collect();
    Json(result).into_response()
}

// API: List members with their ranks
pub async fn api_list_members(State(state): State<AppState>, AuthUser(_user): AuthUser, Path(conv_id): Path<i64>) -> Response {
    let member_ids = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
    let mut result = Vec::new();
    for mid in member_ids {
        if let Ok(Some(u)) = state.db.find_user_by_id(mid).await {
            let rank = state.db.get_member_rank(conv_id, mid).await.ok().flatten();
            let rank_name = rank.as_ref().map(|r| r.name.clone()).unwrap_or_else(|| "Sin rango".into());
            let rank_color = rank.as_ref().and_then(|r| r.color.clone()).unwrap_or_else(|| "#888".into());
            let rank_id = rank.as_ref().map(|r| r.id).unwrap_or(0);
            result.push(serde_json::json!({
                "user_id": u.id,
                "username": u.username,
                "display": u.display(),
                "avatar": u.avatar(),
                "rank_name": rank_name,
                "rank_color": rank_color,
                "rank_id": rank_id,
                "is_owner": rank.as_ref().map(|r| r.is_owner_rank != 0).unwrap_or(false),
            }));
        }
    }
    Json(result).into_response()
}

// API: Set member rank
#[derive(Debug, serde::Deserialize)]
pub struct SetRankForm {
    pub user_id: i64,
    pub rank_id: i64,
}
pub async fn api_set_member_rank(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    Form(form): Form<SetRankForm>,
) -> Response {
    // Check caller has can_assign_ranks
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_assign_ranks == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso para asignar rangos"})).into_response();
        }
    } else {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    let _ = state.db.set_member_rank(conv_id, form.user_id, form.rank_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

// API: Create rank
#[derive(Debug, serde::Deserialize)]
pub struct CreateRankForm {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub can_send_messages: Option<String>,
    #[serde(default)]
    pub can_add_members: Option<String>,
    #[serde(default)]
    pub can_edit_group_info: Option<String>,
    #[serde(default)]
    pub can_delete_messages: Option<String>,
    #[serde(default)]
    pub can_remove_members: Option<String>,
    #[serde(default)]
    pub can_manage_ranks: Option<String>,
    #[serde(default)]
    pub can_assign_ranks: Option<String>,
    #[serde(default)]
    pub can_pin_messages: Option<String>,
    #[serde(default)]
    pub can_transfer_ownership: Option<String>,
}
pub async fn api_create_rank(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    Form(form): Form<CreateRankForm>,
) -> Response {
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_manage_ranks == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso para crear rangos"})).into_response();
        }
    } else {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    let name = form.name.trim();
    if name.is_empty() {
        return Json(serde_json::json!({"error": "Nombre requerido"})).into_response();
    }
    let color = form.color.as_deref().filter(|s| !s.is_empty());
    let id = state.db.create_rank(
        conv_id, name, color,
        form.can_send_messages.is_some(),
        form.can_add_members.is_some(),
        form.can_edit_group_info.is_some(),
        form.can_delete_messages.is_some(),
        form.can_remove_members.is_some(),
        form.can_manage_ranks.is_some(),
        form.can_assign_ranks.is_some(),
        form.can_pin_messages.is_some(),
        form.can_transfer_ownership.is_some(),
    ).await.unwrap_or(0);
    Json(serde_json::json!({"id": id})).into_response()
}

// API: Update rank
pub async fn api_update_rank(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((conv_id, rank_id)): Path<(i64, i64)>,
    Form(form): Form<CreateRankForm>,
) -> Response {
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_manage_ranks == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso"})).into_response();
        }
    } else {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    let _ = state.db.update_rank(
        rank_id, form.name.trim(), form.color.as_deref().filter(|s| !s.is_empty()),
        form.can_send_messages.is_some(),
        form.can_add_members.is_some(),
        form.can_edit_group_info.is_some(),
        form.can_delete_messages.is_some(),
        form.can_remove_members.is_some(),
        form.can_manage_ranks.is_some(),
        form.can_assign_ranks.is_some(),
        form.can_pin_messages.is_some(),
        form.can_transfer_ownership.is_some(),
    ).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

// API: Delete rank
pub async fn api_delete_rank(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((conv_id, rank_id)): Path<(i64, i64)>,
) -> Response {
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_manage_ranks == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso"})).into_response();
        }
    } else {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    match state.db.delete_rank(rank_id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})).into_response(),
        Ok(false) => Json(serde_json::json!({"error": "No se puede eliminar: miembros lo usan o es Owner"})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "Error de DB"})).into_response(),
    }
}

// API: Transfer ownership
#[derive(Debug, serde::Deserialize)]
pub struct TransferForm {
    pub new_owner_id: i64,
}
pub async fn api_transfer_ownership(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    Form(form): Form<TransferForm>,
) -> Response {
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_transfer_ownership == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso para transferir ownership"})).into_response();
        }
    } else {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    let _ = state.db.transfer_ownership(conv_id, form.new_owner_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

// ===== Upload group image (Fase 4) =====

pub async fn upload_group_image(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    mut multipart: Multipart,
) -> Response {
    // Verificar membresía
    if !state.db.is_conversation_member(conv_id, user.id).await {
        return Json(serde_json::json!({"error": "No eres miembro"})).into_response();
    }
    // Verificar que es un grupo (no DM)
    let conv = state.db.get_conversation(conv_id).await.ok().flatten();
    let Some(conv) = conv else {
        return Json(serde_json::json!({"error": "Conversación no encontrada"})).into_response();
    };
    if conv.conv_type != "group" {
        return Json(serde_json::json!({"error": "Solo los grupos pueden tener imagen"})).into_response();
    }
    // Verificar permiso can_edit_group_info
    if let Ok(Some(rank)) = state.db.get_member_rank(conv_id, user.id).await {
        if rank.can_edit_group_info == 0 && rank.is_owner_rank == 0 {
            return Json(serde_json::json!({"error": "No tienes permiso para editar el grupo"})).into_response();
        }
    }

    let mut image_data: Option<Vec<u8>> = None;
    let mut content_type: String = "image/jpeg".into();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "image" {
            content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            if let Ok(bytes) = field.bytes().await {
                image_data = Some(bytes.to_vec());
            }
        }
    }
    let Some(image_data) = image_data else {
        return Json(serde_json::json!({"error": "No se envió imagen"})).into_response();
    };
    if image_data.len() > 5 * 1024 * 1024 {
        return Json(serde_json::json!({"error": "Máximo 5MB"})).into_response();
    }
    let ext = match content_type.as_str() {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    };
    let dir = "data/group_images";
    let _ = std::fs::create_dir_all(dir);
    let temp_path = format!("{}/{}_temp.{}", dir, conv_id, ext);
    let webp_path = format!("{}/{}.webp", dir, conv_id);
    let image_url;
    if std::fs::write(&temp_path, &image_data).is_err() {
        return Json(serde_json::json!({"error": "Error guardando"})).into_response();
    }
    let cwebp = std::process::Command::new("cwebp")
        .arg("-q").arg("80")
        .arg("-resize").arg("400").arg("0")
        .arg(&temp_path)
        .arg("-o").arg(&webp_path)
        .output();
    match cwebp {
        Ok(o) if o.status.success() => {
            let _ = std::fs::remove_file(&temp_path);
            image_url = format!("/group_images/{}.webp?v={}", conv_id, chrono::Utc::now().timestamp());
        }
        _ => {
            let fallback = format!("{}/{}.{}", dir, conv_id, ext);
            let _ = std::fs::rename(&temp_path, &fallback);
            let _ = std::fs::remove_file(&webp_path);
            image_url = format!("/group_images/{}.{}?v={}", conv_id, ext, chrono::Utc::now().timestamp());
        }
    }
    // Actualizar conversations.image_url
    let _ = sqlx::query("UPDATE conversations SET image_url = ? WHERE id = ?")
        .bind(&image_url)
        .bind(conv_id)
        .execute(&state.db.pool)
        .await;
    Json(serde_json::json!({"ok": true, "image_url": image_url})).into_response()
}

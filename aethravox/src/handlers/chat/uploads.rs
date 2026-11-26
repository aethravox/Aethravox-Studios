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


pub async fn upload_chat_image(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    mut multipart: Multipart,
) -> Response {
    // 1. Verificar membresía
    if !state.db.is_conversation_member(conv_id, user.id).await {
        return Json(serde_json::json!({"error": "No eres miembro de esta conversación"})).into_response();
    }

    // 2. Extraer el campo "image" del multipart
    let mut image_data: Option<Vec<u8>> = None;
    let mut content_type: String = "image/jpeg".into();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "image" {
            content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            match field.bytes().await {
                Ok(bytes) => image_data = Some(bytes.to_vec()),
                Err(e) => {
                    return Json(serde_json::json!({"error": format!("Error leyendo imagen: {}", e)}))
                        .into_response();
                }
            }
        }
    }

    let Some(image_data) = image_data else {
        return Json(serde_json::json!({"error": "No se envió ninguna imagen"})).into_response();
    };

    // 3. Validar tamaño (máx 5MB)
    if image_data.len() > 5 * 1024 * 1024 {
        return Json(serde_json::json!({"error": "Imagen demasiado grande (máximo 5MB)"}))
            .into_response();
    }

    // 4. Determinar extensión
    let ext = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "jpg",
    };

    // 5. Guardar a disco y convertir a WebP (más liviano).
    // Usa cwebp (Google WebP encoder) si está instalado. Si no, guarda el original.
    // Para instalar cwebp: sudo apt install webp
    let hex_name = crate::db::random_hex(16);
    let dir = "data/chat_images";
    let _ = std::fs::create_dir_all(dir);
    let original_path = format!("{}/{}_orig.{}", dir, hex_name, ext);
    let webp_path = format!("{}/{}.webp", dir, hex_name);
    let final_filename;

    // Guardar original temporalmente
    if let Err(e) = std::fs::write(&original_path, &image_data) {
        tracing::error!("Error guardando imagen de chat: {e}");
        return Json(serde_json::json!({"error": "Error guardando imagen"})).into_response();
    }

    // Intentar convertir a WebP con cwebp (resize a max 1200px + calidad 80)
    let cwebp_result = std::process::Command::new("cwebp")
        .arg("-q").arg("80")
        .arg("-resize").arg("1200").arg("0")
        .arg(&original_path)
        .arg("-o").arg(&webp_path)
        .output();

    match cwebp_result {
        Ok(output) if output.status.success() => {
            // Conversión exitosa → borrar original, usar .webp
            let _ = std::fs::remove_file(&original_path);
            final_filename = format!("{}.webp", hex_name);
            tracing::info!("Imagen convertida a WebP: {} ({} bytes → {} bytes)",
                final_filename, image_data.len(),
                std::fs::metadata(&webp_path).map(|m| m.len()).unwrap_or(0));
        }
        _ => {
            // cwebp no disponible o falló → usar el original tal cual
            // Renombrar el temp al nombre final
            let fallback_path = format!("{}/{}.{}", dir, hex_name, ext);
            let _ = std::fs::rename(&original_path, &fallback_path);
            final_filename = format!("{}.{}", hex_name, ext);
            tracing::warn!("cwebp no disponible, usando formato original ({})", ext);
        }
    }

    // 6. Crear mensaje en DB con body especial [IMG]
    let body = format!("[IMG]/chat/images/{}", final_filename);
    let msg_id = state.db.send_message(conv_id, user.id, &body).await.unwrap_or(0);
    if msg_id == 0 {
        return Json(serde_json::json!({"error": "Error creando mensaje"})).into_response();
    }

    // 7. Broadcast via WebSocket a todos los miembros
    let time = chrono::Utc::now().format("%H:%M").to_string();
    let sender_name = user.display();
    let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if let Some(tx) = conns.get(&member_id) {
            let msg_for_user = if member_id == user.id {
                serde_json::json!({
                    "type": "message",
                    "id": msg_id,
                    "conversation_id": conv_id,
                    "sender_id": user.id,
                    "sender_name": sender_name,
                    "body": body,
                    "time": time,
                    "is_self": true
                })
            } else {
                serde_json::json!({
                    "type": "message",
                    "id": msg_id,
                    "conversation_id": conv_id,
                    "sender_id": user.id,
                    "sender_name": sender_name,
                    "body": body,
                    "time": time
                })
            };
            let _ = tx.send(msg_for_user.to_string());
        }
    }

    // 8. Devolver el mensaje creado (para que el remitente lo muestre)
    Json(serde_json::json!({
        "id": msg_id,
        "conversation_id": conv_id,
        "sender_id": user.id,
        "sender_name": sender_name,
        "body": body,
        "time": time,
        "is_self": true
    })).into_response()
}

// ===== POST /api/chat/:id/upload_file — Subir archivo genérico (video, audio,
// documento, nota de voz, cualquier MIME). Reutiliza el patrón de upload_chat_image
// pero sin forzar la conversión a WebP. Para notas de voz el frontend graba con
// MediaRecorder y sube aquí; el body del mensaje usa el prefijo [VOICE] para que
// el cliente pueda renderizar un reproductor. Para otros archivos se usa [FILE].
//
// Multipart esperado:
//   - "file": bytes del archivo
//   - "kind": "file" | "voice" (default "file")
//   - "name": nombre original visible (opcional, default "archivo")
//   - "duration": segundos de audio, solo si kind=voice (opcional)
pub async fn upload_chat_file(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(conv_id): Path<i64>,
    mut multipart: Multipart,
) -> Response {
    // 1. Verificar membresía
    if !state.db.is_conversation_member(conv_id, user.id).await {
        return Json(serde_json::json!({"error": "No eres miembro de esta conversación"})).into_response();
    }

    let mut file_data: Option<Vec<u8>> = None;
    let mut content_type: String = "application/octet-stream".into();
    let mut kind: String = "file".into();
    let mut orig_name: String = "archivo".into();
    let mut duration: Option<i64> = None;
    // Máx 60 MB para archivos (videos pueden pesar). El límite global del body
    // ya está a 200 MB en main.rs.
    const MAX_FILE_BYTES: usize = 60 * 1024 * 1024;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                match field.bytes().await {
                    Ok(bytes) => file_data = Some(bytes.to_vec()),
                    Err(e) => {
                        return Json(serde_json::json!({"error": format!("Error leyendo archivo: {}", e)}))
                            .into_response();
                    }
                }
            }
            "kind" => {
                if let Ok(b) = field.bytes().await {
                    kind = String::from_utf8_lossy(&b).to_string();
                }
            }
            "name" => {
                if let Ok(b) = field.bytes().await {
                    orig_name = String::from_utf8_lossy(&b).to_string();
                }
            }
            "duration" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        duration = s.parse::<i64>().ok();
                    }
                }
            }
            _ => { let _ = field.bytes().await; }
        }
    }

    let Some(file_data) = file_data else {
        return Json(serde_json::json!({"error": "No se envió ningún archivo"})).into_response();
    };

    if file_data.len() > MAX_FILE_BYTES {
        return Json(serde_json::json!({"error": format!("Archivo demasiado grande (máximo {} MB)", MAX_FILE_BYTES / 1024 / 1024)}))
            .into_response();
    }

    // 2. Determinar extensión segura según el MIME. Solo permitimos extensiones
    // de archivo conocidas para evitar path traversal / ejecutables.
    let ext = mime_to_ext(&content_type, &orig_name);
    let hex_name = crate::db::random_hex(16);
    let dir = "data/chat_files";
    let _ = std::fs::create_dir_all(dir);
    let final_filename = format!("{}.{}", hex_name, ext);
    let path = format!("{}/{}", dir, final_filename);

    if let Err(e) = std::fs::write(&path, &file_data) {
        tracing::error!("Error guardando archivo de chat: {e}");
        return Json(serde_json::json!({"error": "Error guardando archivo"})).into_response();
    }

    // 3. Body del mensaje con prefijo según el tipo. El frontend parsea estos
    // prefijos en formatBody() para renderizar reproductor/player/link.
    // [FILE]/url:orig_name:mime:size
    // [VOICE]/url:duration
    let body = if kind == "voice" {
        let dur = duration.unwrap_or(0);
        format!("[VOICE]/chat/files/{}:{}", final_filename, dur)
    } else {
        // Sanear orig_name: quitar saltos de línea y limitar longitud.
        let clean_name = orig_name.chars().filter(|c| *c != '\n' && *c != '\r').take(120).collect::<String>();
        format!("[FILE]/chat/files/{}:{}:{}:{}",
            final_filename, clean_name, content_type, file_data.len())
    };

    let msg_id = state.db.send_message(conv_id, user.id, &body).await.unwrap_or(0);
    if msg_id == 0 {
        return Json(serde_json::json!({"error": "Error creando mensaje"})).into_response();
    }

    // 4. Broadcast via WebSocket a todos los miembros
    let time = chrono::Utc::now().format("%H:%M").to_string();
    let sender_name = user.display();
    let members = state.db.get_conversation_members(conv_id).await.unwrap_or_default();
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if let Some(tx) = conns.get(&member_id) {
            let msg_for_user = if member_id == user.id {
                serde_json::json!({
                    "type": "message",
                    "id": msg_id,
                    "conversation_id": conv_id,
                    "sender_id": user.id,
                    "sender_name": sender_name,
                    "body": body,
                    "time": time,
                    "is_self": true
                })
            } else {
                serde_json::json!({
                    "type": "message",
                    "id": msg_id,
                    "conversation_id": conv_id,
                    "sender_id": user.id,
                    "sender_name": sender_name,
                    "body": body,
                    "time": time
                })
            };
            let _ = tx.send(msg_for_user.to_string());
        }
    }

    Json(serde_json::json!({
        "id": msg_id,
        "conversation_id": conv_id,
        "sender_id": user.id,
        "sender_name": sender_name,
        "body": body,
        "time": time,
        "is_self": true
    })).into_response()
}

/// Mapea un Content-Type (o, en fallback, la extensión del nombre original) a
/// una extensión segura. Si no coincide con nada conocido, se usa "bin".
/// Esto evita guardar .html / .svg / .js que podrían ejecutarse al servirse.
fn mime_to_ext(mime: &str, orig_name: &str) -> &'static str {
    // Primero por MIME (más fiable que la extensión que mande el cliente).
    match mime {
        "image/jpeg" | "image/jpg" => return "jpg",
        "image/png" => return "png",
        "image/gif" => return "gif",
        "image/webp" => return "webp",
        "image/bmp" => return "bmp",
        "video/mp4" => return "mp4",
        "video/webm" => return "webm",
        "video/ogg" => return "ogv",
        "video/quicktime" => return "mov",
        "audio/mpeg" | "audio/mp3" => return "mp3",
        "audio/ogg" => return "oga",
        "audio/wav" | "audio/x-wav" => return "wav",
        "audio/webm" => return "weba",
        "audio/aac" => return "aac",
        "audio/mp4" => return "m4a",
        "application/pdf" => return "pdf",
        "application/zip" => return "zip",
        "application/x-zip-compressed" => return "zip",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => return "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => return "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => return "pptx",
        "text/plain" => return "txt",
        "text/csv" => return "csv",
        "application/json" => return "json",
        "application/msword" => return "doc",
        "application/vnd.ms-excel" => return "xls",
        "application/vnd.ms-powerpoint" => return "ppt",
        _ => {}
    }
    // Fallback: inspeccionar la extensión del nombre original del archivo.
    // Solo aceptamos algunas seguras; resto → bin.
    let lower = orig_name.to_lowercase();
    let after_dot = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    match after_dot {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => "img",
        "mp4" | "webm" | "ogv" | "mov" | "m4v" => "mp4",
        "mp3" | "oga" | "wav" | "weba" | "aac" | "m4a" | "ogg" => "mp3",
        "pdf" => "pdf",
        "zip" => "zip",
        "txt" => "txt",
        "csv" => "csv",
        "json" => "json",
        "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" => "doc",
        _ => "bin",
    }
}

// ===== Group management (Fase 3) =====

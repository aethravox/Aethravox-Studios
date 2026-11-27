//! Aris — Asistente de inteligencia artificial.
//!
//! - Chat 1-a-1 + invitados con permisos (leer historial / enviar).
//! - Streaming real vía SSE hacia el cliente.
//! - Backend: Ollama local (Aris) con failover a APIs externas
//!   (Gemini/Groq/etc.) usando api_keys con round-robin automático.
//! - Rate limiting por usuario (hora/día) desde aris_config.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response, Sse, sse::{Event, KeepAlive}},
    Form, Json,
};
use futures::stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

use crate::auth_mw::AuthUser;
use crate::AppState;

pub mod ollama;
pub mod external;
pub mod keys;
pub mod core;

use ollama::{OllamaMessage, OllamaStreamItem, chat_stream_ollama_opts};
use external::chat_with_failover;

// ============================================================
// Modelos
// ============================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArisChat {
    pub id: i64,
    pub owner_id: i64,
    pub title: String,
    pub history_limit: i64,
    pub pinned: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArisMessage {
    pub id: i64,
    pub chat_id: i64,
    pub user_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub meta_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArisCreditsStatus {
    pub daily_limit: i64,
    pub daily_used: i64,
    pub remaining: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArisCreditLogRow {
    pub id: i64,
    pub user_id: i64,
    pub chat_id: Option<i64>,
    pub mode: String,
    pub thinking_enabled: bool,
    pub thinking_level: String,
    pub attachments_summary: String,
    pub credits_cost: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArisChatMember {
    pub chat_id: i64,
    pub user_id: i64,
    pub can_read_history: bool,
    pub can_send: bool,
    pub invited_by: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArisChatMemberView {
    pub chat_id: i64,
    pub user_id: i64,
    pub can_read_history: bool,
    pub can_send: bool,
    pub invited_by: i64,
    pub username: String,
}

/// Fila de api_providers completa para admin y rotación.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiProviderRow {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub default_model: String,
    pub is_preset: i64,
    pub priority: i64,
    pub is_active: i64,
}

/// Fila de api_keys con datos join del provider (para el admin y failover).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub id: i64,
    pub provider_id: i64,
    pub api_key: String,
    pub label: String,
    pub is_active: i64,
    pub total_uses: i64,
    pub daily_uses: i64,
    pub daily_cap: Option<i64>,
    pub rate_limited_until: Option<String>,
    pub last_error: Option<String>,
    pub last_used_at: Option<String>,
    pub provider_name: String,
    pub provider_type: String,
    pub base_url: String,
    pub default_model: String,
    pub provider_priority: i64,
}

impl ApiKeyRow {
    pub fn is_daily_capped(&self) -> bool {
        self.daily_cap.is_some_and(|c| c > 0 && self.daily_uses >= c)
    }

    pub fn daily_cap_display(&self) -> String {
        self.daily_cap
            .filter(|&c| c > 0)
            .map(|c| c.to_string())
            .unwrap_or_else(|| "∞".to_string())
    }

    pub fn daily_cap_input(&self) -> String {
        self.daily_cap
            .filter(|&c| c > 0)
            .map(|c| c.to_string())
            .unwrap_or_default()
    }

    pub fn masked_key(&self) -> String {
        self.api_key_masked()
    }
    pub fn api_key_masked(&self) -> String {
        let chars: Vec<char> = self.api_key.chars().collect();
        if chars.len() <= 8 {
            self.api_key.clone()
        } else {
            let prefix: String = chars.iter().take(8).collect();
            format!("{}…", prefix)
        }
    }
}

/// Entrada de bitácora de rotación y fallbacks.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiRotationLogRow {
    pub id: i64,
    pub source: String,
    pub channel_label: String,
    pub provider_name: String,
    pub key_label: String,
    pub model: String,
    pub status: String,
    pub latency_ms: i64,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateChatResult {
    pub ok: bool,
    pub chat_id: i64,
}

// ============================================================
// Formularios
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateChatForm {
    pub title: Option<String>,
    pub history_limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArisAttachmentInput {
    pub kind: String, // "image" | "audio" | "video"
    pub name: String,
    #[serde(default)]
    pub size_label: Option<String>,
    #[serde(default)]
    pub duration_label: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub base64_data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessagePayload {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub think: Option<bool>,
    #[serde(default)]
    pub thinking_level: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ArisAttachmentInput>,
}

fn decode_form_component(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (
                    (bytes[i + 1] as char).to_digit(16),
                    (bytes[i + 2] as char).to_digit(16),
                ) {
                    out.push(((h << 4) | l) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn parse_urlencoded_send(bytes: &[u8]) -> SendMessagePayload {
    let raw = String::from_utf8_lossy(bytes);
    let mut out = SendMessagePayload {
        content: String::new(),
        source: None,
        mode: None,
        think: None,
        thinking_level: None,
        attachments: Vec::new(),
    };
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = decode_form_component(it.next().unwrap_or(""));
        let v = decode_form_component(it.next().unwrap_or(""));
        match k.as_str() {
            "content" => out.content = v,
            "source" => out.source = Some(v),
            "mode" => out.mode = Some(v),
            "think" => out.think = Some(matches!(v.trim(), "1" | "true" | "on")),
            "thinking_level" => out.thinking_level = Some(v),
            _ => {}
        }
    }
    out
}

pub fn normalize_aris_mode(mode: Option<&str>) -> &'static str {
    match mode.map(str::trim) {
        Some("instantaneo") => "instantaneo",
        _ => "rapido",
    }
}

pub fn normalize_thinking_level(level: Option<&str>) -> &'static str {
    match level.map(str::trim) {
        Some("muy_bajo") => "muy_bajo",
        Some("bajo") => "bajo",
        Some("medio") => "medio",
        Some("alto") => "alto",
        Some("muy_alto") => "muy_alto",
        Some("extremo") => "extremo",
        _ => "muy_bajo",
    }
}

pub fn compute_aris_credit_cost(
    mode: &str,
    think: bool,
    thinking_level: &str,
    attachments: &[ArisAttachmentInput],
) -> i64 {
    let base: i64 = if mode == "instantaneo" { 1 } else { 3 };
    let think_cost: i64 = if !think {
        0
    } else {
        match thinking_level {
            "muy_bajo" => 1,
            "bajo" => 2,
            "medio" => 4,
            "alto" => 7,
            "muy_alto" => 12,
            "extremo" => 18,
            _ => 1,
        }
    };
    let mut media_cost: i64 = 0;
    for att in attachments {
        match att.kind.as_str() {
            "image" => media_cost += 4,
            "audio" => media_cost += 5,
            "video" => media_cost += 10,
            _ => {}
        }
    }
    base + think_cost + media_cost
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberForm {
    pub username: String,
    pub can_read_history: Option<bool>,
    pub can_send: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatForm {
    pub title: Option<String>,
    pub history_limit: Option<i64>,
}

// ============================================================
// Helper: ¿puede este usuario VER este chat?
// Dueño, o miembro invitado (con cualquier permiso) puede ver la sala.
// ============================================================
async fn can_view_chat(state: &AppState, chat_id: i64, user_id: i64) -> bool {
    if let Ok(Some(chat)) = state.db.get_aris_chat(chat_id).await {
        if chat.owner_id == user_id { return true; }
    }
    matches!(
        state.db.get_aris_member(chat_id, user_id).await,
        Ok(Some(_))
    )
}

/// ¿Puede ENVIAR mensajes? (owner siempre; miembro solo si can_send)
async fn can_send_in_chat(state: &AppState, chat_id: i64, user_id: i64) -> bool {
    if let Ok(Some(chat)) = state.db.get_aris_chat(chat_id).await {
        if chat.owner_id == user_id { return true; }
    }
    match state.db.get_aris_member(chat_id, user_id).await {
        Ok(Some(m)) => m.can_send,
        _ => false,
    }
}

/// ¿Puede LEER historial completo? (owner siempre; miembro si can_read_history)
async fn can_read_history_of_chat(state: &AppState, chat_id: i64, user_id: i64) -> bool {
    if let Ok(Some(chat)) = state.db.get_aris_chat(chat_id).await {
        if chat.owner_id == user_id { return true; }
    }
    match state.db.get_aris_member(chat_id, user_id).await {
        Ok(Some(m)) => m.can_read_history,
        _ => false,
    }
}

// ============================================================
// Página principal
// ============================================================

pub async fn aris_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let chats = state.db.list_aris_chats_for_user(user.id).await.unwrap_or_default();
    let (_, role_display) = state.db.role_display(&user).await;
    let credits = state
        .db
        .get_aris_credits_status(user.id)
        .await
        .unwrap_or(ArisCreditsStatus {
            daily_limit: 120,
            daily_used: 0,
            remaining: 120,
        });
    crate::templates::ArisPage {
        user: user.clone(),
        chats,
        is_guest: false,
        display: user.display(),
        role_display,
        avatar: user.avatar(),
        credits_limit: credits.daily_limit,
        credits_used: credits.daily_used,
        credits_remaining: credits.remaining,
    }
    .into_response()
}

/// GET /api/aris/credits — Estado de créditos diarios e historial reciente del usuario.
pub async fn api_get_credits(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let status = state
        .db
        .get_aris_credits_status(user.id)
        .await
        .unwrap_or(ArisCreditsStatus {
            daily_limit: 120,
            daily_used: 0,
            remaining: 120,
        });
    let logs = state.db.list_aris_credit_logs(user.id, 30).await.unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "status": status,
        "logs": logs,
    }))
    .into_response()
}

// ============================================================
// CRUD de chats
// ============================================================

/// POST /api/aris/chats — Crear chat (el que lo crea es dueño).
pub async fn api_create_chat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<CreateChatForm>,
) -> Response {
    let title = form.title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "Chat con Aris".to_string());
    let history_limit = form.history_limit.unwrap_or(50);

    if ![20, 50, 100, 200].contains(&history_limit) {
        return Json(serde_json::json!({"error": "history_limit_invalido"})).into_response();
    }

    match state.db.create_aris_chat(user.id, &title, history_limit).await {
        Ok(id) => Json(serde_json::json!({
            "ok": true,
            "chat_id": id,
            "title": title,
        })).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// GET /api/aris/chats — Mis chats (dueño o invitado).
pub async fn api_list_chats(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let chats = state.db.list_aris_chats_for_user(user.id).await.unwrap_or_default();
    Json(serde_json::json!({"chats": chats})).into_response()
}

/// GET /api/aris/chats/:id/messages — Historial.
/// Si el invitado NO tiene can_read_history, solo recibe los últimos 20
/// (para que pueda seguir la conversación actual sin ver todo el pasado).
pub async fn api_get_messages(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
) -> Response {
    if !can_view_chat(&state, chat_id, user.id).await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    let full = can_read_history_of_chat(&state, chat_id, user.id).await;
    let limit = if full { 500 } else { 20 };

    match state.db.list_aris_messages(chat_id, limit).await {
        Ok(msgs) => Json(serde_json::json!({
            "ok": true,
            "messages": msgs,
            "partial": !full,
        })).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// POST /api/aris/chats/:id/settings — Título / límite (solo dueño).
pub async fn api_update_chat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
    Form(form): Form<UpdateChatForm>,
) -> Response {
    let owner = match state.db.get_aris_chat(chat_id).await {
        Ok(Some(c)) if c.owner_id == user.id => true,
        _ => false,
    };
    if !owner {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    let title = form.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
    match state.db.update_aris_chat_settings(chat_id, title, form.history_limit).await {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// POST /api/aris/chats/:id/pin — Fijar/desfijar chat (solo dueño).
pub async fn api_pin_chat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
) -> Response {
    let owner = matches!(
        state.db.get_aris_chat(chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    match state.db.toggle_aris_chat_pin(chat_id).await {
        Ok(pinned) => Json(serde_json::json!({"ok": true, "pinned": pinned})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// DELETE /api/aris/chats/:id — Borrar chat (solo dueño).
pub async fn api_delete_chat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
) -> Response {
    let owner = matches!(
        state.db.get_aris_chat(chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let _ = state.db.delete_aris_chat(chat_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

// ============================================================
// Envío de mensaje con STREAMING REAL
// ============================================================

/// POST /api/aris/chats/:id/send
/// 1. Guarda el mensaje del usuario.
/// 2. Construye el contexto (últimos N mensajes según history_limit).
/// 3. Intenta Ollama local → si falla, failover a APIs externas.
/// 4. Stream de la respuesta al cliente en tiempo real.
pub async fn api_send_message(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Helper de error como respuesta SSE
    let err_response = |msg: &str| {
        let m = msg.to_string();
        let s = stream::once(async move {
            Ok::<Event, Infallible>(Event::default().event("error").data(m))
        });
        Sse::new(s).into_response()
    };

    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let payload: SendMessagePayload = if content_type.contains("application/json") {
        match serde_json::from_slice::<SendMessagePayload>(&body) {
            Ok(p) => p,
            Err(_) => return err_response("payload_invalido"),
        }
    } else {
        parse_urlencoded_send(&body)
    };

    let raw_content = payload.content.trim().to_string();
    let attachments = payload.attachments;
    if raw_content.is_empty() && attachments.is_empty() {
        return err_response("mensaje_vacio");
    }

    let mode = normalize_aris_mode(payload.mode.as_deref());
    let think = payload.think.unwrap_or(false);
    let thinking_level = normalize_thinking_level(payload.thinking_level.as_deref());

    let lanes = state.db.user_aris_lanes(&user).await;
    let source = payload.source.as_deref().unwrap_or("chat");
    if source == "quick" && !lanes.quick_drawer {
        return err_response(
            "Tu rol no tiene habilitado el carril de Asistente Rápido (aris.lane.quick_drawer).",
        );
    }
    if source == "studio" && !lanes.studio {
        return err_response(
            "Tu rol no tiene habilitado el carril Creativo de Estudio (aris.lane.studio).",
        );
    }
    if source != "quick" && source != "studio" && !lanes.chat {
        return err_response(
            "Tu rol no tiene habilitado el carril de Chat con Aris (aris.lane.chat).",
        );
    }
    if !can_send_in_chat(&state, chat_id, user.id).await {
        return err_response("no_autorizado");
    }

    // ---- Rate limiting clásico ----
    let hourly_limit: i64 = state.db.aris_get_config("hourly_limit").await
        .ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(30);
    let daily_limit: i64 = state.db.aris_get_config("daily_limit").await
        .ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(150);

    match state.db.aris_rate_limit_check(user.id).await {
        Ok((hourly, daily)) => {
            if hourly >= hourly_limit || daily >= daily_limit {
                return err_response("rate_limit");
            }
        }
        Err(_) => return err_response("server"),
    }

    // ---- Créditos diarios de Aris (modos, pensamiento y multimodal) ----
    let credit_cost = compute_aris_credit_cost(mode, think, thinking_level, &attachments);
    let img_count = attachments.iter().filter(|a| a.kind == "image").count();
    let aud_count = attachments.iter().filter(|a| a.kind == "audio").count();
    let vid_count = attachments.iter().filter(|a| a.kind == "video").count();
    let mut summary_parts = Vec::new();
    if img_count > 0 { summary_parts.push(format!("{} img", img_count)); }
    if aud_count > 0 { summary_parts.push(format!("{} audio", aud_count)); }
    if vid_count > 0 { summary_parts.push(format!("{} vídeo", vid_count)); }
    let attachments_summary = summary_parts.join(", ");

    let credits_after = match state
        .db
        .consume_aris_daily_credits(
            user.id,
            Some(chat_id),
            mode,
            think,
            thinking_level,
            &attachments_summary,
            credit_cost,
        )
        .await
    {
        Ok(st) => st,
        Err(msg) => return err_response(&msg),
    };

    // ---- Texto guardado + metadatos seguros (sin base64 pesado en DB) ----
    let stored_content = if raw_content.is_empty() {
        "Analiza los archivos adjuntos".to_string()
    } else {
        raw_content.clone()
    };

    let attachments_meta: Vec<serde_json::Value> = attachments
        .iter()
        .map(|a| {
            serde_json::json!({
                "kind": a.kind,
                "name": a.name,
                "size_label": a.size_label,
                "duration_label": a.duration_label,
                "mime_type": a.mime_type,
            })
        })
        .collect();

    let user_meta_json = serde_json::json!({
        "mode": mode,
        "thinking_enabled": think,
        "thinking_level": thinking_level,
        "credit_cost": credit_cost,
        "attachments": attachments_meta,
    })
    .to_string();

    let _ = state
        .db
        .add_aris_message_with_meta(
            chat_id,
            Some(user.id),
            "user",
            &stored_content,
            None,
            None,
            Some(&user_meta_json),
        )
        .await;
    let _ = state.db.aris_rate_limit_increment(user.id).await;
    let _ = state.db.touch_aris_chat(chat_id).await;

    // ---- Construir contexto ----
    let chat = match state.db.get_aris_chat(chat_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return err_response("chat_no_encontrado"),
        Err(_) => return err_response("server"),
    };

    let history = match state.db.list_aris_messages(chat_id, chat.history_limit).await {
        Ok(h) => h,
        Err(_) => return err_response("server"),
    };

    let image_b64: Vec<String> = attachments
        .iter()
        .filter(|a| a.kind == "image")
        .filter_map(|a| a.base64_data.clone())
        .filter(|s| !s.trim().is_empty())
        .collect();

    let media_notes: Vec<String> = attachments
        .iter()
        .filter(|a| a.kind == "audio" || a.kind == "video")
        .map(|a| {
            let kind_lbl = if a.kind == "audio" { "Audio adjunto" } else { "Vídeo adjunto" };
            let meta = [a.duration_label.as_deref(), a.size_label.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
            if meta.is_empty() {
                format!("[{}: {}]", kind_lbl, a.name)
            } else {
                format!("[{}: {} ({})]", kind_lbl, a.name, meta)
            }
        })
        .collect();

    let mut ollama_messages: Vec<OllamaMessage> = history
        .iter()
        .map(|m| OllamaMessage {
            role: if m.role == "aris" {
                "assistant".to_string()
            } else {
                "user".to_string()
            },
            content: m.content.clone(),
            images: None,
        })
        .collect();

    if let Some(last_msg) = ollama_messages.last_mut() {
        if !media_notes.is_empty() {
            last_msg.content = format!("{}\n\n{}", last_msg.content, media_notes.join("\n"));
        }
        if !image_b64.is_empty() {
            last_msg.images = Some(image_b64);
        }
    }

    let base_system_prompt = state.db.aris_get_config("system_prompt").await
        .ok().flatten()
        .unwrap_or_else(|| "Eres Aris, el asistente conectado al núcleo de Aethravox Studios.".to_string());
    let mut system_prompt = core::build_core_system_prompt(&state, &user, &lanes, &base_system_prompt, &stored_content).await;

    if think {
        let depth_hint = match thinking_level {
            "muy_bajo" => "Pensamiento muy bajo: razona de forma muy breve y concisa.",
            "bajo" => "Pensamiento bajo: incluye solo el razonamiento esencial.",
            "medio" => "Pensamiento medio: analiza pasos y alternativas equilibradamente.",
            "alto" => "Pensamiento alto: analiza en profundidad casos borde y arquitectura.",
            "muy_alto" => "Pensamiento muy alto: razonamiento exhaustivo paso a paso.",
            "extremo" => "Pensamiento extremo: análisis máximo y verificación rigurosa.",
            _ => "",
        };
        if !depth_hint.is_empty() {
            system_prompt.push_str("\n\nMODO DE RAZONAMIENTO: ");
            system_prompt.push_str(depth_hint);
        }
    } else if mode == "instantaneo" {
        system_prompt.push_str("\n\nMODO INSTANTÁNEO: Responde directo al grano sin rodeos ni relleno.");
    }

    let ollama_url = state.db.aris_get_config("ollama_url").await
        .ok().flatten()
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    let pool = state.db.pool.clone();
    let chat_title = chat.title.clone();
    let core_state = state.clone();
    let core_user = user.clone();
    let core_lanes = lanes.clone();
    let mode_owned = mode.to_string();
    let level_owned = thinking_level.to_string();

    // ---- Stream principal: Ollama → failover externo → error amable ----
    let main_stream = async_stream::stream! {
        yield Ok::<Event, Infallible>(Event::default().event("start").data(""));

        let started_at = std::time::Instant::now();
        let mut full_response = String::new();
        let mut thinking_response = String::new();
        let mut used_model = String::from("Aris");
        let mut used_provider = String::from("ollama");

        // 1) INTENTAR OLLAMA LOCAL (Aris)
        match chat_stream_ollama_opts(
            &ollama_url,
            "Aris",
            &system_prompt,
            ollama_messages.clone(),
            think,
            &mode_owned,
            &level_owned,
        )
        .await
        {
            Ok(mut ollama_stream) => {
                use futures::StreamExt;
                while let Some(item_result) = ollama_stream.next().await {
                    match item_result {
                        Ok(OllamaStreamItem::Thinking(t)) => {
                            thinking_response.push_str(&t);
                            let payload = serde_json::json!({ "text": t }).to_string();
                            yield Ok::<Event, Infallible>(Event::default().event("thinking").data(payload));
                        }
                        Ok(OllamaStreamItem::Chunk(text)) => {
                            full_response.push_str(&text);
                            let payload = serde_json::json!({ "text": text }).to_string();
                            yield Ok::<Event, Infallible>(Event::default().event("chunk").data(payload));
                        }
                        Err(e) => {
                            tracing::warn!("Ollama stream error: {}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Ollama no disponible ({}), probando failover externo...", e);
            }
        }

        if !full_response.trim().is_empty() {
            let _ = crate::db::aris::add_rotation_log(
                &pool,
                "aris_chat",
                &chat_title,
                "Aris Local (Ollama)",
                "server-local",
                &used_model,
                "ok",
                started_at.elapsed().as_millis() as i64,
                "Respuesta servida por motor local Ollama",
            ).await;
        }

        // 2) Si Ollama no dio respuesta → FAILOVER a APIs externas
        if full_response.trim().is_empty() {
            match chat_with_failover(&pool, &ollama_messages, &system_prompt, &chat_title).await {
                Ok((text, model, provider)) => {
                    full_response = text;
                    used_model = model;
                    used_provider = provider;
                    let payload = serde_json::json!({ "text": &full_response }).to_string();
                    yield Ok::<Event, Infallible>(Event::default().event("chunk").data(payload));
                }
                Err(e) => {
                    tracing::error!("Failover externo agotado: {}", e);
                    let msg = "Lo siento, no pude generar una respuesta en este momento. Intentá de nuevo en unos segundos.".to_string();
                    yield Ok::<Event, Infallible>(Event::default().event("error").data(msg));
                    return;
                }
            }
        }

        // 3) Procesar acciones del núcleo (DM a amigos, limpiar tags internos)
        let final_response = core::process_aris_core_actions(&core_state, &core_user, &core_lanes, &full_response).await;

        let aris_meta_json = serde_json::json!({
            "mode": mode_owned,
            "thinking_enabled": think,
            "thinking_level": level_owned,
            "thinking_text": if thinking_response.trim().is_empty() { None::<String> } else { Some(thinking_response.clone()) },
            "credit_cost": credit_cost,
        })
        .to_string();

        // 4) Guardar respuesta de Aris + purgar historial al límite
        let _ = crate::db::aris::add_aris_message_with_meta(
            &pool, chat_id, None, "aris", &final_response,
            Some(&used_model), Some(&used_provider), Some(&aris_meta_json),
        ).await;
        let _ = crate::db::aris::purge_aris_history(&pool, chat_id).await;
        let _ = crate::db::aris::touch_aris_chat(&pool, chat_id).await;

        yield Ok::<Event, Infallible>(Event::default().event("done").data(serde_json::json!({
            "model": used_model,
            "provider": used_provider,
            "final_text": final_response,
            "mode": mode_owned,
            "thinking_enabled": think,
            "thinking_level": level_owned,
            "thinking_text": if thinking_response.trim().is_empty() { None::<String> } else { Some(thinking_response) },
            "credit_cost": credit_cost,
            "credits_remaining": credits_after.remaining,
            "credits_used": credits_after.daily_used,
            "credits_limit": credits_after.daily_limit,
        }).to_string()));
    };

    Sse::new(main_stream).keep_alive(KeepAlive::default()).into_response()
}

// ============================================================
// Invitaciones
// ============================================================

/// POST /api/aris/chats/:id/invite — Solo el dueño invita.
pub async fn api_invite_member(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
    Form(form): Form<InviteMemberForm>,
) -> Response {
    // Solo dueño
    let owner = matches!(
        state.db.get_aris_chat(chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    // Resolver username → id
    let target = match state.db.find_user_by_username(&form.username).await {
        Ok(Some(u)) => u,
        _ => return Json(serde_json::json!({"error": "usuario_no_encontrado"})).into_response(),
    };

    if target.id == user.id {
        return Json(serde_json::json!({"error": "no_te_puedes_invitar"})).into_response();
    }

    let can_read = form.can_read_history.unwrap_or(false);
    let can_send = form.can_send.unwrap_or(true);

    match state.db.add_aris_member(chat_id, target.id, can_read, can_send, user.id).await {
        Ok(_) => Json(serde_json::json!({
            "ok": true,
            "user": target.display(),
            "can_read_history": can_read,
            "can_send": can_send,
        })).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// GET /api/aris/chats/:id/members — Lista miembros (dueño o miembro).
pub async fn api_list_members(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(chat_id): Path<i64>,
) -> Response {
    if !can_view_chat(&state, chat_id, user.id).await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let members = state.db.list_aris_members(chat_id).await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "members": members})).into_response()
}

/// DELETE /api/aris/chats/:id/members/:user_id — Expulsar (solo dueño).
pub async fn api_remove_member(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path((chat_id, member_id)): Path<(i64, i64)>,
) -> Response {
    let owner = matches!(
        state.db.get_aris_chat(chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let _ = state.db.remove_aris_member(chat_id, member_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

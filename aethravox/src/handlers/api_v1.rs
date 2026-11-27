use std::collections::HashMap;

use aethravox_core::{
    ApiError, AuthSuccess, CategoryRowDto, ChatConversationDto, ChatMessageDto, HubPayload,
    HubStatsDto, LoginRequest, RegisterRequest, SendMessageRequest, ThemeKind, ThemeSelectRequest,
    UserSummary, WebAppDto,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    db::verify_password,
    models::{validate_email, validate_password, validate_username, Message, User, Web},
    AppState,
};

const HIDDEN_SLUGS: &[&str] = &["hub", "login", "dashboard", "correo"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/me", get(me))
        .route("/api/v1/hub", get(hub))
        .route("/api/v1/theme", post(select_theme))
        .route("/api/v1/chat/conversations", get(chat_conversations))
        .route(
            "/api/v1/chat/conversations/:id/messages",
            get(chat_messages).post(chat_send),
        )
        .route("/api/v1/chat/conversations/:id/read", post(chat_mark_read))
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ApiError { error: msg.into() })).into_response()
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn resolve_bearer_or_guest(state: &AppState, headers: &HeaderMap) -> User {
    if let Some(tok) = extract_bearer_token(headers) {
        if let Ok(Some(u)) = state.db.get_session_user(&tok).await {
            return u;
        }
    }
    let rid = state
        .invitado_role_id
        .get_or_try_init(|| async { state.db.role_id_by_key("invitado").await })
        .await
        .ok()
        .copied()
        .flatten();
    User::guest(rid)
}

async fn require_bearer_user(state: &AppState, headers: &HeaderMap) -> Result<User, Response> {
    let Some(tok) = extract_bearer_token(headers) else {
        return Err(json_err(StatusCode::UNAUTHORIZED, "Token de sesión requerido."));
    };
    match state.db.get_session_user(&tok).await {
        Ok(Some(u)) => Ok(u),
        _ => Err(json_err(StatusCode::UNAUTHORIZED, "Sesión inválida o expirada.")),
    }
}

async fn build_user_summary(state: &AppState, user: &User) -> UserSummary {
    let (role_css_key, role_display) = state.db.role_display(user).await;
    let role_colors = state.role_colors.snapshot();
    let (role_color, role_text_color) = role_colors
        .get(&role_css_key)
        .map(|e| (e.color.clone(), e.text.clone()))
        .unwrap_or_else(|| ("#6b7280".to_string(), "#ffffff".to_string()));

    let can_access_admin = if user.id == 0 {
        false
    } else {
        state.db.user_has_perm(user, "admin.access").await
    };

    let theme = if user.id == 0 {
        ThemeKind::Aethravox
    } else {
        state
            .db
            .get_user_theme(user.id)
            .await
            .map(|t| ThemeKind::from_key(&t))
            .unwrap_or(ThemeKind::Aethravox)
    };

    UserSummary {
        id: user.id,
        username: user.username.clone(),
        display: user.display(),
        role_key: role_css_key,
        role_display,
        role_color,
        role_text_color,
        is_guest: user.id == 0,
        can_access_admin,
        bio: user.bio.clone().unwrap_or_default(),
        theme,
    }
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    let user = match state.db.find_user_by_username(req.username.trim()).await {
        Ok(Some(u)) => u,
        _ => return json_err(StatusCode::UNAUTHORIZED, "Usuario no encontrado."),
    };

    if state.db.is_aexos_only_user(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            "Esta cuenta es exclusiva de aexOS y no tiene contraseña externa.",
        );
    }

    if !verify_password(&req.password, &user.password_hash) {
        return json_err(StatusCode::UNAUTHORIZED, "Contraseña incorrecta.");
    }

    if let Ok(Some(ban)) = state.db.is_user_banned(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            ban.reason
                .unwrap_or_else(|| "Cuenta baneada.".to_string()),
        );
    }

    if let Ok(Some(susp)) = state.db.is_user_suspended(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            susp.reason
                .unwrap_or_else(|| "Cuenta suspendida temporalmente.".to_string()),
        );
    }

    let _ = state.db.touch_last_login(user.id).await;
    let Ok((token, _)) = state.db.create_session(user.id, req.remember).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo crear la sesión.",
        );
    };

    let summary = build_user_summary(&state, &user).await;
    Json(AuthSuccess {
        token,
        user: summary,
    })
    .into_response()
}

async fn register(State(state): State<AppState>, Json(req): Json<RegisterRequest>) -> Response {
    if let Err(e) = validate_username(&req.username) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    if let Err(e) = validate_email(&req.email) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    if let Err(e) = validate_password(&req.password) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }

    if state
        .db
        .find_user_by_username(&req.username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return json_err(StatusCode::CONFLICT, "Ese nombre de usuario ya está en uso.");
    }

    if state
        .db
        .find_user_by_email(&req.email)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return json_err(StatusCode::CONFLICT, "Ese email ya está registrado.");
    }

    let uid = match state
        .db
        .create_user(&req.username, &req.email, &req.password)
        .await
    {
        Ok(id) => id,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, format!("Error creando cuenta: {e}")),
    };

    let Ok(Some(user)) = state.db.find_user_by_id(uid).await else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "Usuario no encontrado tras crear.");
    };

    let Ok((token, _)) = state.db.create_session(uid, true).await else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo crear sesión.");
    };

    let summary = build_user_summary(&state, &user).await;
    Json(AuthSuccess {
        token,
        user: summary,
    })
    .into_response()
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(tok) = extract_bearer_token(&headers) {
        let _ = state.db.delete_session(&tok).await;
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match require_bearer_user(&state, &headers).await {
        Ok(user) => Json(build_user_summary(&state, &user).await).into_response(),
        Err(resp) => resp,
    }
}

async fn select_theme(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ThemeSelectRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let theme = ThemeKind::from_key(&req.theme);
    let _ = state.db.set_user_theme(user.id, theme.as_str()).await;
    Json(serde_json::json!({ "ok": true, "theme": theme.as_str() })).into_response()
}

async fn hub(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let summary = build_user_summary(&state, &user).await;

    let visible: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !HIDDEN_SLUGS.contains(&w.slug.as_str()))
        .collect();

    let cats = state.db.list_categories().await.unwrap_or_default();
    let pairs = state.db.list_app_category_pairs().await.unwrap_or_default();
    let app_to_cat: HashMap<i64, i64> = pairs.into_iter().collect();

    let mut by_cat: HashMap<i64, Vec<WebAppDto>> = HashMap::new();
    for w in &visible {
        if let Some(&cid) = app_to_cat.get(&w.id) {
            let dto = WebAppDto {
                id: w.id,
                slug: w.slug.clone(),
                name: w.name.clone(),
                description: w.description_or_empty(),
                icon: w.icon_emoji(),
                url: w.effective_url(),
                tag: w.tag_normalized(),
                tag_label: w.tag_label().to_string(),
                tag_color: w.tag_color().to_string(),
                clickable: w.is_clickable_for(&user.role),
                lock_message: w.tag_display_message(),
            };
            by_cat.entry(cid).or_default().push(dto);
        }
    }

    let categories: Vec<CategoryRowDto> = cats
        .into_iter()
        .filter_map(|c| {
            let items = by_cat.remove(&c.id)?;
            if items.is_empty() {
                None
            } else {
                Some(CategoryRowDto {
                    name: c.name,
                    slug: c.slug,
                    items,
                })
            }
        })
        .collect();

    let (owner_count, admin_count, staff_count, user_count) =
        state.db.count_users_by_role().await.unwrap_or((0, 0, 0, 0));
    let online_count = state
        .db
        .list_online_user_ids()
        .await
        .unwrap_or_default()
        .len() as i64;
    let doc_count = state.db.list_documents().await.unwrap_or_default().len() as i64;
    let chat_unread = if user.id == 0 {
        0
    } else {
        state
            .db
            .count_total_unread_for_user(user.id)
            .await
            .unwrap_or(0)
    };
    let community_unread = if user.id == 0 {
        0
    } else {
        state
            .db
            .count_community_unread_for_user(user.id)
            .await
            .unwrap_or(0)
    };

    Json(HubPayload {
        user: summary,
        categories,
        stats: HubStatsDto {
            online_count,
            doc_count,
            chat_unread,
            community_unread,
            owner_count,
            admin_count,
            staff_count,
            user_count,
        },
    })
    .into_response()
}

// ===================== Chat (REST) =====================
// Misma lógica y mismas reglas que el chat web (handlers/chat/*), pero con
// autenticación Bearer + JSON para el cliente de escritorio.

/// Convierte los prefijos internos ([IMAGE]/[VOICE]/[FILE]) en texto legible.
fn chat_preview(body: &str) -> String {
    if body.starts_with("[IMAGE]") {
        "📷 Imagen".to_string()
    } else if body.starts_with("[VOICE]") {
        "🎙️ Nota de voz".to_string()
    } else if body.starts_with("[FILE]") {
        "📎 Archivo".to_string()
    } else {
        body.to_string()
    }
}

fn message_to_dto(m: &Message, sender: String, my_id: i64) -> ChatMessageDto {
    let content = if m.deleted_at.is_some() {
        "🚫 Mensaje eliminado".to_string()
    } else {
        chat_preview(&m.body)
    };
    ChatMessageDto {
        id: m.id,
        conversation_id: m.conversation_id,
        sender,
        content,
        created_at: m.created_at.format("%H:%M").to_string(),
        is_mine: m.sender_id == my_id,
    }
}

async fn chat_conversations(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let convs = state
        .db
        .list_user_conversations(user.id)
        .await
        .unwrap_or_default();
    let mut out: Vec<ChatConversationDto> = Vec::with_capacity(convs.len());

    for c in convs {
        let last = state.db.get_last_message(c.id).await.ok().flatten();
        let unread = state
            .db
            .count_unread_in_conversation(c.id, user.id)
            .await
            .unwrap_or(0);
        let is_group = c.conv_type != "dm";

        let title = if is_group {
            c.name.clone().unwrap_or_else(|| "Grupo".to_string())
        } else {
            let members = state
                .db
                .get_conversation_members(c.id)
                .await
                .unwrap_or_default();
            let other_id = members.into_iter().find(|&m| m != user.id).unwrap_or(0);
            state
                .db
                .find_user_by_id(other_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.display())
                .unwrap_or_else(|| "Usuario".to_string())
        };

        let last_message = match last {
            Some(m) if m.deleted_at.is_some() => "🚫 Mensaje eliminado".to_string(),
            Some(m) => chat_preview(&m.body),
            None => String::new(),
        };

        out.push(ChatConversationDto {
            id: c.id,
            title,
            is_group,
            last_message,
            unread_count: unread,
        });
    }

    Json(out).into_response()
}

async fn chat_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.is_conversation_member(id, user.id).await {
        return json_err(StatusCode::FORBIDDEN, "No eres miembro de esta conversación.");
    }

    let msgs = state.db.list_messages(id, 200).await.unwrap_or_default();

    // Nombres de remitentes, con caché para no repetir consultas.
    let mut names: HashMap<i64, String> = HashMap::new();
    for m in &msgs {
        if !names.contains_key(&m.sender_id) {
            let name = state
                .db
                .find_user_by_id(m.sender_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.display())
                .unwrap_or_else(|| format!("User {}", m.sender_id));
            names.insert(m.sender_id, name);
        }
    }

    let out: Vec<ChatMessageDto> = msgs
        .iter()
        .map(|m| {
            let sender = names.get(&m.sender_id).cloned().unwrap_or_default();
            message_to_dto(m, sender, user.id)
        })
        .collect();

    Json(out).into_response()
}

async fn chat_send(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let body = req.body.trim().to_string();
    if body.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El mensaje no puede estar vacío.");
    }
    if body.chars().count() > 4000 {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Mensaje demasiado largo (máx 4000 caracteres).",
        );
    }
    if !state.db.is_conversation_member(id, user.id).await {
        return json_err(StatusCode::FORBIDDEN, "No eres miembro de esta conversación.");
    }

    // Permiso de envío por rango (solo grupos; si no hay rango, se permite).
    if let Ok(Some(rank)) = state.db.get_member_rank(id, user.id).await {
        if rank.can_send_messages == 0 {
            return json_err(
                StatusCode::FORBIDDEN,
                "No tienes permiso para enviar mensajes en este grupo.",
            );
        }
    }

    let msg_id = match state.db.send_message(id, user.id, &body).await {
        Ok(mid) if mid > 0 => mid,
        _ => {
            return json_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "No se pudo guardar el mensaje.",
            )
        }
    };

    let sender_name = user.display();
    let sender_avatar = user.avatar();
    let now = chrono::Utc::now();
    let created_at = now.format("%H:%M").to_string();
    let created_iso = now.to_rfc3339();

    // ¿Hay algún otro miembro conectado por WebSocket? -> 'delivered'
    let members = state
        .db
        .get_conversation_members(id)
        .await
        .unwrap_or_default();
    let other_online = {
        let conns = state.ws_connections.lock().await;
        members
            .iter()
            .any(|&m| m != user.id && conns.contains_key(&m))
    };
    let status = if other_online { "delivered" } else { "sent" };
    if other_online {
        let _ = state.db.mark_messages_delivered(id).await;
    }

    // Difundir en vivo a los clientes web conectados (mismo formato que el WS).
    let mut offline_members: Vec<i64> = Vec::new();
    {
        let conns = state.ws_connections.lock().await;
        for member_id in members {
            if let Some(tx) = conns.get(&member_id) {
                let mut payload = serde_json::json!({
                    "type": "message",
                    "id": msg_id,
                    "conversation_id": id,
                    "sender_id": user.id,
                    "sender_name": sender_name,
                    "sender_avatar": sender_avatar,
                    "body": body,
                    "time": created_at,
                    "created_iso": created_iso,
                    "status": status
                });
                if member_id == user.id {
                    payload["is_self"] = serde_json::json!(true);
                }
                let _ = tx.send(payload.to_string());
            } else if member_id != user.id {
                offline_members.push(member_id);
            }
        }
    }

    // Web Push a los miembros sin conexión.
    if !offline_members.is_empty() {
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
            if body.chars().count() > 120 {
                b + "…"
            } else {
                b
            }
        };
        let payload = serde_json::json!({
            "title": sender_name,
            "body": body_preview,
            "tag": format!("msg-{}", id),
            "type": "message",
            "conversation_id": id,
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

    Json(ChatMessageDto {
        id: msg_id,
        conversation_id: id,
        sender: sender_name,
        content: chat_preview(&body),
        created_at,
        is_mine: true,
    })
    .into_response()
}

async fn chat_mark_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.is_conversation_member(id, user.id).await {
        return json_err(StatusCode::FORBIDDEN, "No eres miembro de esta conversación.");
    }

    let _ = state.db.mark_conversation_read(id, user.id).await;

    // Avisar a los demás miembros conectados (✓✓ leído en tiempo real).
    let members = state
        .db
        .get_conversation_members(id)
        .await
        .unwrap_or_default();
    let conns = state.ws_connections.lock().await;
    for member_id in members {
        if member_id != user.id {
            if let Some(tx) = conns.get(&member_id) {
                let _ = tx.send(
                    serde_json::json!({
                        "type": "status_update",
                        "conversation_id": id,
                        "reader_id": user.id,
                        "status": "read"
                    })
                    .to_string(),
                );
            }
        }
    }

    Json(serde_json::json!({ "ok": true })).into_response()
}

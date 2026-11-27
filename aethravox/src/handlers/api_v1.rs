use std::collections::HashMap;

use aethravox_core::{
    base64_decode, AddGroupMemberRequest, AdminAuditDto, AdminBanRequest, AdminIpBansPayload,
    AdminOverviewDto, AdminRoleRequest, AdminUserDto, AdminWebDto, AexosLinkRequest,
    AexosProfileDto, AexosSavePrefsRequest, AexosSwitchRequest, ApiError, ArisChatDto,
    ArisCreateChatRequest, ArisCreditsDto, ArisMessageDto, ArisSendRequest, ArisSendResponse,
    AssignMemberRankRequest, AuthSuccess, CategoryRowDto, ChangePasswordRequest,
    ChatConversationDto, ChatGroupManageDto, ChatGroupMemberDto, ChatGroupRankDto, ChatMessageDto,
    CommunityDetailDto, CommunityDto, CommunityGroupDto, CreateCommunityGroupRequest,
    CreateCommunityRequest, CreateGroupRankRequest, CreateGroupRequest, CreateIpBanRequest,
    CreateTicketRequest, DocumentDto, DownloadAppDto, DownloadFileDto, DownloadPlatformDto,
    EditMessageRequest, EmailIdBody, EmailRequestBody, EmailVerifyBody, EmailsPayload, FriendDto,
    FriendRequestBody, FriendsPayload, HubPayload, HubStatsDto, InviteCommunityRequest, IpBanDto,
    LinkCodeDto, LinkConsumeRequest, LinkedDeviceDto, LoginRequest, PendingFriendDto,
    PlaygroundGenRequest, PlaygroundGenResult, ProfilePayload, RecoverRequest, RecoverResponse,
    RegisterRequest, ReplyTicketRequest, ResetPasswordRequest, SendMessageRequest, SessionDto,
    StartDmRequest, StartDmResponse, SuspendUserRequest, SystemConfigDto, ThemeKind,
    ThemeSelectRequest, TicketActionRequest, TicketDetailDto, TicketDto, TicketMessageDto,
    TitleDto, TransferGroupOwnershipRequest, UpdateGroupInfoRequest, UpdateProfileRequest,
    UpdateTitlesRequest, UploadVideoRequest, UserEmailDto, UserSearchDto, UserSummary, VideoDto,
    VideoReactRequest, WebAppDto,
};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use futures::StreamExt;
use serde::Deserialize;

use crate::{
    auth_mw::get_client_ip,
    db::verify_password,
    handlers::aris::{
        compute_aris_credit_cost,
        core::{
            build_core_system_prompt, fallback_playground_template, generate_single_completion,
            parse_playground_output, process_aris_core_actions,
        },
        external::chat_with_failover,
        ollama::{chat_stream_ollama_opts, OllamaMessage, OllamaStreamItem},
    },
    models::{
        sanitize_text, validate_email, validate_password, validate_username, ActionOnUser,
        is_role, Message, User, Web,
    },
    AppState,
};

const HIDDEN_SLUGS: &[&str] = &["hub", "login", "dashboard", "correo"];

pub fn router() -> Router<AppState> {
    Router::new()
        // Auth & Hub
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/recover", post(auth_recover))
        .route("/api/v1/auth/reset", post(auth_reset))
        .route("/api/v1/link/code", get(link_code))
        .route("/api/v1/link/consume", post(link_consume))
        .route("/api/v1/me", get(me))
        .route("/api/v1/hub", get(hub))
        .route("/api/v1/theme", post(select_theme))
        // Chat
        .route("/api/v1/chat/conversations", get(chat_conversations))
        .route(
            "/api/v1/chat/conversations/:id/messages",
            get(chat_messages).post(chat_send),
        )
        .route("/api/v1/chat/conversations/:id/read", post(chat_mark_read))
        .route("/api/v1/chat/messages/:id/edit", post(chat_edit_message))
        .route("/api/v1/chat/messages/:id/delete", post(chat_delete_message))
        .route("/api/v1/chat/dm", post(chat_start_dm))
        .route("/api/v1/chat/group", post(chat_create_group))
        .route(
            "/api/v1/chat/conversations/:id/manage",
            get(chat_group_manage),
        )
        .route(
            "/api/v1/chat/conversations/:id/info",
            post(chat_group_update_info),
        )
        .route(
            "/api/v1/chat/conversations/:id/leave",
            post(chat_group_leave),
        )
        .route(
            "/api/v1/chat/conversations/:id/members/add",
            post(chat_group_add_member),
        )
        .route(
            "/api/v1/chat/conversations/:id/members/:user_id/remove",
            post(chat_group_remove_member),
        )
        .route(
            "/api/v1/chat/conversations/:id/ranks/assign",
            post(chat_group_assign_rank),
        )
        .route(
            "/api/v1/chat/conversations/:id/ranks/create",
            post(chat_group_create_rank),
        )
        .route(
            "/api/v1/chat/conversations/:id/ranks/:rank_id",
            delete(chat_group_delete_rank),
        )
        .route(
            "/api/v1/chat/conversations/:id/transfer",
            post(chat_group_transfer_ownership),
        )
        // Aris IA
        .route("/api/v1/aris/credits", get(aris_credits))
        .route(
            "/api/v1/aris/chats",
            get(aris_list_chats).post(aris_create_chat),
        )
        .route("/api/v1/aris/chats/:id", delete(aris_delete_chat))
        .route("/api/v1/aris/chats/:id/pin", post(aris_pin_chat))
        .route("/api/v1/aris/chats/:id/messages", get(aris_get_messages))
        .route("/api/v1/aris/chats/:id/send", post(aris_send_message))
        .route(
            "/api/v1/aris/playground/generate",
            post(aris_playground_generate),
        )
        // Amigos
        .route("/api/v1/friends", get(friends_list))
        .route("/api/v1/friends/search", get(friends_search))
        .route("/api/v1/friends/request", post(friends_send_request))
        .route("/api/v1/friends/accept/:id", post(friends_accept))
        .route("/api/v1/friends/reject/:id", post(friends_reject))
        .route("/api/v1/friends/remove/:id", post(friends_remove))
        // Comunidades
        .route("/api/v1/communities", get(communities_list))
        .route("/api/v1/communities/create", post(communities_create))
        .route("/api/v1/communities/:id", get(communities_detail))
        .route("/api/v1/communities/:id/join", post(communities_join))
        .route("/api/v1/communities/:id/leave", post(communities_leave))
        .route(
            "/api/v1/communities/:id/group",
            post(communities_create_group),
        )
        .route(
            "/api/v1/communities/:id/inviteable",
            get(communities_inviteable),
        )
        .route("/api/v1/communities/:id/invite", post(communities_invite))
        // Documentos & Descargas
        .route("/api/v1/docs", get(docs_list))
        .route("/api/v1/docs/:id", delete(docs_delete))
        .route("/api/v1/downloads", get(downloads_list))
        // Asistencia / Tickets
        .route(
            "/api/v1/support/tickets",
            get(support_list_tickets).post(support_create_ticket),
        )
        .route("/api/v1/support/tickets/:id", get(support_ticket_detail))
        .route(
            "/api/v1/support/tickets/:id/reply",
            post(support_reply_ticket),
        )
        .route(
            "/api/v1/support/tickets/:id/action",
            post(support_ticket_action),
        )
        // Perfil, Títulos y Sesiones
        .route("/api/v1/profile", get(profile_get))
        .route("/api/v1/profile/update", post(profile_update))
        .route("/api/v1/profile/titles", post(profile_titles))
        .route("/api/v1/profile/password", post(profile_password))
        .route("/api/v1/profile/logout_all", post(profile_logout_all))
        .route(
            "/api/v1/profile/sessions/:token/revoke",
            post(profile_revoke_session),
        )
        .route(
            "/api/v1/profile/devices/:token/revoke",
            post(profile_revoke_device),
        )
        // Correo interno (AethraMail)
        .route("/api/v1/emails", get(emails_list))
        .route("/api/v1/emails/request", post(emails_request))
        .route("/api/v1/emails/verify", post(emails_verify))
        .route("/api/v1/emails/primary", post(emails_set_primary))
        .route("/api/v1/emails/remove", post(emails_remove))
        // aexTube
        .route("/api/v1/aextube/videos", get(aextube_list))
        .route("/api/v1/aextube/upload", post(aextube_upload))
        .route(
            "/api/v1/aextube/videos/:id/react",
            post(aextube_react),
        )
        .route("/api/v1/aextube/videos/:id/view", post(aextube_view))
        .route("/api/v1/aextube/videos/:id", delete(aextube_delete))
        .route(
            "/api/v1/aextube/channels/:author/subscribe",
            post(aextube_subscribe),
        )
        // aexOS
        .route("/api/v1/aexos/profiles", get(aexos_profiles))
        .route("/api/v1/aexos/link", post(aexos_link))
        .route("/api/v1/aexos/switch", post(aexos_switch))
        .route("/api/v1/aexos/prefs", post(aexos_save_prefs))
        .route("/api/v1/aexos/profiles/:id", delete(aexos_delete_profile))
        // Admin
        .route("/api/v1/admin/overview", get(admin_overview))
        .route("/api/v1/admin/users/:id/role", post(admin_set_role))
        .route("/api/v1/admin/users/:id/ban", post(admin_ban_user))
        .route("/api/v1/admin/users/:id/suspend", post(admin_suspend_user))
        .route("/api/v1/admin/users/:id/unban", post(admin_unban_user))
        .route("/api/v1/admin/users/:id/delete", post(admin_delete_user))
        .route(
            "/api/v1/admin/users/:id/force_logout",
            post(admin_force_logout),
        )
        .route("/api/v1/admin/webs/:id/toggle", post(admin_toggle_web))
        .route("/api/v1/admin/ip_bans", get(admin_ip_bans_list))
        .route("/api/v1/admin/ip_bans/create", post(admin_ip_bans_create))
        .route(
            "/api/v1/admin/ip_bans/:id/revoke",
            post(admin_ip_bans_revoke),
        )
        .route(
            "/api/v1/admin/config",
            get(admin_config_get).post(admin_config_set),
        )
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
        return Err(json_err(
            StatusCode::UNAUTHORIZED,
            "Token de sesión requerido.",
        ));
    };
    match state.db.get_session_user(&tok).await {
        Ok(Some(u)) => Ok(u),
        _ => Err(json_err(
            StatusCode::UNAUTHORIZED,
            "Sesión inválida o expirada.",
        )),
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

// ===================== Auth & Hub =====================

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
            ban.reason.unwrap_or_else(|| "Cuenta baneada.".to_string()),
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
        return json_err(
            StatusCode::CONFLICT,
            "Ese nombre de usuario ya está en uso.",
        );
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
        Err(e) => {
            return json_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error creando cuenta: {e}"),
            )
        }
    };

    let Ok(Some(user)) = state.db.find_user_by_id(uid).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Usuario no encontrado tras crear.",
        );
    };

    let Ok((token, _)) = state.db.create_session(uid, true).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo crear sesión.",
        );
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
        return json_err(
            StatusCode::FORBIDDEN,
            "No eres miembro de esta conversación.",
        );
    }

    let msgs = state.db.list_messages(id, 200).await.unwrap_or_default();

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
        return json_err(
            StatusCode::FORBIDDEN,
            "No eres miembro de esta conversación.",
        );
    }

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

    if !offline_members.is_empty() {
        let push_arc = state.push.clone();
        let db = state.db.clone();
        let body_preview = chat_preview(&body);
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
        return json_err(
            StatusCode::FORBIDDEN,
            "No eres miembro de esta conversación.",
        );
    }

    let _ = state.db.mark_conversation_read(id, user.id).await;

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

async fn chat_start_dm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<StartDmRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let target = match state.db.find_user_by_username(req.username.trim()).await {
        Ok(Some(u)) => u,
        _ => return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado."),
    };
    if target.id == user.id {
        return json_err(StatusCode::BAD_REQUEST, "No puedes abrir DM contigo mismo.");
    }
    if let Ok(Some(existing)) = state.db.find_dm(user.id, target.id).await {
        return Json(StartDmResponse {
            conversation_id: existing.id,
        })
        .into_response();
    }
    let Ok(conv_id) = state.db.create_conversation("dm", None, user.id).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo crear la conversación.",
        );
    };
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    let _ = state.db.add_conversation_member(conv_id, target.id).await;
    Json(StartDmResponse {
        conversation_id: conv_id,
    })
    .into_response()
}

async fn chat_create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateGroupRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let name = req.name.trim();
    if name.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El nombre del grupo es obligatorio.");
    }
    let desc = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Ok(conv_id) = state.db.create_group(name, desc, user.id).await else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo crear el grupo.");
    };
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    let _ = state.db.create_default_ranks(conv_id, user.id).await;
    for uname in &req.members {
        let uname = uname.trim();
        if uname.is_empty() {
            continue;
        }
        if let Ok(Some(u)) = state.db.find_user_by_username(uname).await {
            let _ = state.db.add_conversation_member(conv_id, u.id).await;
        }
    }
    Json(StartDmResponse {
        conversation_id: conv_id,
    })
    .into_response()
}

// ===================== Aris IA (REST) =====================

async fn aris_credits(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let st = crate::db::aris::get_aris_credits_status(&state.db.pool, user.id)
        .await
        .unwrap_or(crate::handlers::aris::ArisCreditsStatus {
            daily_limit: 120,
            daily_used: 0,
            remaining: 120,
        });
    Json(ArisCreditsDto {
        daily_limit: st.daily_limit,
        daily_used: st.daily_used,
        remaining: st.remaining,
    })
    .into_response()
}

async fn aris_list_chats(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let chats = crate::db::aris::list_aris_chats_for_user(&state.db.pool, user.id)
        .await
        .unwrap_or_default();
    let out: Vec<ArisChatDto> = chats
        .into_iter()
        .map(|c| ArisChatDto {
            id: c.id,
            title: c.title,
            pinned: c.pinned,
            updated_at: c.updated_at,
        })
        .collect();
    Json(out).into_response()
}

async fn aris_create_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ArisCreateChatRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let title = req
        .title
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Chat con Aris".to_string());
    match crate::db::aris::create_aris_chat(&state.db.pool, user.id, &title, 50).await {
        Ok(id) => Json(ArisChatDto {
            id,
            title,
            pinned: false,
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .into_response(),
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo crear el chat."),
    }
}

async fn aris_delete_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let owner = matches!(
        crate::db::aris::get_aris_chat(&state.db.pool, chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let _ = crate::db::aris::delete_aris_chat(&state.db.pool, chat_id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn aris_pin_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let owner = matches!(
        crate::db::aris::get_aris_chat(&state.db.pool, chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    if !owner {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    match crate::db::aris::toggle_aris_chat_pin(&state.db.pool, chat_id).await {
        Ok(pinned) => Json(serde_json::json!({ "ok": true, "pinned": pinned })).into_response(),
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error actualizando pin."),
    }
}

async fn aris_get_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let is_owner = matches!(
        crate::db::aris::get_aris_chat(&state.db.pool, chat_id).await,
        Ok(Some(c)) if c.owner_id == user.id
    );
    let member = crate::db::aris::get_aris_member(&state.db.pool, chat_id, user.id)
        .await
        .ok()
        .flatten();
    if !is_owner && member.is_none() {
        return json_err(StatusCode::FORBIDDEN, "No tienes acceso a este chat.");
    }
    let full = is_owner || member.map(|m| m.can_read_history).unwrap_or(false);
    let limit = if full { 200 } else { 20 };

    match crate::db::aris::list_aris_messages(&state.db.pool, chat_id, limit).await {
        Ok(msgs) => {
            let out: Vec<ArisMessageDto> = msgs
                .into_iter()
                .map(|m| ArisMessageDto {
                    id: m.id,
                    role: if m.role == "aris" {
                        "assistant".to_string()
                    } else {
                        m.role
                    },
                    content: m.content,
                    model: m.model,
                    created_at: m.created_at,
                })
                .collect();
            Json(out).into_response()
        }
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error cargando mensajes."),
    }
}

async fn aris_send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(chat_id): Path<i64>,
    Json(req): Json<ArisSendRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El mensaje no puede estar vacío.");
    }

    let lanes = state.db.user_aris_lanes(&user).await;
    if !lanes.chat {
        return json_err(
            StatusCode::FORBIDDEN,
            "Tu rol no tiene habilitado el carril de Chat con Aris.",
        );
    }

    let chat = match crate::db::aris::get_aris_chat(&state.db.pool, chat_id).await {
        Ok(Some(c)) => c,
        _ => return json_err(StatusCode::NOT_FOUND, "Chat de Aris no encontrado."),
    };
    let can_send = if chat.owner_id == user.id {
        true
    } else {
        crate::db::aris::get_aris_member(&state.db.pool, chat_id, user.id)
            .await
            .ok()
            .flatten()
            .map(|m| m.can_send)
            .unwrap_or(false)
    };
    if !can_send {
        return json_err(StatusCode::FORBIDDEN, "No autorizado para enviar en este chat.");
    }

    let mode = req.mode.api_mode();
    let think = req.mode.think();
    let thinking_level = if think { "medio" } else { "muy_bajo" };
    let cost = compute_aris_credit_cost(mode, think, thinking_level, &[]);

    let credits_after = match crate::db::aris::consume_aris_daily_credits(
        &state.db.pool,
        user.id,
        Some(chat_id),
        mode,
        think,
        thinking_level,
        "",
        cost,
    )
    .await
    {
        Ok(st) => st,
        Err(msg) => return json_err(StatusCode::TOO_MANY_REQUESTS, msg),
    };

    let user_msg_id = crate::db::aris::add_aris_message_with_meta(
        &state.db.pool,
        chat_id,
        Some(user.id),
        "user",
        &content,
        None,
        None,
        None,
    )
    .await
    .unwrap_or(0);
    let _ = crate::db::aris::aris_rate_limit_increment(&state.db.pool, user.id).await;
    let _ = crate::db::aris::touch_aris_chat(&state.db.pool, chat_id).await;

    let history = crate::db::aris::list_aris_messages(&state.db.pool, chat_id, chat.history_limit)
        .await
        .unwrap_or_default();
    let ollama_messages: Vec<OllamaMessage> = history
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

    let base_system = crate::db::aris::aris_get_config(&state.db.pool, "system_prompt")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            "Eres Aris, el asistente conectado al núcleo de Aethravox Studios.".to_string()
        });
    let mut system_prompt =
        build_core_system_prompt(&state, &user, &lanes, &base_system, &content).await;
    if think {
        system_prompt
            .push_str("\n\nMODO DE RAZONAMIENTO: analiza pasos y alternativas equilibradamente.");
    } else if mode == "instantaneo" {
        system_prompt
            .push_str("\n\nMODO INSTANTÁNEO: Responde directo al grano sin rodeos ni relleno.");
    }

    let ollama_url = crate::db::aris::aris_get_config(&state.db.pool, "ollama_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    let mut full_response = String::new();
    let mut used_model = String::from("Aris");
    let mut used_provider = String::from("ollama");

    if let Ok(mut stream) = chat_stream_ollama_opts(
        &ollama_url,
        "Aris",
        &system_prompt,
        ollama_messages.clone(),
        think,
        mode,
        thinking_level,
    )
    .await
    {
        while let Some(item) = stream.next().await {
            if let Ok(OllamaStreamItem::Chunk(t)) = item {
                full_response.push_str(&t);
            }
        }
    }

    if full_response.trim().is_empty() {
        match chat_with_failover(&state.db.pool, &ollama_messages, &system_prompt, &chat.title)
            .await
        {
            Ok((text, model, provider)) => {
                full_response = text;
                used_model = model;
                used_provider = provider;
            }
            Err(e) => {
                return json_err(
                    StatusCode::SERVICE_UNAVAILABLE,
                    format!("Aris no pudo responder ahora: {e}"),
                );
            }
        }
    }

    let final_response =
        process_aris_core_actions(&state, &user, &lanes, &full_response).await;
    let asst_msg_id = crate::db::aris::add_aris_message_with_meta(
        &state.db.pool,
        chat_id,
        None,
        "aris",
        &final_response,
        Some(&used_model),
        Some(&used_provider),
        None,
    )
    .await
    .unwrap_or(0);
    let _ = crate::db::aris::purge_aris_history(&state.db.pool, chat_id).await;
    let _ = crate::db::aris::touch_aris_chat(&state.db.pool, chat_id).await;

    let now_str = chrono::Utc::now().format("%H:%M").to_string();
    Json(ArisSendResponse {
        user_message: ArisMessageDto {
            id: user_msg_id,
            role: "user".into(),
            content,
            model: None,
            created_at: now_str.clone(),
        },
        assistant_message: ArisMessageDto {
            id: asst_msg_id,
            role: "assistant".into(),
            content: final_response,
            model: Some(used_model),
            created_at: now_str,
        },
        credits: ArisCreditsDto {
            daily_limit: credits_after.daily_limit,
            daily_used: credits_after.daily_used,
            remaining: credits_after.remaining,
        },
    })
    .into_response()
}

async fn aris_playground_generate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<PlaygroundGenRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&user, "aris.lane.playground").await {
        return json_err(
            StatusCode::FORBIDDEN,
            "Tu rol no tiene permiso para usar Aris en Playground (aris.lane.playground).",
        );
    }
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Describe qué quieres construir en Playground.",
        );
    }
    let system_prompt = "Eres Aris, copiloto de código web en Aethravox Playground. \
Responde ÚNICAMENTE con un objeto JSON válido con las claves exactas: \
\"html\" (solo contenido dentro de <body>), \"css\", \"js\", \"note\" (breve resumen en español).";
    let user_prompt = if req.current_html.trim().is_empty() {
        format!("Genera una experiencia web interactiva para Playground sobre: {prompt}")
    } else {
        format!(
            "Solicitud: {prompt}\n\nHTML actual:\n{}",
            req.current_html.chars().take(1800).collect::<String>()
        )
    };

    match generate_single_completion(&state, system_prompt, &user_prompt, "playground").await {
        Ok((raw, model, provider)) => {
            let (html, css, js, note) = parse_playground_output(&raw, prompt);
            Json(PlaygroundGenResult {
                html,
                css,
                js,
                note,
                model,
                provider,
            })
            .into_response()
        }
        Err(_) => {
            let (html, css, js, note) = fallback_playground_template(prompt);
            Json(PlaygroundGenResult {
                html,
                css,
                js,
                note,
                model: "Aris-Local-Scaffold".into(),
                provider: "aethravox-core".into(),
            })
            .into_response()
        }
    }
}

// ===================== Amigos =====================

async fn friends_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let online_ids = state.db.list_online_user_ids().await.unwrap_or_default();
    let role_colors = state.role_colors.snapshot();

    let mut friends = Vec::new();
    for fid in friend_ids {
        if let Ok(Some(f)) = state.db.find_user_by_id(fid).await {
            let (role_key, role_display) = state.db.role_display(&f).await;
            let role_color = role_colors
                .get(&role_key)
                .map(|e| e.color.clone())
                .unwrap_or_else(|| "#6b7280".to_string());
            friends.push(FriendDto {
                id: f.id,
                username: f.username.clone(),
                display: f.display(),
                online: online_ids.contains(&f.id),
                role_display,
                role_color,
            });
        }
    }
    friends.sort_by(|a, b| {
        b.online
            .cmp(&a.online)
            .then_with(|| a.display.to_lowercase().cmp(&b.display.to_lowercase()))
    });

    let req_ids = state
        .db
        .list_pending_requests(user.id)
        .await
        .unwrap_or_default();
    let mut pending = Vec::new();
    for rid in req_ids {
        if let Ok(Some(r)) = state.db.find_user_by_id(rid).await {
            pending.push(PendingFriendDto {
                id: r.id,
                username: r.username.clone(),
                display: r.display(),
            });
        }
    }

    Json(FriendsPayload { friends, pending }).into_response()
}

#[derive(Deserialize)]
struct SearchQ {
    q: String,
}

async fn friends_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SearchQ>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let all = state.db.list_users().await.unwrap_or_default();
    let query = q.q.trim().to_lowercase();
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let pending = state
        .db
        .list_pending_requests(user.id)
        .await
        .unwrap_or_default();

    let mut out = Vec::new();
    for u in &all {
        if u.id == user.id || u.status != "active" {
            continue;
        }
        if !query.is_empty()
            && !u.username.to_lowercase().contains(&query)
            && !u.display().to_lowercase().contains(&query)
        {
            continue;
        }
        if friend_ids.contains(&u.id) || pending.contains(&u.id) {
            continue;
        }
        let (_, role_display) = state.db.role_display(u).await;
        out.push(UserSearchDto {
            id: u.id,
            username: u.username.clone(),
            display: u.display(),
            role_display,
        });
    }
    Json(out).into_response()
}

async fn friends_send_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<FriendRequestBody>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let target = match state.db.find_user_by_username(req.username.trim()).await {
        Ok(Some(u)) => u,
        _ => return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado."),
    };
    if target.id == user.id {
        return json_err(
            StatusCode::BAD_REQUEST,
            "No puedes enviarte solicitud a ti mismo.",
        );
    }
    if state
        .db
        .are_friends(user.id, target.id)
        .await
        .unwrap_or(false)
    {
        return json_err(StatusCode::CONFLICT, "Ya son amigos.");
    }
    let _ = state.db.send_friend_request(user.id, target.id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn friends_accept(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(requester_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.accept_friend_request(user.id, requester_id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn friends_reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(requester_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.reject_friend_request(user.id, requester_id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn friends_remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(friend_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.remove_friend(user.id, friend_id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== Comunidades =====================

async fn communities_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let all = state.db.list_all_communities().await.unwrap_or_default();
    let mut out = Vec::with_capacity(all.len());
    for c in all {
        let is_member = if user.id == 0 {
            false
        } else {
            state.db.is_community_member(c.id, user.id).await
        };
        let member_count = state
            .db
            .list_community_members(c.id)
            .await
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        out.push(CommunityDto {
            id: c.id,
            name: c.name,
            description: c.description.unwrap_or_default(),
            icon: c.icon.unwrap_or_else(|| "🌐".into()),
            is_member,
            is_owner: c.owner_id == user.id,
            member_count,
        });
    }
    Json(out).into_response()
}

async fn communities_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCommunityRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let name = req.name.trim();
    if name.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El nombre es obligatorio.");
    }
    let desc = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let icon = req.icon.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let Ok(id) = state.db.create_community(name, desc, icon, user.id).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo crear la comunidad.",
        );
    };
    let _ = state
        .db
        .audit(
            Some(user.id),
            "community_created",
            Some(&format!("id={id} name={name}")),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true, "id": id })).into_response()
}

async fn communities_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let Some(c) = state.db.get_community(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Comunidad no encontrada.");
    };
    let is_member = if user.id == 0 {
        false
    } else {
        state.db.is_community_member(c.id, user.id).await
    };
    let member_count = state
        .db
        .list_community_members(c.id)
        .await
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let groups = state
        .db
        .list_community_conversations(c.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|g| CommunityGroupDto {
            id: g.id,
            name: g.name.unwrap_or_else(|| "Grupo".into()),
            description: g.description.unwrap_or_default(),
        })
        .collect();
    Json(CommunityDetailDto {
        community: CommunityDto {
            id: c.id,
            name: c.name,
            description: c.description.unwrap_or_default(),
            icon: c.icon.unwrap_or_else(|| "🌐".into()),
            is_member,
            is_owner: c.owner_id == user.id,
            member_count,
        },
        groups,
    })
    .into_response()
}

async fn communities_join(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.join_community(id, user.id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn communities_leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.leave_community(id, user.id).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn communities_create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<CreateCommunityGroupRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.is_community_member(id, user.id).await {
        return json_err(StatusCode::FORBIDDEN, "Debes unirte a la comunidad primero.");
    }
    let name = req.name.trim();
    if name.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El nombre del grupo es obligatorio.");
    }
    let desc = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Ok(conv_id) = state
        .db
        .create_community_group(id, name, desc, user.id)
        .await
    else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error creando grupo.");
    };
    let _ = state.db.add_conversation_member(conv_id, user.id).await;
    let _ = state.db.create_default_ranks(conv_id, user.id).await;
    Json(serde_json::json!({ "ok": true, "group_id": conv_id })).into_response()
}

async fn communities_inviteable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let friend_ids = state.db.list_friends(user.id).await.unwrap_or_default();
    let mut out = Vec::new();
    for fid in friend_ids {
        if state.db.is_community_member(id, fid).await {
            continue;
        }
        if let Ok(Some(f)) = state.db.find_user_by_id(fid).await {
            let (_, role_display) = state.db.role_display(&f).await;
            out.push(UserSearchDto {
                id: f.id,
                username: f.username.clone(),
                display: f.display(),
                role_display,
            });
        }
    }
    Json(out).into_response()
}

async fn communities_invite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<InviteCommunityRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.is_community_member(id, user.id).await {
        return json_err(StatusCode::FORBIDDEN, "No eres miembro.");
    }
    let friends = state.db.list_friends(user.id).await.unwrap_or_default();
    for uid in req.user_ids {
        if friends.contains(&uid) && !state.db.is_community_member(id, uid).await {
            let _ = state.db.join_community(id, uid).await;
        }
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== Documentos & Descargas =====================

async fn docs_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let _user = resolve_bearer_or_guest(&state, &headers).await;
    let docs = state.db.list_documents().await.unwrap_or_default();
    let out: Vec<DocumentDto> = docs
        .into_iter()
        .map(|d| DocumentDto {
            id: d.id,
            title: d.title.clone(),
            description: d.description.clone().unwrap_or_default(),
            file_name: d.file_name.clone(),
            file_type: d.file_type.clone(),
            icon: d.icon().to_string(),
            size: d.human_size(),
            created_at: d.created_at.format("%d/%m/%Y").to_string(),
            has_preview: d.has_preview(),
            downloaded: d.downloaded,
        })
        .collect();
    Json(out).into_response()
}

fn format_dl_size(bytes: i64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

async fn downloads_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let _user = resolve_bearer_or_guest(&state, &headers).await;
    let rows = state.db.list_active_downloads().await.unwrap_or_default();
    let order: &[(&str, &str, &str, bool)] = &[
        ("windows", "Windows", "🪟", false),
        ("linux", "Linux", "🐧", false),
        ("android", "Android", "🤖", false),
        ("macos", "macOS", "🍎", true),
        ("ios", "iOS", "📱", true),
    ];

    let platforms: Vec<DownloadPlatformDto> = order
        .iter()
        .map(|(key, name, icon, cs)| {
            let mut apps: Vec<DownloadAppDto> = Vec::new();
            for r in rows.iter().filter(|r| r.platform == *key) {
                let fr = DownloadFileDto {
                    id: r.id,
                    version: if r.version.is_empty() {
                        "—".into()
                    } else {
                        r.version.clone()
                    },
                    orig_name: if r.orig_name.is_empty() {
                        r.filename.clone()
                    } else {
                        r.orig_name.clone()
                    },
                    size_mb: format_dl_size(r.size_bytes),
                    created_at: r.created_at.chars().take(10).collect(),
                    url: format!("/descargas/files/{}", r.filename),
                };
                if let Some(pos) = apps.iter().position(|a| a.slug == r.app_slug) {
                    apps[pos].files.push(fr);
                } else {
                    apps.push(DownloadAppDto {
                        slug: r.app_slug.clone(),
                        name: r.app_name.clone(),
                        files: vec![fr],
                    });
                }
            }
            DownloadPlatformDto {
                key: (*key).to_string(),
                name: (*name).to_string(),
                icon: (*icon).to_string(),
                coming_soon: *cs,
                apps,
            }
        })
        .collect();

    Json(platforms).into_response()
}

// ===================== Asistencia / Tickets =====================

async fn support_list_tickets(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    if user.id == 0 {
        return Json(Vec::<TicketDto>::new()).into_response();
    }
    let can_view_all = state.db.user_has_perm(&user, "tickets.view").await;
    let rows = if can_view_all {
        state.db.list_all_tickets().await.unwrap_or_default()
    } else {
        state.db.list_user_tickets(user.id).await.unwrap_or_default()
    };

    let mut out = Vec::with_capacity(rows.len());
    for t in rows {
        let owner_name = state
            .db
            .find_user_by_id(t.user_id)
            .await
            .ok()
            .flatten()
            .map(|u| u.username)
            .unwrap_or_else(|| "usuario".into());
        out.push(TicketDto {
            id: t.id,
            ticket_type: t.ticket_type,
            subject: t.subject,
            body: t.body,
            status: t.status,
            resolucion: t.resolucion.unwrap_or_default(),
            created_at: t.created_at.format("%d/%m/%Y %H:%M").to_string(),
            owner_name,
        });
    }
    Json(out).into_response()
}

async fn support_create_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTicketRequest>,
) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let subject = req.subject.trim();
    let body = req.body.trim();
    if subject.is_empty() || body.is_empty() {
        return json_err(
            StatusCode::BAD_REQUEST,
            "El asunto y el mensaje no pueden estar vacíos.",
        );
    }
    if user.id == 0 {
        let tt = req.ticket_type.as_str();
        if tt != "bug" && tt != "sugerencia" {
            return json_err(
                StatusCode::FORBIDDEN,
                "Categoría no permitida para invitados.",
            );
        }
        let Some(owner) = state
            .db
            .find_user_by_username("invitado")
            .await
            .ok()
            .flatten()
        else {
            return json_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cuenta invitado no disponible.",
            );
        };
        let ip = get_client_ip(&headers).unwrap_or_default();
        if !state.guest_rate_limiter.allow(&ip).await {
            return json_err(
                StatusCode::TOO_MANY_REQUESTS,
                "Demasiados tickets desde tu IP. Espera un rato.",
            );
        }
        let id = state
            .db
            .create_guest_ticket(owner.id, tt, subject, body, &ip)
            .await
            .unwrap_or(0);
        return Json(serde_json::json!({ "ok": true, "id": id })).into_response();
    }

    let id = state
        .db
        .create_ticket(user.id, &req.ticket_type, subject, body)
        .await
        .unwrap_or(0);
    Json(serde_json::json!({ "ok": true, "id": id })).into_response()
}

async fn support_ticket_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(t) = state.db.get_ticket(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Ticket no encontrado.");
    };
    if t.user_id != user.id && !state.db.user_has_perm(&user, "tickets.view").await {
        return json_err(StatusCode::FORBIDDEN, "No tienes acceso a este ticket.");
    }

    let owner_name = state
        .db
        .find_user_by_id(t.user_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_else(|| "desconocido".into());
    let reclamado_por_nombre = match t.reclamado_por {
        Some(aid) => state
            .db
            .find_user_by_id(aid)
            .await
            .ok()
            .flatten()
            .map(|u| u.display())
            .unwrap_or_else(|| "Admin".into()),
        None => String::new(),
    };
    let is_admin = state.db.user_has_perm(&user, "tickets.respond").await;

    let msgs = state.db.list_ticket_messages(id).await.unwrap_or_default();
    let mut msg_dtos = Vec::with_capacity(msgs.len());
    for m in msgs {
        let sender_name = state
            .db
            .find_user_by_id(m.from_user_id)
            .await
            .ok()
            .flatten()
            .map(|u| u.display())
            .unwrap_or_else(|| "Usuario".into());
        msg_dtos.push(TicketMessageDto {
            id: m.id,
            body: m.body,
            sender_name,
            is_admin: m.is_admin != 0,
            created_at: m.created_at.format("%d/%m/%Y %H:%M").to_string(),
        });
    }

    Json(TicketDetailDto {
        ticket: TicketDto {
            id: t.id,
            ticket_type: t.ticket_type,
            subject: t.subject,
            body: t.body,
            status: t.status,
            resolucion: t.resolucion.unwrap_or_default(),
            created_at: t.created_at.format("%d/%m/%Y %H:%M").to_string(),
            owner_name,
        },
        reclamado_por_nombre,
        is_admin,
        messages: msg_dtos,
    })
    .into_response()
}

async fn support_reply_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<ReplyTicketRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(t) = state.db.get_ticket(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Ticket no encontrado.");
    };
    let is_admin = state.db.user_has_perm(&user, "tickets.respond").await;
    if t.user_id != user.id && !is_admin {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let body = req.body.trim();
    if body.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El mensaje está vacío.");
    }
    let flag = if is_admin { 1 } else { 0 };
    let _ = state.db.add_ticket_message(id, user.id, flag, body).await;
    if is_admin {
        let _ = state.db.reclamar_ticket_if_unclaimed(id, user.id).await;
    }
    let _ = state
        .db
        .update_ticket_status(id, if is_admin { "answered" } else { "open" })
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn support_ticket_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<TicketActionRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&user, "tickets.respond").await {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    match req.action.as_str() {
        "claim" => {
            let _ = state.db.reclamar_ticket(id, user.id).await;
        }
        "resolve" => {
            let _ = state.db.terminar_ticket(id).await;
        }
        "reject" => {
            let _ = state.db.rechazar_ticket(id).await;
        }
        "close" => {
            let _ = state.db.cerrar_ticket(id).await;
        }
        _ => return json_err(StatusCode::BAD_REQUEST, "Acción inválida."),
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== Perfil, Títulos y Sesiones =====================

async fn profile_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let current_tok = extract_bearer_token(&headers).unwrap_or_default();
    let (_, role_display) = state.db.role_display(&user).await;

    let titles = state
        .db
        .list_user_titles(user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| TitleDto {
            id: t.id,
            name: t.name,
            effect: t.effect,
            description: t.description.unwrap_or_default(),
            selected: user.visible_title_1 == Some(t.id)
                || user.visible_title_2 == Some(t.id)
                || user.visible_title_3 == Some(t.id),
            is_primary: user.primary_title_id == Some(t.id),
        })
        .collect();

    let sessions = state
        .db
        .list_user_sessions(user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|s| SessionDto {
            token_prefix: s.token.chars().take(10).collect(),
            is_current: s.token == current_tok,
            token: s.token,
            remember: s.remember != 0,
            created_at: s.created_at.format("%d/%m/%Y %H:%M").to_string(),
            last_seen: s.last_seen_at.format("%d/%m/%Y %H:%M").to_string(),
            expires: s.expires_at.format("%d/%m/%Y %H:%M").to_string(),
        })
        .collect();

    let now = chrono::Utc::now();
    let linked_devices = state
        .db
        .list_linked_devices(user.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|d| LinkedDeviceDto {
            token: d.token,
            device_name: d.device_name.unwrap_or_else(|| "(sin nombre)".into()),
            created_at: d.created_at.format("%d/%m/%Y %H:%M").to_string(),
            last_seen: d.last_seen_at.format("%d/%m/%Y %H:%M").to_string(),
            expires: d
                .link_expires_at
                .map(|t| t.format("%d/%m/%Y %H:%M").to_string())
                .unwrap_or_else(|| "—".into()),
            active: d.link_expires_at.map(|t| t > now).unwrap_or(false),
        })
        .collect();

    Json(ProfilePayload {
        username: user.username.clone(),
        email: user.email.clone(),
        display_name: user.display_name.clone().unwrap_or_default(),
        bio: user.bio.clone().unwrap_or_default(),
        role_display,
        created_at: user.created_at.format("%d/%m/%Y").to_string(),
        last_login: user
            .last_login_at
            .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|| "—".into()),
        usage: user.usage_string(),
        titles,
        sessions,
        linked_devices,
    })
    .into_response()
}

async fn profile_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateProfileRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let dn = sanitize_text(&req.display_name, 64);
    let bio = sanitize_text(&req.bio, 500);
    let dn_opt = if dn.is_empty() { None } else { Some(dn.as_str()) };
    let bio_opt = if bio.is_empty() { None } else { Some(bio.as_str()) };
    let _ = state.db.update_profile(user.id, dn_opt, bio_opt, None).await;
    let _ = state
        .db
        .audit(Some(user.id), "profile_update", Some("desktop"), None)
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn profile_titles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateTitlesRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let v1 = req.visible_ids.first().copied();
    let v2 = req.visible_ids.get(1).copied();
    let v3 = req.visible_ids.get(2).copied();
    match state
        .db
        .set_user_title_selection(user.id, req.primary_id, v1, v2, v3)
        .await
    {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_err(
            StatusCode::BAD_REQUEST,
            format!("No se pudo guardar títulos: {e}"),
        ),
    }
}

async fn profile_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if let Err(e) = validate_password(&req.new_password) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    if !verify_password(&req.current, &user.password_hash) {
        return json_err(
            StatusCode::UNAUTHORIZED,
            "La contraseña actual es incorrecta.",
        );
    }
    if state
        .db
        .update_password(user.id, &req.new_password)
        .await
        .is_err()
    {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error actualizando contraseña.",
        );
    }
    let _ = state
        .db
        .audit(Some(user.id), "password_change", Some("desktop"), None)
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn profile_revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let sessions = state
        .db
        .list_user_sessions(user.id)
        .await
        .unwrap_or_default();
    if !sessions.iter().any(|s| s.token == token) {
        return json_err(StatusCode::FORBIDDEN, "Sesión no encontrada.");
    }
    let _ = state.db.revoke_session_by_token(&token).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn profile_revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.revoke_linked_device(user.id, &token).await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== aexTube =====================

async fn aextube_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let rows = crate::handlers::aextube::list_videos_for_user(&state, user.id).await;
    let out: Vec<VideoDto> = rows
        .into_iter()
        .map(|v| VideoDto {
            id: v.id,
            user_id: v.user_id,
            author: v.author,
            title: v.title,
            description: v.description,
            is_short: v.is_short,
            file_path: v.file_path,
            mime_type: v.mime_type,
            size_bytes: v.size_bytes,
            likes: v.likes,
            dislikes: v.dislikes,
            views: v.views,
            created_at: v.created_at,
            user_reaction: v.user_reaction,
            subscribed: v.subscribed,
            channel_subscribers: v.channel_subscribers,
        })
        .collect();
    Json(out).into_response()
}

async fn aextube_react(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(video_id): Path<i64>,
    Json(req): Json<VideoReactRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if req.kind != "like" && req.kind != "dislike" {
        return json_err(StatusCode::BAD_REQUEST, "Reacción inválida.");
    }
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT reaction FROM aextube_reactions WHERE video_id = ? AND user_id = ?",
    )
    .bind(video_id)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await
    .unwrap_or(None);

    if let Some((prev,)) = existing {
        if prev == req.kind {
            let _ =
                sqlx::query("DELETE FROM aextube_reactions WHERE video_id = ? AND user_id = ?")
                    .bind(video_id)
                    .bind(user.id)
                    .execute(&state.db.pool)
                    .await;
        } else {
            let _ = sqlx::query(
                "UPDATE aextube_reactions SET reaction = ? WHERE video_id = ? AND user_id = ?",
            )
            .bind(&req.kind)
            .bind(video_id)
            .bind(user.id)
            .execute(&state.db.pool)
            .await;
        }
    } else {
        let _ = sqlx::query(
            "INSERT INTO aextube_reactions (video_id, user_id, reaction) VALUES (?, ?, ?)",
        )
        .bind(video_id)
        .bind(user.id)
        .bind(&req.kind)
        .execute(&state.db.pool)
        .await;
    }

    let likes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aextube_reactions WHERE video_id = ? AND reaction = 'like'",
    )
    .bind(video_id)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);
    let dislikes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM aextube_reactions WHERE video_id = ? AND reaction = 'dislike'",
    )
    .bind(video_id)
    .fetch_one(&state.db.pool)
    .await
    .unwrap_or(0);
    let _ = sqlx::query("UPDATE aextube_videos SET likes = ?, dislikes = ? WHERE id = ?")
        .bind(likes)
        .bind(dislikes)
        .bind(video_id)
        .execute(&state.db.pool)
        .await;

    Json(serde_json::json!({ "ok": true, "likes": likes, "dislikes": dislikes })).into_response()
}

async fn aextube_view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(video_id): Path<i64>,
) -> Response {
    let _user = resolve_bearer_or_guest(&state, &headers).await;
    let _ = sqlx::query("UPDATE aextube_videos SET views = views + 1 WHERE id = ?")
        .bind(video_id)
        .execute(&state.db.pool)
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn aextube_subscribe(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(author): Path<String>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let clean_author = author.trim().to_string();
    let exists: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM aextube_subscriptions WHERE channel_author = ? AND user_id = ?",
    )
    .bind(&clean_author)
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await
    .unwrap_or(None);

    if exists.is_some() {
        let _ = sqlx::query(
            "DELETE FROM aextube_subscriptions WHERE channel_author = ? AND user_id = ?",
        )
        .bind(&clean_author)
        .bind(user.id)
        .execute(&state.db.pool)
        .await;
    } else {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO aextube_subscriptions (channel_author, user_id) VALUES (?, ?)",
        )
        .bind(&clean_author)
        .bind(user.id)
        .execute(&state.db.pool)
        .await;
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn aextube_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(video_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT user_id, file_path FROM aextube_videos WHERE id = ?")
            .bind(video_id)
            .fetch_optional(&state.db.pool)
            .await
            .unwrap_or(None);
    let Some((owner_id, file_path)) = row else {
        return json_err(StatusCode::NOT_FOUND, "Vídeo no encontrado.");
    };
    let can_mod = state.db.user_has_perm(&user, "admin.access").await;
    if owner_id != user.id && !can_mod {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let _ = sqlx::query("DELETE FROM aextube_videos WHERE id = ?")
        .bind(video_id)
        .execute(&state.db.pool)
        .await;
    if let Some(fname) = file_path.strip_prefix("/aextube/media/") {
        if !fname.contains("..") && !fname.contains('/') {
            let _ = std::fs::remove_file(format!("data/aextube/{fname}"));
        }
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== aexOS =====================

async fn build_aexos_profile_dtos(state: &AppState, owner: &User) -> Vec<AexosProfileDto> {
    let rows = state
        .db
        .list_aexos_accounts(owner)
        .await
        .unwrap_or_default();
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let user_opt = state.db.find_user_by_id(r.user_id).await.ok().flatten();
        let can_admin = match &user_opt {
            Some(u) => state.db.user_has_perm(u, "admin.access").await,
            None => false,
        };
        let (_, role_display) = match &user_opt {
            Some(u) => state.db.role_display(u).await,
            None => ("usuario".into(), "Usuario".into()),
        };
        out.push(AexosProfileDto {
            user_id: r.user_id,
            is_primary: r.user_id == owner.id,
            username: r.username.clone(),
            display_name: r.display_name.unwrap_or_else(|| r.username),
            role_display,
            can_admin,
            wallpaper: r.wallpaper,
            show_desktop_icons: r.show_desktop_icons != 0,
            pinned_apps_json: r.pinned_apps_json,
            files_json: r.files_json,
            notes_text: r.notes_text,
        });
    }
    out
}

async fn aexos_profiles(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.ensure_primary_aexos_prefs(&user).await;
    Json(build_aexos_profile_dtos(&state, &user).await).into_response()
}

async fn aexos_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AexosLinkRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match state
        .db
        .link_existing_aexos_account(&user, &req.username, &req.password)
        .await
    {
        Ok(_) => Json(build_aexos_profile_dtos(&state, &user).await).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn aexos_switch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AexosSwitchRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if req.user_id != user.id {
        if let Err(e) = state
            .db
            .verify_and_create_aexos_session(user.id, req.user_id, req.password.as_deref())
            .await
        {
            return json_err(StatusCode::UNAUTHORIZED, e.to_string());
        }
    }
    Json(serde_json::json!({ "ok": true, "active_user_id": req.user_id })).into_response()
}

async fn aexos_save_prefs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AexosSavePrefsRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match state
        .db
        .save_aexos_prefs(
            user.id,
            req.profile_user_id,
            &req.wallpaper,
            req.show_desktop_icons,
            &req.pinned_apps_json,
            &req.files_json,
            &req.notes_text,
        )
        .await
    {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn aexos_delete_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(target_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match state.db.delete_aexos_account(user.id, target_id).await {
        Ok(_) => Json(build_aexos_profile_dtos(&state, &user).await).into_response(),
        Err(e) => json_err(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

// ===================== Admin =====================

async fn admin_overview(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "admin.access").await {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes acceso al panel de administración.",
        );
    }

    let (owner_count, admin_count, staff_count, user_count) =
        state.db.count_users_by_role().await.unwrap_or((0, 0, 0, 0));
    let total_users = owner_count + admin_count + staff_count + user_count;
    let webs_raw = state.db.list_all_webs().await.unwrap_or_default();
    let audit_total = state.db.count_audit_entries().await.unwrap_or(0);
    let visits_7d = state.db.total_visits_in_days(7).await.unwrap_or(0);

    let users_raw = state.db.list_users().await.unwrap_or_default();
    let mut users = Vec::with_capacity(users_raw.len());
    for u in users_raw {
        users.push(AdminUserDto {
            id: u.id,
            username: u.username.clone(),
            display: u.display(),
            email: u.email.clone(),
            role: u.role.clone(),
            status: u.status.clone(),
            last_login: u
                .last_login_at
                .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
                .unwrap_or_else(|| "—".into()),
        });
    }

    let webs = webs_raw
        .iter()
        .map(|w| AdminWebDto {
            id: w.id,
            slug: w.slug.clone(),
            name: w.name.clone(),
            icon: w.icon_emoji(),
            is_active: w.is_active != 0,
            is_public: w.is_public != 0,
            tag: w.tag_label().to_string(),
        })
        .collect();

    let audit_raw = state
        .db
        .list_audit_entries(40, 0)
        .await
        .unwrap_or_default();
    let mut audit = Vec::with_capacity(audit_raw.len());
    for a in audit_raw {
        let username = match a.user_id {
            Some(uid) => state
                .db
                .find_user_by_id(uid)
                .await
                .ok()
                .flatten()
                .map(|u| u.username)
                .unwrap_or_else(|| format!("#{uid}")),
            None => "sistema".into(),
        };
        audit.push(AdminAuditDto {
            id: a.id,
            username,
            action: a.action,
            detail: a.detail.unwrap_or_default(),
            created_at: a.created_at.format("%d/%m %H:%M").to_string(),
        });
    }

    Json(AdminOverviewDto {
        total_users,
        owner_count,
        admin_count,
        staff_count,
        user_count,
        web_count: webs_raw.len() as i64,
        audit_total,
        visits_7d,
        users,
        webs,
        audit,
    })
    .into_response()
}

async fn admin_set_role(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<AdminRoleRequest>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !is_role(&req.role) {
        return json_err(StatusCode::BAD_REQUEST, "Rol inválido.");
    }
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if !state.db.user_has_perm(&actor, "users.role.change").await
        || !actor.can_act_on(&target, ActionOnUser::ChangeRole)
    {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let _ = state.db.set_user_role(id, &req.role).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "role_change",
            Some(&format!("{} -> {} (id={id})", target.username, req.role)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_ban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<AdminBanRequest>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    let perm = if req.permanent {
        "users.ban.permanent"
    } else {
        "users.ban.temporary"
    };
    let act = if req.permanent {
        ActionOnUser::BanPermanent
    } else {
        ActionOnUser::BanTemporary
    };
    if !state.db.user_has_perm(&actor, perm).await || !actor.can_act_on(&target, act) {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let reason = if req.reason.trim().is_empty() {
        None
    } else {
        Some(req.reason.trim())
    };
    let _ = state
        .db
        .create_ban(id, actor.id, reason, req.permanent, req.duration_hours)
        .await;
    let _ = state.db.set_user_status(id, "banned").await;
    let _ = state.db.revoke_all_user_sessions(id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "user_ban",
            Some(&format!("{} (id={id})", target.username)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_unban_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if !state.db.user_has_perm(&actor, "users.unban").await
        || !actor.can_act_on(&target, ActionOnUser::BanTemporary)
    {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let now = chrono::Utc::now();
    let _ = sqlx::query(
        "UPDATE user_bans SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .execute(&state.db.pool)
    .await;
    let _ = sqlx::query(
        "UPDATE user_suspensions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .execute(&state.db.pool)
    .await;
    let _ = state.db.set_user_status(id, "active").await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "user_unban",
            Some(&format!("{} (id={id})", target.username)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_force_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if !state.db.user_has_perm(&actor, "users.force_logout").await
        || !actor.can_act_on(&target, ActionOnUser::ForceLogout)
    {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let _ = state.db.revoke_all_user_sessions(id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "force_logout",
            Some(&format!("{} (id={id})", target.username)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_toggle_web(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "webs.manage").await {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let Some(w) = state.db.get_web_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Sección no encontrada.");
    };
    let next = w.is_active == 0;
    let _ = state.db.set_web_active(id, next).await;
    Json(serde_json::json!({ "ok": true, "is_active": next })).into_response()
}

// ===================== Recuperación, Vinculación, Correo y Chat Edit/Delete =====================

async fn auth_recover(
    State(state): State<AppState>,
    Json(req): Json<RecoverRequest>,
) -> Response {
    let email = req.email.trim().to_string();
    if let Err(e) = validate_email(&email) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    let info_msg = "Si el email existe, recibirás instrucciones de recuperación.";
    let Some(user) = state.db.find_user_by_email(&email).await.ok().flatten() else {
        return Json(RecoverResponse {
            ok: true,
            info: info_msg.to_string(),
            dev_token: None,
        })
        .into_response();
    };

    let Ok(token) = state.db.create_recovery_token(user.id).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo generar el token de recuperación.",
        );
    };

    let base = state.cfg.public_url.trim_end_matches('/');
    let link = format!("{base}/reset?token={token}");
    let cancel_link = format!("{base}/reset/cancel?token={token}");
    let internal_email = state.db.primary_verified_email(user.id).await.ok().flatten();

    if let (Some(client), Some(to)) = (state.aethramail.as_ref().as_ref(), &internal_email) {
        let body = format!(
            "Recuperación de contraseña — Aethravox Studios\n\n\
             Recibimos una solicitud para cambiar la contraseña de tu cuenta.\n\n\
             ➜ Token de recuperación: {token}\n\
             ➜ Enlace web: {link}\n\
             ➜ ¿No fuiste tú? Anula el enlace: {cancel_link}\n\n\
             El token expira en 30 minutos.\n\n— Aethravox Studios"
        );
        let _ = client
            .send_email(to.as_str(), "Recuperación de contraseña — Aethravox", &body)
            .await;
        return Json(RecoverResponse {
            ok: true,
            info: format!("{info_msg} Enviado a tu correo interno {to}."),
            dev_token: None,
        })
        .into_response();
    }

    Json(RecoverResponse {
        ok: true,
        info: format!("{info_msg} (Modo dev: usa el token mostrado abajo)"),
        dev_token: Some(token),
    })
    .into_response()
}

async fn auth_reset(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Response {
    let tok = req.token.trim();
    if tok.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "Token de recuperación requerido.");
    }
    if let Err(e) = validate_password(&req.new_password) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    match state.db.consume_recovery_token(tok).await {
        Ok(Some(user_id)) => {
            if state
                .db
                .update_password(user_id, &req.new_password)
                .await
                .is_err()
            {
                return json_err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error al actualizar la contraseña.",
                );
            }
            let _ = state
                .db
                .audit(Some(user_id), "password_reset", Some("desktop"), None)
                .await;
            Json(serde_json::json!({ "ok": true })).into_response()
        }
        _ => json_err(
            StatusCode::BAD_REQUEST,
            "Token inválido, expirado o ya utilizado.",
        ),
    }
}

async fn link_code(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.cleanup_link_codes().await;
    match state.db.create_link_code(user.id).await {
        Ok((code, numeric_code, expires)) => Json(LinkCodeDto {
            code,
            numeric_code,
            expires_at: expires.format("%H:%M:%S UTC").to_string(),
            target_username: user.username,
        })
        .into_response(),
        Err(_) => json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo generar el código de vinculación.",
        ),
    }
}

async fn link_consume(
    State(state): State<AppState>,
    Json(req): Json<LinkConsumeRequest>,
) -> Response {
    let normalized: String = req
        .code
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if normalized.is_empty() {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Introduce el código de 6 dígitos o token de vinculación.",
        );
    }
    let Some(link) = state
        .db
        .find_active_link_code(&normalized)
        .await
        .ok()
        .flatten()
    else {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Código inválido, usado o expirado (dura 60 s).",
        );
    };

    let device_name = req
        .device_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_else(|| "Cliente Nativo".to_string());

    let Ok((token, _)) = state
        .db
        .create_linked_session(link.user_id, link.user_id, Some(&device_name))
        .await
    else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error creando la sesión vinculada.",
        );
    };

    let won = state
        .db
        .consume_link_code(&link.code, &token)
        .await
        .unwrap_or(false);
    if !won {
        let _ = state.db.revoke_session_by_token(&token).await;
        return json_err(
            StatusCode::CONFLICT,
            "Ese código ya fue usado por otro dispositivo.",
        );
    }

    let Some(user) = state.db.find_user_by_id(link.user_id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    let _ = state
        .db
        .audit(
            Some(user.id),
            "device_linked",
            Some(&format!("desktop device={device_name}")),
            None,
        )
        .await;
    let summary = build_user_summary(&state, &user).await;
    Json(AuthSuccess {
        token,
        user: summary,
    })
    .into_response()
}

async fn chat_edit_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(msg_id): Path<i64>,
    Json(req): Json<EditMessageRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let new_body = req.body.trim();
    if new_body.is_empty() || new_body.chars().count() > 4000 {
        return json_err(
            StatusCode::BAD_REQUEST,
            "El mensaje debe tener entre 1 y 4000 caracteres.",
        );
    }
    match state.db.edit_message(msg_id, user.id, new_body).await {
        Ok(true) => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => json_err(
            StatusCode::FORBIDDEN,
            "No puedes editar este mensaje (solo autor y máx. 15 min).",
        ),
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error editando mensaje."),
    }
}

async fn chat_delete_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(msg_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match state.db.soft_delete_message(msg_id, user.id).await {
        Ok(true) => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => json_err(
            StatusCode::FORBIDDEN,
            "No puedes borrar este mensaje (solo autor y máx. 30 min).",
        ),
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error borrando mensaje."),
    }
}

async fn profile_logout_all(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.revoke_all_user_sessions(user.id).await;
    let _ = state
        .db
        .audit(Some(user.id), "logout_all", Some("desktop"), None)
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn build_emails_payload(state: &AppState, user_id: i64) -> EmailsPayload {
    let rows = state.db.list_user_emails(user_id).await.unwrap_or_default();
    let emails = rows
        .into_iter()
        .map(|e| UserEmailDto {
            id: e.id,
            email: e.email.clone(),
            verified: e.is_verified(),
            is_primary: e.is_primary != 0,
        })
        .collect();
    let aethramail_url = crate::handlers::correo::resolve_aethramail_url(state).await;
    EmailsPayload {
        emails,
        aethramail_url,
    }
}

async fn emails_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    Json(build_emails_payload(&state, user.id).await).into_response()
}

async fn emails_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EmailRequestBody>,
) -> Response {
    use rand::RngCore;
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let email = req.email.trim().to_string();
    if let Err(err) = validate_email(&email) {
        return json_err(StatusCode::BAD_REQUEST, err);
    }
    if state
        .db
        .user_email_exists(user.id, &email)
        .await
        .unwrap_or(false)
        && state
            .db
            .user_email_is_verified(user.id, &email)
            .await
            .unwrap_or(false)
    {
        return json_err(
            StatusCode::CONFLICT,
            "Ese correo ya está vinculado y verificado.",
        );
    }
    let Some(client) = state.aethramail.as_ref().as_ref() else {
        return json_err(
            StatusCode::SERVICE_UNAVAILABLE,
            "El servicio AethraMail no está disponible en este servidor.",
        );
    };

    let mut buf = [0u8; 4];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    let code = format!("{:08}", u32::from_le_bytes(buf) % 100_000_000);
    let expires = chrono::Utc::now() + chrono::Duration::minutes(15);

    if state
        .db
        .create_email_change_request(user.id, &email, &code, expires)
        .await
        .is_err()
    {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error guardando solicitud de verificación.",
        );
    }

    let body = format!(
        "Verificación de correo — Aethravox Studios\n\n\
         Tu código de verificación (8 dígitos): {code}\n\n\
         Introdúcelo en Ajustes → Correo Interno. Expira en 15 minutos.\n\n— Aethravox Studios"
    );
    match client
        .send_email(&email, "Código de verificación — Aethravox", &body)
        .await
    {
        Ok(_) => Json(serde_json::json!({
            "ok": true,
            "message": format!("Código de 8 dígitos enviado a {email}.")
        }))
        .into_response(),
        Err(_) => json_err(
            StatusCode::BAD_REQUEST,
            "No se pudo enviar el código. ¿Existe ese correo en AethraMail?",
        ),
    }
}

async fn emails_verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EmailVerifyBody>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let email = req.email.trim().to_string();
    let code = req.code.trim().to_string();
    if code.len() != 8 || !code.chars().all(|c| c.is_ascii_digit()) {
        return json_err(StatusCode::BAD_REQUEST, "El código debe ser de 8 dígitos.");
    }
    match state.db.find_valid_email_change(user.id, &code).await {
        Ok(Some((req_id, new_email))) => {
            if new_email != email {
                return json_err(
                    StatusCode::BAD_REQUEST,
                    "Ese código no corresponde a ese correo.",
                );
            }
            let _ = state.db.consume_email_change(req_id).await;
            if state
                .db
                .add_verified_user_email(user.id, &email)
                .await
                .is_err()
            {
                return json_err(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error al vincular el correo.",
                );
            }
            let _ = state
                .db
                .audit(Some(user.id), "email_linked", Some(&email), None)
                .await;
            Json(build_emails_payload(&state, user.id).await).into_response()
        }
        _ => json_err(
            StatusCode::BAD_REQUEST,
            "Código incorrecto, expirado o ya utilizado.",
        ),
    }
}

async fn emails_set_primary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EmailIdBody>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.set_primary_user_email(user.id, req.email_id).await;
    Json(build_emails_payload(&state, user.id).await).into_response()
}

async fn emails_remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EmailIdBody>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let _ = state.db.remove_user_email(user.id, req.email_id).await;
    Json(build_emails_payload(&state, user.id).await).into_response()
}

// ===================== Gestión de Grupos (Chat) =====================

async fn build_group_manage_dto(
    state: &AppState,
    conv_id: i64,
    viewer_id: i64,
) -> Result<ChatGroupManageDto, Response> {
    if !state.db.is_conversation_member(conv_id, viewer_id).await {
        return Err(json_err(
            StatusCode::FORBIDDEN,
            "No eres miembro de esta conversación.",
        ));
    }
    let Some(conv) = state.db.get_conversation(conv_id).await.ok().flatten() else {
        return Err(json_err(StatusCode::NOT_FOUND, "Conversación no encontrada."));
    };
    if conv.conv_type == "dm" {
        return Err(json_err(
            StatusCode::BAD_REQUEST,
            "Solo los grupos tienen administración de miembros y rangos.",
        ));
    }

    let owner_id = conv.owner_id.unwrap_or(conv.created_by);
    let mut ranks_raw = state
        .db
        .list_conversation_ranks(conv_id)
        .await
        .unwrap_or_default();
    if ranks_raw.is_empty() {
        let _ = state.db.create_default_ranks(conv_id, owner_id).await;
        ranks_raw = state
            .db
            .list_conversation_ranks(conv_id)
            .await
            .unwrap_or_default();
    }

    let my_rank = state
        .db
        .get_member_rank(conv_id, viewer_id)
        .await
        .ok()
        .flatten();
    let is_owner =
        owner_id == viewer_id || my_rank.as_ref().map(|r| r.is_owner_rank != 0).unwrap_or(false);

    let ranks: Vec<ChatGroupRankDto> = ranks_raw
        .into_iter()
        .map(|r| ChatGroupRankDto {
            id: r.id,
            name: r.name,
            color: r.color.unwrap_or_else(|| "#888888".into()),
            position: r.position,
            is_owner_rank: r.is_owner_rank != 0,
            can_send_messages: r.can_send_messages != 0,
            can_add_members: r.can_add_members != 0,
            can_edit_group_info: r.can_edit_group_info != 0,
            can_delete_messages: r.can_delete_messages != 0,
            can_remove_members: r.can_remove_members != 0,
            can_manage_ranks: r.can_manage_ranks != 0,
            can_assign_ranks: r.can_assign_ranks != 0,
            can_pin_messages: r.can_pin_messages != 0,
            can_transfer_ownership: r.can_transfer_ownership != 0,
        })
        .collect();

    let member_ids = state
        .db
        .get_conversation_members(conv_id)
        .await
        .unwrap_or_default();
    let mut members = Vec::with_capacity(member_ids.len());
    for mid in member_ids {
        if let Ok(Some(u)) = state.db.find_user_by_id(mid).await {
            let r = state.db.get_member_rank(conv_id, mid).await.ok().flatten();
            let rank_id = r.as_ref().map(|x| x.id).unwrap_or(0);
            let rank_name = r
                .as_ref()
                .map(|x| x.name.clone())
                .unwrap_or_else(|| "Miembro".into());
            let rank_color = r
                .as_ref()
                .and_then(|x| x.color.clone())
                .unwrap_or_else(|| "#888888".into());
            let member_is_owner =
                mid == owner_id || r.as_ref().map(|x| x.is_owner_rank != 0).unwrap_or(false);
            members.push(ChatGroupMemberDto {
                user_id: u.id,
                username: u.username.clone(),
                display: u.display(),
                rank_id,
                rank_name,
                rank_color,
                is_owner: member_is_owner,
            });
        }
    }

    Ok(ChatGroupManageDto {
        conversation_id: conv.id,
        name: conv.name.unwrap_or_else(|| "Grupo".into()),
        description: conv.description.unwrap_or_default(),
        is_group: conv.conv_type != "dm",
        owner_id,
        my_user_id: viewer_id,
        can_add_members: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_add_members != 0)
                .unwrap_or(false),
        can_edit_group_info: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_edit_group_info != 0)
                .unwrap_or(false),
        can_remove_members: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_remove_members != 0)
                .unwrap_or(false),
        can_manage_ranks: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_manage_ranks != 0)
                .unwrap_or(false),
        can_assign_ranks: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_assign_ranks != 0)
                .unwrap_or(false),
        can_transfer_ownership: is_owner
            || my_rank
                .as_ref()
                .map(|r| r.can_transfer_ownership != 0)
                .unwrap_or(false),
        members,
        ranks,
    })
}

async fn chat_group_manage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_update_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
    Json(req): Json<UpdateGroupInfoRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_edit_group_info {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para editar la información del grupo.",
        );
    }
    let name = req.name.trim();
    if name.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El nombre del grupo es obligatorio.");
    }
    let desc = req
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let _ = sqlx::query("UPDATE conversations SET name = ?, description = ? WHERE id = ?")
        .bind(name)
        .bind(desc)
        .bind(conv_id)
        .execute(&state.db.pool)
        .await;
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_leave(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if info.owner_id == user.id && info.members.len() > 1 {
        return json_err(
            StatusCode::BAD_REQUEST,
            "Eres el propietario del grupo. Transfiere el ownership antes de salir.",
        );
    }
    let _ = sqlx::query("DELETE FROM conversation_members WHERE conversation_id = ? AND user_id = ?")
        .bind(conv_id)
        .bind(user.id)
        .execute(&state.db.pool)
        .await;
    Json(serde_json::json!({ "ok": true, "conversation_id": conv_id })).into_response()
}

async fn chat_group_add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
    Json(req): Json<AddGroupMemberRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_add_members {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para añadir miembros al grupo.",
        );
    }
    let uname = req.username.trim().trim_start_matches('@');
    let Some(target) = state.db.find_user_by_username(uname).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if state.db.is_conversation_member(conv_id, target.id).await {
        return json_err(StatusCode::CONFLICT, "Ese usuario ya pertenece al grupo.");
    }
    let _ = state.db.add_conversation_member(conv_id, target.id).await;
    if let Some(default_rank) = info.ranks.iter().find(|r| !r.is_owner_rank && r.name == "Miembro")
        .or_else(|| info.ranks.iter().find(|r| !r.is_owner_rank))
    {
        let _ = state
            .db
            .set_member_rank(conv_id, target.id, default_rank.id)
            .await;
    }
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_remove_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conv_id, target_id)): Path<(i64, i64)>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_remove_members {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para expulsar miembros.",
        );
    }
    if target_id == info.owner_id {
        return json_err(
            StatusCode::BAD_REQUEST,
            "No puedes expulsar al propietario del grupo.",
        );
    }
    let _ = sqlx::query("DELETE FROM conversation_members WHERE conversation_id = ? AND user_id = ?")
        .bind(conv_id)
        .bind(target_id)
        .execute(&state.db.pool)
        .await;
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_assign_rank(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
    Json(req): Json<AssignMemberRankRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_assign_ranks {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para asignar rangos.",
        );
    }
    if !state.db.is_conversation_member(conv_id, req.user_id).await {
        return json_err(StatusCode::NOT_FOUND, "Miembro no encontrado en el grupo.");
    }
    if !info.ranks.iter().any(|r| r.id == req.rank_id) {
        return json_err(StatusCode::BAD_REQUEST, "Rango inválido para este grupo.");
    }
    let _ = state
        .db
        .set_member_rank(conv_id, req.user_id, req.rank_id)
        .await;
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_create_rank(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
    Json(req): Json<CreateGroupRankRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_manage_ranks {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para gestionar rangos.",
        );
    }
    let name = req.name.trim();
    if name.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El nombre del rango es obligatorio.");
    }
    let color = req
        .color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let _ = state
        .db
        .create_rank(
            conv_id,
            name,
            color,
            req.can_send_messages,
            req.can_add_members,
            req.can_edit_group_info,
            req.can_delete_messages,
            req.can_remove_members,
            req.can_manage_ranks,
            req.can_assign_ranks,
            req.can_pin_messages,
            req.can_transfer_ownership,
        )
        .await;
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

async fn chat_group_delete_rank(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((conv_id, rank_id)): Path<(i64, i64)>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_manage_ranks {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para eliminar rangos.",
        );
    }
    if !info.ranks.iter().any(|r| r.id == rank_id) {
        return json_err(StatusCode::NOT_FOUND, "Rango no encontrado.");
    }
    match state.db.delete_rank(rank_id).await {
        Ok(true) => match build_group_manage_dto(&state, conv_id, user.id).await {
            Ok(dto) => Json(dto).into_response(),
            Err(resp) => resp,
        },
        Ok(false) => json_err(
            StatusCode::BAD_REQUEST,
            "No se puede eliminar: miembros lo usan o es el rango Owner.",
        ),
        Err(_) => json_err(StatusCode::INTERNAL_SERVER_ERROR, "Error eliminando rango."),
    }
}

async fn chat_group_transfer_ownership(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(conv_id): Path<i64>,
    Json(req): Json<TransferGroupOwnershipRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let info = match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(d) => d,
        Err(resp) => return resp,
    };
    if !info.can_transfer_ownership {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para transferir el ownership.",
        );
    }
    if !state
        .db
        .is_conversation_member(conv_id, req.new_owner_id)
        .await
    {
        return json_err(StatusCode::NOT_FOUND, "El nuevo propietario debe ser miembro.");
    }
    let _ = state.db.transfer_ownership(conv_id, req.new_owner_id).await;
    if req.new_owner_id != user.id {
        if let Some(fallback_rank) = info
            .ranks
            .iter()
            .find(|r| !r.is_owner_rank && r.name == "Admin")
            .or_else(|| info.ranks.iter().find(|r| !r.is_owner_rank))
        {
            let _ = state
                .db
                .set_member_rank(conv_id, user.id, fallback_rank.id)
                .await;
        }
    }
    match build_group_manage_dto(&state, conv_id, user.id).await {
        Ok(dto) => Json(dto).into_response(),
        Err(resp) => resp,
    }
}

// ===================== Admin Extendido (Bans IP, Config, Suspensión, Eliminar) =====================

async fn admin_suspend_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(req): Json<SuspendUserRequest>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if !state.db.user_has_perm(&actor, "users.suspend").await
        || !actor.can_act_on(&target, ActionOnUser::Suspend)
    {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let hours = req.duration_hours.max(1);
    let reason = if req.reason.trim().is_empty() {
        None
    } else {
        Some(req.reason.trim())
    };
    let _ = state
        .db
        .create_suspension(id, actor.id, reason, hours)
        .await;
    let _ = state.db.set_user_status(id, "suspended").await;
    let _ = state.db.revoke_all_user_sessions(id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "user_suspend",
            Some(&format!("{} (id={id}) {}h", target.username, hours)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(target) = state.db.find_user_by_id(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Usuario no encontrado.");
    };
    if target.role == "owner" {
        return json_err(
            StatusCode::FORBIDDEN,
            "No se puede eliminar la cuenta del Owner.",
        );
    }
    if !state.db.user_has_perm(&actor, "users.delete").await
        || !actor.can_act_on(&target, ActionOnUser::Delete)
    {
        return json_err(StatusCode::FORBIDDEN, "No autorizado.");
    }
    let _ = state.db.delete_user(id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "user_delete",
            Some(&format!("{} (id={id})", target.username)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_ip_bans_list(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "ip_bans.view").await {
        return json_err(StatusCode::FORBIDDEN, "No autorizado para ver bans de IP.");
    }

    let active_raw = state.db.list_active_ip_bans().await.unwrap_or_default();
    let history_raw = state.db.list_ip_bans_history(50).await.unwrap_or_default();

    let mut admin_names: HashMap<i64, String> = HashMap::new();
    for b in active_raw.iter().chain(history_raw.iter()) {
        if let Some(uid) = b.banned_by {
            if !admin_names.contains_key(&uid) {
                if let Ok(Some(u)) = state.db.find_user_by_id(uid).await {
                    admin_names.insert(uid, u.username);
                }
            }
        }
    }

    let map_ban = |b: crate::models::IpBan| IpBanDto {
        id: b.id,
        ip: b.ip,
        reason: b.reason,
        banned_by_username: b.banned_by.and_then(|id| admin_names.get(&id).cloned()),
        created_at: b.created_at.format("%d/%m/%Y %H:%M").to_string(),
        expires_at: b
            .expires_at
            .map(|dt| dt.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|| "Permanente".into()),
        is_permanent: b.expires_at.is_none(),
        revoked: b.revoked_at.is_some(),
    };

    Json(AdminIpBansPayload {
        active: active_raw.into_iter().map(&map_ban).collect(),
        history: history_raw.into_iter().map(map_ban).collect(),
    })
    .into_response()
}

async fn admin_ip_bans_create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateIpBanRequest>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "ip_bans.create").await {
        return json_err(StatusCode::FORBIDDEN, "No autorizado para crear bans de IP.");
    }
    let ip = req.ip.trim().to_string();
    let reason = req.reason.trim().to_string();
    if ip.is_empty() || reason.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "La IP y el motivo son obligatorios.");
    }
    let expires_at = if req.permanent {
        None
    } else {
        let hours = req.duration_hours.unwrap_or(24).max(1);
        Some(chrono::Utc::now() + chrono::Duration::hours(hours))
    };
    let _ = state
        .db
        .ban_ip(&ip, &reason, Some(actor.id), expires_at)
        .await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "ip_ban_created",
            Some(&format!("ip={ip} reason={reason}")),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn admin_ip_bans_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(ban_id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "ip_bans.revoke").await {
        return json_err(StatusCode::FORBIDDEN, "No autorizado para revocar bans de IP.");
    }
    let _ = state.db.revoke_ip_ban(ban_id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "ip_ban_revoked",
            Some(&format!("ban_id={ban_id}")),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn build_system_config_dto(state: &AppState) -> SystemConfigDto {
    let entries = state.db.list_config().await.unwrap_or_default();
    let map: HashMap<String, String> = entries
        .into_iter()
        .map(|c| (c.key, c.value))
        .collect();
    SystemConfigDto {
        studio_name: map.get("studio_name").cloned().unwrap_or_default(),
        foundation_date: map.get("foundation_date").cloned().unwrap_or_default(),
        allow_public_register: map
            .get("allow_public_register")
            .map(|s| s == "1")
            .unwrap_or(true),
        default_role: map
            .get("default_role")
            .cloned()
            .unwrap_or_else(|| "usuario".into()),
        maintenance_mode: map
            .get("maintenance_mode")
            .map(|s| s == "1")
            .unwrap_or(false),
    }
}

async fn admin_config_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "config.edit").await {
        return json_err(
            StatusCode::FORBIDDEN,
            "No autorizado para editar la configuración del sistema.",
        );
    }
    Json(build_system_config_dto(&state).await).into_response()
}

async fn admin_config_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SystemConfigDto>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "config.edit").await {
        return json_err(
            StatusCode::FORBIDDEN,
            "No autorizado para editar la configuración del sistema.",
        );
    }
    let studio_name = if req.studio_name.trim().is_empty() {
        "Aethravox Studios"
    } else {
        req.studio_name.trim()
    };
    let _ = state
        .db
        .set_config("studio_name", studio_name, actor.id)
        .await;
    let _ = state
        .db
        .set_config("foundation_date", req.foundation_date.trim(), actor.id)
        .await;
    let _ = state
        .db
        .set_config(
            "allow_public_register",
            if req.allow_public_register { "1" } else { "0" },
            actor.id,
        )
        .await;
    if is_role(req.default_role.trim()) {
        let _ = state
            .db
            .set_config("default_role", req.default_role.trim(), actor.id)
            .await;
    }
    let _ = state
        .db
        .set_config(
            "maintenance_mode",
            if req.maintenance_mode { "1" } else { "0" },
            actor.id,
        )
        .await;
    let _ = state
        .db
        .audit(Some(actor.id), "config_update", Some("system_config"), None)
        .await;
    Json(build_system_config_dto(&state).await).into_response()
}

async fn docs_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Response {
    let actor = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !state.db.user_has_perm(&actor, "docs.delete").await
        && !state.db.user_has_perm(&actor, "admin.access").await
    {
        return json_err(
            StatusCode::FORBIDDEN,
            "No tienes permiso para eliminar documentos.",
        );
    }
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return json_err(StatusCode::NOT_FOUND, "Documento no encontrado.");
    };
    let _ = std::fs::remove_file(&doc.file_path);
    if let Some(p) = &doc.preview_path {
        if !p.is_empty() {
            let _ = std::fs::remove_file(p);
        }
    }
    let _ = state.db.delete_document(id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "doc_delete",
            Some(&format!("id={} {}", id, doc.title)),
            None,
        )
        .await;
    Json(serde_json::json!({ "ok": true })).into_response()
}

// ===================== aexTube Subida Nativa (JSON + Base64) =====================

async fn aextube_upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UploadVideoRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let title = req.title.trim().to_string();
    if title.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El título es obligatorio.");
    }
    let bytes = match base64_decode(&req.data_base64) {
        Ok(b) => b,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e),
    };
    if bytes.is_empty() {
        return json_err(StatusCode::BAD_REQUEST, "El archivo de vídeo está vacío.");
    }
    if bytes.len() > 20 * 1024 * 1024 {
        return json_err(
            StatusCode::PAYLOAD_TOO_LARGE,
            "El vídeo supera el límite de 20 MB.",
        );
    }

    let lower_name = req.filename.to_lowercase();
    let ct_lower = req.content_type.to_lowercase();
    let ext = if ct_lower.contains("webm") || lower_name.ends_with(".webm") {
        "webm"
    } else if ct_lower.contains("quicktime") || lower_name.ends_with(".mov") {
        "mov"
    } else if ct_lower.contains("ogg") || lower_name.ends_with(".ogv") || lower_name.ends_with(".ogg")
    {
        "ogv"
    } else {
        "mp4"
    };
    let mime_type = if req.content_type.trim().is_empty() {
        match ext {
            "webm" => "video/webm".to_string(),
            "mov" => "video/quicktime".to_string(),
            "ogv" => "video/ogg".to_string(),
            _ => "video/mp4".to_string(),
        }
    } else {
        req.content_type.clone()
    };

    if std::fs::create_dir_all("data/aextube").is_err() {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo preparar la carpeta multimedia.",
        );
    }
    let filename = format!("{}.{}", crate::db::random_hex(12), ext);
    let disk_path = format!("data/aextube/{filename}");
    if std::fs::write(&disk_path, &bytes).is_err() {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo guardar el vídeo en el servidor.",
        );
    }

    let public_url = format!("/aextube/media/{filename}");
    let author = user.display();
    let size_bytes = bytes.len() as i64;
    let desc = req.description.trim().to_string();

    let res = sqlx::query_scalar::<_, i64>(
        "INSERT INTO aextube_videos (user_id, author, title, description, is_short, file_path, mime_type, size_bytes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(user.id)
    .bind(&author)
    .bind(&title)
    .bind(&desc)
    .bind(req.is_short as i64)
    .bind(&public_url)
    .bind(&mime_type)
    .bind(size_bytes)
    .fetch_one(&state.db.pool)
    .await;

    match res {
        Ok(_) => {
            let rows = crate::handlers::aextube::list_videos_for_user(&state, user.id).await;
            let out: Vec<VideoDto> = rows
                .into_iter()
                .map(|v| VideoDto {
                    id: v.id,
                    user_id: v.user_id,
                    author: v.author,
                    title: v.title,
                    description: v.description,
                    is_short: v.is_short,
                    file_path: v.file_path,
                    mime_type: v.mime_type,
                    size_bytes: v.size_bytes,
                    likes: v.likes,
                    dislikes: v.dislikes,
                    views: v.views,
                    created_at: v.created_at,
                    user_reaction: v.user_reaction,
                    subscribed: v.subscribed,
                    channel_subscribers: v.channel_subscribers,
                })
                .collect();
            Json(out).into_response()
        }
        Err(_) => {
            let _ = std::fs::remove_file(&disk_path);
            json_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error registrando el vídeo en la base de datos.",
            )
        }
    }
}

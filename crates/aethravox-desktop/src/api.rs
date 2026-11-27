use aethravox_core::{
    AdminBanRequest, AdminOverviewDto, AdminRoleRequest, AexosLinkRequest, AexosProfileDto,
    AexosSavePrefsRequest, AexosSwitchRequest, ApiError, ArisChatDto, ArisCreateChatRequest,
    ArisCreditsDto, ArisMessageDto, ArisMode, ArisSendRequest, ArisSendResponse, AuthSuccess,
    CategoryRowDto, ChangePasswordRequest, ChatConversationDto, ChatMessageDto, CommunityDetailDto,
    CommunityDto, CreateCommunityGroupRequest, CreateCommunityRequest, CreateGroupRequest,
    CreateTicketRequest, DocumentDto, DownloadAppDto, DownloadFileDto, DownloadPlatformDto,
    EditMessageRequest, EmailIdBody, EmailRequestBody, EmailVerifyBody, EmailsPayload, FriendDto,
    FriendRequestBody, FriendsPayload, HubPayload, HubStatsDto, InviteCommunityRequest,
    LinkCodeDto, LinkConsumeRequest, LoginRequest, PendingFriendDto, PlaygroundGenRequest,
    PlaygroundGenResult, ProfilePayload, RecoverRequest, RecoverResponse, RegisterRequest,
    ReplyTicketRequest, ResetPasswordRequest, SendMessageRequest, StartDmRequest,
    StartDmResponse, ThemeKind, ThemeSelectRequest, TicketActionRequest, TicketDetailDto,
    TicketDto, UpdateProfileRequest, UpdateTitlesRequest, UserSearchDto, UserSummary, VideoDto,
    VideoReactRequest, WebAppDto,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
pub type AsyncRt = tokio::runtime::Handle;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Default)]
pub struct AsyncRt;

#[cfg(target_arch = "wasm32")]
impl AsyncRt {
    pub fn spawn<F>(&self, fut: F)
    where
        F: std::future::Future<Output = ()> + 'static,
    {
        wasm_bindgen_futures::spawn_local(fut);
    }
}

#[derive(Debug, Clone)]
pub enum ApiEvent {
    AuthOk(AuthSuccess),
    AuthErr(String),
    HubLoaded(HubPayload),
    HubErr(String),
    #[allow(dead_code)]
    ThemeSaved(ThemeKind),
    #[allow(dead_code)]
    StatusNote(String),

    // Chat
    ConversationsLoaded(Vec<ChatConversationDto>),
    MessagesLoaded {
        conv_id: i64,
        messages: Vec<ChatMessageDto>,
    },
    MessageSent(ChatMessageDto),
    ChatRoomOpened(i64),
    ChatErr(String),
    ChatSendErr(String),

    // Aris IA
    ArisCreditsLoaded(ArisCreditsDto),
    ArisChatsLoaded(Vec<ArisChatDto>),
    ArisChatCreated(ArisChatDto),
    ArisChatDeleted(i64),
    ArisChatPinned {
        chat_id: i64,
        pinned: bool,
    },
    ArisMessagesLoaded {
        chat_id: i64,
        messages: Vec<ArisMessageDto>,
    },
    ArisReplyOk {
        chat_id: i64,
        res: ArisSendResponse,
    },
    ArisErr(String),

    // Amigos
    FriendsLoaded(FriendsPayload),
    FriendSearchLoaded(Vec<UserSearchDto>),
    FriendsActionOk(String),
    FriendsErr(String),

    // Comunidades
    CommunitiesLoaded(Vec<CommunityDto>),
    CommunityDetailLoaded(CommunityDetailDto),
    CommunityInviteableLoaded(Vec<UserSearchDto>),
    CommunitiesActionOk(String),
    CommunitiesErr(String),

    // Documentos y Descargas
    DocsLoaded(Vec<DocumentDto>),
    DocsErr(String),
    DownloadsLoaded(Vec<DownloadPlatformDto>),
    DownloadsErr(String),

    // Asistencia / Tickets
    TicketsLoaded(Vec<TicketDto>),
    TicketDetailLoaded(TicketDetailDto),
    SupportActionOk(String),
    SupportErr(String),

    // Perfil y Sesiones
    ProfileLoaded(ProfilePayload),
    ProfileActionOk(String),
    ProfileErr(String),

    // aexTube
    VideosLoaded(Vec<VideoDto>),
    AextubeActionOk(String),
    AextubeErr(String),

    // Playground
    PlaygroundGenOk(PlaygroundGenResult),
    PlaygroundGenErr(String),

    // aexOS
    AexosProfilesLoaded(Vec<AexosProfileDto>),
    AexosSwitched(i64),
    AexosActionOk(String),
    AexosErr(String),

    // Admin
    AdminOverviewLoaded(AdminOverviewDto),
    AdminActionOk(String),
    AdminErr(String),

    // Recuperación, Vinculación y Correo interno
    RecoverOk(RecoverResponse),
    ResetOk,
    LinkCodeLoaded(LinkCodeDto),
    EmailsLoaded(EmailsPayload),
    EmailRequestOk(String),
    EmailErr(String),
    LogoutAllOk,
}

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let is_unreserved = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if is_unreserved {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

async fn read_error(resp: reqwest::Response) -> String {
    let status = resp.status();
    resp.json::<ApiError>()
        .await
        .map(|e| e.error)
        .unwrap_or_else(|_| format!("HTTP {status}"))
}

async fn get_json<T: DeserializeOwned>(
    base_url: &str,
    path: &str,
    token: &Option<String>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut req = client.get(&url);
    if let Some(tok) = token {
        if !tok.is_empty() {
            req = req.bearer_auth(tok);
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<T>()
            .await
            .map_err(|e| format!("Error parseando respuesta: {e}")),
        Ok(resp) => Err(read_error(resp).await),
        Err(e) => Err(format!("Servidor no disponible: {e}")),
    }
}

async fn post_json<B: Serialize, T: DeserializeOwned>(
    base_url: &str,
    path: &str,
    token: &Option<String>,
    body: &B,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut req = client.post(&url).json(body);
    if let Some(tok) = token {
        if !tok.is_empty() {
            req = req.bearer_auth(tok);
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<T>()
            .await
            .map_err(|e| format!("Error parseando respuesta: {e}")),
        Ok(resp) => Err(read_error(resp).await),
        Err(e) => Err(format!("Servidor no disponible: {e}")),
    }
}

async fn post_empty<T: DeserializeOwned>(
    base_url: &str,
    path: &str,
    token: &Option<String>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut req = client.post(&url);
    if let Some(tok) = token {
        if !tok.is_empty() {
            req = req.bearer_auth(tok);
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<T>()
            .await
            .map_err(|e| format!("Error parseando respuesta: {e}")),
        Ok(resp) => Err(read_error(resp).await),
        Err(e) => Err(format!("Servidor no disponible: {e}")),
    }
}

async fn delete_json<T: DeserializeOwned>(
    base_url: &str,
    path: &str,
    token: &Option<String>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let mut req = client.delete(&url);
    if let Some(tok) = token {
        if !tok.is_empty() {
            req = req.bearer_auth(tok);
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<T>()
            .await
            .map_err(|e| format!("Error parseando respuesta: {e}")),
        Ok(resp) => Err(read_error(resp).await),
        Err(e) => Err(format!("Servidor no disponible: {e}")),
    }
}

// ===================== Auth & Hub =====================

pub fn spawn_login(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: LoginRequest,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/auth/login", base_url.trim_end_matches('/'));
        match client.post(&url).json(&req).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<AuthSuccess>().await {
                Ok(data) => {
                    let _ = tx.send(ApiEvent::AuthOk(data));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::AuthErr(format!("Error parseando respuesta: {e}")));
                }
            },
            Ok(resp) => {
                let msg = read_error(resp).await;
                let _ = tx.send(ApiEvent::AuthErr(msg));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(format!(
                    "No se pudo conectar al servidor ({url}): {e}"
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_register(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: RegisterRequest,
) {
    rt.spawn(async move {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/auth/register", base_url.trim_end_matches('/'));
        match client.post(&url).json(&req).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<AuthSuccess>().await {
                Ok(data) => {
                    let _ = tx.send(ApiEvent::AuthOk(data));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::AuthErr(format!("Error parseando respuesta: {e}")));
                }
            },
            Ok(resp) => {
                let msg = read_error(resp).await;
                let _ = tx.send(ApiEvent::AuthErr(msg));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(format!(
                    "No se pudo conectar al servidor ({url}): {e}"
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_logout(rt: &AsyncRt, base_url: String, token: Option<String>) {
    rt.spawn(async move {
        if token.as_ref().map(|t| !t.is_empty()).unwrap_or(false) {
            let _: Result<serde_json::Value, _> =
                post_empty(&base_url, "/api/v1/auth/logout", &token).await;
        }
    });
}

pub fn spawn_fetch_hub(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<HubPayload>(&base_url, "/api/v1/hub", &token).await {
            Ok(hub) => {
                let _ = tx.send(ApiEvent::HubLoaded(hub));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::HubErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_save_theme(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    theme: ThemeKind,
) {
    rt.spawn(async move {
        if token.as_ref().map(|t| !t.is_empty()).unwrap_or(false) {
            let _: Result<serde_json::Value, _> = post_json(
                &base_url,
                "/api/v1/theme",
                &token,
                &ThemeSelectRequest {
                    theme: theme.as_str().to_string(),
                },
            )
            .await;
        }
        let _ = tx.send(ApiEvent::ThemeSaved(theme));
        ctx.request_repaint();
    });
}

// ===================== Chat =====================

pub fn spawn_fetch_conversations(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<ChatConversationDto>>(&base_url, "/api/v1/chat/conversations", &token)
            .await
        {
            Ok(list) => {
                let _ = tx.send(ApiEvent::ConversationsLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_messages(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/messages");
        match get_json::<Vec<ChatMessageDto>>(&base_url, &path, &token).await {
            Ok(messages) => {
                let _ = tx.send(ApiEvent::MessagesLoaded { conv_id, messages });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_send_message(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    body: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/messages");
        match post_json::<SendMessageRequest, ChatMessageDto>(
            &base_url,
            &path,
            &token,
            &SendMessageRequest { body },
        )
        .await
        {
            Ok(msg) => {
                let _ = tx.send(ApiEvent::MessageSent(msg));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatSendErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_mark_read(
    rt: &AsyncRt,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/read");
        let _: Result<serde_json::Value, _> = post_empty(&base_url, &path, &token).await;
    });
}

pub fn spawn_start_dm(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    username: String,
) {
    rt.spawn(async move {
        match post_json::<StartDmRequest, StartDmResponse>(
            &base_url,
            "/api/v1/chat/dm",
            &token,
            &StartDmRequest { username },
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ChatRoomOpened(res.conversation_id));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_create_group(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: CreateGroupRequest,
) {
    rt.spawn(async move {
        match post_json::<CreateGroupRequest, StartDmResponse>(
            &base_url,
            "/api/v1/chat/group",
            &token,
            &req,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ChatRoomOpened(res.conversation_id));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Aris IA =====================

pub fn spawn_aris_credits(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        if let Ok(c) = get_json::<ArisCreditsDto>(&base_url, "/api/v1/aris/credits", &token).await {
            let _ = tx.send(ApiEvent::ArisCreditsLoaded(c));
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_chats(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<ArisChatDto>>(&base_url, "/api/v1/aris/chats", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::ArisChatsLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_create_chat(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title: Option<String>,
) {
    rt.spawn(async move {
        match post_json::<ArisCreateChatRequest, ArisChatDto>(
            &base_url,
            "/api/v1/aris/chats",
            &token,
            &ArisCreateChatRequest { title },
        )
        .await
        {
            Ok(chat) => {
                let _ = tx.send(ApiEvent::ArisChatCreated(chat));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_delete_chat(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    chat_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aris/chats/{chat_id}");
        match delete_json::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ArisChatDeleted(chat_id));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_pin_chat(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    chat_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aris/chats/{chat_id}/pin");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(v) => {
                let pinned = v.get("pinned").and_then(|b| b.as_bool()).unwrap_or(false);
                let _ = tx.send(ApiEvent::ArisChatPinned { chat_id, pinned });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_messages(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    chat_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aris/chats/{chat_id}/messages");
        match get_json::<Vec<ArisMessageDto>>(&base_url, &path, &token).await {
            Ok(messages) => {
                let _ = tx.send(ApiEvent::ArisMessagesLoaded { chat_id, messages });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aris_send(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    chat_id: i64,
    content: String,
    mode: ArisMode,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aris/chats/{chat_id}/send");
        match post_json::<ArisSendRequest, ArisSendResponse>(
            &base_url,
            &path,
            &token,
            &ArisSendRequest { content, mode },
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ArisReplyOk { chat_id, res });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ArisErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Amigos =====================

pub fn spawn_fetch_friends(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<FriendsPayload>(&base_url, "/api/v1/friends", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::FriendsLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::FriendsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_search_friends(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    query: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/friends/search?q={}", encode_uri_component(&query));
        match get_json::<Vec<UserSearchDto>>(&base_url, &path, &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::FriendSearchLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::FriendsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_send_friend_request(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    username: String,
) {
    rt.spawn(async move {
        match post_json::<FriendRequestBody, serde_json::Value>(
            &base_url,
            "/api/v1/friends/request",
            &token,
            &FriendRequestBody {
                username: username.clone(),
            },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::FriendsActionOk(format!(
                    "Solicitud enviada a @{username}."
                )));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::FriendsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_friend_action(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    action: &'static str, // "accept" | "reject" | "remove"
    target_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/friends/{action}/{target_id}");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let msg = match action {
                    "accept" => "Solicitud aceptada.",
                    "reject" => "Solicitud rechazada.",
                    _ => "Amigo eliminado.",
                };
                let _ = tx.send(ApiEvent::FriendsActionOk(msg.to_string()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::FriendsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Comunidades =====================

pub fn spawn_fetch_communities(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<CommunityDto>>(&base_url, "/api/v1/communities", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::CommunitiesLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_create_community(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: CreateCommunityRequest,
) {
    rt.spawn(async move {
        match post_json::<CreateCommunityRequest, serde_json::Value>(
            &base_url,
            "/api/v1/communities/create",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::CommunitiesActionOk("Comunidad creada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_community_detail(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/communities/{id}");
        match get_json::<CommunityDetailDto>(&base_url, &path, &token).await {
            Ok(detail) => {
                let _ = tx.send(ApiEvent::CommunityDetailLoaded(detail));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_community_membership(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    id: i64,
    join: bool,
) {
    rt.spawn(async move {
        let path = if join {
            format!("/api/v1/communities/{id}/join")
        } else {
            format!("/api/v1/communities/{id}/leave")
        };
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let msg = if join {
                    "Te has unido a la comunidad."
                } else {
                    "Has salido de la comunidad."
                };
                let _ = tx.send(ApiEvent::CommunitiesActionOk(msg.into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_community_create_group(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    community_id: i64,
    req: CreateCommunityGroupRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/communities/{community_id}/group");
        match post_json::<CreateCommunityGroupRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::CommunitiesActionOk("Grupo creado en la comunidad.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_community_inviteable(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    community_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/communities/{community_id}/inviteable");
        if let Ok(list) = get_json::<Vec<UserSearchDto>>(&base_url, &path, &token).await {
            let _ = tx.send(ApiEvent::CommunityInviteableLoaded(list));
        }
        ctx.request_repaint();
    });
}

pub fn spawn_community_invite(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    community_id: i64,
    user_ids: Vec<i64>,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/communities/{community_id}/invite");
        match post_json::<InviteCommunityRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &InviteCommunityRequest { user_ids },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::CommunitiesActionOk("Invitaciones enviadas.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::CommunitiesErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Documentos & Descargas =====================

pub fn spawn_fetch_docs(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<DocumentDto>>(&base_url, "/api/v1/docs", &token).await {
            Ok(docs) => {
                let _ = tx.send(ApiEvent::DocsLoaded(docs));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::DocsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_downloads(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<DownloadPlatformDto>>(&base_url, "/api/v1/downloads", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::DownloadsLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::DownloadsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Asistencia / Tickets =====================

pub fn spawn_fetch_tickets(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<TicketDto>>(&base_url, "/api/v1/support/tickets", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::TicketsLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::SupportErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_create_ticket(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: CreateTicketRequest,
) {
    rt.spawn(async move {
        match post_json::<CreateTicketRequest, serde_json::Value>(
            &base_url,
            "/api/v1/support/tickets",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::SupportActionOk("Ticket enviado correctamente.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::SupportErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_ticket_detail(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/support/tickets/{id}");
        match get_json::<TicketDetailDto>(&base_url, &path, &token).await {
            Ok(d) => {
                let _ = tx.send(ApiEvent::TicketDetailLoaded(d));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::SupportErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_reply_ticket(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    id: i64,
    body: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/support/tickets/{id}/reply");
        match post_json::<ReplyTicketRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &ReplyTicketRequest { body },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::SupportActionOk("Respuesta enviada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::SupportErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_ticket_action(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    id: i64,
    action: &str,
) {
    let action = action.to_string();
    rt.spawn(async move {
        let path = format!("/api/v1/support/tickets/{id}/action");
        match post_json::<TicketActionRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &TicketActionRequest { action },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::SupportActionOk("Estado del ticket actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::SupportErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Perfil, Títulos y Sesiones =====================

pub fn spawn_fetch_profile(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<ProfilePayload>(&base_url, "/api/v1/profile", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::ProfileLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_update_profile(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: UpdateProfileRequest,
) {
    rt.spawn(async move {
        match post_json::<UpdateProfileRequest, serde_json::Value>(
            &base_url,
            "/api/v1/profile/update",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Perfil actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_update_titles(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: UpdateTitlesRequest,
) {
    rt.spawn(async move {
        match post_json::<UpdateTitlesRequest, serde_json::Value>(
            &base_url,
            "/api/v1/profile/titles",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Títulos guardados.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_change_password(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: ChangePasswordRequest,
) {
    rt.spawn(async move {
        match post_json::<ChangePasswordRequest, serde_json::Value>(
            &base_url,
            "/api/v1/profile/password",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Contraseña actualizada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_revoke_session(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    target_token: String,
) {
    rt.spawn(async move {
        let path = format!(
            "/api/v1/profile/sessions/{}/revoke",
            encode_uri_component(&target_token)
        );
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Sesión revocada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_revoke_device(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    target_token: String,
) {
    rt.spawn(async move {
        let path = format!(
            "/api/v1/profile/devices/{}/revoke",
            encode_uri_component(&target_token)
        );
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Dispositivo desvinculado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== aexTube =====================

pub fn spawn_fetch_videos(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<VideoDto>>(&base_url, "/api/v1/aextube/videos", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::VideosLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AextubeErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_react_video(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    video_id: i64,
    kind: &str,
) {
    let kind = kind.to_string();
    rt.spawn(async move {
        let path = format!("/api/v1/aextube/videos/{video_id}/react");
        match post_json::<VideoReactRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &VideoReactRequest { kind },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AextubeActionOk("Reacción actualizada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AextubeErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_view_video(
    rt: &AsyncRt,
    base_url: String,
    token: Option<String>,
    video_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aextube/videos/{video_id}/view");
        let _: Result<serde_json::Value, _> = post_empty(&base_url, &path, &token).await;
    });
}

pub fn spawn_subscribe_channel(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    author: String,
) {
    rt.spawn(async move {
        let path = format!(
            "/api/v1/aextube/channels/{}/subscribe",
            encode_uri_component(&author)
        );
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AextubeActionOk("Suscripción actualizada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AextubeErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_delete_video(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    video_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aextube/videos/{video_id}");
        match delete_json::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AextubeActionOk("Vídeo eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AextubeErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Playground =====================

pub fn spawn_playground_generate(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: PlaygroundGenRequest,
) {
    rt.spawn(async move {
        match post_json::<PlaygroundGenRequest, PlaygroundGenResult>(
            &base_url,
            "/api/v1/aris/playground/generate",
            &token,
            &req,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::PlaygroundGenOk(res));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::PlaygroundGenErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== aexOS =====================

pub fn spawn_aexos_profiles(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<AexosProfileDto>>(&base_url, "/api/v1/aexos/profiles", &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::AexosProfilesLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AexosErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aexos_link(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AexosLinkRequest,
) {
    rt.spawn(async move {
        match post_json::<AexosLinkRequest, Vec<AexosProfileDto>>(
            &base_url,
            "/api/v1/aexos/link",
            &token,
            &req,
        )
        .await
        {
            Ok(list) => {
                let _ = tx.send(ApiEvent::AexosProfilesLoaded(list));
                let _ = tx.send(ApiEvent::AexosActionOk("Cuenta vinculada a aexOS.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AexosErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aexos_switch(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AexosSwitchRequest,
) {
    rt.spawn(async move {
        let target_id = req.user_id;
        match post_json::<AexosSwitchRequest, serde_json::Value>(
            &base_url,
            "/api/v1/aexos/switch",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AexosSwitched(target_id));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AexosErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aexos_save_prefs(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AexosSavePrefsRequest,
) {
    rt.spawn(async move {
        match post_json::<AexosSavePrefsRequest, serde_json::Value>(
            &base_url,
            "/api/v1/aexos/prefs",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AexosActionOk("Preferencias de aexOS guardadas.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AexosErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_aexos_delete(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    profile_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/aexos/profiles/{profile_id}");
        match delete_json::<Vec<AexosProfileDto>>(&base_url, &path, &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::AexosProfilesLoaded(list));
                let _ = tx.send(ApiEvent::AexosActionOk("Perfil desvinculado de aexOS.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AexosErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Admin =====================

pub fn spawn_admin_overview(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<AdminOverviewDto>(&base_url, "/api/v1/admin/overview", &token).await {
            Ok(data) => {
                let _ = tx.send(ApiEvent::AdminOverviewLoaded(data));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_set_role(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
    role: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/role");
        match post_json::<AdminRoleRequest, serde_json::Value>(
            &base_url,
            &path,
            &token,
            &AdminRoleRequest { role },
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Rol actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_ban_user(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
    req: AdminBanRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/ban");
        match post_json::<AdminBanRequest, serde_json::Value>(&base_url, &path, &token, &req)
            .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Usuario sancionado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_simple_action(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    path: String,
    ok_msg: &'static str,
) {
    rt.spawn(async move {
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk(ok_msg.into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Recuperación, Vinculación, Correo y Chat Edit/Delete =====================

pub fn spawn_recover(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: RecoverRequest,
) {
    rt.spawn(async move {
        match post_json::<RecoverRequest, RecoverResponse>(
            &base_url,
            "/api/v1/auth/recover",
            &None,
            &req,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::RecoverOk(res));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_reset(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: ResetPasswordRequest,
) {
    rt.spawn(async move {
        match post_json::<ResetPasswordRequest, serde_json::Value>(
            &base_url,
            "/api/v1/auth/reset",
            &None,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ResetOk);
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_link_code(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<LinkCodeDto>(&base_url, "/api/v1/link/code", &token).await {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::LinkCodeLoaded(dto));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_link_consume(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    req: LinkConsumeRequest,
) {
    rt.spawn(async move {
        match post_json::<LinkConsumeRequest, AuthSuccess>(
            &base_url,
            "/api/v1/link/consume",
            &None,
            &req,
        )
        .await
        {
            Ok(auth) => {
                let _ = tx.send(ApiEvent::AuthOk(auth));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AuthErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_edit_message(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    msg_id: i64,
    body: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/messages/{msg_id}/edit");
        let req = EditMessageRequest { body };
        match post_json::<EditMessageRequest, serde_json::Value>(&base_url, &path, &token, &req)
            .await
        {
            Ok(_) => {
                let msgs_path = format!("/api/v1/chat/conversations/{conv_id}/messages");
                if let Ok(list) = get_json::<Vec<ChatMessageDto>>(&base_url, &msgs_path, &token).await
                {
                    let _ = tx.send(ApiEvent::MessagesLoaded {
                        conv_id,
                        messages: list,
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_delete_message(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    msg_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/messages/{msg_id}/delete");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let msgs_path = format!("/api/v1/chat/conversations/{conv_id}/messages");
                if let Ok(list) = get_json::<Vec<ChatMessageDto>>(&base_url, &msgs_path, &token).await
                {
                    let _ = tx.send(ApiEvent::MessagesLoaded {
                        conv_id,
                        messages: list,
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_logout_all(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match post_empty::<serde_json::Value>(&base_url, "/api/v1/profile/logout_all", &token).await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::LogoutAllOk);
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_emails(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<EmailsPayload>(&base_url, "/api/v1/emails", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::EmailsLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::EmailErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_request_email(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    email: String,
) {
    rt.spawn(async move {
        let req = EmailRequestBody { email };
        match post_json::<EmailRequestBody, serde_json::Value>(
            &base_url,
            "/api/v1/emails/request",
            &token,
            &req,
        )
        .await
        {
            Ok(val) => {
                let msg = val
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Código de 8 dígitos enviado.")
                    .to_string();
                let _ = tx.send(ApiEvent::EmailRequestOk(msg));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::EmailErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_verify_email(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    email: String,
    code: String,
) {
    rt.spawn(async move {
        let req = EmailVerifyBody { email, code };
        match post_json::<EmailVerifyBody, EmailsPayload>(
            &base_url,
            "/api/v1/emails/verify",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::EmailsLoaded(p));
                let _ = tx.send(ApiEvent::ProfileActionOk(
                    "Correo interno vinculado y verificado.".into(),
                ));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::EmailErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_set_primary_email(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    email_id: i64,
) {
    rt.spawn(async move {
        let req = EmailIdBody { email_id };
        match post_json::<EmailIdBody, EmailsPayload>(
            &base_url,
            "/api/v1/emails/primary",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::EmailsLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::EmailErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_remove_email(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    email_id: i64,
) {
    rt.spawn(async move {
        let req = EmailIdBody { email_id };
        match post_json::<EmailIdBody, EmailsPayload>(
            &base_url,
            "/api/v1/emails/remove",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::EmailsLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::EmailErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Demo / Preview local =====================

pub fn offline_demo_hub(username: &str, is_guest: bool, theme: ThemeKind) -> HubPayload {
    let user = UserSummary {
        id: if is_guest { 0 } else { 1 },
        username: username.to_string(),
        display: username.to_string(),
        role_key: if is_guest { "invitado" } else { "owner" }.to_string(),
        role_display: if is_guest { "Invitado" } else { "Owner" }.to_string(),
        role_color: if is_guest { "#6b7280" } else { "#ef4444" }.to_string(),
        role_text_color: "#ffffff".to_string(),
        is_guest,
        can_access_admin: !is_guest,
        bio: "Modo vista previa local de Aethravox Desktop (egui).".to_string(),
        theme,
    };

    let app = |id: i64, slug: &str, name: &str, desc: &str, icon: &str, tag: &str| WebAppDto {
        id,
        slug: slug.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        icon: icon.to_string(),
        url: format!("/{slug}"),
        tag: tag.to_string(),
        tag_label: if tag.is_empty() {
            "Activo".to_string()
        } else {
            tag.to_string()
        },
        tag_color: match tag {
            "Nuevo" => "#00d4ff",
            "Beta" => "#ff3e3e",
            "Próximamente" => "#e0a308",
            _ => "#00ff88",
        }
        .to_string(),
        clickable: tag != "Próximamente",
        lock_message: "Esta sección estará disponible pronto.".to_string(),
    };

    let categories = vec![
        CategoryRowDto {
            name: "Inteligencia Artificial".into(),
            slug: "ia".into(),
            items: vec![app(
                1,
                "aris",
                "Aris",
                "Tu asistente de inteligencia artificial. Conversa, pregunta, crea.",
                "✨",
                "Nuevo",
            )],
        },
        CategoryRowDto {
            name: "Comunicación".into(),
            slug: "comunicacion".into(),
            items: vec![
                app(2, "chat", "Mensajería", "Chat privado y grupal en tiempo real.", "💬", ""),
                app(3, "friends", "Amigos", "Lista de amigos y solicitudes pendientes.", "👥", ""),
                app(4, "communities", "Comunidades", "Espacios con múltiples grupos organizados.", "🌐", ""),
            ],
        },
        CategoryRowDto {
            name: "Entretenimiento".into(),
            slug: "entretenimiento".into(),
            items: vec![app(
                5,
                "neon-canvas",
                "Neon Canvas",
                "Juego de dibujo neón 2D nativo en egui Painter.",
                "🎨",
                "Beta",
            )],
        },
        CategoryRowDto {
            name: "Creatividad".into(),
            slug: "creatividad".into(),
            items: vec![
                app(6, "docs", "Documentos", "Galería y visor nativo de documentos.", "📄", ""),
                app(7, "aextube", "aexTube", "Plataforma de vídeo del estudio.", "📺", ""),
                app(8, "playground", "Playground", "Laboratorio interactivo de código.", "🧪", ""),
            ],
        },
        CategoryRowDto {
            name: "Herramientas".into(),
            slug: "herramientas".into(),
            items: vec![
                app(9, "aexos", "aexOS", "Entorno de escritorio interactivo.", "🖥", ""),
                app(10, "transmitir", "Transmitir", "Comparte tu pantalla en red local.", "📡", ""),
                app(11, "descargas", "Descargas", "Clientes nativos y releases.", "⬇", ""),
            ],
        },
        CategoryRowDto {
            name: "Asistencia".into(),
            slug: "asistencia".into(),
            items: vec![app(
                12,
                "support",
                "Asistencia",
                "Envía sugerencias, reportes o consultas al equipo.",
                "🛟",
                "",
            )],
        },
    ];

    HubPayload {
        user,
        categories,
        stats: HubStatsDto {
            online_count: 1,
            doc_count: 4,
            chat_unread: 2,
            community_unread: 0,
            owner_count: 1,
            admin_count: 0,
            staff_count: 0,
            user_count: 1,
        },
    }
}

pub fn demo_chat_conversations() -> Vec<ChatConversationDto> {
    vec![
        ChatConversationDto {
            id: 1,
            title: "Equipo Aethravox".into(),
            is_group: true,
            last_message: "Migrando la UI a egui nativo 🚀".into(),
            unread_count: 2,
        },
        ChatConversationDto {
            id: 2,
            title: "Aris · Notas rápidas".into(),
            is_group: false,
            last_message: "Listo el sistema de temas.".into(),
            unread_count: 0,
        },
    ]
}

pub fn demo_chat_messages() -> Vec<ChatMessageDto> {
    vec![
        ChatMessageDto {
            id: 1,
            conversation_id: 1,
            sender: "Aethravox".into(),
            content: "Bienvenido al cliente nativo de Aethravox Studios en Rust + egui.".into(),
            created_at: "13:00".into(),
            is_mine: false,
        },
        ChatMessageDto {
            id: 2,
            conversation_id: 1,
            sender: "Tú".into(),
            content: "Adiós Askama y CSS, ahora todo corre directo en GPU.".into(),
            created_at: "13:02".into(),
            is_mine: true,
        },
    ]
}

pub fn demo_aris_chats() -> (Vec<ArisChatDto>, Vec<ArisMessageDto>) {
    (
        vec![ArisChatDto {
            id: 1,
            title: "Arquitectura Desktop".into(),
            pinned: true,
            updated_at: "Ahora".into(),
        }],
        vec![ArisMessageDto {
            id: 1,
            role: "assistant".into(),
            content: "Hola, soy Aris. Estoy lista en el nuevo cliente nativo de Aethravox Studios."
                .into(),
            model: Some("aris-core".into()),
            created_at: "Ahora".into(),
        }],
    )
}

pub fn demo_friends() -> FriendsPayload {
    FriendsPayload {
        friends: vec![FriendDto {
            id: 2,
            username: "aris".into(),
            display: "Aris Core".into(),
            online: true,
            role_display: "Staff".into(),
            role_color: "#00d4ff".into(),
        }],
        pending: vec![PendingFriendDto {
            id: 3,
            username: "shadow_dev".into(),
            display: "Shadow Dev".into(),
        }],
    }
}

pub fn demo_communities() -> Vec<CommunityDto> {
    vec![CommunityDto {
        id: 1,
        name: "Aethravox Core".into(),
        description: "Comunidad principal de ingeniería, Rust, IA y videojuegos.".into(),
        icon: "🌐".into(),
        is_member: true,
        is_owner: true,
        member_count: 12,
    }]
}

pub fn demo_docs() -> Vec<DocumentDto> {
    vec![DocumentDto {
        id: 1,
        title: "Guía de Arquitectura Aethravox".into(),
        description: "Resumen técnico del backend Axum + cliente nativo egui.".into(),
        file_name: "aethravox_arch.pdf".into(),
        file_type: "pdf".into(),
        icon: "📕".into(),
        size: "1.4 MB".into(),
        created_at: "26/11/2026".into(),
        has_preview: true,
        downloaded: 42,
    }]
}

pub fn demo_downloads() -> Vec<DownloadPlatformDto> {
    vec![
        DownloadPlatformDto {
            key: "linux".into(),
            name: "Linux".into(),
            icon: "🐧".into(),
            coming_soon: false,
            apps: vec![DownloadAppDto {
                slug: "aethravox-desktop".into(),
                name: "Aethravox Desktop".into(),
                files: vec![DownloadFileDto {
                    id: 1,
                    version: "0.1.0".into(),
                    orig_name: "aethravox-desktop-linux-x86_64.tar.gz".into(),
                    size_mb: "8.2 MB".into(),
                    created_at: "2026-11-26".into(),
                    url: "/descargas".into(),
                }],
            }],
        },
        DownloadPlatformDto {
            key: "windows".into(),
            name: "Windows".into(),
            icon: "🪟".into(),
            coming_soon: false,
            apps: vec![],
        },
    ]
}

pub fn demo_tickets() -> Vec<TicketDto> {
    vec![TicketDto {
        id: 1,
        ticket_type: "sugerencia".into(),
        subject: "Cliente nativo en egui".into(),
        body: "Vista previa de ticket en modo demostración local.".into(),
        status: "open".into(),
        resolucion: String::new(),
        created_at: "26/11/2026 13:00".into(),
        owner_name: "demo".into(),
    }]
}

pub fn demo_videos() -> Vec<VideoDto> {
    vec![VideoDto {
        id: 1,
        user_id: 1,
        author: "Aethravox".into(),
        title: "Presentación Aethravox Desktop en Rust".into(),
        description: "Demostración del cliente nativo egui conectado a Axum.".into(),
        is_short: false,
        file_path: "/aextube".into(),
        mime_type: "video/mp4".into(),
        size_bytes: 12_400_000,
        likes: 24,
        dislikes: 0,
        views: 180,
        created_at: "Hoy".into(),
        user_reaction: None,
        subscribed: false,
        channel_subscribers: 5,
    }]
}

pub fn demo_aexos_profiles(username: &str) -> Vec<AexosProfileDto> {
    vec![AexosProfileDto {
        user_id: 1,
        is_primary: true,
        username: username.into(),
        display_name: username.into(),
        role_display: "Owner".into(),
        can_admin: true,
        wallpaper: "nebula".into(),
        show_desktop_icons: true,
        pinned_apps_json: "[\"aris\",\"chat\",\"neon-canvas\",\"docs\"]".into(),
        files_json: "[]".into(),
        notes_text: "Notas rápidas de aexOS en cliente nativo egui.".into(),
    }]
}

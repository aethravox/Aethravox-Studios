use aethravox_core::{
    base64_encode, AddGroupMemberRequest, AdminAddApiKeyRequest, AdminApisConfigDto,
    AdminApisPayload, AdminBanRequest, AdminCreateDownloadRequest, AdminCreateUserRequest,
    AdminCreateWebRequest, AdminDownloadRowDto, AdminIpBansPayload, AdminKeyCapRequest,
    AdminOverviewDto, AdminReorderProviderRequest, AdminSaveProviderRequest, AdminTestApiKeyResult,
    AdminToggleRequest, AdminUpdateDocRequest, AdminUploadDocRequest, AdminWebHtmlRequest,
    AdminWebTagRequest,
    AdminRoleRequest, AdminRolesPayload, AdminTitlesPayload, AexosLinkRequest, AexosProfileDto,
    AexosSavePrefsRequest, AexosSwitchRequest, ApiError, ArisChatDto, ArisCreateChatRequest,
    ArisCreditsDto, ArisMessageDto, ArisMode, ArisSendRequest, ArisSendResponse,
    AssignMemberRankRequest, AssignUserRoleIdRequest, AuthSuccess, BulkDeleteMessagesRequest,
    BulkDeleteMessagesResponse, CategoryRowDto, ChangePasswordRequest, ChatConversationDto,
    ChatGroupManageDto, ChatMessageDto, ChatMessageInfoDto, CommunityDetailDto, CommunityDto,
    CreateCommunityGroupRequest, CreateCommunityRequest, CreateGroupRankRequest,
    CreateGroupRequest, CreateIpBanRequest, CreateTicketRequest, DeleteRoleRequest, DocumentDto,
    DownloadAppDto, DownloadFileDto, DownloadPlatformDto, EditMessageRequest, EmailIdBody,
    EmailRequestBody, EmailVerifyBody, EmailsPayload, FriendDto, FriendRequestBody,
    FriendsPayload, HubPayload, HubStatsDto, InviteCommunityRequest, LinkCodeDto,
    LinkConsumeRequest, LoginRequest, PendingFriendDto, PlaygroundGenRequest, PlaygroundGenResult,
    ProfilePayload, PublicProfileDto, RecoverRequest, RecoverResponse, RegisterRequest,
    ReplyTicketRequest, ResetPasswordRequest, SaveRoleRequest, SaveTitleRequest,
    SendMessageRequest, SetRolePermsRequest, SetRoleWebPermsRequest, StartDmRequest,
    StartDmResponse, SuspendUserRequest, SystemConfigDto, ThemeKind, ThemeSelectRequest,
    TicketActionRequest, TicketDetailDto, TicketDto, TitleAssignRequest,
    TransferGroupOwnershipRequest, UpdateGroupInfoRequest, UpdateProfileRequest,
    UpdateTitlesRequest, UploadAvatarRequest, UploadChatMediaRequest, UploadGroupImageRequest,
    UploadVideoRequest, UserIpHistoryDto, UserSearchDto, UserSummary, VideoDto, VideoReactRequest, WebAppDto, ZLibAccountPayload, ZLibCreateTokenRequest, ZLibCreateTokenResponse, ZLibPackageDetailDto, ZLibPackageDto, ZLibPublishRequest, ZLibPublishResponse, ZLibVersionDto,
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
    ChatGroupManageLoaded(ChatGroupManageDto),
    ChatGroupActionOk(String),
    ChatGroupLeft(i64),
    ChatMessageInfoLoaded(ChatMessageInfoDto),
    ChatBulkDeleted {
        ok: Vec<i64>,
        denied: usize,
    },
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
    DocsActionOk(String),
    DocsErr(String),
    DownloadsLoaded(Vec<DownloadPlatformDto>),
    DownloadsErr(String),

    // Asistencia / Tickets
    TicketsLoaded(Vec<TicketDto>),
    TicketDetailLoaded(TicketDetailDto),
    SupportActionOk(String),
    SupportErr(String),

    // Perfil, Perfil Público y Sesiones
    ProfileLoaded(ProfilePayload),
    ProfileActionOk(String),
    ProfileErr(String),
    PublicProfileLoaded(PublicProfileDto),
    PublicProfileErr(String),

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
    AdminIpBansLoaded(AdminIpBansPayload),
    AdminConfigLoaded(SystemConfigDto),
    AdminRolesLoaded(AdminRolesPayload),
    AdminTitlesLoaded(AdminTitlesPayload),
    AdminUserIpsLoaded {
        user_id: i64,
        ips: Vec<UserIpHistoryDto>,
    },
    AdminApisLoaded(AdminApisPayload),
    AdminApiKeyTested {
        key_id: i64,
        res: AdminTestApiKeyResult,
    },
    AdminDocsLoaded(Vec<DocumentDto>),
    AdminDownloadsLoaded(Vec<AdminDownloadRowDto>),
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
    // ZLibs (Zen Registry)
    ZLibsPackagesLoaded(Vec<ZLibPackageDto>),
    ZLibsDetailLoaded(ZLibPackageDetailDto),
    ZLibsAccountLoaded(ZLibAccountPayload),
    ZLibTokenCreated(ZLibCreateTokenResponse),
    ZLibPublished(ZLibPublishResponse),
    ZLibsErr(String),
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

pub fn spawn_docs_delete(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    doc_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/docs/{doc_id}");
        match delete_json::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::DocsActionOk("Documento eliminado.".into()));
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

fn infer_video_mime(filename: &str, hint: &str) -> String {
    if !hint.trim().is_empty() {
        return hint.trim().to_string();
    }
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".webm") {
        "video/webm".into()
    } else if lower.ends_with(".mov") {
        "video/quicktime".into()
    } else if lower.ends_with(".ogv") || lower.ends_with(".ogg") {
        "video/ogg".into()
    } else {
        "video/mp4".into()
    }
}

pub fn spawn_aextube_upload_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title: String,
    description: String,
    is_short: bool,
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        if bytes.is_empty() {
            let _ = tx.send(ApiEvent::AextubeErr("El archivo de vídeo está vacío.".into()));
            ctx.request_repaint();
            return;
        }
        if bytes.len() > 20 * 1024 * 1024 {
            let _ = tx.send(ApiEvent::AextubeErr("El vídeo supera el límite de 20 MB.".into()));
            ctx.request_repaint();
            return;
        }
        let mime = infer_video_mime(&filename, &content_type);
        let req = UploadVideoRequest {
            title,
            description,
            is_short,
            filename,
            content_type: mime,
            data_base64: base64_encode(&bytes),
        };
        match post_json::<UploadVideoRequest, Vec<VideoDto>>(
            &base_url,
            "/api/v1/aextube/upload",
            &token,
            &req,
        )
        .await
        {
            Ok(list) => {
                let _ = tx.send(ApiEvent::VideosLoaded(list));
                let _ = tx.send(ApiEvent::AextubeActionOk("Vídeo publicado en aexTube.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AextubeErr(e));
            }
        }
        ctx.request_repaint();
    });
}

#[allow(unused_variables)]
pub fn spawn_aextube_upload_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title: String,
    description: String,
    is_short: bool,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(ApiEvent::AextubeErr(
            "En navegador web arrastra el archivo directamente al área de subida.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::AextubeErr(format!(
                        "No se pudo leer el archivo ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            if bytes.is_empty() {
                let _ = tx.send(ApiEvent::AextubeErr("El archivo de vídeo está vacío.".into()));
                ctx.request_repaint();
                return;
            }
            if bytes.len() > 20 * 1024 * 1024 {
                let _ = tx.send(ApiEvent::AextubeErr("El vídeo supera el límite de 20 MB.".into()));
                ctx.request_repaint();
                return;
            }
            let filename = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("video.mp4")
                .to_string();
            let mime = infer_video_mime(&filename, "");
            let req = UploadVideoRequest {
                title,
                description,
                is_short,
                filename,
                content_type: mime,
                data_base64: base64_encode(&bytes),
            };
            match post_json::<UploadVideoRequest, Vec<VideoDto>>(
                &base_url,
                "/api/v1/aextube/upload",
                &token,
                &req,
            )
            .await
            {
                Ok(list) => {
                    let _ = tx.send(ApiEvent::VideosLoaded(list));
                    let _ = tx.send(ApiEvent::AextubeActionOk("Vídeo publicado en aexTube.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::AextubeErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
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

// ===================== Gestión de Grupos (Chat) =====================

pub fn spawn_chat_group_manage(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/manage");
        match get_json::<ChatGroupManageDto>(&base_url, &path, &token).await {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_update_info(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    req: UpdateGroupInfoRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/info");
        match post_json::<UpdateGroupInfoRequest, ChatGroupManageDto>(&base_url, &path, &token, &req)
            .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Información del grupo actualizada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_add_member(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    username: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/members/add");
        let req = AddGroupMemberRequest { username };
        match post_json::<AddGroupMemberRequest, ChatGroupManageDto>(&base_url, &path, &token, &req)
            .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Miembro añadido al grupo.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_remove_member(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    user_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/members/{user_id}/remove");
        match post_empty::<ChatGroupManageDto>(&base_url, &path, &token).await {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Miembro expulsado del grupo.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_assign_rank(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    user_id: i64,
    rank_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/ranks/assign");
        let req = AssignMemberRankRequest { user_id, rank_id };
        match post_json::<AssignMemberRankRequest, ChatGroupManageDto>(
            &base_url,
            &path,
            &token,
            &req,
        )
        .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Rango asignado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_create_rank(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    req: CreateGroupRankRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/ranks/create");
        match post_json::<CreateGroupRankRequest, ChatGroupManageDto>(&base_url, &path, &token, &req)
            .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Rango creado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_delete_rank(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    rank_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/ranks/{rank_id}");
        match delete_json::<ChatGroupManageDto>(&base_url, &path, &token).await {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Rango eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_transfer(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    new_owner_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/transfer");
        let req = TransferGroupOwnershipRequest { new_owner_id };
        match post_json::<TransferGroupOwnershipRequest, ChatGroupManageDto>(
            &base_url,
            &path,
            &token,
            &req,
        )
        .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Ownership transferido.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_group_leave(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/conversations/{conv_id}/leave");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ChatGroupLeft(conv_id));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Admin Extendido (Suspender, Eliminar, Bans IP, Config) =====================

pub fn spawn_admin_suspend_user(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
    req: SuspendUserRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/suspend");
        match post_json::<SuspendUserRequest, serde_json::Value>(&base_url, &path, &token, &req)
            .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Usuario suspendido.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_user(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/delete");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Usuario eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_ip_bans(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<AdminIpBansPayload>(&base_url, "/api/v1/admin/ip_bans", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminIpBansLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_create_ip_ban(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: CreateIpBanRequest,
) {
    rt.spawn(async move {
        match post_json::<CreateIpBanRequest, serde_json::Value>(
            &base_url,
            "/api/v1/admin/ip_bans/create",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                if let Ok(p) =
                    get_json::<AdminIpBansPayload>(&base_url, "/api/v1/admin/ip_bans", &token).await
                {
                    let _ = tx.send(ApiEvent::AdminIpBansLoaded(p));
                }
                let _ = tx.send(ApiEvent::AdminActionOk("IP baneada correctamente.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_revoke_ip_ban(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    ban_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/ip_bans/{ban_id}/revoke");
        match post_empty::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                if let Ok(p) =
                    get_json::<AdminIpBansPayload>(&base_url, "/api/v1/admin/ip_bans", &token).await
                {
                    let _ = tx.send(ApiEvent::AdminIpBansLoaded(p));
                }
                let _ = tx.send(ApiEvent::AdminActionOk("Ban de IP revocado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_config_get(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<SystemConfigDto>(&base_url, "/api/v1/admin/config", &token).await {
            Ok(cfg) => {
                let _ = tx.send(ApiEvent::AdminConfigLoaded(cfg));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_config_save(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: SystemConfigDto,
) {
    rt.spawn(async move {
        match post_json::<SystemConfigDto, SystemConfigDto>(
            &base_url,
            "/api/v1/admin/config",
            &token,
            &req,
        )
        .await
        {
            Ok(cfg) => {
                let _ = tx.send(ApiEvent::AdminConfigLoaded(cfg));
                let _ = tx.send(ApiEvent::AdminActionOk(
                    "Configuración del sistema guardada.".into(),
                ));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Avatar y Perfil Público =====================

fn infer_mime_generic(filename: &str, ct: &str) -> String {
    if !ct.trim().is_empty() {
        return ct.trim().to_string();
    }
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".txt") {
        "text/plain"
    } else if lower.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
    .to_string()
}

pub fn spawn_upload_avatar_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        if bytes.is_empty() {
            let _ = tx.send(ApiEvent::ProfileErr("La imagen está vacía.".into()));
            ctx.request_repaint();
            return;
        }
        if bytes.len() > 5 * 1024 * 1024 {
            let _ = tx.send(ApiEvent::ProfileErr("El avatar supera el límite de 5 MB.".into()));
            ctx.request_repaint();
            return;
        }
        let mime = infer_mime_generic(&filename, &content_type);
        let req = UploadAvatarRequest {
            filename,
            content_type: mime,
            data_base64: base64_encode(&bytes),
            remove: false,
        };
        match post_json::<UploadAvatarRequest, serde_json::Value>(
            &base_url,
            "/api/v1/profile/avatar",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Avatar actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

#[allow(unused_variables)]
pub fn spawn_upload_avatar_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(ApiEvent::ProfileErr(
            "En navegador web arrastra la imagen directamente al área de avatar.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::ProfileErr(format!(
                        "No se pudo leer la imagen ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            let filename = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("avatar.png")
                .to_string();
            if bytes.is_empty() || bytes.len() > 5 * 1024 * 1024 {
                let _ = tx.send(ApiEvent::ProfileErr(
                    "El avatar debe pesar entre 1 byte y 5 MB.".into(),
                ));
                ctx.request_repaint();
                return;
            }
            let mime = infer_mime_generic(&filename, "");
            let req = UploadAvatarRequest {
                filename,
                content_type: mime,
                data_base64: base64_encode(&bytes),
                remove: false,
            };
            match post_json::<UploadAvatarRequest, serde_json::Value>(
                &base_url,
                "/api/v1/profile/avatar",
                &token,
                &req,
            )
            .await
            {
                Ok(_) => {
                    let _ = tx.send(ApiEvent::ProfileActionOk("Avatar actualizado.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::ProfileErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
}

pub fn spawn_remove_avatar(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        let req = UploadAvatarRequest {
            filename: String::new(),
            content_type: String::new(),
            data_base64: String::new(),
            remove: true,
        };
        match post_json::<UploadAvatarRequest, serde_json::Value>(
            &base_url,
            "/api/v1/profile/avatar",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::ProfileActionOk("Avatar eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_public_profile(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    username: String,
) {
    rt.spawn(async move {
        let clean = username.trim().trim_start_matches('@');
        let path = format!("/api/v1/users/{}/profile", encode_uri_component(clean));
        match get_json::<PublicProfileDto>(&base_url, &path, &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::PublicProfileLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::PublicProfileErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Chat Multimedia, Info, Grupo Imagen y Borrado Masivo =====================

pub fn spawn_chat_upload_media_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    is_image: bool,
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        if bytes.is_empty() {
            let _ = tx.send(ApiEvent::ChatSendErr("El archivo está vacío.".into()));
            ctx.request_repaint();
            return;
        }
        let max_bytes = if is_image {
            5 * 1024 * 1024
        } else {
            20 * 1024 * 1024
        };
        if bytes.len() > max_bytes {
            let _ = tx.send(ApiEvent::ChatSendErr(format!(
                "El archivo supera el límite de {} MB.",
                max_bytes / (1024 * 1024)
            )));
            ctx.request_repaint();
            return;
        }
        let mime = infer_mime_generic(&filename, &content_type);
        let req = UploadChatMediaRequest {
            kind: if is_image {
                "image".to_string()
            } else {
                "file".to_string()
            },
            filename,
            content_type: mime,
            data_base64: base64_encode(&bytes),
            duration: None,
        };
        let path = format!("/api/v1/chat/conversations/{conv_id}/upload");
        match post_json::<UploadChatMediaRequest, ChatMessageDto>(&base_url, &path, &token, &req)
            .await
        {
            Ok(msg) => {
                let _ = tx.send(ApiEvent::MessageSent(msg));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Adjunto enviado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatSendErr(e));
            }
        }
        ctx.request_repaint();
    });
}

#[allow(unused_variables)]
pub fn spawn_chat_upload_media_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    is_image: bool,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(ApiEvent::ChatSendErr(
            "En navegador web arrastra el archivo directamente al chat.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::ChatSendErr(format!(
                        "No se pudo leer el archivo ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            let max_bytes = if is_image {
                5 * 1024 * 1024
            } else {
                20 * 1024 * 1024
            };
            if bytes.is_empty() || bytes.len() > max_bytes {
                let _ = tx.send(ApiEvent::ChatSendErr(format!(
                    "Tamaño inválido (máximo {} MB).",
                    max_bytes / (1024 * 1024)
                )));
                ctx.request_repaint();
                return;
            }
            let filename = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(if is_image { "imagen.png" } else { "archivo.bin" })
                .to_string();
            let mime = infer_mime_generic(&filename, "");
            let req = UploadChatMediaRequest {
                kind: if is_image {
                    "image".to_string()
                } else {
                    "file".to_string()
                },
                filename,
                content_type: mime,
                data_base64: base64_encode(&bytes),
                duration: None,
            };
            let api_path = format!("/api/v1/chat/conversations/{conv_id}/upload");
            match post_json::<UploadChatMediaRequest, ChatMessageDto>(
                &base_url, &api_path, &token, &req,
            )
            .await
            {
                Ok(msg) => {
                    let _ = tx.send(ApiEvent::MessageSent(msg));
                    let _ = tx.send(ApiEvent::ChatGroupActionOk("Adjunto enviado.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::ChatSendErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
}

pub fn spawn_chat_group_image_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        if bytes.is_empty() || bytes.len() > 5 * 1024 * 1024 {
            let _ = tx.send(ApiEvent::ChatErr(
                "La imagen de grupo debe pesar máximo 5 MB.".into(),
            ));
            ctx.request_repaint();
            return;
        }
        let mime = infer_mime_generic(&filename, &content_type);
        let req = UploadGroupImageRequest {
            filename,
            content_type: mime,
            data_base64: base64_encode(&bytes),
        };
        let path = format!("/api/v1/chat/conversations/{conv_id}/group_image");
        match post_json::<UploadGroupImageRequest, ChatGroupManageDto>(
            &base_url, &path, &token, &req,
        )
        .await
        {
            Ok(dto) => {
                let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                let _ = tx.send(ApiEvent::ChatGroupActionOk("Imagen del grupo actualizada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

#[allow(unused_variables)]
pub fn spawn_chat_group_image_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    conv_id: i64,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = tx.send(ApiEvent::ChatErr(
            "En navegador web arrastra la imagen directamente al panel del grupo.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::ChatErr(format!(
                        "No se pudo leer la imagen ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            if bytes.is_empty() || bytes.len() > 5 * 1024 * 1024 {
                let _ = tx.send(ApiEvent::ChatErr(
                    "La imagen de grupo debe pesar máximo 5 MB.".into(),
                ));
                ctx.request_repaint();
                return;
            }
            let filename = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("grupo.png")
                .to_string();
            let mime = infer_mime_generic(&filename, "");
            let req = UploadGroupImageRequest {
                filename,
                content_type: mime,
                data_base64: base64_encode(&bytes),
            };
            let api_path = format!("/api/v1/chat/conversations/{conv_id}/group_image");
            match post_json::<UploadGroupImageRequest, ChatGroupManageDto>(
                &base_url, &api_path, &token, &req,
            )
            .await
            {
                Ok(dto) => {
                    let _ = tx.send(ApiEvent::ChatGroupManageLoaded(dto));
                    let _ = tx.send(ApiEvent::ChatGroupActionOk("Imagen del grupo actualizada.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::ChatErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
}

pub fn spawn_chat_bulk_delete(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    ids: Vec<i64>,
) {
    rt.spawn(async move {
        let req = BulkDeleteMessagesRequest { ids };
        match post_json::<BulkDeleteMessagesRequest, BulkDeleteMessagesResponse>(
            &base_url,
            "/api/v1/chat/messages/bulk_delete",
            &token,
            &req,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ChatBulkDeleted {
                    ok: res.ok,
                    denied: res.denied.len(),
                });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_chat_message_info(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    msg_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/chat/messages/{msg_id}/info");
        match get_json::<ChatMessageInfoDto>(&base_url, &path, &token).await {
            Ok(info) => {
                let _ = tx.send(ApiEvent::ChatMessageInfoLoaded(info));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ChatErr(e));
            }
        }
        ctx.request_repaint();
    });
}

// ===================== Admin Roles y Títulos =====================

pub fn spawn_admin_assign_role_id(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
    role_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/role_id");
        let req = AssignUserRoleIdRequest { role_id };
        match post_json::<AssignUserRoleIdRequest, serde_json::Value>(
            &base_url, &path, &token, &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Rol asignado al usuario.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_roles(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<AdminRolesPayload>(&base_url, "/api/v1/admin/roles", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_create_role(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: SaveRoleRequest,
) {
    rt.spawn(async move {
        match post_json::<SaveRoleRequest, AdminRolesPayload>(
            &base_url,
            "/api/v1/admin/roles/create",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Rol creado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_update_role(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    role_id: i64,
    req: SaveRoleRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/roles/{role_id}/update");
        match post_json::<SaveRoleRequest, AdminRolesPayload>(&base_url, &path, &token, &req).await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Rol actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_role(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    role_id: i64,
    reassign_to: Option<i64>,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/roles/{role_id}/delete");
        let req = DeleteRoleRequest { reassign_to };
        match post_json::<DeleteRoleRequest, AdminRolesPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Rol eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_set_role_perms(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    role_id: i64,
    perms: Vec<String>,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/roles/{role_id}/perms");
        let req = SetRolePermsRequest { perms };
        match post_json::<SetRolePermsRequest, AdminRolesPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Permisos del rol guardados.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_set_role_web_perms(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    role_id: i64,
    web_ids: Vec<i64>,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/roles/{role_id}/web_perms");
        let req = SetRoleWebPermsRequest { web_ids };
        match post_json::<SetRoleWebPermsRequest, AdminRolesPayload>(
            &base_url, &path, &token, &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminRolesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Secciones del rol guardadas.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_titles(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<AdminTitlesPayload>(&base_url, "/api/v1/admin/titles", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_create_title(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: SaveTitleRequest,
) {
    rt.spawn(async move {
        match post_json::<SaveTitleRequest, AdminTitlesPayload>(
            &base_url,
            "/api/v1/admin/titles/create",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Título creado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_update_title(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title_id: i64,
    req: SaveTitleRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/titles/{title_id}/update");
        match post_json::<SaveTitleRequest, AdminTitlesPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Título actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_title(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/titles/{title_id}/delete");
        match post_empty::<AdminTitlesPayload>(&base_url, &path, &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Título eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_assign_title(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title_id: i64,
    user_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/titles/{title_id}/assign");
        let req = TitleAssignRequest { user_id };
        match post_json::<TitleAssignRequest, AdminTitlesPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Título asignado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_revoke_title(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title_id: i64,
    user_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/titles/{title_id}/revoke");
        let req = TitleAssignRequest { user_id };
        match post_json::<TitleAssignRequest, AdminTitlesPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminTitlesLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Título revocado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_create_user(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AdminCreateUserRequest,
) {
    rt.spawn(async move {
        match post_json::<AdminCreateUserRequest, serde_json::Value>(
            &base_url,
            "/api/v1/admin/users/create",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Usuario creado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_user_ips(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    user_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/users/{user_id}/ips");
        match get_json::<Vec<UserIpHistoryDto>>(&base_url, &path, &token).await {
            Ok(ips) => {
                let _ = tx.send(ApiEvent::AdminUserIpsLoaded { user_id, ips });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_create_web(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AdminCreateWebRequest,
) {
    rt.spawn(async move {
        match post_json::<AdminCreateWebRequest, serde_json::Value>(
            &base_url,
            "/api/v1/admin/webs/create",
            &token,
            &req,
        )
        .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Sección creada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_update_web_html(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    web_id: i64,
    html_content: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/webs/{web_id}/html");
        let req = AdminWebHtmlRequest { html_content };
        match post_json::<AdminWebHtmlRequest, serde_json::Value>(&base_url, &path, &token, &req)
            .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("HTML actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_set_web_tag(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    web_id: i64,
    req: AdminWebTagRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/webs/{web_id}/tag");
        match post_json::<AdminWebTagRequest, serde_json::Value>(&base_url, &path, &token, &req)
            .await
        {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Etiqueta de sección guardada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_web(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    web_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/webs/{web_id}");
        match delete_json::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                let _ = tx.send(ApiEvent::AdminActionOk("Sección eliminada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_apis(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<AdminApisPayload>(&base_url, "/api/v1/admin/apis", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_add_api_key(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AdminAddApiKeyRequest,
) {
    rt.spawn(async move {
        match post_json::<AdminAddApiKeyRequest, AdminApisPayload>(
            &base_url,
            "/api/v1/admin/apis/keys",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("API Key(s) añadidas.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_toggle_api_key(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    key_id: i64,
    active: bool,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/keys/{key_id}/toggle");
        let req = AdminToggleRequest { active };
        match post_json::<AdminToggleRequest, AdminApisPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Estado de key actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_update_api_key_cap(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    key_id: i64,
    daily_cap: Option<i64>,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/keys/{key_id}/cap");
        let req = AdminKeyCapRequest { daily_cap };
        match post_json::<AdminKeyCapRequest, AdminApisPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Tope diario actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_test_api_key(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    key_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/keys/{key_id}/test");
        match post_empty::<AdminTestApiKeyResult>(&base_url, &path, &token).await {
            Ok(res) => {
                let _ = tx.send(ApiEvent::AdminApiKeyTested { key_id, res });
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_api_key(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    key_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/keys/{key_id}");
        match delete_json::<AdminApisPayload>(&base_url, &path, &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Key eliminada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_reset_api_cooldowns(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match post_empty::<AdminApisPayload>(
            &base_url,
            "/api/v1/admin/apis/keys/reset-cooldowns",
            &token,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Cooldowns liberados.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_add_api_provider(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AdminSaveProviderRequest,
) {
    rt.spawn(async move {
        match post_json::<AdminSaveProviderRequest, AdminApisPayload>(
            &base_url,
            "/api/v1/admin/apis/providers",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Proveedor creado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_update_api_provider(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    provider_id: i64,
    req: AdminSaveProviderRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/providers/{provider_id}/update");
        match post_json::<AdminSaveProviderRequest, AdminApisPayload>(
            &base_url, &path, &token, &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Proveedor actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_toggle_api_provider(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    provider_id: i64,
    active: bool,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/providers/{provider_id}/toggle");
        let req = AdminToggleRequest { active };
        match post_json::<AdminToggleRequest, AdminApisPayload>(&base_url, &path, &token, &req)
            .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Proveedor actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_reorder_api_provider(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    provider_id: i64,
    delta: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/providers/{provider_id}/priority");
        let req = AdminReorderProviderRequest { delta };
        match post_json::<AdminReorderProviderRequest, AdminApisPayload>(
            &base_url, &path, &token, &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_delete_api_provider(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    provider_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/apis/providers/{provider_id}");
        match delete_json::<AdminApisPayload>(&base_url, &path, &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Proveedor eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_save_apis_config(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: AdminApisConfigDto,
) {
    rt.spawn(async move {
        match post_json::<AdminApisConfigDto, AdminApisPayload>(
            &base_url,
            "/api/v1/admin/apis/config",
            &token,
            &req,
        )
        .await
        {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk(
                    "Configuración de Aris y rotación guardada.".into(),
                ));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_clear_rotation_logs(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match delete_json::<AdminApisPayload>(&base_url, "/api/v1/admin/apis/logs", &token).await {
            Ok(p) => {
                let _ = tx.send(ApiEvent::AdminApisLoaded(p));
                let _ = tx.send(ApiEvent::AdminActionOk("Bitácora vaciada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_docs_list(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<DocumentDto>>(&base_url, "/api/v1/docs", &token).await {
            Ok(d) => {
                let _ = tx.send(ApiEvent::AdminDocsLoaded(d));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_doc_upload_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title: String,
    description: Option<String>,
    file_name: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        let req = AdminUploadDocRequest {
            title,
            description,
            file_name,
            data_base64: base64_encode(&bytes),
        };
        match post_json::<AdminUploadDocRequest, Vec<DocumentDto>>(
            &base_url,
            "/api/v1/admin/docs/upload",
            &token,
            &req,
        )
        .await
        {
            Ok(docs) => {
                let _ = tx.send(ApiEvent::AdminDocsLoaded(docs));
                let _ = tx.send(ApiEvent::AdminActionOk("Documento subido.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_doc_upload_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    title: String,
    description: Option<String>,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (rt, base_url, token, title, description, path);
        let _ = tx.send(ApiEvent::AdminErr(
            "En navegador web arrastra el documento al área de subida.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::AdminErr(format!(
                        "No se pudo leer el archivo ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            let file_name = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("documento.pdf")
                .to_string();
            let req = AdminUploadDocRequest {
                title,
                description,
                file_name,
                data_base64: base64_encode(&bytes),
            };
            match post_json::<AdminUploadDocRequest, Vec<DocumentDto>>(
                &base_url,
                "/api/v1/admin/docs/upload",
                &token,
                &req,
            )
            .await
            {
                Ok(docs) => {
                    let _ = tx.send(ApiEvent::AdminDocsLoaded(docs));
                    let _ = tx.send(ApiEvent::AdminActionOk("Documento subido.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::AdminErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
}

pub fn spawn_admin_doc_update(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    doc_id: i64,
    req: AdminUpdateDocRequest,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/docs/{doc_id}/update");
        match post_json::<AdminUpdateDocRequest, Vec<DocumentDto>>(&base_url, &path, &token, &req)
            .await
        {
            Ok(docs) => {
                let _ = tx.send(ApiEvent::AdminDocsLoaded(docs));
                let _ = tx.send(ApiEvent::AdminActionOk("Documento actualizado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_doc_regenerate(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    doc_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/docs/{doc_id}/regenerate");
        match post_empty::<Vec<DocumentDto>>(&base_url, &path, &token).await {
            Ok(docs) => {
                let _ = tx.send(ApiEvent::AdminDocsLoaded(docs));
                let _ = tx.send(ApiEvent::AdminActionOk(
                    "Regeneración de preview lanzada.".into(),
                ));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_doc_delete(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    doc_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/docs/{doc_id}");
        match delete_json::<serde_json::Value>(&base_url, &path, &token).await {
            Ok(_) => {
                if let Ok(docs) = get_json::<Vec<DocumentDto>>(&base_url, "/api/v1/docs", &token).await {
                    let _ = tx.send(ApiEvent::AdminDocsLoaded(docs));
                }
                let _ = tx.send(ApiEvent::AdminActionOk("Documento eliminado.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_downloads_list(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<Vec<AdminDownloadRowDto>>(&base_url, "/api/v1/admin/downloads", &token)
            .await
        {
            Ok(rows) => {
                let _ = tx.send(ApiEvent::AdminDownloadsLoaded(rows));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_download_create_bytes(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    platform: String,
    app_slug: String,
    app_name: String,
    version: String,
    notes: Option<String>,
    file_name: String,
    bytes: Vec<u8>,
) {
    rt.spawn(async move {
        let req = AdminCreateDownloadRequest {
            platform,
            app_slug,
            app_name,
            version,
            notes,
            file_name,
            data_base64: base64_encode(&bytes),
        };
        match post_json::<AdminCreateDownloadRequest, Vec<AdminDownloadRowDto>>(
            &base_url,
            "/api/v1/admin/downloads/create",
            &token,
            &req,
        )
        .await
        {
            Ok(rows) => {
                let _ = tx.send(ApiEvent::AdminDownloadsLoaded(rows));
                let _ = tx.send(ApiEvent::AdminActionOk("Descarga publicada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_admin_download_create_path(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    platform: String,
    app_slug: String,
    app_name: String,
    version: String,
    notes: Option<String>,
    path: String,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (
            rt, base_url, token, platform, app_slug, app_name, version, notes, path,
        );
        let _ = tx.send(ApiEvent::AdminErr(
            "En navegador web arrastra el binario al área de subida.".into(),
        ));
        ctx.request_repaint();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        rt.spawn(async move {
            let trimmed = path.trim().trim_matches('"').to_string();
            let bytes = match std::fs::read(&trimmed) {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(ApiEvent::AdminErr(format!(
                        "No se pudo leer el archivo ({trimmed}): {e}"
                    )));
                    ctx.request_repaint();
                    return;
                }
            };
            let file_name = std::path::Path::new(&trimmed)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("app.bin")
                .to_string();
            let req = AdminCreateDownloadRequest {
                platform,
                app_slug,
                app_name,
                version,
                notes,
                file_name,
                data_base64: base64_encode(&bytes),
            };
            match post_json::<AdminCreateDownloadRequest, Vec<AdminDownloadRowDto>>(
                &base_url,
                "/api/v1/admin/downloads/create",
                &token,
                &req,
            )
            .await
            {
                Ok(rows) => {
                    let _ = tx.send(ApiEvent::AdminDownloadsLoaded(rows));
                    let _ = tx.send(ApiEvent::AdminActionOk("Descarga publicada.".into()));
                }
                Err(e) => {
                    let _ = tx.send(ApiEvent::AdminErr(e));
                }
            }
            ctx.request_repaint();
        });
    }
}

pub fn spawn_admin_download_delete(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    download_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/admin/downloads/{download_id}");
        match delete_json::<Vec<AdminDownloadRowDto>>(&base_url, &path, &token).await {
            Ok(rows) => {
                let _ = tx.send(ApiEvent::AdminDownloadsLoaded(rows));
                let _ = tx.send(ApiEvent::AdminActionOk("Descarga eliminada.".into()));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::AdminErr(e));
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
        avatar_url: String::new(),
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
                app(13, "zlibs", "ZLibs", "Registro de librerías Zen con repositorios en GitHub y tokens CLI.", "📦", "Nuevo"),
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


// ===================== ZLibs (Zen Registry) =====================

pub fn spawn_fetch_zlibs_packages(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    query: String,
) {
    rt.spawn(async move {
        let q = query.trim();
        let path = if q.is_empty() {
            "/api/v1/zlibs/packages".to_string()
        } else {
            format!("/api/v1/zlibs/packages?q={}", encode_uri_component(q))
        };
        match get_json::<Vec<ZLibPackageDto>>(&base_url, &path, &token).await {
            Ok(list) => {
                let _ = tx.send(ApiEvent::ZLibsPackagesLoaded(list));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_zlib_detail(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    name: String,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/zlibs/packages/{}", encode_uri_component(&name));
        match get_json::<ZLibPackageDetailDto>(&base_url, &path, &token).await {
            Ok(detail) => {
                let _ = tx.send(ApiEvent::ZLibsDetailLoaded(detail));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_fetch_zlibs_mine(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
) {
    rt.spawn(async move {
        match get_json::<ZLibAccountPayload>(&base_url, "/api/v1/zlibs/mine", &token).await {
            Ok(acc) => {
                let _ = tx.send(ApiEvent::ZLibsAccountLoaded(acc));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_create_zlib_token(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    label: String,
) {
    rt.spawn(async move {
        let body = ZLibCreateTokenRequest { label };
        match post_json::<ZLibCreateTokenRequest, ZLibCreateTokenResponse>(
            &base_url,
            "/api/v1/zlibs/tokens",
            &token,
            &body,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ZLibTokenCreated(res));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_revoke_zlib_token(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    token_id: i64,
) {
    rt.spawn(async move {
        let path = format!("/api/v1/zlibs/tokens/{token_id}");
        match delete_json::<ZLibAccountPayload>(&base_url, &path, &token).await {
            Ok(acc) => {
                let _ = tx.send(ApiEvent::ZLibsAccountLoaded(acc));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn spawn_publish_zlib(
    rt: &AsyncRt,
    tx: tokio::sync::mpsc::UnboundedSender<ApiEvent>,
    ctx: egui::Context,
    base_url: String,
    token: Option<String>,
    req: ZLibPublishRequest,
) {
    rt.spawn(async move {
        match post_json::<ZLibPublishRequest, ZLibPublishResponse>(
            &base_url,
            "/api/v1/zlibs/publish",
            &token,
            &req,
        )
        .await
        {
            Ok(res) => {
                let _ = tx.send(ApiEvent::ZLibPublished(res));
            }
            Err(e) => {
                let _ = tx.send(ApiEvent::ZLibsErr(e));
            }
        }
        ctx.request_repaint();
    });
}

pub fn demo_zlibs() -> Vec<ZLibPackageDto> {
    vec![ZLibPackageDto {
        id: 1,
        name: "zen_math".into(),
        owner_username: "Aethravox".into(),
        description: "Utilidades matemáticas y vectores para Zen.".into(),
        git_url: "https://github.com/aethravox/zen_math.git".into(),
        latest_version: "1.0.0".into(),
        dep_line: "dep zen_math "1.0.0"".into(),
        downloads_count: 42,
        updated_at: "2026-11-27".into(),
        versions: vec![ZLibVersionDto {
            version: "1.0.0".into(),
            git_ref: "1.0.0".into(),
            dep_line: "dep zen_math "1.0.0"".into(),
            files_count: 3,
            created_at: "2026-11-27".into(),
        }],
    }]
}

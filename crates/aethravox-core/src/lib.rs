use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    #[default]
    Aethravox,
    Shadow,
    CommonCents,
}

impl ThemeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeKind::Aethravox => "aethravox",
            ThemeKind::Shadow => "shadow",
            ThemeKind::CommonCents => "commoncents",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ThemeKind::Aethravox => "Aethravox Studios",
            ThemeKind::Shadow => "Shadow Studios",
            ThemeKind::CommonCents => "CommonCents",
        }
    }

    pub fn from_key(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "shadow" => ThemeKind::Shadow,
            "commoncents" => ThemeKind::CommonCents,
            _ => ThemeKind::Aethravox,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceProfile {
    #[default]
    Auto,
    Desktop,
    Mobile,
    Tv,
    Pdi,
}

impl DeviceProfile {
    pub fn label(&self) -> &'static str {
        match self {
            DeviceProfile::Auto => "Automático",
            DeviceProfile::Desktop => "PC / Escritorio",
            DeviceProfile::Mobile => "Móvil (Android / iPhone)",
            DeviceProfile::Tv => "Smart TV / Android TV",
            DeviceProfile::Pdi => "Pizarra Digital (PDI)",
        }
    }

    pub fn from_str_opt(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "desktop" | "pc" => DeviceProfile::Desktop,
            "mobile" | "movil" | "android" | "ios" | "iphone" => DeviceProfile::Mobile,
            "tv" | "smarttv" | "androidtv" => DeviceProfile::Tv,
            "pdi" | "whiteboard" | "pizarra" => DeviceProfile::Pdi,
            _ => DeviceProfile::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "default_remember")]
    pub remember: bool,
}

fn default_remember() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub role_key: String,
    pub role_display: String,
    pub role_color: String,
    pub role_text_color: String,
    pub is_guest: bool,
    pub can_access_admin: bool,
    pub bio: String,
    pub theme: ThemeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSuccess {
    pub token: String,
    pub user: UserSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAppDto {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub url: String,
    pub tag: String,
    pub tag_label: String,
    pub tag_color: String,
    pub clickable: bool,
    pub lock_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRowDto {
    pub name: String,
    pub slug: String,
    pub items: Vec<WebAppDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HubStatsDto {
    pub online_count: i64,
    pub doc_count: i64,
    pub chat_unread: i64,
    pub community_unread: i64,
    pub owner_count: i64,
    pub admin_count: i64,
    pub staff_count: i64,
    pub user_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubPayload {
    pub user: UserSummary,
    pub categories: Vec<CategoryRowDto>,
    pub stats: HubStatsDto,
}

// ===================== Aris IA =====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArisMode {
    #[default]
    Instantaneo,
    Rapido,
    Pensando,
}

impl ArisMode {
    pub fn label(&self) -> &'static str {
        match self {
            ArisMode::Instantaneo => "Instantáneo",
            ArisMode::Rapido => "Rápido",
            ArisMode::Pensando => "Pensando",
        }
    }

    pub fn api_mode(&self) -> &'static str {
        match self {
            ArisMode::Instantaneo => "instantaneo",
            ArisMode::Rapido | ArisMode::Pensando => "rapido",
        }
    }

    pub fn think(&self) -> bool {
        matches!(self, ArisMode::Pensando)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArisChatDto {
    pub id: i64,
    pub title: String,
    pub pinned: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArisMessageDto {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArisCreditsDto {
    pub daily_limit: i64,
    pub daily_used: i64,
    pub remaining: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArisCreateChatRequest {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArisSendRequest {
    pub content: String,
    pub mode: ArisMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArisSendResponse {
    pub user_message: ArisMessageDto,
    pub assistant_message: ArisMessageDto,
    pub credits: ArisCreditsDto,
}

// ===================== Chat =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversationDto {
    pub id: i64,
    pub title: String,
    pub is_group: bool,
    pub last_message: String,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub id: i64,
    pub conversation_id: i64,
    pub sender: String,
    pub content: String,
    pub created_at: String,
    pub is_mine: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSelectRequest {
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartDmRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartDmResponse {
    pub conversation_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<String>,
}

// ===================== Amigos =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendDto {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub online: bool,
    pub role_display: String,
    pub role_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingFriendDto {
    pub id: i64,
    pub username: String,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSearchDto {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub role_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendsPayload {
    pub friends: Vec<FriendDto>,
    pub pending: Vec<PendingFriendDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestBody {
    pub username: String,
}

// ===================== Comunidades =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityDto {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub is_member: bool,
    pub is_owner: bool,
    pub member_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityGroupDto {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityDetailDto {
    pub community: CommunityDto,
    pub groups: Vec<CommunityGroupDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommunityRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommunityGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCommunityRequest {
    pub user_ids: Vec<i64>,
}

// ===================== Documentos =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDto {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub file_name: String,
    pub file_type: String,
    pub icon: String,
    pub size: String,
    pub created_at: String,
    pub has_preview: bool,
    pub downloaded: i64,
}

// ===================== Asistencia / Tickets =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDto {
    pub id: i64,
    pub ticket_type: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub resolucion: String,
    pub created_at: String,
    pub owner_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketMessageDto {
    pub id: i64,
    pub body: String,
    pub sender_name: String,
    pub is_admin: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDetailDto {
    pub ticket: TicketDto,
    pub reclamado_por_nombre: String,
    pub is_admin: bool,
    pub messages: Vec<TicketMessageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTicketRequest {
    pub ticket_type: String,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyTicketRequest {
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketActionRequest {
    pub action: String, // "claim" | "resolve" | "reject" | "close"
}

// ===================== Descargas =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadFileDto {
    pub id: i64,
    pub version: String,
    pub orig_name: String,
    pub size_mb: String,
    pub created_at: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadAppDto {
    pub slug: String,
    pub name: String,
    pub files: Vec<DownloadFileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadPlatformDto {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub coming_soon: bool,
    pub apps: Vec<DownloadAppDto>,
}

// ===================== Perfil, Títulos y Sesiones =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleDto {
    pub id: i64,
    pub name: String,
    pub effect: String,
    pub description: String,
    pub selected: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDto {
    pub token_prefix: String,
    pub token: String,
    pub is_current: bool,
    pub remember: bool,
    pub created_at: String,
    pub last_seen: String,
    pub expires: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedDeviceDto {
    pub token: String,
    pub device_name: String,
    pub created_at: String,
    pub last_seen: String,
    pub expires: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePayload {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub bio: String,
    pub role_display: String,
    pub created_at: String,
    pub last_login: String,
    pub usage: String,
    pub titles: Vec<TitleDto>,
    pub sessions: Vec<SessionDto>,
    pub linked_devices: Vec<LinkedDeviceDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: String,
    pub bio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTitlesRequest {
    pub primary_id: Option<i64>,
    pub visible_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub current: String,
    pub new_password: String,
}

// ===================== aexTube =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDto {
    pub id: i64,
    pub user_id: i64,
    pub author: String,
    pub title: String,
    pub description: String,
    pub is_short: bool,
    pub file_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub likes: i64,
    pub dislikes: i64,
    pub views: i64,
    pub created_at: String,
    pub user_reaction: Option<String>,
    pub subscribed: bool,
    pub channel_subscribers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoReactRequest {
    pub kind: String, // "like" | "dislike"
}

// ===================== Playground =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundGenRequest {
    pub prompt: String,
    pub current_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundGenResult {
    pub html: String,
    pub css: String,
    pub js: String,
    pub note: String,
    pub model: String,
    pub provider: String,
}

// ===================== aexOS =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AexosProfileDto {
    pub user_id: i64,
    pub is_primary: bool,
    pub username: String,
    pub display_name: String,
    pub role_display: String,
    pub can_admin: bool,
    pub wallpaper: String,
    pub show_desktop_icons: bool,
    pub pinned_apps_json: String,
    pub files_json: String,
    pub notes_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AexosLinkRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AexosSwitchRequest {
    pub user_id: i64,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AexosSavePrefsRequest {
    pub profile_user_id: i64,
    pub wallpaper: String,
    pub show_desktop_icons: bool,
    pub pinned_apps_json: String,
    pub files_json: String,
    pub notes_text: String,
}

// ===================== Admin =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserDto {
    pub id: i64,
    pub username: String,
    pub display: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub last_login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminWebDto {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub icon: String,
    pub is_active: bool,
    pub is_public: bool,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAuditDto {
    pub id: i64,
    pub username: String,
    pub action: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminOverviewDto {
    pub total_users: i64,
    pub owner_count: i64,
    pub admin_count: i64,
    pub staff_count: i64,
    pub user_count: i64,
    pub web_count: i64,
    pub audit_total: i64,
    pub visits_7d: i64,
    pub users: Vec<AdminUserDto>,
    pub webs: Vec<AdminWebDto>,
    pub audit: Vec<AdminAuditDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminRoleRequest {
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminBanRequest {
    pub reason: String,
    pub permanent: bool,
    pub duration_hours: Option<i64>,
}

// ===================== Recuperación, Vinculación y Correo =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverResponse {
    pub ok: bool,
    pub info: String,
    pub dev_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkCodeDto {
    pub code: String,
    pub numeric_code: String,
    pub expires_at: String,
    pub target_username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkConsumeRequest {
    pub code: String,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEmailDto {
    pub id: i64,
    pub email: String,
    pub verified: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailsPayload {
    pub emails: Vec<UserEmailDto>,
    pub aethramail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailRequestBody {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerifyBody {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailIdBody {
    pub email_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMessageRequest {
    pub body: String,
}

// ===================== Gestión de Grupos (Chat) =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGroupRankDto {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub is_owner_rank: bool,
    pub can_send_messages: bool,
    pub can_add_members: bool,
    pub can_edit_group_info: bool,
    pub can_delete_messages: bool,
    pub can_remove_members: bool,
    pub can_manage_ranks: bool,
    pub can_assign_ranks: bool,
    pub can_pin_messages: bool,
    pub can_transfer_ownership: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGroupMemberDto {
    pub user_id: i64,
    pub username: String,
    pub display: String,
    pub rank_id: i64,
    pub rank_name: String,
    pub rank_color: String,
    pub is_owner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGroupManageDto {
    pub conversation_id: i64,
    pub name: String,
    pub description: String,
    pub is_group: bool,
    pub owner_id: i64,
    pub my_user_id: i64,
    pub can_add_members: bool,
    pub can_edit_group_info: bool,
    pub can_remove_members: bool,
    pub can_manage_ranks: bool,
    pub can_assign_ranks: bool,
    pub can_transfer_ownership: bool,
    pub members: Vec<ChatGroupMemberDto>,
    pub ranks: Vec<ChatGroupRankDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGroupInfoRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignMemberRankRequest {
    pub user_id: i64,
    pub rank_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGroupRankRequest {
    pub name: String,
    pub color: Option<String>,
    pub can_send_messages: bool,
    pub can_add_members: bool,
    pub can_edit_group_info: bool,
    pub can_delete_messages: bool,
    pub can_remove_members: bool,
    pub can_manage_ranks: bool,
    pub can_assign_ranks: bool,
    pub can_pin_messages: bool,
    pub can_transfer_ownership: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferGroupOwnershipRequest {
    pub new_owner_id: i64,
}

// ===================== Admin Extendido (Bans IP, Config, Suspensión) =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpBanDto {
    pub id: i64,
    pub ip: String,
    pub reason: String,
    pub banned_by_username: Option<String>,
    pub created_at: String,
    pub expires_at: String,
    pub is_permanent: bool,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminIpBansPayload {
    pub active: Vec<IpBanDto>,
    pub history: Vec<IpBanDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIpBanRequest {
    pub ip: String,
    pub reason: String,
    pub permanent: bool,
    pub duration_hours: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigDto {
    pub studio_name: String,
    pub foundation_date: String,
    pub allow_public_register: bool,
    pub default_role: String,
    pub maintenance_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuspendUserRequest {
    pub reason: String,
    pub duration_hours: i64,
}

// ===================== aexTube Subida Nativa =====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadVideoRequest {
    pub title: String,
    pub description: String,
    pub is_short: bool,
    pub filename: String,
    pub content_type: String,
    pub data_base64: String,
}

pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    fn val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    if clean.len() % 4 != 0 {
        return Err("Base64 con longitud inválida.".into());
    }
    let mut out = Vec::with_capacity((clean.len() / 4) * 3);
    for chunk in clean.chunks_exact(4) {
        let c0 = val(chunk[0]).ok_or_else(|| "Base64 inválido.".to_string())?;
        let c1 = val(chunk[1]).ok_or_else(|| "Base64 inválido.".to_string())?;
        let pad2 = chunk[2] == b'=';
        let pad3 = chunk[3] == b'=';
        if pad2 && !pad3 {
            return Err("Padding base64 inválido.".into());
        }
        let c2 = if pad2 {
            0
        } else {
            val(chunk[2]).ok_or_else(|| "Base64 inválido.".to_string())?
        };
        let c3 = if pad3 {
            0
        } else {
            val(chunk[3]).ok_or_else(|| "Base64 inválido.".to_string())?
        };
        let n = ((c0 as u32) << 18) | ((c1 as u32) << 12) | ((c2 as u32) << 6) | (c3 as u32);
        out.push(((n >> 16) & 0xFF) as u8);
        if !pad2 {
            out.push(((n >> 8) & 0xFF) as u8);
        }
        if !pad3 {
            out.push((n & 0xFF) as u8);
        }
    }
    Ok(out)
}


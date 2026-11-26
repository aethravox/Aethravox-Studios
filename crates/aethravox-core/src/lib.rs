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

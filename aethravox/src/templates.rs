use askama::Template;

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub title: String,
    pub content: String,
}

#[derive(Template)]
#[template(path = "auth/login.html")]
pub struct LoginTemplate {
    pub next: String,
    pub error: Option<String>,
    pub info: Option<String>,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
pub struct RegisterTemplate {
    pub error: Option<String>,
    pub username: String,
    pub email: String,
}

/// Plantilla pública de los documentos legales (privacidad + términos).
/// `mode` = "privacidad" | "terminos" controla qué pestaña se muestra activa.
#[derive(Template)]
#[template(path = "auth/legal.html")]
pub struct LegalTemplate {
    pub mode: String,
}

#[derive(Template)]
#[template(path = "auth/recover.html")]
pub struct RecoverTemplate {
    pub error: Option<String>,
    pub info: Option<String>,
    pub email: String,
}

#[derive(Template)]
#[template(path = "auth/reset.html")]
pub struct ResetTemplate {
    pub token: String,
    pub error: Option<String>,
}

/// Sección 6.5 — página "No soy yo": confirma que se anuló el enlace de
/// recuperación. `cancelled=true` si el token existed y se invalidó.
#[derive(Template)]
#[template(path = "auth/reset_cancel.html")]
pub struct ResetCancelTemplate {
    pub cancelled: bool,
    pub info: String,
}

/// Página `/vincular` — puerta de entrada al flujo de vinculación de
/// dispositivos. Tiene dos modos según quién la abra:
///   mode = "primary"   → el usuario YA tiene sesión: ve el generador de QR +
///                        código numérico para que otro dispositivo se vincule.
///   mode = "secondary" → visitante sin sesión: ve el formulario para teclear
///                        el código de 6 dígitos que le muestra el principal.
/// `next_url` se usa cuando el flujo viene de /login?next=… para volver ahí.
#[derive(Template)]
#[template(path = "link/vincular.html")]
pub struct VincularTemplate {
    pub mode: String,
    pub username: String,
    pub avatar: String,
    pub next_url: String,
    pub error: Option<String>,
}

/// Página `/vincular?c=<code>` — confirmación tras escanear el QR. Muestra a
/// qué cuenta se va a vincular el dispositivo y un botón "Confirmar". Sin
/// auto-vincular: el usuario debe pulsar Aceptar (permite abortar si escaneó
/// un QR por error).
#[derive(Template)]
#[template(path = "link/vincular_confirm.html")]
pub struct VincularConfirmTemplate {
    pub code: String,
    pub target_username: String,
    pub target_avatar: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "ia/aris/aris.html")]
pub struct ArisPage {
    pub user: crate::models::User,
    pub chats: Vec<crate::handlers::aris::ArisChat>,
    pub is_guest: bool,
    pub display: String,
    pub role_display: String,
    pub avatar: String,
    pub credits_limit: i64,
    pub credits_used: i64,
    pub credits_remaining: i64,
}

#[derive(Template)]
#[template(path = "admin/apis.html")]
pub struct AdminApisPage {
    pub user: crate::models::User,
    pub keys: Vec<crate::handlers::aris::ApiKeyRow>,
    pub providers: Vec<crate::handlers::aris::ApiProviderRow>,
    pub logs: Vec<crate::handlers::aris::ApiRotationLogRow>,
    pub rotation_enabled: bool,
    pub rotation_strategy: String,
    pub cooldown_minutes: i64,
    pub auto_failover: bool,
    pub ollama_url: String,
    pub hourly_limit: i64,
    pub daily_limit: i64,
    pub daily_credits_limit: i64,
    pub aethramail_url: String,
    pub system_prompt: String,
}

#[derive(Template)]
#[template(path = "theme.html")]
pub struct ThemePage {
    pub user: crate::models::User,
    pub current: String,
    pub themes: Vec<(&'static str, &'static str)>,
}

impl ThemePage {
    pub fn eq_current(&self, id: &str) -> bool {
        self.current == id
    }
}

#[derive(Template)]
#[template(path = "creatividad/aextube/aextube.html")]
pub struct AextubeTemplate {
    pub username: String,
    pub display: String,
    pub role: String,
    pub user_id: i64,
    pub can_moderate: bool,
    pub can_use_aris_studio: bool,
    pub videos_json: String,
}

#[derive(Template)]
#[template(path = "herramientas/aexos/aexos.html")]
pub struct AexosTemplate {
    pub username: String,
    pub display: String,
    pub role_display: String,
    pub avatar: String,
    pub can_access_admin: bool,
}

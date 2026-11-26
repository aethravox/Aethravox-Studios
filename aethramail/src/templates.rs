use askama::Template;

use crate::models::Email;

/// Login — página standalone (sin sesión aún).
#[derive(Template)]
#[template(path = "correo/login.html")]
pub struct LoginTemplate {
    pub domains: Vec<String>,
    pub error: Option<String>,
    pub address: String,
    pub domain: String,
}

/// Registro — standalone.
#[derive(Template)]
#[template(path = "correo/register.html")]
pub struct RegisterTemplate {
    pub domains: Vec<String>,
    pub error: Option<String>,
    pub address: String,
    pub domain: String,
}

#[derive(Template)]
#[template(path = "correo/bandeja.html")]
pub struct BandejaTemplate {
    pub account_full: String,
    pub unread: i64,
    pub folder: String,
    pub emails: Vec<Email>,
}

#[derive(Template)]
#[template(path = "correo/leer.html")]
pub struct LeerTemplate {
    pub account_full: String,
    pub unread: i64,
    pub email: Email,
}

#[derive(Template)]
#[template(path = "correo/redactar.html")]
pub struct RedactarTemplate {
    pub account_full: String,
    pub unread: i64,
    pub to: String,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "correo/configuracion.html")]
pub struct ConfiguracionTemplate {
    pub account_full: String,
    pub address: String,
    pub domain: String,
    pub created_at: String,
    pub unread: i64,
    pub info: Option<String>,
    pub error: Option<String>,
}

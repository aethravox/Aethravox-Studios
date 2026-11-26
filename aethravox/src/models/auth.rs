#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub remember: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    #[serde(rename = "password_confirm")]
    pub password_confirm: String,
    /// Sección 6.3 — correo interno opcional para vincular al crear la cuenta.
    /// Si se da y es válido, queda en user_emails como primario NO verificado;
    /// el usuario lo verifica después desde /cuenta/correo (flujo 6.2).
    #[serde(default)]
    pub correo_interno: Option<String>,
    /// Checkbox "He leído y acepto los Términos y la Política de Privacidad".
    /// Acepta "1", "true", "on" — el checkbox HTML envía "1" cuando se marca.
    /// `default` = false para que el formulario sin la casilla (p.ej. bot
    /// malicioso) la rechace de forma segura.
    #[serde(default, alias = "accept_terms")]
    pub accept_terms: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct RecoverForm {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetForm {
    pub token: String,
    pub password: String,
    #[serde(rename = "password_confirm")]
    pub password_confirm: String,
}


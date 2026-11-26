#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

/// Dispositivo vinculado a una cuenta (vista del principal en /dashboard).
/// Solo sesiones is_primary=0; las primary se muestran aparte.
#[derive(Debug, Clone, FromRow)]
pub struct LinkedDevice {
    pub token: String,
    pub device_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub link_expires_at: Option<DateTime<Utc>>,
}

/// Código de vinculación activo (uno por principal al pedir /api/link/code).
#[derive(Debug, Clone, FromRow)]
pub struct LinkCode {
    pub code: String,
    pub numeric_code: String,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
}

/// Formulario público: el dispositivo que se vincula teclea el código de 6
/// dígitos (o pega el token del QR si lo escaneó a mano). No requiere sesión.
#[derive(Debug, Deserialize)]
pub struct LinkConsumeForm {
    pub code: String,
    #[serde(default)]
    pub device_name: Option<String>,
}


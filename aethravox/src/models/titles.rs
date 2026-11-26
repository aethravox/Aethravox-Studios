#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

/// Título público que el Owner otorga a un usuario. Es puramente cosmético
/// (no da permisos): se muestra como badge junto al nombre y en el perfil.
/// El usuario elige hasta 3 visibles en su perfil y 1 principal (el principal
/// se ve junto al nombre en todo el sitio).
#[derive(Debug, Clone, FromRow)]
pub struct Title {
    pub id: i64,
    pub name: String,
    /// Color composite "#hex|text|opacity" o "#hex~#hex2|text|opacity"
    /// (mismo formato que roles.color). Lo parsea role_color::parse_role_color.
    pub color: Option<String>,
    /// Efecto visual: none | glow | shimmer | gradient.
    pub effect: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Datos de un título para render (con id + color css ya resuelto + effect).
/// Lo construye el handler a partir de Title + parse_role_color.
#[derive(Debug, Clone)]
pub struct TitleView {
    pub id: i64,
    pub name: String,
    pub color_css: String,
    pub effect: String,
    pub description: Option<String>,
}


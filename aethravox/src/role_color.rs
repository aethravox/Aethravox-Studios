//! Colores de rol: parsea el composite del color picker y sirve CSS + JSON.
//!
//! El color picker de /admin/roles (`role_color_picker.js`) guarda en el campo
//! hidden `color` un composite:
//!   "<bg>|<text>|<opacity>"
//!   - <bg>: "#hex" (sólido) o "#hex1~#hex2" (degradado de 2 colores).
//!   - <text>: "#hex" para la letra, o vacío = auto-contraste sobre el fondo.
//!   - <opacity>: 0-100 (%), o vacío = 100.
//!
//! Este módulo lo parsea a CSS listo y expone dos rutas públicas:
//!   - GET /api/role_colors        → JSON { role_key: { color, text, name } }
//!   - GET /static/role-colors.css → CSS `.role-badge.role-<key>{background;color}`
//!
//! El CSS se enlaza en `<head>` (base.html) para pintar los badges en el primer
//! render y evitar el FOUC: antes `role_colors.js` aplicaba los colores tarde,
//! vía fetch a /api/role_colors, así que los badges aparecían sin fondo un
//! instante. Ahora el CSS ya trae los colores; `role_colors.js` se queda solo
//! para reemplazar el role_key por el nombre display en badges dinámicos.

use std::collections::HashMap;
use std::sync::RwLock;

use axum::extract::State;
use axum::response::{IntoResponse, Json, Response};
use serde::Serialize;

use crate::AppState;
use crate::db::Db;

/// Una entrada de color de rol, lista para serializar al JSON que espera
/// `role_colors.js` y para volcar al CSS.
#[derive(Clone, Serialize)]
pub struct RoleColorEntry {
    /// CSS de fondo: hex sólido ("#8b5cf6"), rgba con alpha o linear-gradient.
    pub color: String,
    /// Color de texto legible (hex). Resuelto por el admin o auto-contraste.
    pub text: String,
    /// Nombre display del rol (p.ej. "Moderador"), para reemplazar el role_key.
    pub name: String,
}

/// Caché en memoria del mapa { role_key → RoleColorEntry }. Se refresca al
/// arrancar y tras cada create/update/delete de rol, así /api/role_colors y
/// /static/role-colors.css reflejan los cambios sin reiniciar el binario.
#[derive(Default)]
pub struct RoleColorCache {
    inner: RwLock<HashMap<String, RoleColorEntry>>,
}

impl RoleColorCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recarga el mapa desde la DB.
    pub async fn refresh(&self, db: &Db) {
        let roles = db.list_roles().await.unwrap_or_default();
        let mut map = HashMap::with_capacity(roles.len());
        for r in roles {
            let (bg, text) = parse_role_color(r.color.as_deref());
            map.insert(
                r.role_key.clone(),
                RoleColorEntry {
                    color: bg,
                    text,
                    name: r.name.clone(),
                },
            );
        }
        if let Ok(mut w) = self.inner.write() {
            *w = map;
        }
    }

    /// Snapshot clonado para responder sin mantener el lock.
    pub fn snapshot(&self) -> HashMap<String, RoleColorEntry> {
        self.inner.read().map(|r| r.clone()).unwrap_or_default()
    }
}

/// Normaliza un hex a 6 dígitos minúsculas SIN '#'. Acepta "#abc", "abc",
/// "#aabbcc", "aabbcc". Devuelve None si no es un hex válido.
fn norm_hex(s: &str) -> Option<String> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 3 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = String::with_capacity(6);
        for c in s.chars() {
            out.push(c);
            out.push(c);
        }
        return Some(out.to_lowercase());
    }
    if s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(s.to_lowercase());
    }
    None
}

/// hex6 (sin '#') → "rgba(r,g,b,a)" si opacity < 100, si no "#hex6".
fn rgba(hex6: &str, opacity: u8) -> String {
    if opacity >= 100 {
        return format!("#{}", hex6);
    }
    let r = u8::from_str_radix(&hex6[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex6[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex6[4..6], 16).unwrap_or(0);
    let a = opacity as f32 / 100.0;
    format!("rgba({},{},{},{})", r, g, b, a)
}

/// Color de texto legible sobre un fondo hex6 (auto-contraste).
/// Luminancia > 0.6 → oscuro (#1a1a1a); si no → blanco.
fn text_color_for(hex6: &str) -> String {
    let r = u8::from_str_radix(&hex6[0..2], 16).unwrap_or(0) as f32;
    let g = u8::from_str_radix(&hex6[2..4], 16).unwrap_or(0) as f32;
    let b = u8::from_str_radix(&hex6[4..6], 16).unwrap_or(0) as f32;
    let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
    if lum > 0.6 {
        "#1a1a1a".into()
    } else {
        "#ffffff".into()
    }
}

/// Parsea el composite "<bg>|<text>|<opacity>" (o un hex suelto, o None) a
/// (background_css, text_css). Devuelve ("", "") si no hay color.
///
/// Esto es lo que `role_color_picker.js` construye en el hidden `color` y lo
/// que el backend lee tanto al guardar (roles.rs) como al servir (aquí).
pub fn parse_role_color(raw: Option<&str>) -> (String, String) {
    let raw = match raw {
        Some(s) => s.trim(),
        None => "",
    };
    if raw.is_empty() {
        return (String::new(), String::new());
    }

    let parts: Vec<&str> = raw.split('|').collect();
    let bg = parts.first().map(|s| s.trim()).unwrap_or("");
    let text_part = parts.get(1).map(|s| s.trim()).unwrap_or("");
    let opacity: u8 = parts
        .get(2)
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(100);

    // bg: "#hex" sólido o "#hex1~#hex2" degradado.
    let (bg_css, c1): (String, Option<String>) = if let Some(idx) = bg.find('~') {
        let (a, rest) = bg.split_at(idx);
        let b = &rest[1..]; // saltar el '~'
        match (norm_hex(a), norm_hex(b)) {
            (Some(c1), Some(c2)) => (
                format!(
                    "linear-gradient(135deg, {}, {})",
                    rgba(&c1, opacity),
                    rgba(&c2, opacity)
                ),
                Some(c1),
            ),
            _ => (String::new(), None),
        }
    } else {
        match norm_hex(bg) {
            Some(c1) => (rgba(&c1, opacity), Some(c1)),
            None => (String::new(), None),
        }
    };

    let text_css = if !text_part.is_empty() {
        norm_hex(text_part)
            .map(|h| format!("#{}", h))
            .unwrap_or_else(|| "#ffffff".into())
    } else if let Some(c1) = c1 {
        text_color_for(&c1)
    } else {
        "#ffffff".into()
    };

    (bg_css, text_css)
}

// ===== GET /api/role_colors → JSON { role_key: { color, text, name } } =====
// Público (sin AuthUser): los badges se pintan en todas las páginas, incluidas
// las públicas, y role_colors.js lo pide sin sesión.
pub async fn api_role_colors(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.role_colors.snapshot())
}

// ===== GET /static/role-colors.css → CSS para <head> (evita FOUC) =====
pub async fn role_colors_css(State(state): State<AppState>) -> Response {
    let map = state.role_colors.snapshot();
    let mut css = String::from(
        "/* Generado server-side desde la tabla roles. No editar a mano. */\n",
    );
    for (key, e) in &map {
        if e.color.is_empty() {
            continue;
        }
        // role_key es alfanumérico + "_" (owner, admin, custom_5): seguro en CSS.
        css.push_str(&format!(
            ".role-badge.role-{}{{background:{};color:{}}}\n",
            key, e.color, e.text
        ));
    }
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8".to_string()),
            (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        axum::body::Body::from(css),
    )
        .into_response()
}

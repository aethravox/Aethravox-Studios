#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct Web {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub url: String,
    pub is_public: i64,
    pub created_at: DateTime<Utc>,
    pub tag: Option<String>,
    /// Mensaje custom para mostrar cuando la web está próximamente/mantenimiento.
    /// Si es None, se usa un mensaje default calculado a partir del tag.
    pub tag_message: Option<String>,
    /// Fecha/hora (UTC) en la que la sección volverá a estar disponible.
    /// Solo aplica para tags "próximamente" y "mantenimiento".
    pub tag_opens_at: Option<DateTime<Utc>>,
    /// 1 = visible en el hub, 0 = desactivada (oculta).
    pub is_active: i64,
    /// 1 = sección custom creada por el owner (se puede eliminar),
    /// 0 = sección del sistema (no se puede eliminar, solo desactivar).
    pub is_custom: i64,
    /// HTML a servir cuando alguien visita /slug. None si la sección usa URL
    /// externa o ruta hardcoded.
    pub html_content: Option<String>,
}

impl Web {
    pub fn icon_emoji(&self) -> String {
        self.icon.clone().unwrap_or_else(|| "📄".into())
    }
    pub fn description_or_empty(&self) -> String {
        self.description.clone().unwrap_or_default()
    }
    pub fn tag_normalized(&self) -> String {
        self.tag.as_ref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).unwrap_or_else(|| "activo".to_string())
    }
    pub fn is_clickable(&self) -> bool {
        let t = self.tag_normalized();
        t != "próximamente" && t != "proximamente" && t != "proximante" && t != "mantenimiento"
    }
    /// Igual que `is_clickable` pero tiene en cuenta el rol del usuario que ve
    /// la card. Para el tag "en_proceso": admin/staff/owner PUEDEN entrar
    /// (bypass, es una sección que están probando); usuario ve el modal igual
    /// que con "próximamente". El resto de tags no dependen del rol.
    pub fn is_clickable_for(&self, role: &str) -> bool {
        let t = self.tag_normalized();
        if t == "en_proceso" || t == "proceso" {
            return matches!(role, "owner" | "admin" | "staff");
        }
        self.is_clickable()
    }
    pub fn tag_color(&self) -> &'static str {
        match self.tag_normalized().as_str() {
            "próximamente" | "proximamente" => "#e0a308",
            "en_proceso" | "proceso" => "#3b82f6",
            "mantenimiento" => "#ff8c42",
            "beta" => "#ff3e3e",
            "nuevo" => "#00d4ff",
            _ => "#00ff88",
        }
    }
    pub fn tag_icon(&self) -> &'static str {
        match self.tag_normalized().as_str() {
            "próximamente" | "proximante" => "🔜",
            "en_proceso" | "proceso" => "🔧",
            "mantenimiento" => "🚧",
            "beta" => "🧪",
            "nuevo" => "✨",
            _ => "✅",
        }
    }
    pub fn tag_label(&self) -> &'static str {
        match self.tag_normalized().as_str() {
            "próximamente" | "proximante" => "Próximamente",
            "en_proceso" | "proceso" => "En proceso",
            "mantenimiento" => "En mantenimiento",
            "beta" => "Beta",
            "nuevo" => "Nuevo",
            _ => "Activo",
        }
    }
    /// Mensaje a mostrar cuando el usuario hace click en una card no-clickable.
    /// Prioridad: tag_message custom > default según tag.
    pub fn tag_display_message(&self) -> String {
        if let Some(m) = &self.tag_message {
            let m = m.trim();
            if !m.is_empty() {
                return m.to_string();
            }
        }
        // Defaults según tag.
        let tag = self.tag_normalized();
        match tag.as_str() {
            "próximamente" | "proximamente" | "proximante" => {
                match self.tag_opens_at {
                    Some(dt) => format!(
                        "⏳ Esta sección estará disponible el {}. ¡Vuelve pronto!",
                        dt.format("%d/%m/%Y a las %H:%M")
                    ),
                    None => "🚧 Esta sección está en construcción. Pronto estará disponible.".into(),
                }
            }
            "en_proceso" | "proceso" => {
                match self.tag_opens_at {
                    Some(dt) => format!(
                        "🔧 Sección en desarrollo. Estimación de entrega: {}.",
                        dt.format("%d/%m/%Y a las %H:%M")
                    ),
                    None => "🔧 Esta sección está en desarrollo. ¡Pronto estará lista!".into(),
                }
            }
            "mantenimiento" => {
                match self.tag_opens_at {
                    Some(dt) => format!(
                        "🛠️ Volverá a estar disponible el {}. Gracias por tu paciencia.",
                        dt.format("%d/%m/%Y a las %H:%M")
                    ),
                    None => "🛠️ Esta sección está temporalmente en mantenimiento. Disculpa las molestias.".into(),
                }
            }
            _ => "Esta sección no está disponible en este momento.".into(),
        }
    }
    /// ¿Tiene fecha de apertura configurada?
    pub fn has_opens_at(&self) -> bool { self.tag_opens_at.is_some() }

    /// URL efectiva para el link de la card en el hub:
    /// - Si la URL empieza con http (externa) → /embed/slug (abre dentro de Aethravox en iframe)
    /// - Si empieza con / (interna o custom HTML) → la URL tal cual
    pub fn effective_url(&self) -> String {
        if self.url.starts_with("http://") || self.url.starts_with("https://") {
            format!("/embed/{}", self.slug)
        } else {
            self.url.clone()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWebForm {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub url: String,
    #[serde(default)]
    pub is_public: Option<String>,
    /// Tipo de URL: "external" (URL externa como Discord) o "custom" (HTML propio).
    /// Si es None o vacío, se asume "external" (comportamiento antiguo).
    #[serde(default)]
    pub url_type: Option<String>,
    /// HTML content para secciones custom. Solo se usa si url_type="custom".
    #[serde(default)]
    pub html_content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebHtmlForm {
    pub html_content: String,
}

#[derive(Debug, Deserialize)]
pub struct WebPermsForm {
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub admin: Option<String>,
    #[serde(default)]
    pub staff: Option<String>,
    #[serde(default)]
    pub usuario: Option<String>,
}


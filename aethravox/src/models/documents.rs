#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: i64,
    pub preview_path: Option<String>,
    pub preview_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub downloaded: i64,
}

impl Document {
    /// Tipo de archivo como enum (para usar helpers de docs.rs).
    pub fn file_type_enum(&self) -> crate::docs::FileType {
        match self.file_type.as_str() {
            "pdf" => crate::docs::FileType::Pdf,
            "docx" => crate::docs::FileType::Docx,
            "pptx" => crate::docs::FileType::Pptx,
            _ => crate::docs::FileType::Unknown,
        }
    }

    /// Icono emoji según tipo.
    pub fn icon(&self) -> &'static str {
        self.file_type_enum().icon()
    }

    /// MIME type para descargar.
    pub fn mime(&self) -> &'static str {
        self.file_type_enum().mime()
    }

    /// Tamaño legible ("2.4 MB").
    pub fn human_size(&self) -> String {
        crate::docs::human_size(self.file_size)
    }

    /// ¿Tiene preview lista para mostrar?
    pub fn has_preview(&self) -> bool {
        self.preview_status == "ready"
            && self
                .preview_path
                .as_ref()
                .map(|p| !p.is_empty())
                .unwrap_or(false)
    }
}

#[derive(Debug, Deserialize)]
pub struct DocUploadForm {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DocUpdateForm {
    pub title: String,
    pub description: Option<String>,
}


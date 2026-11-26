#![allow(unused_imports)]
// Web 4 — Aethravox Docs
// Galería de documentos. Solo el Owner sube/edita/elimina.
// Todos los logueados pueden ver y descargar.

use askama::Template;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use std::path::PathBuf;

use crate::{
    auth_mw::AuthUser,
    docs::{convert_to_pdf, human_size, sanitize_filename, FileType},
    models::{DocUpdateForm, Document},
    AppState,
};

const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024; // 100 MB

// =====================================================
// /docs — Galería pública (todos los logueados)
// =====================================================


pub async fn list(State(state): State<AppState>, AuthUser(user): AuthUser) -> impl IntoResponse {
    let documents = state.db.list_documents().await.unwrap_or_default();
    let total = documents.len() as i64;
    let total_size: i64 = documents.iter().map(|d| d.file_size).sum();

    DocsListTemplate {
        documents: documents
            .iter()
            .map(|d| DocRow {
                id: d.id,
                title: d.title.clone(),
                description: d.description.clone().unwrap_or_default(),
                file_name: d.file_name.clone(),
                file_type: d.file_type.clone(),
                icon: d.icon().to_string(),
                size: d.human_size(),
                created_at: d.created_at.format("%d/%m/%Y").to_string(),
                has_preview: d.has_preview(),
                downloaded: d.downloaded,
            })
            .collect(),
        total,
        total_size: human_size(total_size),
        is_guest: user.id == 0,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "creatividad/docs/docs_list.html")]
pub struct DocsListTemplate {
    pub documents: Vec<DocRow>,
    pub total: i64,
    pub total_size: String,
    /// true si el viewer es invitado sin sesión (User sintético id=0).
    pub is_guest: bool,
}

pub struct DocRow {
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

// =====================================================
// /docs/:id — Visor con PDF.js
// =====================================================
pub async fn view(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return DocsNotFoundTemplate { id }.into_response();
    };

    // Determina la URL del PDF a mostrar:
    // - Si es PDF original y no necesita conversión: /docs/:id/raw
    // - Si tiene preview_path generado: /docs/:id/preview
    let preview_url = if doc.file_type == "pdf" {
        format!("/docs/{}/raw", doc.id)
    } else if doc.has_preview() {
        format!("/docs/{}/preview", doc.id)
    } else {
        // No hay preview todavía — mostramos un mensaje de "convirtiendo..." o error.
        String::new()
    };

    DocsViewTemplate {
        id: doc.id,
        title: doc.title.clone(),
        description: doc.description.clone().unwrap_or_default(),
        file_name: doc.file_name.clone(),
        file_type: doc.file_type.clone(),
        icon: doc.icon().to_string(),
        size: doc.human_size(),
        created_at: doc.created_at.format("%d/%m/%Y").to_string(),
        downloaded: doc.downloaded,
        preview_url,
        preview_status: doc.preview_status.clone(),
        has_preview: doc.has_preview(),
        is_guest: user.id == 0,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "creatividad/docs/docs_view.html")]
pub struct DocsViewTemplate {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub file_name: String,
    pub file_type: String,
    pub icon: String,
    pub size: String,
    pub created_at: String,
    pub downloaded: i64,
    pub preview_url: String,
    pub preview_status: String,
    pub has_preview: bool,
    /// true si el viewer es invitado sin sesión. Oculta el botón Descargar
    /// (Sección 3.4: guest puede ver documentos públicos pero no descargar).
    pub is_guest: bool,
}

#[derive(Template)]
#[template(path = "creatividad/docs/docs_not_found.html")]
pub struct DocsNotFoundTemplate {
    pub id: i64,
}

// =====================================================
// /docs/:id/raw — Sirve el archivo ORIGINAL (para PDFs)
// =====================================================
pub async fn serve_raw(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = PathBuf::from(&doc.file_path);
    if !path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_file(&path, doc.mime(), &doc.file_name, false).await
}

// =====================================================
// /docs/:id/preview — Sirve el PDF generado por LibreOffice
// =====================================================
pub async fn serve_preview(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if !doc.has_preview() {
        // Si todavía no tiene preview, intentamos generarlo on-demand.
        if doc.file_type == "pdf" {
            // PDFs no necesitan conversión; el handler /raw ya los sirve.
            return Redirect::to(&format!("/docs/{}/raw", id)).into_response();
        }
        // Si es DOCX/PPTX y está "pending" o "failed", intentamos convertir.
        if doc.preview_status == "pending" || doc.preview_status == "failed" {
            let _ = state.db.set_document_preview(id, None, "converting").await;
            match try_generate_preview(&state, &doc).await {
                Ok(()) => {
                    let _ = state.db.set_document_preview(
                        id,
                        Some(&format!("data/previews/{}.pdf", doc.id)),
                        "ready",
                    ).await;
                }
                Err(e) => {
                    tracing::error!("Preview falló para doc {}: {}", id, e);
                    let _ = state.db.set_document_preview(id, None, "failed").await;
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        } else {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    let preview_path = match doc.preview_path.as_ref() {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if !preview_path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }
    serve_file(&preview_path, "application/pdf", "preview.pdf", true).await
}

/// Genera el preview de un DOCX/PPTX convirtiéndolo a PDF con LibreOffice.
async fn try_generate_preview(state: &AppState, doc: &Document) -> anyhow::Result<()> {
    use tokio::task;

    let input = PathBuf::from(&doc.file_path);
    let out_dir_str = "data/previews".to_string();
    let out_dir = PathBuf::from(&out_dir_str);
    let id = doc.id;

    // LibreOffice puede tardar 5-30s. Lo hacemos en un blocking task.
    let pdf_path = task::spawn_blocking(move || convert_to_pdf(&input, &out_dir))
        .await
        .map_err(|e| anyhow::anyhow!("spawn blocking falló: {}", e))??;

    // Renombrar el PDF a {id}.pdf (predecible)
    let final_path = PathBuf::from(&out_dir_str).join(format!("{}.pdf", id));
    if final_path.exists() {
        std::fs::remove_file(&final_path)?;
    }
    if pdf_path != final_path {
        std::fs::rename(&pdf_path, &final_path)?;
    }

    let _ = state
        .db
        .audit(Some(0), "doc_preview_generated", Some(&format!("id={}", id)), None)
        .await;

    Ok(())
}

// =====================================================
// /docs/:id/download — Descarga del original (con counter)
// =====================================================
pub async fn download(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = PathBuf::from(&doc.file_path);
    if !path.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let _ = state.db.increment_document_downloads(id).await;
    serve_file(&path, doc.mime(), &doc.file_name, true).await
}

// =====================================================
// Helper: servir un archivo con cabeceras correctas
// =====================================================
pub async fn serve_file(
    path: &PathBuf,
    mime: &str,
    filename: &str,
    attachment: bool,
) -> Response {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            let cd = if attachment {
                format!(
                    "attachment; filename=\"{}\"",
                    filename.replace('"', "_")
                )
            } else {
                "inline".to_string()
            };
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime.to_string()),
                    (header::CONTENT_DISPOSITION, cd),
                    (header::CONTENT_LENGTH, bytes.len().to_string()),
                ],
                Body::from(bytes),
            )
                .into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

// =====================================================
// /admin/docs — Gestión (solo Owner)
// =====================================================

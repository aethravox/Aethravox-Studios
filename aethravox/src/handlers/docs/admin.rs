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

use super::public::serve_file;

pub async fn admin_list(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.view").await {
        return Redirect::to("/admin").into_response();
    }
    let documents = state.db.list_documents().await.unwrap_or_default();

    AdminDocsTemplate {
        documents: documents
            .iter()
            .map(|d| AdminDocRow {
                id: d.id,
                title: d.title.clone(),
                description: d.description.clone().unwrap_or_default(),
                file_name: d.file_name.clone(),
                file_type: d.file_type.clone(),
                icon: d.icon().to_string(),
                size: d.human_size(),
                created_at: d.created_at.format("%d/%m/%Y %H:%M").to_string(),
                preview_status: d.preview_status.clone(),
                downloaded: d.downloaded,
            })
            .collect(),
        libreoffice_ok: crate::docs::libreoffice_available(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/docs/admin_docs.html")]
pub struct AdminDocsTemplate {
    pub documents: Vec<AdminDocRow>,
    pub libreoffice_ok: bool,
}

pub struct AdminDocRow {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub file_name: String,
    pub file_type: String,
    pub icon: String,
    pub size: String,
    pub created_at: String,
    pub preview_status: String,
    pub downloaded: i64,
}

// =====================================================
// /admin/docs/new — Form de subida (GET)
// =====================================================
pub async fn admin_new_get(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.create").await {
        return Redirect::to("/admin/docs").into_response();
    }
    AdminDocsNewTemplate {
        error: None,
        max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/docs/admin_docs_new.html")]
pub struct AdminDocsNewTemplate {
    pub error: Option<String>,
    pub max_mb: i64,
}

// =====================================================
// /admin/docs/new — Subida (POST multipart)
// =====================================================
pub async fn admin_new_post(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    mut multipart: Multipart,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.create").await {
        return Redirect::to("/admin/docs").into_response();
    }

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = Some(field.text().await.unwrap_or_default());
            }
            "description" => {
                description = Some(field.text().await.unwrap_or_default());
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map(|b| b.to_vec())
                    .unwrap_or_default();
                if !bytes.is_empty() {
                    file_bytes = Some(bytes);
                }
            }
            _ => {
                // Otros campos ignorados
            }
        }
    }

    let title = match title {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            return AdminDocsNewTemplate {
                error: Some("El título es obligatorio.".into()),
                max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
            }
            .into_response();
        }
    };

    let file_name = match file_name {
        Some(f) if !f.is_empty() => f,
        _ => {
            return AdminDocsNewTemplate {
                error: Some("Debes seleccionar un archivo.".into()),
                max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
            }
            .into_response();
        }
    };

    let file_bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => {
            return AdminDocsNewTemplate {
                error: Some("El archivo está vacío.".into()),
                max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
            }
            .into_response();
        }
    };

    if file_bytes.len() > MAX_UPLOAD_BYTES {
        return AdminDocsNewTemplate {
            error: Some(format!(
                "El archivo supera el límite de {} MB.",
                MAX_UPLOAD_BYTES / (1024 * 1024)
            )),
            max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
        }
        .into_response();
    }

    let ftype = FileType::from_filename(&file_name);
    if ftype == FileType::Unknown {
        return AdminDocsNewTemplate {
            error: Some("Tipo de archivo no soportado. Solo PDF, DOCX y PPTX.".into()),
            max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
        }
        .into_response();
    }

    let description = description
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());

    // 1. Crear registro en DB (necesitamos el ID para nombrar el archivo en disco).
    let doc_id = match state
        .db
        .create_document(
            &title,
            description.as_deref(),
            &file_name,
            "", // se actualizará abajo
            ftype.as_str(),
            file_bytes.len() as i64,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Error creando doc en DB: {e}");
            return AdminDocsNewTemplate {
                error: Some("Error interno al guardar en la base de datos.".into()),
                max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
            }
            .into_response();
        }
    };

    // 2. Guardar el archivo en disco.
    let safe_name = sanitize_filename(&file_name);
    let disk_name = format!("{}_{}", doc_id, safe_name);
    let disk_path = format!("data/files/{}", disk_name);

    // Asegurar que el directorio data/files/ existe.
    if let Err(e) = std::fs::create_dir_all("data/files") {
        tracing::error!("Error creando data/files/: {e}");
        let _ = state.db.delete_document(doc_id).await;
        return AdminDocsNewTemplate {
            error: Some("Error creando el directorio de archivos.".into()),
            max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
        }
        .into_response();
    }

    if let Err(e) = std::fs::write(&disk_path, &file_bytes) {
        tracing::error!("Error guardando archivo en disco: {e}");
        // Limpieza: borrar el registro DB
        let _ = state.db.delete_document(doc_id).await;
        return AdminDocsNewTemplate {
            error: Some("Error guardando el archivo en disco.".into()),
            max_mb: (MAX_UPLOAD_BYTES / (1024 * 1024)) as i64,
        }
        .into_response();
    }

    // 3. Actualizar el registro con la ruta correcta.
    let _ = sqlx::query("UPDATE documents SET file_path = ? WHERE id = ?")
        .bind(&disk_path)
        .bind(doc_id)
        .execute(&state.db.pool)
        .await;

    // 4. Si es PDF, marcar preview listo (no necesita conversión).
    if ftype == FileType::Pdf {
        let _ = state
            .db
            .set_document_preview(doc_id, Some(&disk_path), "ready")
            .await;
    } else {
        // 5. Para DOCX/PPTX, lanzar conversión en background.
        let state_clone = state.clone();
        let doc_id_clone = doc_id;
        let disk_path_clone = disk_path.clone();
        tokio::spawn(async move {
            let input = PathBuf::from(&disk_path_clone);
            let out_dir = PathBuf::from("data/previews");
            match crate::docs::convert_to_pdf(&input, &out_dir) {
                Ok(pdf_path) => {
                    let final_path = out_dir.join(format!("{}.pdf", doc_id_clone));
                    if final_path.exists() {
                        let _ = std::fs::remove_file(&final_path);
                    }
                    if pdf_path != final_path {
                        let _ = std::fs::rename(&pdf_path, &final_path);
                    }
                    let _ = state_clone
                        .db
                        .set_document_preview(
                            doc_id_clone,
                            Some(&format!("data/previews/{}.pdf", doc_id_clone)),
                            "ready",
                        )
                        .await;
                    tracing::info!("Preview generado para doc {}", doc_id_clone);
                }
                Err(e) => {
                    tracing::error!("Preview falló para doc {}: {}", doc_id_clone, e);
                    let _ = state_clone
                        .db
                        .set_document_preview(doc_id_clone, None, "failed")
                        .await;
                }
            }
        });
    }

    let _ = state
        .db
        .audit(Some(actor.id), "doc_upload", Some(&format!("id={} {}", doc_id, title)), None)
        .await;

    Redirect::to("/admin/docs?ok=uploaded").into_response()
}

// =====================================================
// /admin/docs/:id/edit — Editar metadatos (GET muestra form, POST actualiza)
// =====================================================

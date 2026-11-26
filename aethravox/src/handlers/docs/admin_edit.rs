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

pub async fn admin_edit_get(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.edit").await {
        return Redirect::to("/admin/docs").into_response();
    }
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return Redirect::to("/admin/docs?err=not_found").into_response();
    };
    AdminDocsEditTemplate {
        id: doc.id,
        title: doc.title.clone(),
        description: doc.description.clone().unwrap_or_default(),
        file_name: doc.file_name.clone(),
        file_type: doc.file_type.clone(),
        icon: doc.icon().to_string(),
        size: doc.human_size(),
        error: None,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/docs/admin_docs_edit.html")]
pub struct AdminDocsEditTemplate {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub file_name: String,
    pub file_type: String,
    pub icon: String,
    pub size: String,
    pub error: Option<String>,
}

pub async fn admin_edit_post(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<DocUpdateForm>,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.edit").await {
        return Redirect::to("/admin/docs").into_response();
    }
    let title = form.title.trim().to_string();
    if title.is_empty() {
        return AdminDocsEditTemplate {
            id,
            title: title.clone(),
            description: form.description.unwrap_or_default(),
            file_name: String::new(),
            file_type: String::new(),
            icon: "📄".into(),
            size: String::new(),
            error: Some("El título es obligatorio.".into()),
        }
        .into_response();
    }
    let description = form
        .description
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());
    let _ = state
        .db
        .update_document(id, &title, description.as_deref())
        .await;
    let _ = state
        .db
        .audit(Some(actor.id), "doc_edit", Some(&format!("id={} {}", id, title)), None)
        .await;
    Redirect::to("/admin/docs?ok=edited").into_response()
}

// =====================================================
// /admin/docs/:id/regenerate — Regenerar preview
// =====================================================
pub async fn admin_regenerate(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.edit").await {
        return Redirect::to("/admin/docs").into_response();
    }
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return Redirect::to("/admin/docs?err=not_found").into_response();
    };
    if doc.file_type == "pdf" {
        return Redirect::to("/admin/docs?err=already_pdf").into_response();
    }

    let _ = state.db.set_document_preview(id, None, "converting").await;

    let state_clone = state.clone();
    let disk_path = doc.file_path.clone();
    tokio::spawn(async move {
        let input = PathBuf::from(&disk_path);
        let out_dir = PathBuf::from("data/previews");
        match crate::docs::convert_to_pdf(&input, &out_dir) {
            Ok(pdf_path) => {
                let final_path = out_dir.join(format!("{}.pdf", id));
                if final_path.exists() {
                    let _ = std::fs::remove_file(&final_path);
                }
                if pdf_path != final_path {
                    let _ = std::fs::rename(&pdf_path, &final_path);
                }
                let _ = state_clone
                    .db
                    .set_document_preview(id, Some(&format!("data/previews/{}.pdf", id)), "ready")
                    .await;
            }
            Err(e) => {
                tracing::error!("Regenerar preview falló para {}: {}", id, e);
                let _ = state_clone.db.set_document_preview(id, None, "failed").await;
            }
        }
    });

    let _ = state
        .db
        .audit(Some(actor.id), "doc_regenerate_preview", Some(&format!("id={}", id)), None)
        .await;
    Redirect::to("/admin/docs?ok=regenerating").into_response()
}

// =====================================================
// /admin/docs/:id/delete — Eliminar (archivo + preview + DB)
// =====================================================
pub async fn admin_delete(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&actor, "docs.delete").await {
        return Redirect::to("/admin/docs").into_response();
    }
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return Redirect::to("/admin/docs?err=not_found").into_response();
    };

    // Borrar archivo original
    let _ = std::fs::remove_file(&doc.file_path);
    // Borrar preview si existe
    if let Some(p) = &doc.preview_path {
        if !p.is_empty() {
            let _ = std::fs::remove_file(p);
        }
    }
    let _ = state.db.delete_document(id).await;
    let _ = state
        .db
        .audit(Some(actor.id), "doc_delete", Some(&format!("id={} {}", id, doc.title)), None)
        .await;
    Redirect::to("/admin/docs?ok=deleted").into_response()
}

// Helper para tests y silenciar warning de Uri no usado
#[allow(dead_code)]
fn _silence(_: Uri) {}

// ===== /d/:uuid — Visor PÚBLICO (sin login) =====
pub async fn public_view(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Response {
    let Some(doc) = state.db.get_document_by_uuid(&uuid).await.ok().flatten() else {
        return axum::response::Html("<h1>Documento no encontrado</h1>").into_response();
    };
    let _ = state.db.record_document_view(doc.id, None, None).await;
    format!("<h1>{}</h1><p>{}</p>", doc.title, doc.file_name).into_response()
}

// ===== /docs/:id/download/:format — Descarga multi-formato =====
pub async fn download_as(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path((id, format_code)): Path<(i64, String)>,
) -> Response {
    // Simplified: just download original for now
    let Some(doc) = state.db.get_document(id).await.ok().flatten() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if format_code == "original" {
        let path = PathBuf::from(&doc.file_path);
        if !path.exists() { return StatusCode::NOT_FOUND.into_response(); }
        let _ = state.db.increment_document_downloads(id).await;
        serve_file(&path, doc.mime(), &doc.file_name, true).await
    } else {
        StatusCode::NOT_IMPLEMENTED.into_response()
    }
}

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

mod public;
pub use self::public::*;
mod admin;
pub use self::admin::*;
mod admin_edit;
pub use self::admin_edit::*;

#![allow(dead_code)]
// Sección 7 — Descargas nativas: página pública /descargas + panel admin
// (/admin/downloads) con subida (multipart) y borrado de ejecutables.
//
// Modelos: categorias_page (página guest), upload_chat_file (multipart),
// webs_view / delete_web (admin CRUD). Los binarios se guardan en
// data/downloads/ y se sirven vía ServeDir en /descargas/files (main.rs).
use askama::Template;
use axum::{
    extract::{Multipart, Path, State},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    auth_mw::AuthUser,
    db::downloads::Download,
    AppState,
};

// ============================================================
// GET /descargas — página pública (guest-allowed). Lista las plataformas
// Windows/Linux/Android con sus apps y archivos; macOS/iOS "próximamente".
// ============================================================
pub async fn descargas_page(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let rows = state.db.list_active_downloads().await.unwrap_or_default();
    let platforms = build_platforms(&rows);
    DescargasTemplate {
        username: user.username.clone(),
        is_guest: user.id == 0,
        platforms,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "herramientas/descargas/descargas.html")]
pub struct DescargasTemplate {
    pub username: String,
    pub is_guest: bool,
    pub platforms: Vec<PlatformSection>,
}

pub struct PlatformSection {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub coming_soon: bool,
    pub apps: Vec<AppSection>,
}

pub struct AppSection {
    pub slug: String,
    pub name: String,
    pub files: Vec<FileRow>,
}

pub struct FileRow {
    pub id: i64,
    pub version: String,
    pub orig_name: String,
    pub size_mb: String,
    pub created_at: String,
    pub url: String,
}

/// Construye la vista por plataformas en orden fijo (Windows, Linux, Android,
/// macOS próximamente, iOS próximamente). Agrupa los archivos por app_slug.
fn build_platforms(rows: &[Download]) -> Vec<PlatformSection> {
    // (key, name, icon, coming_soon)
    let order: &[(&str, &str, &str, bool)] = &[
        ("windows", "Windows", "🪟", false),
        ("linux", "Linux", "🐧", false),
        ("android", "Android", "🤖", false),
        ("macos", "macOS", "🍎", true),
        ("ios", "iOS", "📱", true),
    ];
    order
        .iter()
        .map(|(key, name, icon, cs)| PlatformSection {
            key: (*key).to_string(),
            name: (*name).to_string(),
            icon: (*icon).to_string(),
            coming_soon: *cs,
            apps: build_apps(rows, key),
        })
        .collect()
}

fn build_apps(rows: &[Download], platform: &str) -> Vec<AppSection> {
    let mut apps: Vec<AppSection> = Vec::new();
    for r in rows.iter().filter(|r| r.platform == platform) {
        let fr = FileRow {
            id: r.id,
            version: if r.version.is_empty() {
                "—".into()
            } else {
                r.version.clone()
            },
            orig_name: if r.orig_name.is_empty() {
                r.filename.clone()
            } else {
                r.orig_name.clone()
            },
            size_mb: format_size(r.size_bytes),
            created_at: r.created_at.chars().take(10).collect(),
            url: format!("/descargas/files/{}", r.filename),
        };
        if let Some(pos) = apps.iter().position(|a| a.slug == r.app_slug) {
            apps[pos].files.push(fr);
        } else {
            apps.push(AppSection {
                slug: r.app_slug.clone(),
                name: r.app_name.clone(),
                files: vec![fr],
            });
        }
    }
    apps
}

fn format_size(bytes: i64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

// ============================================================
// GET /admin/downloads — panel admin (lista plana de todos los archivos).
// ============================================================
pub async fn admin_downloads(State(state): State<AppState>, AuthUser(actor): AuthUser) -> impl IntoResponse {
    if actor.role != "owner" && actor.role != "admin" {
        return Redirect::to("/admin").into_response();
    }
    let rows = state.db.list_all_downloads().await.unwrap_or_default();
    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    AdminDownloadsTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        files: rows
            .iter()
            .map(|r| AdminFileRow {
                id: r.id,
                platform: r.platform.clone(),
                app_name: r.app_name.clone(),
                app_slug: r.app_slug.clone(),
                version: r.version.clone(),
                orig_name: r.orig_name.clone(),
                size_mb: format_size(r.size_bytes),
                created_at: r.created_at.chars().take(10).collect(),
                is_active: r.is_active != 0,
            })
            .collect(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/descargas/admin_downloads.html")]
pub struct AdminDownloadsTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub files: Vec<AdminFileRow>,
}

pub struct AdminFileRow {
    pub id: i64,
    pub platform: String,
    pub app_name: String,
    pub app_slug: String,
    pub version: String,
    pub orig_name: String,
    pub size_mb: String,
    pub created_at: String,
    pub is_active: bool,
}

// ============================================================
// POST /admin/downloads/create — subida multipart de un ejecutable.
// Campos: platform, app_slug, app_name, version, notes, file.
// ============================================================
pub async fn create_download(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if actor.role != "owner" && actor.role != "admin" {
        return Redirect::to("/admin").into_response();
    }

    let mut platform = String::new();
    let mut app_slug = String::new();
    let mut app_name = String::new();
    let mut version = String::new();
    let mut notes: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut orig_name = String::new();
    // 150 MB (el límite global del body en main.rs es 200 MB).
    const MAX_FILE_BYTES: usize = 150 * 1024 * 1024;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "platform" => {
                if let Ok(b) = field.bytes().await {
                    platform = String::from_utf8_lossy(&b).trim().to_string();
                }
            }
            "app_slug" => {
                if let Ok(b) = field.bytes().await {
                    app_slug = String::from_utf8_lossy(&b).trim().to_string();
                }
            }
            "app_name" => {
                if let Ok(b) = field.bytes().await {
                    app_name = String::from_utf8_lossy(&b).trim().to_string();
                }
            }
            "version" => {
                if let Ok(b) = field.bytes().await {
                    version = String::from_utf8_lossy(&b).trim().to_string();
                }
            }
            "notes" => {
                if let Ok(b) = field.bytes().await {
                    let s = String::from_utf8_lossy(&b).trim().to_string();
                    if !s.is_empty() {
                        notes = Some(s);
                    }
                }
            }
            "file" => {
                if let Some(fn0) = field.file_name() {
                    orig_name = fn0.to_string();
                }
                match field.bytes().await {
                    Ok(b) => file_data = Some(b.to_vec()),
                    Err(e) => {
                        tracing::error!("descargas upload read: {e}");
                        return Redirect::to("/admin/downloads?err=read").into_response();
                    }
                }
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    // Validación.
    if !["windows", "linux", "android", "macos", "ios"].contains(&platform.as_str()) {
        return Redirect::to("/admin/downloads?err=platform").into_response();
    }
    let slug = sanitize_slug(&app_slug);
    if slug.is_empty() || app_name.is_empty() {
        return Redirect::to("/admin/downloads?err=campos").into_response();
    }
    let Some(file_data) = file_data else {
        return Redirect::to("/admin/downloads?err=nofile").into_response();
    };
    if file_data.len() > MAX_FILE_BYTES {
        return Redirect::to("/admin/downloads?err=big").into_response();
    }

    // Saneo el nombre original y valido la extensión (solo ejecutables known).
    let safe_orig = sanitize_filename(&orig_name);
    if !has_allowed_ext(&safe_orig) {
        return Redirect::to("/admin/downloads?err=ext").into_response();
    }

    // Nombre guardado: {hex}_{orig} — único y legible a la vez.
    let hex = crate::db::random_hex(16);
    let stored = format!("{}_{}", hex, safe_orig);
    let dir = "data/downloads";
    let _ = std::fs::create_dir_all(dir);
    let path = format!("{}/{}", dir, stored);
    if let Err(e) = std::fs::write(&path, &file_data) {
        tracing::error!("descargas write: {e}");
        return Redirect::to("/admin/downloads?err=write").into_response();
    }

    let size_bytes = file_data.len() as i64;
    let res = state
        .db
        .create_download(
            &platform,
            &slug,
            &app_name,
            &version,
            &stored,
            &safe_orig,
            size_bytes,
            notes.as_deref(),
        )
        .await;
    match res {
        Ok(id) => {
            let _ = state
                .db
                .audit(
                    Some(actor.id),
                    "download_create",
                    Some(&format!("id={} platform={} app={}", id, platform, slug)),
                    None,
                )
                .await;
            Redirect::to("/admin/downloads?ok=created").into_response()
        }
        Err(e) => {
            tracing::error!("descargas db insert: {e}");
            let _ = std::fs::remove_file(&path); // limpia el archivo huérfano
            Redirect::to("/admin/downloads?err=db").into_response()
        }
    }
}

// ============================================================
// POST /admin/downloads/:id/delete — borra la fila + el archivo en disco.
// ============================================================
pub async fn delete_download(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if actor.role != "owner" && actor.role != "admin" {
        return Redirect::to("/admin").into_response();
    }
    if let Ok(Some(row)) = state.db.get_download(id).await {
        let path = format!("data/downloads/{}", row.filename);
        let _ = std::fs::remove_file(&path);
    }
    let _ = state.db.delete_download(id).await;
    let _ = state
        .db
        .audit(Some(actor.id), "download_delete", Some(&format!("id={}", id)), None)
        .await;
    Redirect::to("/admin/downloads?ok=deleted").into_response()
}

// ============================================================
// Helpers
// ============================================================

/// Sanea un app_slug: solo [a-z0-9_-], max 48. Para agrupar archivos de una
/// misma app sin path traversal ni rarezas en la URL/group key.
fn sanitize_slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(48)
        .collect::<String>()
        .to_lowercase()
}

/// Sanea el nombre de archivo original: quita path components (anti traversal)
/// y se queda solo con [a-zA-Z0-9._-], max 96.
fn sanitize_filename(s: &str) -> String {
    let base = s.rsplit(['/', '\\']).next().unwrap_or(s);
    base.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .take(96)
        .collect()
}

/// Solo permitimos extensiones de ejecutables/paquetes conocidos. Evita que se
/// suba un .html/.svg/.js que el ServeDir serviría y podría ejecutarse en el
/// navegador al navegarlo (en vez de descargarse).
fn has_allowed_ext(name: &str) -> bool {
    let lower = name.to_lowercase();
    const OK: &[&str] = &[
        ".exe", ".msi", ".deb", ".rpm", ".appimage", ".apk", ".dmg", ".ipa", ".zip",
        ".tar.gz", ".bin",
    ];
    OK.iter().any(|e| lower.ends_with(e))
}

// Web 3 — Dashboard / perfil propio.
// El usuario logueado ve y edita su perfil, gestiona sus sesiones activas,
// cambia contraseña y avatar.

use askama::Template;
use axum::{
    extract::{Multipart, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Form,
};

use crate::{
    auth_mw::AuthUser,
    models::{ChangePasswordForm, UpdateProfileForm},
    AppState,
};

// ===== GET /dashboard =====
pub async fn dashboard(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    headers: HeaderMap,
) -> impl IntoResponse {
    let zlibs_public_url = crate::handlers::zlibs::effective_public_url(&state.cfg, &headers);
    let zlibs_account =
        crate::handlers::zlibs::build_zlib_account_payload(&state, user.id, &zlibs_public_url).await;
    let sessions = state.db.list_user_sessions(user.id).await.unwrap_or_default();
    let current_token = String::new();

    let active_session_count = sessions.len() as i64;

    // Dispositivos vinculados (is_primary=0). El principal puede verlos y
    // revocarlos desde el dashboard. Se renderizan server-side para que la
    // lista funcione sin JS (igual que las sesiones normales).
    let linked = state.db.list_linked_devices(user.id).await.unwrap_or_default();
    let now = chrono::Utc::now();
    let linked_devices: Vec<LinkedDeviceRow> = linked
        .into_iter()
        .map(|d| LinkedDeviceRow {
            token: d.token,
            device_name: d.device_name.unwrap_or_else(|| "(sin nombre)".into()),
            created_at: d.created_at.format("%d/%m/%Y %H:%M").to_string(),
            last_seen: d.last_seen_at.format("%d/%m/%Y %H:%M").to_string(),
            expires: d
                .link_expires_at
                .map(|t| t.format("%d/%m/%Y %H:%M").to_string())
                .unwrap_or_else(|| "—".into()),
            active: d.link_expires_at.map(|t| t > now).unwrap_or(false),
        })
        .collect();

    // Badge de rol: si el usuario tiene un rol custom, mostrar su nombre real
    // (p.ej. "Moderador") en vez del legacy "admin".
    let (role_css_key, role_display) = state.db.role_display(&user).await;

    DashboardTemplate {
        username: user.username.clone(),
        role: role_display.clone(),
        role_css_key,
        email: user.email.clone(),
        avatar: user.avatar(),
        display_name: user.display_name.clone().unwrap_or_default(),
        bio: user.bio.clone().unwrap_or_default(),
        created_at: user.created_at.format("%d/%m/%Y").to_string(),
        last_login: user
            .last_login_at
            .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_else(|| "—".into()),
        usage: user.usage_string(),
        active_session_count,
        sessions: sessions
            .iter()
            .map(|s| SessionRow {
                token: s.token.clone(),
                is_current: s.token == current_token,
                remember: s.remember != 0,
                created_at: s.created_at.format("%d/%m/%Y %H:%M").to_string(),
                last_seen: s.last_seen_at.format("%d/%m/%Y %H:%M").to_string(),
                expires: s.expires_at.format("%d/%m/%Y %H:%M").to_string(),
            })
            .collect(),
        linked_devices,
        zlibs_public_url,
        zlibs_tokens: zlibs_account.tokens,
        zlibs_packages: zlibs_account.my_packages,
        info: None,
        error: None,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "productividad/dashboard/dashboard.html")]
pub struct DashboardTemplate {
    pub username: String,
    pub role: String,
    pub role_css_key: String,
    pub email: String,
    pub avatar: String,
    pub display_name: String,
    pub bio: String,
    pub created_at: String,
    pub last_login: String,
    pub usage: String,
    pub active_session_count: i64,
    pub sessions: Vec<SessionRow>,
    pub linked_devices: Vec<LinkedDeviceRow>,
    pub zlibs_public_url: String,
    pub zlibs_tokens: Vec<aethravox_core::ZLibTokenDto>,
    pub zlibs_packages: Vec<aethravox_core::ZLibPackageDto>,
    pub info: Option<String>,
    pub error: Option<String>,
}

pub struct SessionRow {
    pub token: String,
    pub is_current: bool,
    pub remember: bool,
    pub created_at: String,
    pub last_seen: String,
    pub expires: String,
}

/// Fila de "Dispositivos vinculados" en el dashboard del principal.
/// `active` = link_expires_at > now (las caducadas pero no GCeadas se atenúan).
pub struct LinkedDeviceRow {
    pub token: String,
    pub device_name: String,
    pub created_at: String,
    pub last_seen: String,
    pub expires: String,
    pub active: bool,
}

// ===== POST /dashboard/profile =====
pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<UpdateProfileForm>,
) -> impl IntoResponse {
    let display_name = form
        .display_name
        .as_deref()
        .map(|s| crate::models::sanitize_text(s, 64))
        .filter(|s| !s.is_empty());
    let bio = form
        .bio
        .as_deref()
        .map(|s| crate::models::sanitize_text(s, 500))
        .filter(|s| !s.is_empty());

    let avatar_url: Option<&str> = if form.remove_avatar.is_some() {
        Some("")
    } else {
        None
    };

    let _ = state
        .db
        .update_profile(user.id, display_name.as_deref(), bio.as_deref(), avatar_url)
        .await;
    let _ = state
        .db
        .audit(Some(user.id), "profile_update", Some("self"), None)
        .await;
    Redirect::to("/dashboard?ok=profile")
}

// ===== POST /dashboard/password =====
pub async fn change_password(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    if form.new != form.new_confirm {
        return Redirect::to("/dashboard?err=pw_match");
    }
    if let Err(_) = crate::models::validate_password(&form.new) {
        return Redirect::to("/dashboard?err=pw_weak");
    }
    if !crate::db::verify_password(&form.current, &user.password_hash) {
        return Redirect::to("/dashboard?err=pw_current");
    }

    if let Err(e) = state.db.update_password(user.id, &form.new).await {
        tracing::error!("Error actualizando password: {e}");
        return Redirect::to("/dashboard?err=internal");
    }

    let _ = state
        .db
        .audit(Some(user.id), "password_change", Some("self"), None)
        .await;
    let _ = state.db.revoke_all_user_sessions(user.id).await;
    Redirect::to("/login?ok=pw_changed")
}

// ===== POST /dashboard/sessions/:token/revoke =====
pub async fn revoke_session(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let sessions = state.db.list_user_sessions(user.id).await.unwrap_or_default();
    if !sessions.iter().any(|s| s.token == token) {
        return Redirect::to("/dashboard?err=not_your_session");
    }

    let _ = state.db.revoke_session_by_token(&token).await;
    let _ = state
        .db
        .audit(Some(user.id), "session_revoke", Some("self"), None)
        .await;
    Redirect::to("/dashboard?ok=revoked")
}

// ===== POST /dashboard/devices/:token/revoke =====
// Revoca un dispositivo VINCULADO (is_primary=0) del usuario actual. A
// diferencia de revoke_session (que borra cualquier sesión del usuario),
// este solo borra vinculadas — no se puede auto-revocar la primary por
// aquí (para eso está /logout). Devuelve redirect al dashboard con ok/err.
pub async fn revoke_linked_device(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let ok = state
        .db
        .revoke_linked_device(user.id, &token)
        .await
        .unwrap_or(false);
    if ok {
        let _ = state
            .db
            .audit(
                Some(user.id),
                "linked_device_revoked",
                Some(&format!("token={}…", token.get(..8).unwrap_or(&token))),
                None,
            )
            .await;
        Redirect::to("/dashboard?ok=device_revoked")
    } else {
        Redirect::to("/dashboard?err=device_not_found")
    }
}

// ===== POST /dashboard/logout_all =====
pub async fn logout_all(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let _ = state.db.revoke_all_user_sessions(user.id).await;
    let _ = state
        .db
        .audit(Some(user.id), "logout_all", Some("self"), None)
        .await;
    Redirect::to("/logout")
}

// ===== POST /dashboard/avatar — Subir foto de perfil =====
// Acepta multipart con campo "avatar". Convierte a WebP (cwebp), resize a 400px.
// Guarda en data/avatars/<user_id>.webp y actualiza users.avatar_url.
// El avatar se muestra en TODO el sitio (hub, chat, friends, profile, etc.)

pub async fn upload_avatar(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    mut multipart: Multipart,
) -> Response {
    let mut image_data: Option<Vec<u8>> = None;
    let mut content_type: String = "image/jpeg".into();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "avatar" {
            content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            match field.bytes().await {
                Ok(bytes) => image_data = Some(bytes.to_vec()),
                Err(_) => {
                    return Redirect::to("/dashboard?err=avatar_read").into_response();
                }
            }
        }
    }

    let Some(image_data) = image_data else {
        return Redirect::to("/dashboard?err=avatar_empty").into_response();
    };

    // Validar tamaño (máx 5MB)
    if image_data.len() > 5 * 1024 * 1024 {
        return Redirect::to("/dashboard?err=avatar_big").into_response();
    }

    // Determinar extensión
    let ext = match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    };

    // Guardar temporal y convertir a WebP
    let dir = "data/avatars";
    let _ = std::fs::create_dir_all(dir);
    let temp_path = format!("{}/{}_temp.{}", dir, user.id, ext);
    let webp_path = format!("{}/{}.webp", dir, user.id);
    let avatar_url;

    if let Err(_) = std::fs::write(&temp_path, &image_data) {
        return Redirect::to("/dashboard?err=avatar_write").into_response();
    }

    // Intentar cwebp (resize 400px + calidad 80)
    let cwebp_result = std::process::Command::new("cwebp")
        .arg("-q").arg("80")
        .arg("-resize").arg("400").arg("0")
        .arg(&temp_path)
        .arg("-o").arg(&webp_path)
        .output();

    match cwebp_result {
        Ok(output) if output.status.success() => {
            let _ = std::fs::remove_file(&temp_path);
            avatar_url = format!("/avatars/{}.webp?v={}", user.id, chrono::Utc::now().timestamp());
        }
        _ => {
            // Fallback: usar el archivo original
            let fallback_path = format!("{}/{}.{}", dir, user.id, ext);
            let _ = std::fs::rename(&temp_path, &fallback_path);
            // Borrar .webp viejo si existe (para que no sirva el anterior)
            let _ = std::fs::remove_file(&webp_path);
            avatar_url = format!("/avatars/{}.{}?v={}", user.id, ext, chrono::Utc::now().timestamp());
        }
    }

    // Actualizar avatar_url en la DB
    let _ = state.db.update_profile(user.id, None, None, Some(&avatar_url)).await;
    let _ = state.db.audit(Some(user.id), "avatar_updated", Some(&avatar_url), None).await;

    Redirect::to("/dashboard?ok=avatar").into_response()
}

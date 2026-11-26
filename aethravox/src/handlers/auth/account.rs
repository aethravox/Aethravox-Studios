#![allow(unused_imports)]
use axum::{
    extract::{Form, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    auth_mw::get_client_ip,
    db::verify_password,
    models::{LoginForm, RecoverForm, RegisterForm, ResetForm},
    templates::*,
    AppState,
};

// ===== GET / =====


pub async fn register_get(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    if crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name)
        .await
        .is_some()
    {
        return Redirect::to("/hub").into_response();
    }
    RegisterTemplate {
        error: None,
        username: String::new(),
        email: String::new(),
    }
    .into_response()
}

// ===== POST /register =====
pub async fn register_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    let (username, email) = (form.username.clone(), form.email.clone());

    if let Err(e) = crate::models::validate_username(&form.username) {
        return RegisterTemplate {
            error: Some(e),
            username,
            email,
        }
        .into_response();
    }
    if let Err(e) = crate::models::validate_email(&form.email) {
        return RegisterTemplate {
            error: Some(e),
            username,
            email,
        }
        .into_response();
    }
    if let Some(c) = form.correo_interno.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Err(e) = crate::models::validate_email(c) {
            return RegisterTemplate {
                error: Some(e),
                username,
                email,
            }
            .into_response();
        }
    }
    if let Err(e) = crate::models::validate_password(&form.password) {
        return RegisterTemplate {
            error: Some(e),
            username,
            email,
        }
        .into_response();
    }
    if form.password != form.password_confirm {
        return RegisterTemplate {
            error: Some("Las contraseñas no coinciden.".into()),
            username,
            email,
        }
        .into_response();
    }
    // Términos y condiciones: el checkbox es obligatorio. Aceptamos los
    // valores típicos que envía un navegador ("1", "true", "on"). Si falta o
    // llega cualquier otra cosa, se rechaza.
    let accepted = match &form.accept_terms {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) => matches!(s.as_str(), "1" | "true" | "on" | "yes"),
        serde_json::Value::Number(n) => n.as_i64().map(|v| v != 0).unwrap_or(false),
        _ => false,
    };
    if !accepted {
        return RegisterTemplate {
            error: Some("Debes aceptar los Términos y la Política de Privacidad para crear tu cuenta.".into()),
            username,
            email,
        }
        .into_response();
    }

    if state
        .db
        .find_user_by_username(&form.username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return RegisterTemplate {
            error: Some("Ese nombre de usuario ya está en uso.".into()),
            username,
            email,
        }
        .into_response();
    }
    if state
        .db
        .find_user_by_email(&form.email)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return RegisterTemplate {
            error: Some("Ese email ya está registrado.".into()),
            username,
            email,
        }
        .into_response();
    }

    match state
        .db
        .create_user(&form.username, &form.email, &form.password)
        .await
    {
        Ok(uid) => {
            let _ = state
                .db
                .audit(Some(uid), "register", Some(form.username.as_str()), None)
                .await;
            if let Ok((token, expires)) = state.db.create_session(uid, false).await {
                set_session_cookie(&state, &cookies, &token, expires, false);
            }

            // Sección 6.3: si el usuario dio un correo interno válido, lo
            // vinculamos como primario NO verificado y lo mandamos a
            // /cuenta/correo para que lo verifique (flujo 6.2). Si no dio
            // correo o es inválido, flujo normal al /hub.
            let correo = form
                .correo_interno
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let redirect_target = match correo {
                Some(c) if crate::handlers::correo::validate_internal_email(&c) => {
                    if let Err(e) = state.db.add_unverified_user_email(uid, &c).await {
                        tracing::warn!("No se pudo vincular correo {c} al registrar: {e:#}");
                    } else {
                        let _ = state
                            .db
                            .audit(Some(uid), "email_linked_unverified", Some(&c), None)
                            .await;
                        tracing::info!("📧 Correo {c} vinculado (sin verificar) al registrar user {uid}");
                    }
                    "/cuenta/correo"
                }
                _ => "/hub",
            };
            Redirect::to(redirect_target).into_response()
        }
        Err(e) => {
            tracing::error!("Error registrando: {e}");
            RegisterTemplate {
                error: Some("Error al crear la cuenta. Intenta de nuevo.".into()),
                username,
                email,
            }
            .into_response()
        }
    }
}

// ===== GET /recover =====
pub async fn recover_get(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    if crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name)
        .await
        .is_some()
    {
        return Redirect::to("/hub").into_response();
    }
    RecoverTemplate {
        error: None,
        info: None,
        email: String::new(),
    }
    .into_response()
}

// ===== POST /recover =====
pub async fn recover_post(
    State(state): State<AppState>,
    Form(form): Form<RecoverForm>,
) -> impl IntoResponse {
    let email = form.email.trim().to_string();
    if let Err(e) = crate::models::validate_email(&email) {
        return RecoverTemplate {
            error: Some(e),
            info: None,
            email,
        }
        .into_response();
    }
    let info_msg = "Si el email existe, recibirás un enlace de recuperación.";

    let user = state.db.find_user_by_email(&email).await.ok().flatten();
    let Some(user) = user else {
        return RecoverTemplate {
            error: None,
            info: Some(info_msg.to_string()),
            email,
        }
        .into_response();
    };

    match state.db.create_recovery_token(user.id).await {
        Ok(token) => {
            let base = state.cfg.public_url.trim_end_matches('/');
            let link = format!("{base}/reset?token={token}");
            let cancel_link = format!("{base}/reset/cancel?token={token}");

            // Sección 6.4: si el usuario tiene correo interno verificado Y el
            // cliente mTLS contra AethraMail está disponible, mandamos el
            // enlace por correo. Si no (dev sin certs, o usuario sin correo
            // interno), fallback al modo dev previo: link visible inline.
            let internal_email = state.db.primary_verified_email(user.id).await.ok().flatten();

            if let (Some(client), Some(to)) =
                (state.aethramail.as_ref().as_ref(), &internal_email)
            {
                let body = format!(
                    "Recuperación de contraseña — Aethravox Studios\n\n\
                     Recibimos una solicitud para cambiar la contraseña de tu cuenta.\n\n\
                     ➜ Cambiar contraseña: {link}\n\
                     ➜ No fuiste tú? Anula el enlace: {cancel_link}\n\n\
                     El enlace expira en 30 minutos. Si no lo solicitaste, usa \
                     \"No soy yo\" para invalidarlo de inmediato.\n\n— Aethravox Studios"
                );
                match client
                    .send_email(to.as_str(), "Recuperación de contraseña — Aethravox", &body)
                    .await
                {
                    Ok(_) => tracing::info!("📧 Recovery link enviado a {to} vía AethraMail"),
                    Err(e) => tracing::warn!("No se pudo mandar recovery por AethraMail a {to}: {e:#}"),
                }
                RecoverTemplate {
                    error: None,
                    info: Some(format!(
                        "{info_msg} Te enviamos el enlace a tu correo interno {to}."
                    )),
                    email,
                }
                .into_response()
            } else {
                // Fallback dev: link visible inline.
                tracing::info!("📧 Recovery link para {} (dev): {}", user.email, link);
                RecoverTemplate {
                    error: None,
                    info: Some(format!(
                        "{} (Modo dev — link visible aquí: {})",
                        info_msg, link
                    )),
                    email,
                }
                .into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error creando token de recuperación: {e}");
            RecoverTemplate {
                error: Some("Error interno. Intenta más tarde.".into()),
                info: None,
                email,
            }
            .into_response()
        }
    }
}

// ===== GET /reset?token=xxx =====
#[derive(Deserialize)]
pub struct ResetQuery {
    #[serde(default)]
    pub token: String,
}

pub async fn reset_get(
    Query(q): Query<ResetQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if q.token.is_empty() {
        return ResetTemplate {
            token: String::new(),
            error: Some("Token inválido o ausente.".into()),
        }
        .into_response();
    }

    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM recovery_tokens WHERE token = ? AND used = 0 AND expires_at > ?",
    )
    .bind(&q.token)
    .bind(chrono::Utc::now())
    .fetch_optional(&state.db.pool)
    .await
    .ok()
    .flatten();

    if row.is_none() {
        return ResetTemplate {
            token: String::new(),
            error: Some("Token inválido, expirado o ya utilizado.".into()),
        }
        .into_response();
    }

    ResetTemplate {
        token: q.token,
        error: None,
    }
    .into_response()
}

// ===== POST /reset =====
pub async fn reset_post(
    State(state): State<AppState>,
    Form(form): Form<ResetForm>,
) -> impl IntoResponse {
    if form.password != form.password_confirm {
        return ResetTemplate {
            token: form.token.clone(),
            error: Some("Las contraseñas no coinciden.".into()),
        }
        .into_response();
    }
    if let Err(e) = crate::models::validate_password(&form.password) {
        return ResetTemplate {
            token: form.token,
            error: Some(e),
        }
        .into_response();
    }

    match state.db.consume_recovery_token(&form.token).await {
        Ok(Some(user_id)) => {
            if let Err(e) = state.db.update_password(user_id, &form.password).await {
                tracing::error!("Error actualizando password: {e}");
                return ResetTemplate {
                    token: String::new(),
                    error: Some("Error al cambiar la contraseña.".into()),
                }
                .into_response();
            }
            let _ = state
                .db
                .audit(Some(user_id), "password_reset", None, None)
                .await;
            Redirect::to("/login?reset=ok").into_response()
        }
        _ => ResetTemplate {
            token: String::new(),
            error: Some("Token inválido, expirado o ya utilizado.".into()),
        }
        .into_response(),
    }
}

// ===== GET /reset/cancel — "No soy yo" (Sección 6.5) =====
// Invalida el enlace de recuperación y muestra confirmación. Llega desde el
// botón "No soy yo" del correo mandado por AethraMail en 6.4.
pub async fn reset_cancel_get(
    Query(q): Query<ResetQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // consume_recovery_token invalida el token (used=1) y devuelve el user_id
    // si existía y estaba válido; None si ya estaba usado/expirado/inexistente.
    let res = state.db.consume_recovery_token(&q.token).await.ok().flatten();
    let cancelled = res.is_some();
    if let Some(uid) = res {
        let _ = state
            .db
            .audit(Some(uid), "reset_cancelled", Some("no-soy-yo"), None)
            .await;
    }
    ResetCancelTemplate {
        cancelled,
        info: if cancelled {
            "El enlace quedó anulado. Si fuiste tú, tu cuenta está segura.".to_string()
        } else {
            "Ese enlace ya no era válido o ya había sido usado.".to_string()
        },
    }
    .into_response()
}

// ===== Cookies helpers =====

pub fn set_session_cookie(
    state: &AppState,
    cookies: &Cookies,
    token: &str,
    _expires: chrono::DateTime<chrono::Utc>,
    _remember: bool,
) {
    let mut builder = tower_cookies::Cookie::build((
        state.cfg.cookie_name.clone(),
        token.to_string(),
    ))
    .path("/")
    .http_only(true)
    .same_site(tower_cookies::cookie::SameSite::Lax)
    // La cookie acompaña a la sesión: 1 año siempre (no caduca sola;
    // solo se borra al pulsar "Cerrar sesión"). El flag `remember` ya
    // no cambia el Max-Age.
    .max_age(time::Duration::days(365));
    if !state.cfg.cookie_domain.is_empty() {
        builder = builder.domain(state.cfg.cookie_domain.clone());
    }
    if state.cfg.production {
        builder = builder.secure(true);
    }
    cookies.add(builder.build());
}

pub fn clear_session_cookie(state: &AppState, cookies: &Cookies) {
    let mut builder = tower_cookies::Cookie::build((
        state.cfg.cookie_name.clone(),
        String::new(),
    ))
    .path("/")
    .http_only(true)
    .max_age(time::Duration::seconds(0));
    if !state.cfg.cookie_domain.is_empty() {
        builder = builder.domain(state.cfg.cookie_domain.clone());
    }
    cookies.add(builder.build());
}

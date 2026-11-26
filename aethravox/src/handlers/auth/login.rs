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

use super::account::{set_session_cookie, clear_session_cookie};

pub async fn index(State(state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    if crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name)
        .await
        .is_some()
    {
        return Redirect::to("/hub").into_response();
    }
    LoginTemplate {
        next: String::new(),
        error: None,
        info: None,
    }
    .into_response()
}

// ===== GET /privacidad y /terminos — páginas públicas (sin sesión) =====
// Rinden la misma plantilla legal.html con distinta pestaña activa. El enlace
// del checkbox del registro apunta aquí para que el usuario pueda leerlos
// antes de crear cuenta.
pub async fn legal_privacidad() -> impl IntoResponse {
    LegalTemplate {
        mode: "privacidad".into(),
    }
}
pub async fn legal_terminos() -> impl IntoResponse {
    LegalTemplate {
        mode: "terminos".into(),
    }
}

// ===== GET /login =====
#[derive(Deserialize)]
pub struct NextQuery {
    #[serde(default)]
    pub next: Option<String>,
}

pub async fn login_get(
    Query(q): Query<NextQuery>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    if crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name)
        .await
        .is_some()
    {
        return Redirect::to("/hub").into_response();
    }
    LoginTemplate {
        next: q.next.unwrap_or_default(),
        error: None,
        info: None,
    }
    .into_response()
}

// ===== POST /login =====
pub async fn login_post(
    State(state): State<AppState>,
    cookies: Cookies,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let next_safe = form
        .next
        .as_deref()
        .filter(|n| !n.is_empty() && n.starts_with('/'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/hub".to_string());
    let client_ip = get_client_ip(&headers);

    let user = match state.db.find_user_by_username(&form.username).await {
        Ok(Some(u)) => u,
        _ => {
            return LoginTemplate {
                next: next_safe,
                error: Some("Usuario no encontrado.".into()),
                info: None,
            }
            .into_response();
        }
    };

    if state.db.is_aexos_only_user(user.id).await {
        return LoginTemplate {
            next: next_safe,
            error: Some("Esta cuenta es exclusiva de aexOS y no tiene contraseña externa.".into()),
            info: None,
        }
        .into_response();
    }

    if !verify_password(&form.password, &user.password_hash) {
        let _ = state
            .db
            .audit(None, "login_failed", Some(user.username.as_str()), client_ip.as_deref())
            .await;
        return LoginTemplate {
            next: next_safe,
            error: Some("Contraseña incorrecta.".into()),
            info: None,
        }
        .into_response();
    }

    // CONTRASEÑA CORRECTA. Antes de crear sesión, comprobar si la cuenta está
    // baneada o suspendida. Si lo está, NO crear sesión y devolver pantalla de ban.
    // Esto evita que alguien con la contraseña correcta entre "y luego lo echen".
    if let Ok(Some(ban)) = state.db.is_user_banned(user.id).await {
        let _ = state.db.audit(
            Some(user.id),
            "login_blocked_banned_account",
            Some(&format!("username={} ban_id={}", user.username, ban.id)),
            client_ip.as_deref(),
        ).await;
        return crate::handlers::admin::ban_screen_response(
            crate::handlers::admin::BanKind::UserBan,
            ban.id,
            user.username.clone(),
            ban.reason.clone().unwrap_or_else(|| "No se especificó motivo.".into()),
            ban.created_at,
            ban.expires_at,
        ).into_response();
    }
    if let Ok(Some(susp)) = state.db.is_user_suspended(user.id).await {
        let _ = state.db.audit(
            Some(user.id),
            "login_blocked_suspended_account",
            Some(&format!("username={} suspension_id={} expires={}", user.username, susp.id, susp.expires_at)),
            client_ip.as_deref(),
        ).await;
        return crate::handlers::admin::ban_screen_response(
            crate::handlers::admin::BanKind::UserSuspension,
            susp.id,
            user.username.clone(),
            susp.reason.clone().unwrap_or_else(|| "No se especificó motivo.".into()),
            susp.created_at,
            Some(susp.expires_at),
        ).into_response();
    }

    let remember = form.remember.is_some();
    let (token, expires) = match state.db.create_session(user.id, remember).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Error creando sesión: {e}");
            return LoginTemplate {
                next: next_safe,
                error: Some("Error interno. Intenta de nuevo.".into()),
                info: None,
            }
            .into_response();
        }
    };

    // Vinculación de dispositivos (V4.5): si el usuario ya tenía una sesión
    // PRIMARY abierta en otro dispositivo, la cerramos. Solo puede haber una
    // primary por cuenta — para tener varios dispositivos activos a la vez
    // hay que VINCULARLOS (sin contraseña, vía /vincular), no volver a
    // loguearse. Las sesiones vinculadas (is_primary=0) NO se tocan.
    if let Err(e) = state.db.revoke_other_primary_sessions(user.id, &token).await {
        tracing::warn!("No se pudo limpiar primary anterior de {}: {e}", user.username);
    }

    let _ = state.db.touch_last_login(user.id).await;

    // === Registrar IP en historial de logins ===
    // Si es una IP NUEVA para este usuario, audit con action="login_new_ip" para
    // alertar al admin en /admin/audit. Si es conocida, audit normal "login".
    if let Some(ip) = client_ip.as_deref() {
        match state.db.record_login_ip(user.id, ip).await {
            Ok(true) => {
                // IP nueva para este usuario — login potencialmente sospechoso.
                let _ = state.db.audit(
                    Some(user.id),
                    "login_new_ip",
                    Some(&format!("ip={} (primera vez desde esta IP)", ip)),
                    Some(ip),
                ).await;
            }
            Ok(false) => {
                // IP conocida — login normal.
                let _ = state.db.audit(Some(user.id), "login", None, Some(ip)).await;
            }
            Err(e) => {
                tracing::error!("Error registrando IP de login: {e}");
                let _ = state.db.audit(Some(user.id), "login", None, Some(ip)).await;
            }
        }
    } else {
        // Sin header X-Forwarded-For/X-Real-IP (modo dev sin proxy).
        let _ = state.db.audit(Some(user.id), "login", None, None).await;
    }

    set_session_cookie(&state, &cookies, &token, expires, remember);

    Redirect::to(&next_safe).into_response()
}

// ===== POST /logout =====
pub async fn logout(State(state): State<AppState>, cookies: Cookies) -> impl IntoResponse {
    if let Some(c) = cookies.get(&state.cfg.cookie_name) {
        let _ = state.db.delete_session(c.value()).await;
        let _ = state.db.audit(None, "logout", None, None).await;
    }
    clear_session_cookie(&state, &cookies);
    Redirect::to("/")
}

// ===== GET /register =====

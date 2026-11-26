// Sección 6.2 — Ajustes → Cuenta → Correo.
// Página dedicada /cuenta/correo (enlazada desde la pestaña "Cuenta" del
// dashboard) para vincular un correo interno de AethraMail a la cuenta del
// hub, con verificación de código de 8 dígitos enviado por mTLS.

use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use rand::RngCore;

use crate::{auth_mw::AuthUser, db::emails::UserEmail, AppState};

// ===== Template =====
#[derive(Template)]
#[template(path = "servicios/aethramail/correo.html")]
pub struct CorreoTemplate {
    pub username: String,
    pub emails: Vec<UserEmailRow>,
    /// Correo pendiente de verificar (se acaba de mandar el código). Cuando
    /// es Some, la plantilla muestra el formulario para teclear el código.
    pub pending_email: Option<String>,
    pub info: Option<String>,
    pub error: Option<String>,
    pub aethramail_url: Option<String>,
}

pub struct UserEmailRow {
    pub id: i64,
    pub email: String,
    pub verified: bool,
    pub is_primary: bool,
}

fn row_from(e: &UserEmail) -> UserEmailRow {
    UserEmailRow {
        id: e.id,
        email: e.email.clone(),
        verified: e.is_verified(),
        is_primary: e.is_primary != 0,
    }
}

/// Resuelve la URL web externa de AethraMail (si está configurada en aris_config,
/// en la tabla webs slug='correo' con http(s)://, o en env AETHRAMAIL_WEB_URL).
pub async fn resolve_aethramail_url(state: &AppState) -> Option<String> {
    if let Ok(Some(cfg_url)) = state.db.aris_get_config("aethramail_url").await {
        let trimmed = cfg_url.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(Some(web)) = state.db.get_web_by_slug("correo").await {
        let u = web.url.trim();
        if u.starts_with("http://") || u.starts_with("https://") {
            return Some(u.to_string());
        }
    }
    if let Ok(env_url) = std::env::var("AETHRAMAIL_WEB_URL") {
        let trimmed = env_url.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Carga la lista de correos del usuario + renderiza la página.
async fn render_correo(
    state: &AppState,
    user_id: i64,
    username: &str,
    pending_email: Option<String>,
    info: Option<String>,
    error: Option<String>,
) -> Response {
    let emails = state.db.list_user_emails(user_id).await.unwrap_or_default();
    let aethramail_url = resolve_aethramail_url(state).await;
    CorreoTemplate {
        username: username.to_string(),
        emails: emails.iter().map(row_from).collect(),
        pending_email,
        info,
        error,
        aethramail_url,
    }
    .into_response()
}

// ===== GET /correo — Redirección directa a la web externa de AethraMail (o /cuenta/correo) =====
pub async fn correo_redirect(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(url) = resolve_aethramail_url(&state).await {
        return Redirect::temporary(&url);
    }
    Redirect::to("/cuenta/correo")
}

// ===== GET /cuenta/correo =====
pub async fn correo_get(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    render_correo(&state, user.id, &user.username, None, None, None).await
}

// ===== Formularios =====
#[derive(serde::Deserialize)]
pub struct SolicitarForm {
    pub email: String,
}

#[derive(serde::Deserialize)]
pub struct VerificarForm {
    pub email: String,
    pub code: String,
}

#[derive(serde::Deserialize)]
pub struct EmailIdForm {
    pub email_id: i64,
}

/// Valida formato mínimo de correo interno: un @, local y dominio no vacíos,
/// sin espacios. Pública para reusarla en 6.3 (registro).
pub fn validate_internal_email(email: &str) -> bool {
    crate::models::validate_email(email).is_ok()
}

/// Genera un código de 8 dígitos (0..99.999.999, rellenado con ceros).
fn gen_code() -> String {
    let mut buf = [0u8; 4];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    let n = u32::from_le_bytes(buf) % 100_000_000;
    format!("{n:08}")
}

// ===== POST /cuenta/correo/solicitar =====
// El usuario pide vincular un correo interno. Validamos formato, generamos el
// código de 8 dígitos, lo guardamos en email_change_requests (expira 15 min) y
// lo mandamos por AethraMail (mTLS). Si el envío falla (correo inexistente,
// cliente no disponible), lo decimos sin filtrar el motivo interno.
pub async fn correo_solicitar_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<SolicitarForm>,
) -> impl IntoResponse {
    let email = form.email.trim().to_string();

    if let Err(err_msg) = crate::models::validate_email(&email) {
        return render_correo(
            &state, user.id, &user.username, None, None,
            Some(err_msg),
        ).await;
    }

    // ¿Ya lo tiene vinculado y verificado? Si existe pero NO verificado (p.ej.
    // correo dado al registrarse en 6.3), dejamos re-enviar el código.
    if let Ok(true) = state.db.user_email_exists(user.id, &email).await {
        if let Ok(true) = state.db.user_email_is_verified(user.id, &email).await {
            return render_correo(
                &state, user.id, &user.username, None, None,
                Some("Ese correo ya está vinculado y verificado.".into()),
            ).await;
        }
    }

    let Some(client) = state.aethramail.as_ref().as_ref() else {
        return render_correo(
            &state, user.id, &user.username, None, None,
            Some("El servicio de correo interno no está disponible ahora mismo.".into()),
        ).await;
    };

    let code = gen_code();
    let expires = chrono::Utc::now() + chrono::Duration::minutes(15);
    if let Err(e) = state
        .db
        .create_email_change_request(user.id, &email, &code, expires)
        .await
    {
        tracing::error!("No se pudo crear email_change_request: {e}");
        return render_correo(
            &state, user.id, &user.username, None, None,
            Some("Error interno guardando la solicitud. Intenta más tarde.".into()),
        ).await;
    }

    let body = format!(
        "Verificación de correo — Aethravox Studios\n\n\
         Estás vinculando el correo interno {email} a tu cuenta de Aethravox ({user}).\n\n\
         Tu código de verificación (8 dígitos):\n\n    {code}\n\n\
         Introdúcelo en la página de Correo (Ajustes → Cuenta) para confirmar.\n\
         El código expira en 15 minutos.\n\n\
         Si no fuiste tú, ignora este correo.\n\n— Aethravox Studios",
        user = user.username
    );

    match client
        .send_email(&email, "Código de verificación — Aethravox", &body)
        .await
    {
        Ok(_) => {
            tracing::info!("📧 Código de verificación enviado a {email} vía AethraMail");
            render_correo(
                &state, user.id, &user.username, Some(email),
                Some("Te enviamos un código de 8 dígitos a ese correo. Introúcelo abajo.".into()),
                None,
            ).await
        }
        Err(e) => {
            tracing::warn!("No se pudo mandar código de verificación a {email}: {e:#}");
            render_correo(
                &state, user.id, &user.username, None, None,
                Some("No se pudo enviar el código. ¿Existe ese correo interno en AethraMail?".into()),
            ).await
        }
    }
}

// ===== POST /cuenta/correo/verificar =====
// El usuario teclea el código. Si es válido y no expirado, consumimos la
// solicitud e insertamos el correo como verificado+primario.
pub async fn correo_verificar_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<VerificarForm>,
) -> impl IntoResponse {
    let email = form.email.trim().to_string();
    let code = form.code.trim().to_string();

    if code.len() != 8 || !code.chars().all(|c| c.is_ascii_digit()) {
        return render_correo(
            &state, user.id, &user.username, Some(email), None,
            Some("El código debe ser 8 dígitos.".into()),
        ).await;
    }

    match state.db.find_valid_email_change(user.id, &code).await {
        Ok(Some((req_id, new_email))) => {
            // El correo de la solicitud debe coincidir con el que el usuario
            // tenía pendiente (defensa contra mezclar códigos).
            if new_email != email {
                return render_correo(
                    &state, user.id, &user.username, Some(email), None,
                    Some("Ese código no corresponde a este correo.".into()),
                ).await;
            }
            let _ = state.db.consume_email_change(req_id).await;
            if let Err(e) = state.db.add_verified_user_email(user.id, &email).await {
                tracing::error!("No se pudo insertar user_email verificado: {e}");
                return render_correo(
                    &state, user.id, &user.username, None, None,
                    Some("Error interno al vincular el correo.".into()),
                ).await;
            }
            let _ = state
                .db
                .audit(Some(user.id), "email_linked", Some(&email), None)
                .await;
            tracing::info!("📧 Correo {email} vinculado+verificado para user {}", user.id);
            render_correo(
                &state, user.id, &user.username, None,
                Some(format!("Correo {email} vinculado y verificado.")),
                None,
            ).await
        }
        Ok(None) => render_correo(
            &state, user.id, &user.username, Some(email), None,
            Some("Código incorrecto, expirado o ya utilizado.".into()),
        ).await,
        Err(e) => {
            tracing::error!("find_valid_email_change falló: {e}");
            render_correo(
                &state, user.id, &user.username, Some(email), None,
                Some("Error interno verificando el código.".into()),
            ).await
        }
    }
}

// ===== POST /cuenta/correo/primary =====
pub async fn correo_primary_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<EmailIdForm>,
) -> impl IntoResponse {
    let _ = state.db.set_primary_user_email(user.id, form.email_id).await;
    Redirect::to("/cuenta/correo")
}

// ===== POST /cuenta/correo/remove =====
pub async fn correo_remove_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<EmailIdForm>,
) -> impl IntoResponse {
    let _ = state.db.remove_user_email(user.id, form.email_id).await;
    Redirect::to("/cuenta/correo")
}

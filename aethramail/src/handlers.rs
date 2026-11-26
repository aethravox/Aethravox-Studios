use axum::{
    extract::{Path, Query, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
    Form,
    http::StatusCode,
};
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    auth::{current_account, hash_password, verify_password},
    models::{
        count_unread, get_email, insert_email, list_emails, mark_read, EmailAccount,
    },
    templates::{BandejaTemplate, ConfiguracionTemplate, LeerTemplate, RedactarTemplate},
    AppState,
};

/// Middleware: exige sesión activa. Si no → redirect a /login.
/// Si sí → inserta la cuenta en las extensiones para que los handlers la lean.
pub async fn require_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut req: axum::extract::Request,
    next: Next,
) -> Response {
    match current_account(&state, &cookies).await {
        Some(acc) => {
            req.extensions_mut().insert(acc);
            next.run(req).await
        }
        None => Redirect::to("/login").into_response(),
    }
}

async fn unread_of(state: &AppState, acc: &EmailAccount) -> i64 {
    count_unread(&state.db.pool, &acc.full()).await.unwrap_or(0)
}

// ── Raíz ────────────────────────────────────────────────────────────
pub async fn index() -> impl IntoResponse {
    Redirect::to("/bandeja?folder=inbox")
}

// ── Bandeja ─────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct BandejaQuery {
    pub folder: Option<String>,
}

pub async fn bandeja(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
    Query(q): Query<BandejaQuery>,
) -> impl IntoResponse {
    let folder = q.folder.unwrap_or_else(|| "inbox".into());
    // Solo permitimos estos folders para evitar inyección / rutas raras.
    let folder = match folder.as_str() {
        "inbox" | "sent" | "drafts" | "trash" => folder,
        _ => "inbox".to_string(),
    };
    let emails = list_emails(&state.db.pool, &acc.full(), &folder)
        .await
        .unwrap_or_default();
    BandejaTemplate {
        account_full: acc.full(),
        unread: unread_of(&state, &acc).await,
        folder: folder.clone(),
        emails,
    }
}

// ── Leer ────────────────────────────────────────────────────────────
pub async fn leer(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
    Path(id): Path<i64>,
) -> Response {
    let email = match get_email(&state.db.pool, id).await.ok().flatten() {
        Some(e) => e,
        None => return (StatusCode::NOT_FOUND, "Correo no encontrado").into_response(),
    };
    // Solo el destinatario o el remitente pueden leerlo.
    let mine = email.to_address == acc.full() || email.from_address == acc.full();
    if !mine {
        return (StatusCode::FORBIDDEN, "No tienes acceso a este correo").into_response();
    }
    if email.folder == "inbox" && email.to_address == acc.full() {
        let _ = mark_read(&state.db.pool, id).await;
    }
    LeerTemplate {
        account_full: acc.full(),
        unread: unread_of(&state, &acc).await,
        email,
    }
    .into_response()
}

// ── Redactar ────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct RedactarQuery {
    pub to: Option<String>,
}

pub async fn redactar_get(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
    Query(q): Query<RedactarQuery>,
) -> impl IntoResponse {
    RedactarTemplate {
        account_full: acc.full(),
        unread: unread_of(&state, &acc).await,
        to: q.to.unwrap_or_default(),
        error: None,
    }
}

// ── Enviar ──────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct ComposeForm {
    pub to: String,
    pub subject: String,
    pub body: String,
}

pub async fn enviar_post(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
    Form(form): Form<ComposeForm>,
) -> Response {
    let to = form.to.trim().to_lowercase();
    let subject = form.subject.trim();
    let body = form.body.trim();
    let from = acc.full();

    let err = |to: String, msg: &str| {
        RedactarTemplate {
            account_full: from.clone(),
            unread: 0,
            to,
            error: Some(msg.into()),
        }
        .into_response()
    };

    let (to_local, to_domain) = match crate::auth::normalize_full(&to) {
        Some(v) => v,
        None => return err(to, "Destinatario inválido (usa usuario@dominio)."),
    };
    if !state.cfg.is_allowed_domain(&to_domain) {
        return err(to, "El dominio del destinatario no es interno.");
    }
    // 5.9 — solo se aceptan destinatarios que existen en la DB de AethraMail.
    let exists = crate::models::account_exists(&state.db.pool, &to_local, &to_domain)
        .await
        .unwrap_or(false);
    if !exists {
        return err(to, "El destinatario no existe en AethraMail.");
    }
    if subject.is_empty() && body.is_empty() {
        return err(to, "Asunto o cuerpo no pueden estar vacíos.");
    }

    let to_full = format!("{}@{}", to_local, to_domain);
    // Fila en INBOX del destinatario.
    if let Err(e) = insert_email(&state.db.pool, &from, &to_full, subject, body, "inbox").await {
        tracing::error!("insert inbox: {e}");
        return err(to, "No se pudo enviar el correo.");
    }
    // Fila en SENT del remitente.
    let _ = insert_email(&state.db.pool, &from, &to_full, subject, body, "sent").await;

    Redirect::to("/bandeja?folder=inbox").into_response()
}

// ── Configuración ───────────────────────────────────────────────────
pub async fn config_get(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
) -> impl IntoResponse {
    ConfiguracionTemplate {
        account_full: acc.full(),
        address: acc.address.clone(),
        domain: acc.domain.clone(),
        created_at: acc.created_at.clone(),
        unread: unread_of(&state, &acc).await,
        info: None,
        error: None,
    }
}

#[derive(Deserialize)]
pub struct ChangePasswordForm {
    pub current: String,
    pub new: String,
    pub confirm: String,
}

pub async fn config_password_post(
    State(state): State<AppState>,
    Extension(acc): Extension<EmailAccount>,
    Form(form): Form<ChangePasswordForm>,
) -> impl IntoResponse {
    let unread = unread_of(&state, &acc).await;
    let fail = |msg: &str| {
        ConfiguracionTemplate {
            account_full: acc.full(),
            address: acc.address.clone(),
            domain: acc.domain.clone(),
            created_at: acc.created_at.clone(),
            unread,
            info: None,
            error: Some(msg.into()),
        }
    };

    if !verify_password(&form.current, &acc.password_hash) {
        return fail("La contraseña actual no es correcta.");
    }
    if form.new.len() < 8 {
        return fail("La nueva contraseña debe tener al menos 8 caracteres.");
    }
    if form.new != form.confirm {
        return fail("Las contraseñas nuevas no coinciden.");
    }
    let hash = match hash_password(&form.new) {
        Ok(h) => h,
        Err(_) => return fail("Error interno al guardar."),
    };
    let res = sqlx::query("UPDATE email_accounts SET password_hash = ? WHERE id = ?")
        .bind(&hash)
        .bind(acc.id)
        .execute(&state.db.pool)
        .await;
    match res {
        Ok(_) => ConfiguracionTemplate {
            account_full: acc.full(),
            address: acc.address.clone(),
            domain: acc.domain.clone(),
            created_at: acc.created_at.clone(),
            unread,
            info: Some("Contraseña actualizada.".into()),
            error: None,
        },
        Err(_) => fail("No se pudo actualizar la contraseña."),
    }
}

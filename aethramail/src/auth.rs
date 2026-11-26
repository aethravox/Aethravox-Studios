use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::{
    extract::State,
    Form,
    response::{IntoResponse, Redirect},
};
use rand::RngCore;
use serde::Deserialize;
use tower_cookies::{cookie::SameSite, Cookies};
use tracing::warn;

use crate::{
    models::{create_account, find_account_by_full, find_account_by_id, split_address, EmailAccount},
    templates::{LoginTemplate, RegisterTemplate},
    AppState,
};

const SESSION_TTL_SECS: i64 = 60 * 60 * 24 * 7; // 7 días

/// Hash argon2 (mismo esquema que el hub).
pub fn hash_password(pw: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("argon2 hash: {e}"))?;
    Ok(hash.to_string())
}

pub fn verify_password(pw: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .and_then(|parsed| Argon2::default().verify_password(pw.as_bytes(), &parsed))
        .is_ok()
}

pub fn new_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Resuelve la cuenta logueada desde la cookie de sesión, o None.
pub async fn current_account(state: &AppState, cookies: &Cookies) -> Option<EmailAccount> {
    let value = cookies.get(&state.cfg.cookie_name)?.value().to_string();
    let id = {
        let sessions = state.sessions.lock().await;
        sessions.get(&value).copied()?
    };
    find_account_by_id(&state.db.pool, id).await.ok().flatten()
}

// ── Login ───────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct LoginForm {
    pub address: String,
    pub domain: String,
    pub password: String,
}

pub async fn login_get(State(state): State<AppState>) -> impl IntoResponse {
    LoginTemplate {
        domains: state.cfg.domains.clone(),
        error: None,
        address: String::new(),
        domain: state.cfg.domains.first().cloned().unwrap_or_default(),
    }
}

pub async fn login_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let address = form.address.trim().to_lowercase();
    let domain = form.domain.trim().to_lowercase();

    let acc = find_account_by_full(&state.db.pool, &address, &domain)
        .await
        .ok()
        .flatten();

    let ok = match &acc {
        Some(a) => verify_password(&form.password, &a.password_hash),
        None => false,
    };

    if !ok {
        return LoginTemplate {
            domains: state.cfg.domains.clone(),
            error: Some("Dirección o contraseña incorrectas.".into()),
            address,
            domain,
        }
        .into_response();
    }

    let acc = acc.unwrap();
    set_session(&state, &cookies, acc.id).await;
    Redirect::to("/bandeja").into_response()
}

// ── Register ────────────────────────────────────────────────────────
#[derive(Deserialize)]
pub struct RegisterForm {
    pub address: String,
    pub domain: String,
    pub password: String,
    pub confirm: String,
}

pub async fn register_get(State(state): State<AppState>) -> impl IntoResponse {
    RegisterTemplate {
        domains: state.cfg.domains.clone(),
        error: None,
        address: String::new(),
        domain: state.cfg.domains.first().cloned().unwrap_or_default(),
    }
}

pub async fn register_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    let address = form.address.trim().to_lowercase();
    let domain = form.domain.trim().to_lowercase();

    let tpl_err = |msg: &str, addr: String, dom: String| {
        RegisterTemplate {
            domains: state.cfg.domains.clone(),
            error: Some(msg.into()),
            address: addr,
            domain: dom,
        }
        .into_response()
    };

    if address.is_empty() || address.len() > 32 {
        return tpl_err("El nombre de usuario debe tener entre 1 y 32 caracteres.", address, domain);
    }
    if !address.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-') {
        return tpl_err("El nombre solo admite letras, números, '.', '_' y '-'.", address, domain);
    }
    if !state.cfg.is_allowed_domain(&domain) {
        return tpl_err("Dominio no permitido.", address, domain);
    }
    if form.password.len() < 8 {
        return tpl_err("La contraseña debe tener al menos 8 caracteres.", address, domain);
    }
    if form.password != form.confirm {
        return tpl_err("Las contraseñas no coinciden.", address, domain);
    }
    if find_account_by_full(&state.db.pool, &address, &domain)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return tpl_err("Esa dirección ya existe.", address, domain);
    }

    let hash = match hash_password(&form.password) {
        Ok(h) => h,
        Err(e) => {
            warn!("hash_password: {e}");
            return tpl_err("Error interno al crear la cuenta.", address, domain);
        }
    };
    if let Err(e) = create_account(&state.db.pool, &address, &domain, &hash).await {
        warn!("create_account: {e}");
        return tpl_err("Error al crear la cuenta.", address, domain);
    }

    let acc = match find_account_by_full(&state.db.pool, &address, &domain)
        .await
        .ok()
        .flatten()
    {
        Some(a) => a,
        None => return tpl_err("Error inesperado tras crear la cuenta.", address, domain),
    };
    set_session(&state, &cookies, acc.id).await;
    Redirect::to("/bandeja").into_response()
}

// ── Logout ──────────────────────────────────────────────────────────
pub async fn logout_get(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    if let Some(c) = cookies.get(&state.cfg.cookie_name) {
        let v = c.value().to_string();
        state.sessions.lock().await.remove(&v);
    }
    clear_session_cookie(&state, &cookies);
    Redirect::to("/login")
}

// ── Cookie helpers ──────────────────────────────────────────────────
pub async fn set_session(state: &AppState, cookies: &Cookies, account_id: i64) {
    let token = new_session_token();
    state
        .sessions
        .lock()
        .await
        .insert(token.clone(), account_id);

    let cookie = tower_cookies::Cookie::build((state.cfg.cookie_name.clone(), token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(SESSION_TTL_SECS))
        .build();
    cookies.add(cookie);
}

pub fn clear_session_cookie(state: &AppState, cookies: &Cookies) {
    cookies.remove(
        tower_cookies::Cookie::build(state.cfg.cookie_name.clone())
            .path("/")
            .build(),
    );
}

/// Normaliza una dirección completa a (local, domain) en minúsculas.
pub fn normalize_full(full: &str) -> Option<(String, String)> {
    let (l, d) = split_address(full)?;
    Some((l.to_lowercase(), d.to_lowercase()))
}

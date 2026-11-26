#![allow(dead_code)]

mod auth;
mod config;
mod db;
mod handlers;
mod internal_api;
mod models;
mod templates;
mod tls;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub cfg: config::Config,
    pub db: db::Db,
    /// Sesiones en memoria: token → account_id.
    /// Al reiniciar el binario las sesiones caducan (los usuarios re-loguean).
    pub sessions: Arc<Mutex<HashMap<String, i64>>>,
    /// Rate limiter de la API interna (5.10).
    pub rate_limiter: Arc<internal_api::RateLimiter>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let cfg = config::Config::from_env();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cfg.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    if cfg.domains.is_empty() {
        anyhow::bail!("AETHRAMAIL_DOMAINS vacío — definí al menos un dominio interno");
    }

    tracing::info!("📧 AethraMail arrancando");
    tracing::info!("   dominios: {}", cfg.domains.join(", "));
    tracing::info!("   webmail UI:  http://{}", cfg.http_addr);
    tracing::info!("   API interna: https://{} (mTLS)", cfg.api_addr);

    let db = db::Db::connect(&cfg.db_path).await?;
    let state = AppState {
        cfg: cfg.clone(),
        db,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        rate_limiter: Arc::new(internal_api::RateLimiter::new(cfg.api_rate_per_min)),
    };

    // ── Webmail UI (HTTP, navegador) ────────────────────────────────
    let public = Router::new()
        .route("/login", get(auth::login_get).post(auth::login_post))
        .route("/register", get(auth::register_get).post(auth::register_post))
        .route("/logout", get(auth::logout_get));

    let protected = Router::new()
        .route("/", get(handlers::index))
        .route("/bandeja", get(handlers::bandeja))
        .route("/leer/:id", get(handlers::leer))
        .route("/redactar", get(handlers::redactar_get))
        .route("/enviar", post(handlers::enviar_post))
        .route("/configuracion", get(handlers::config_get))
        .route("/configuracion/password", post(handlers::config_password_post))
        .route_layer(from_fn_with_state(state.clone(), handlers::require_auth));

    let webmail = Router::new()
        .merge(public)
        .merge(protected)
        .nest_service("/static", ServeDir::new("static"))
        .layer(CookieManagerLayer::new())
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // 5 MiB por formulario
        .with_state(state.clone());

    // ── API interna (HTTPS + mTLS, solo el hub) ─────────────────────
    let tls_config = tls::build_mtls_server_config(
        &cfg.tls_server_cert,
        &cfg.tls_server_key,
        &cfg.tls_client_ca,
    )?;
    let api_addr: SocketAddr = cfg.api_addr.parse()?;
    let internal = internal_api::internal_router(state.clone());

    tokio::spawn(async move {
        if let Err(e) = tls::serve_mtls(internal, api_addr, tls_config).await {
            tracing::error!("API interna terminó con error: {e}");
        }
    });
    tracing::info!("   API interna escuchando (mTLS requerido)");

    // ── Sirve el webmail (bloquea) ──────────────────────────────────
    let http_addr: SocketAddr = cfg.http_addr.parse()?;
    let http_listener = TcpListener::bind(http_addr).await?;
    tracing::info!("✅ AethraMail listo. Abrí http://{}", cfg.http_addr);
    axum::serve(http_listener, webmail.into_make_service()).await?;
    Ok(())
}

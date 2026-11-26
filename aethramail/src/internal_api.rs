use std::collections::HashMap;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

use crate::{
    auth::normalize_full,
    models::{account_exists, insert_email},
    AppState,
};

/// Rate limiter por clave (from_address) — ventana fija de 60s (5.10).
pub struct RateLimiter {
    limit: u32,
    map: Mutex<HashMap<String, Bucket>>,
}

struct Bucket {
    count: u32,
    window_start: i64,
}

impl RateLimiter {
    pub fn new(limit: u32) -> Self {
        Self {
            limit,
            map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn allow(&self, key: &str) -> bool {
        let now = now_secs();
        let mut map = self.map.lock().await;
        let b = map.entry(key.to_string()).or_insert(Bucket {
            count: 0,
            window_start: now,
        });
        if now - b.window_start >= 60 {
            b.count = 0;
            b.window_start = now;
        }
        b.count += 1;
        b.count <= self.limit
    }
}

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── POST /api/internal/send ─────────────────────────────────────────
#[derive(Deserialize)]
pub struct SendBody {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct SendResp {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SendResp {
    fn ok(id: i64) -> (axum::http::StatusCode, Json<Self>) {
        (
            axum::http::StatusCode::OK,
            Json(Self { ok: true, id: Some(id), error: None }),
        )
    }
    fn err(status: axum::http::StatusCode, msg: &str) -> (axum::http::StatusCode, Json<Self>) {
        (
            status,
            Json(Self { ok: false, id: None, error: Some(msg.into()) }),
        )
    }
}

pub async fn send(
    State(state): State<AppState>,
    Json(body): Json<SendBody>,
) -> (axum::http::StatusCode, Json<SendResp>) {
    let from = body.from.trim();
    let to = body.to.trim();

    // Validación del `from`: debe ser una dirección interna bien formada
    // (no exigimos que exista como cuenta — el hub usa remitentes de sistema
    // como "noreply@aethra.sac").
    let (from_local, from_domain) = match normalize_full(from) {
        Some(v) => v,
        None => return SendResp::err(axum::http::StatusCode::BAD_REQUEST, "`from` inválido (usa usuario@dominio)."),
    };
    if !state.cfg.is_allowed_domain(&from_domain) {
        return SendResp::err(axum::http::StatusCode::BAD_REQUEST, "`from` usa un dominio no interno.");
    }

    // Rate limit estricto por remitente (5.10).
    let rl_key = format!("{from_local}@{from_domain}");
    if !state.rate_limiter.allow(&rl_key).await {
        warn!("rate limit excedido para {}", rl_key);
        return SendResp::err(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "Rate limit excedido. Reintenta en un minuto.",
        );
    }

    // Validación de destinatario (5.9): solo se acepta si existe en la DB.
    let (to_local, to_domain) = match normalize_full(to) {
        Some(v) => v,
        None => return SendResp::err(axum::http::StatusCode::UNPROCESSABLE_ENTITY, "`to` inválido."),
    };
    if !state.cfg.is_allowed_domain(&to_domain) {
        return SendResp::err(axum::http::StatusCode::UNPROCESSABLE_ENTITY, "`to` no es un dominio interno.");
    }
    let exists = account_exists(&state.db.pool, &to_local, &to_domain)
        .await
        .unwrap_or(false);
    if !exists {
        return SendResp::err(axum::http::StatusCode::NOT_FOUND, "Destinatario inexistente.");
    }

    let from_full = format!("{from_local}@{from_domain}");
    let to_full = format!("{to_local}@{to_domain}");

    match insert_email(&state.db.pool, &from_full, &to_full, &body.subject, &body.body, "inbox").await {
        Ok(id) => SendResp::ok(id),
        Err(e) => {
            warn!("insert_email (internal): {e}");
            SendResp::err(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "No se pudo guardar el correo.")
        }
    }
}

/// Construye el router de la API interna (mTLS).
pub fn internal_router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/api/internal/send", axum::routing::post(send))
        .with_state(state)
}

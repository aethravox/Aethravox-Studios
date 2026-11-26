//! Handlers de Web Push: exponer la clave pública VAPID, guardar/borrar
//! suscripciones, y servir el Service Worker en `/sw.js` (con scope `/`).
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth_mw::AuthUser;
use crate::AppState;

/// Respuesta de `GET /api/notifications/vapid-public-key`.
#[derive(Serialize)]
pub struct VapidPublicKeyResp {
    pub public_key: String,
    pub supported: bool,
}

/// `GET /api/notifications/vapid-public-key` → clave pública VAPID en
/// base64url (lista para `applicationServerKey` del navegador). 503 si el
/// PushManager no arrancó (falta config / error de claves).
pub async fn vapid_public_key(State(state): State<AppState>) -> Response {
    match &*state.push {
        Some(pm) => Json(VapidPublicKeyResp {
            public_key: pm.public_key_b64.clone(),
            supported: true,
        })
        .into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(VapidPublicKeyResp {
                public_key: String::new(),
                supported: false,
            }),
        )
            .into_response(),
    }
}

/// Body de `POST /api/notifications/subscribe`. Mapea el `pushSubscription`
/// que entrega el navegador (formato estándar Web Push).
#[derive(Deserialize)]
pub struct SubscribeBody {
    pub endpoint: String,
    pub keys: SubscribeKeys,
}
#[derive(Deserialize)]
pub struct SubscribeKeys {
    pub p256dh: String,
    pub auth: String,
}

/// `POST /api/notifications/subscribe` — guarda la suscripción del usuario
/// que hace la petición (AuthUser). Si el PushManager no está disponible,
/// responde 503 para que el frontend no siga reintentando.
pub async fn subscribe(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<SubscribeBody>,
) -> Response {
    if state.push.is_none() {
        return (StatusCode::SERVICE_UNAVAILABLE, "push no disponible").into_response();
    }
    match state
        .db
        .add_push_subscription(user.id, &body.endpoint, &body.keys.p256dh, &body.keys.auth)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::warn!("add_push_subscription: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "no se pudo guardar").into_response()
        }
    }
}

/// Body de `POST /api/notifications/unsubscribe`.
#[derive(Deserialize)]
pub struct UnsubscribeBody {
    pub endpoint: String,
}

/// `POST /api/notifications/unsubscribe` — borra la suscripción por endpoint.
/// Sin auth estricta: el endpoint es un secreto opaco de por sí; si alguien
/// lo tiene, ya podría deregistrarla ante el push service.
pub async fn unsubscribe(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(body): Json<UnsubscribeBody>,
) -> Response {
    let _ = state.db.remove_push_subscription(&body.endpoint).await;
    StatusCode::NO_CONTENT.into_response()
}

/// `GET /sw.js` — sirve el Service Worker con headers correctos:
/// - `Content-Type: application/javascript` (sin él el navegador rechaza el
///   registro).
/// - `Cache-Control: no-cache` para que las actualizaciones del SW se pinten
///   en cuanto se publique una versión nueva (el SW se cachea agresivamente
///   por defecto y eso dificulta desplegar cambios).
/// - `Service-Worker-Allowed: /` para permitir scope `/` aunque el fichero
///   viva bajo `/sw.js` (ya es root, pero el header es buena práctica).
///
/// El fichero físico está en `static/sw.js`. Lo leemos en cada petición (es
/// pequeño) para no tener que embeberlo ni gestionar versiones a mano.
pub async fn serve_sw() -> Response {
    let bytes = match std::fs::read("static/sw.js") {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("No se pudo leer static/sw.js: {e}");
            return (StatusCode::NOT_FOUND, "service worker no encontrado").into_response();
        }
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    headers.insert(
        "Service-Worker-Allowed",
        HeaderValue::from_static("/"),
    );
    (StatusCode::OK, headers, bytes).into_response()
}

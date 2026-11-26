//! Middleware de detección de cliente (web vs nativo) — Sección 7.
//!
//! Si la petición trae `?client=native` o la cabecera `X-Aethravox-Client:
//! native`, setea la cookie `aethravox_client=native` (1 año, path=/, NO
//! HttpOnly para que el JS de `base.html` pueda leerla vía `document.cookie`).
//!
//! Un `<script>` inline en `templates/base.html` lee esa cookie y expone
//! `window.AETHRAVOX_CLIENT` ('native' | 'web'). Así, cuando el Tauri abre
//! `/hub?client=native` por primera vez, la cookie persiste y todas las
//! páginas siguientes (navegación interna, sin el query param) siguen
//! viéndose como nativas.
//!
//! Registro: capa global sobre `app` en `main.rs` (junto a `require_no_ip_ban`).
//!
//! NOTA: NO usa el extractor `Cookies` de tower_cookies. Ese extractor exige
//! que `CookieManagerLayer` sea una capa EXTERNA (corra antes que él). Como en
//! main.rs este middleware queda FUERA de CookieManagerLayer, el extractor
//! fallaría con 500 en TODAS las rutas. En su lugar seteamos `Set-Cookie` en
//! la respuesta (tras `next.run(req)`), que no depende del orden de capas.
use axum::{
    extract::{Request, State},
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

use crate::AppState;

pub async fn detect_client(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // native si la cabecera o el query param lo dicen.
    let header_native = req
        .headers()
        .get("x-aethravox-client")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("native"))
        .unwrap_or(false);
    let query_native = req
        .uri()
        .query()
        .map(|q| q.split('&').any(|kv| kv == "client=native"))
        .unwrap_or(false);

    let mut resp = next.run(req).await;

    if header_native || query_native {
        // Set-Cookie en la respuesta (NO usamos el extractor Cookies: así no
        // dependemos del orden de capas frente a CookieManagerLayer, que en
        // main.rs queda INTERNA a este middleware). Sin HttpOnly para que el
        // <script> de base.html lea document.cookie; SameSite=Lax; Secure en
        // production. Max-Age=31536000 = 365 días.
        let mut cookie =
            String::from("aethravox_client=native; Path=/; Max-Age=31536000; SameSite=Lax");
        if state.cfg.production {
            cookie.push_str("; Secure");
        }
        if let Ok(val) = HeaderValue::from_str(&cookie) {
            resp.headers_mut().append(header::SET_COOKIE, val);
        }
    }

    resp
}

// Middleware de autenticación y autorización para axum 0.7.
// - require_auth: middleware que valida sesión y mete User en extensions
// - require_admin_unlock: middleware para rutas /admin/* (re-auth)
// - require_no_ip_ban: middleware que bloquea IPs baneadas antes de cualquier ruta
// - AuthUser: extractor propio que lee User desde extensions sin tocar el body

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, request::Parts},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_cookies::Cookies;

use crate::{models::User, AppState};

pub const AEXOS_COOKIE_NAME: &str = "aethravox_aexos_session";

pub fn set_aexos_session_cookie(state: &AppState, cookies: &Cookies, token: &str) {
    let mut builder = tower_cookies::Cookie::build((AEXOS_COOKIE_NAME, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .max_age(time::Duration::days(30));
    if !state.cfg.cookie_domain.is_empty() {
        builder = builder.domain(state.cfg.cookie_domain.clone());
    }
    if state.cfg.production {
        builder = builder.secure(true);
    }
    cookies.add(builder.build());
}

pub fn clear_aexos_session_cookie(state: &AppState, cookies: &Cookies) {
    let mut builder = tower_cookies::Cookie::build((AEXOS_COOKIE_NAME, String::new()))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0));
    if !state.cfg.cookie_domain.is_empty() {
        builder = builder.domain(state.cfg.cookie_domain.clone());
    }
    cookies.add(builder.build());
}

/// Copia los datos del request que necesita la resolución de sesión aexOS.
/// Evita mantener un `&Request` vivo a través de los `.await` (Body no es Sync,
/// y eso hacía que el future del middleware no fuera Send → error E0277 de axum).
fn request_meta(req: &Request) -> (String, String, String, String) {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let header = |name: &str| {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    };
    (path, query, header("sec-fetch-dest"), header("referer"))
}

async fn resolve_effective_aexos_user(
    state: &AppState,
    cookies: &Cookies,
    primary: User,
    path: String,
    query: String,
    sec_dest: String,
    referer: String,
) -> User {
    if path == "/aexos" || path.starts_with("/api/aexos/") {
        return primary;
    }

    let Some(c) = cookies.get(AEXOS_COOKIE_NAME) else {
        return primary;
    };
    let token = c.value().to_string();
    if token.is_empty() {
        return primary;
    }

    let is_embed_query = query.split('&').any(|kv| kv == "embed=1");

    // Si es una navegación top-level fuera de aexOS (p.ej. salió al /hub),
    // limpiar la sesión temporal de aexOS y operar como la cuenta principal.
    if sec_dest == "document" && !is_embed_query {
        let _ = state.db.delete_aexos_session_by_token(&token).await;
        clear_aexos_session_cookie(state, cookies);
        return primary;
    }

    // Si hay otra pestaña fuera de aexOS haciendo fetch (referer != /aexos y sin embed=1 y no iframe),
    // respetar el contexto de esa pestaña sin interferir.
    let from_outside_tab = sec_dest != "iframe"
        && !is_embed_query
        && !referer.is_empty()
        && !referer.contains("/aexos")
        && !referer.contains("embed=1");
    if from_outside_tab {
        return primary;
    }

    state
        .db
        .get_aexos_session_user(primary.id, &token)
        .await
        .unwrap_or(primary)
}

/// Obtiene la IP real del cliente desde los headers de proxy inverso.
/// Prioridad:
///   1. X-Forwarded-For (toma el primer IP de la lista comma-separated)
///   2. X-Real-IP
///   3. None si no hay headers (desarrollo local sin proxy)
///
/// Detrás de nginx con `proxy_set_header X-Real-IP $remote_addr` y
/// `proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for`, esto funciona.
pub fn get_client_ip(headers: &HeaderMap) -> Option<String> {
    // 1. X-Forwarded-For: "client, proxy1, proxy2" → tomar el primero
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            let first = s.split(',').map(|s| s.trim()).next().unwrap_or("");
            if !first.is_empty() && !first.eq_ignore_ascii_case("unknown") {
                return Some(first.to_string());
            }
        }
    }
    // 2. X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            let s = s.trim();
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

/// Middleware: requiere usuario autenticado y en good standing.
pub async fn require_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut req: Request,
    next: Next,
) -> Response {
    let primary_user = crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name).await;
    let Some(primary_user) = primary_user else {
        let next_url = req
            .uri()
            .path_and_query()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/".to_string());
        return Redirect::to(&format!("/login?next={}", urlencoding(&next_url))).into_response();
    };
    let (path, query, sec_dest, referer) = request_meta(&req);
    let user = resolve_effective_aexos_user(&state, &cookies, primary_user, path, query, sec_dest, referer).await;

    if let Ok(Some(ban)) = state.db.is_user_banned(user.id).await {
        // Borrar la sesión activa (no debe seguir usando la app con la cuenta baneada)
        if let Some(c) = cookies.get(&state.cfg.cookie_name) {
            let _ = state.db.delete_session(c.value()).await;
        }
        crate::handlers::auth::clear_session_cookie(&state, &cookies);
        // Mostrar pantalla de ban (permanente de cuenta) en vez de redirect a /login
        let _ = state.db.audit(
            Some(user.id),
            "account_ban_screen_shown",
            Some(&format!("username={} ban_id={}", user.username, ban.id)),
            None,
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
        // Suspensión temporal de cuenta
        let detail = format!("Suspendido hasta {}", susp.expires_at.format("%d/%m/%Y %H:%M"));
        let _ = state
            .db
            .audit(Some(user.id), "session_blocked_suspended", Some(&detail), None)
            .await;
        if let Some(c) = cookies.get(&state.cfg.cookie_name) {
            let _ = state.db.delete_session(c.value()).await;
        }
        crate::handlers::auth::clear_session_cookie(&state, &cookies);
        // Mostrar pantalla de suspensión con countdown
        return crate::handlers::admin::ban_screen_response(
            crate::handlers::admin::BanKind::UserSuspension,
            susp.id,
            user.username.clone(),
            susp.reason.clone().unwrap_or_else(|| "No se especificó motivo.".into()),
            susp.created_at,
            Some(susp.expires_at),
        ).into_response();
    }

    // ===== IP tracking en cada visita (debounce 5 min) =====
    // Obtiene la IP del cliente y la compara con last_ip del usuario.
    // Si es distinta O han pasado 5 min desde la última actualización,
    // actualiza users.last_ip y registra en user_ip_history.
    let client_ip = {
        let headers = req.headers();
        get_client_ip(headers)
    };
    if let Some(ip) = &client_ip {
        let needs_update = user.last_ip.as_deref() != Some(ip.as_str());
        if needs_update {
            // IP nueva → registrar en historial + actualizar last_ip
            let _ = state.db.record_login_ip(user.id, ip).await;
        } else {
            // Misma IP pero hace más de 5 min → actualizar last_seen del historial
            // (para que el "última vez" en el admin esté fresco)
            let now = chrono::Utc::now();
            let five_mins_ago = now - chrono::Duration::minutes(5);
            // Solo actualizar si el usuario tiene last_login_at y es viejo
            if let Some(last_login) = user.last_login_at {
                if last_login < five_mins_ago {
                    let _ = state.db.touch_last_login(user.id).await;
                }
            }
        }
    }

    req.extensions_mut().insert(user);
    next.run(req).await
}

/// Middleware para rutas guest-allowed (hub, docs, comunidades, soporte):
/// resuelve el usuario real si hay sesión; si NO la hay, asigna un User
/// invitado sintético (stateless, sin fila en `users`) para que el handler
/// reciba un `AuthUser` válido y el invitado pueda ver lo público.
///
/// A diferencia de `require_auth`, NUNCA redirige a /login: el invitado fluye.
/// El gating fuerte (chat/friends/dashboard/transmitir/admin/notifications)
/// lo sigue haciendo `require_auth` sobre las rutas hard-protected.
///
/// Skip de ban/suspend/IP-tracking: esas comprobaciones solo aplican a
/// usuarios reales con sesión (un invitado no tiene cuenta que banear ni
/// last_ip que trackear).
pub async fn resolve_user_or_guest(
    State(state): State<AppState>,
    cookies: Cookies,
    mut req: Request,
    next: Next,
) -> Response {
    let user = crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name).await;
    let user = match user {
        Some(u) => {
            let (path, query, sec_dest, referer) = request_meta(&req);
            resolve_effective_aexos_user(&state, &cookies, u, path, query, sec_dest, referer).await
        }
        None => {
            // Sin sesión → rol Invitado. role_id se cachea en AppState
            // (OnceCell: el primer guest lo resuelve de la DB, el resto lo
            // reutiliza sin tocar la base). Si el lookup falla (migración 036
            // sin correr), rid=None → el invitado no tiene perms pero igual
            // ve lo público por el routing.
            let rid: Option<i64> = state
                .invitado_role_id
                .get_or_try_init(|| async {
                    state.db.role_id_by_key("invitado").await
                })
                .await
                .ok()
                .copied()
                .flatten();
            crate::models::User::guest(rid)
        }
    };
    req.extensions_mut().insert(user);
    next.run(req).await
}

/// Middleware para rutas de admin (idea 1: re-autenticación).
///
/// Comprueba que existe una cookie `aethravox_admin_unlock` (nombre configurable)
/// con un token válido en `state.admin_unlocks`. Si no existe o está expirado,
/// redirige a `/admin/unlock?next=<ruta original>` para que el usuario meta su
/// password otra vez.
///
/// Debe ir DESPUÉS de `require_auth` en el stack de layers (require_auth inserta
/// el User en extensions, este lo lee).
pub async fn require_admin_unlock(
    State(state): State<AppState>,
    cookies: Cookies,
    req: Request,
    next: Next,
) -> Response {
    // require_auth ya puso el User en extensions; lo leemos.
    let user = match req.extensions().get::<User>().cloned() {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // Acceso al panel = permiso "admin.access". El Owner lo tiene por bypass;
    // admin/staff por baseline; un usuario común SOLO si su rol se lo
    // concede (capabilities). Sin el permiso → fuera del panel.
    if !state.db.user_has_perm(&user, "admin.access").await {
        return Redirect::to("/hub").into_response();
    }

    let cookie_name = state.cfg.admin_unlock_cookie_name.as_str();
    let ttl_secs = state.cfg.admin_unlock_ttl_minutes * 60;
    let now_epoch = chrono::Utc::now().timestamp();

    // GC perezoso: limpiar tokens expirados para que el HashMap no crezca infinito.
    {
        let mut unlocks = state.admin_unlocks.lock().await;
        unlocks.retain(|_, v| v.expires_at > now_epoch);
    }

    // ¿Tiene cookie válida?
    let token_valid = if let Some(cookie) = cookies.get(cookie_name) {
        let token = cookie.value().to_string();
        if token.is_empty() {
            false
        } else {
            let unlocks = state.admin_unlocks.lock().await;
            match unlocks.get(&token) {
                Some(entry) => entry.user_id == user.id && entry.expires_at > now_epoch,
                None => false,
            }
        }
    } else {
        false
    };

    if token_valid {
        // Refrescar el TTL: el usuario sigue activo, extiende la sesión de admin.
        if let Some(cookie) = cookies.get(cookie_name) {
            let token = cookie.value().to_string();
            let mut unlocks = state.admin_unlocks.lock().await;
            if let Some(entry) = unlocks.get_mut(&token) {
                entry.expires_at = now_epoch + ttl_secs;
            }
        }
        next.run(req).await
    } else {
        // No desbloqueado → redirigir a /admin/unlock con el destino original.
        let next_url = req
            .uri()
            .path_and_query()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/admin".to_string());
        Redirect::to(&format!("/admin/unlock?next={}", urlencoding(&next_url))).into_response()
    }
}

/// Extractor que devuelve el User autenticado leyéndolo desde extensions.
pub struct AuthUser(pub User);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts.extensions.get::<User>().cloned();
        match user {
            Some(u) => Ok(AuthUser(u)),
            None => Err(Redirect::to("/login")),
        }
    }
}

/// Helper para obtener el usuario desde extensions (puesto por require_auth).
pub fn user_from_req(req: &axum::http::Request<axum::body::Body>) -> Option<User> {
    req.extensions().get::<User>().cloned()
}

/// Middleware OUTER (debe ir antes de require_auth en el stack) que comprueba
/// si la IP del cliente está baneada. Si lo está, devuelve la pantalla de ban
/// con countdown en tiempo real (sin permitir acceso a ninguna otra ruta).
///
/// Esta es la primera línea de defensa contra IPs abusivas (DDoS, fuerza bruta).
/// Incluso /login está bloqueado para una IP baneada — ven la pantalla de ban.
pub async fn require_no_ip_ban(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let headers = req.headers().clone();
    let client_ip = get_client_ip(&headers);

    if let Some(ip) = &client_ip {
        if let Ok(Some(ban)) = state.db.is_ip_banned(ip).await {
            // Ban encontrado — devolver pantalla de ban con countdown
            return crate::handlers::admin::ban_screen_response(
                crate::handlers::admin::BanKind::IpBan,
                ban.id,
                ip.clone(),
                ban.reason.clone(),
                ban.created_at,
                ban.expires_at,
            ).into_response();
        }
    }
    next.run(req).await
}

pub fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b;
        if c.is_ascii_alphanumeric() || c == b'/' || c == b'-' || c == b'_' || c == b'.' {
            out.push(c as char);
        } else {
            out.push_str(&format!("%{:02X}", c));
        }
    }
    out
}

/// Rate limiter en memoria para acciones de invitado (Sección 3.6).
/// Cuenta por IP los hits en una ventana deslizante; si supera MAX en
/// WINDOW_SECS, deniega. En memoria porque es stateless y efímero: al
/// reiniciar el binario se resetea (aceptable para rate-limiting de guests).
/// Aplicado a las pocas acciones write que un invitado puede hacer
/// (p.ej. /support/create) para evitar spam de tickets anónimos.
pub struct GuestRateLimiter {
    hits: tokio::sync::Mutex<std::collections::HashMap<String, Vec<i64>>>,
}

impl GuestRateLimiter {
    /// Máximo de acciones por IP dentro de la ventana.
    const MAX: usize = 5;
    /// Ventana deslizante en segundos (1 hora).
    const WINDOW_SECS: i64 = 3600;

    pub fn new() -> Self {
        Self {
            hits: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Devuelve true si la IP puede actuar (y registra el hit); false si está
    /// rate-limited. IP vacía → siempre allow (no podemos trackear sin IP; cae
    /// al caso de desarrollo sin proxy que setee X-Forwarded-For/X-Real-IP).
    pub async fn allow(&self, ip: &str) -> bool {
        if ip.is_empty() {
            return true;
        }
        let now = chrono::Utc::now().timestamp();
        let mut map = self.hits.lock().await;
        let v = map.entry(ip.to_string()).or_default();
        // Purga perezosa: descarta hits fuera de la ventana para que el Vec no
        // crezca infinito y el conteo sea solo "reciente".
        v.retain(|&t| now - t < Self::WINDOW_SECS);
        if v.len() >= Self::MAX {
            return false;
        }
        v.push(now);
        true
    }
}

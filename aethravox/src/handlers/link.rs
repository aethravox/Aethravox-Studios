//! Vinculación de dispositivos (V4.5) — estilo WhatsApp Web / Telegram.
//!
//! Flujo:
//!   1. El dispositivo **principal** (el que tiene sesión iniciada con
//!      user+pass) abre `/vincular` → ve un QR + un código numérico de 6
//!      dígitos que rotan cada 60 s. Lo genera con `GET /api/link/code`
//!      (auth requerida); el backend inserta una fila en `link_codes`.
//!   2. El dispositivo **secundario** (sin sesión) abre `/vincular` → ve un
//!      formulario para teclear el código, o escanea el QR con su cámara.
//!      Si escanea el QR aterriza en `/vincular?c=<code>` → página de
//!      confirmación "¿Vincular como <usuario>?" → al confirmar,
//!      `POST /vincular/confirm` consume el código (atómicamente) y crea
//!      una sesión vinculada (is_primary=0) con `link_expires_at = now + 3d`.
//!   3. La sesión vinculada **rueda** +3 días en cada visita (ver
//!      `db::get_session_user`); si pasan 3 días sin tocarla, se cierra sola.
//!   4. El principal puede ver y revocar las vinculadas desde
//!      `/dashboard → Dispositivos vinculados` (`GET /api/link/devices`,
//!      `POST /api/link/revoke/:token`).
//!   5. Si un segundo dispositivo intenta LOGUEARSE (user+pass) en una
//!      cuenta que ya tiene primary abierta, esa primary se cierra
//!      (`db::revoke_other_primary_sessions`, llamado desde `auth::login_post`).
//!      Las vinculadas no se tocan. Así se fuerza a usar vinculación, no
//!      re-login, para tener varios dispositivos activos.

use axum::{
    extract::{Form, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;

use crate::{
    auth_mw::AuthUser,
    handlers::auth::set_session_cookie,
    helpers,
    models::LinkConsumeForm,
    templates::{VincularConfirmTemplate, VincularTemplate},
    AppState,
};

/// Query string de `/vincular` — opcionalmente `?c=<code>` (QR escaneado) o
/// `?next=<url>` (volver ahí tras vincular, p.ej. tras venir de /login).
#[derive(Debug, Deserialize, Default)]
pub struct VincularQuery {
    #[serde(default)]
    pub c: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
}

/// `GET /vincular` — página pública.
///
/// - Sin `?c=` y SIN sesión → vista "secondary": formulario para teclear
///   el código de 6 dígitos que le muestra el principal.
/// - Sin `?c=` y CON sesión → vista "primary": generador de QR + código
///   (la página llama a `/api/link/code` por JS para tener un código fresco
///   y refrescarlo cada 50 s).
/// - Con `?c=<code>` → redirige al flujo de confirmación (vista aparte:
///   `vincular_confirm.html`). Lo gestiona `vincular_confirm_get` abajo.
pub async fn vincular_page(
    State(state): State<AppState>,
    cookies: Cookies,
    Query(q): Query<VincularQuery>,
) -> Response {
    // Si vino con ?c=<code>, va al flujo de confirmación (otra handler).
    if let Some(code) = q.c.as_deref().filter(|c| !c.trim().is_empty()) {
        return vincular_confirm_get(State(state), code.trim().to_string()).await;
    }

    let next_safe = q
        .next
        .as_deref()
        .filter(|n| !n.is_empty() && n.starts_with('/'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/hub".to_string());

    // ¿Tiene sesión? Si sí → modo "primary" (genera QR para otros). Si no →
    // modo "secondary" (teclea código o escanea QR de otro).
    match helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name).await {
        Some(user) => VincularTemplate {
            mode: "primary".into(),
            username: user.username.clone(),
            avatar: user.avatar(),
            next_url: next_safe,
            error: None,
        }
        .into_response(),
        None => VincularTemplate {
            mode: "secondary".into(),
            username: String::new(),
            avatar: String::new(),
            next_url: next_safe,
            error: None,
        }
        .into_response(),
    }
}

/// `GET /vincular?c=<code>` — página de confirmación tras escanear el QR.
///
/// Busca el código. Si es válido y no usado, muestra "¿Vincular como
/// @<username>?" con un botón Aceptar. Si no, muestra error. No vincula
/// automáticamente: el usuario debe confirmar (permite abortar si escaneó
/// un QR por error o uno caducado).
pub async fn vincular_confirm_get(State(state): State<AppState>, code: String) -> Response {
    let code_lookup = match state.db.find_active_link_code(&code).await {
        Ok(Some(link)) => link,
        _ => {
            return VincularConfirmTemplate {
                code: String::new(),
                target_username: String::new(),
                target_avatar: String::new(),
                error: Some(
                    "El código no es válido, ya se usó o caducó (dura 60 s). \
                     Pídele al principal que genere uno nuevo."
                        .into(),
                ),
            }
            .into_response();
        }
    };
    let user = match state.db.find_user_by_id(code_lookup.user_id).await {
        Ok(Some(u)) => u,
        _ => {
            return VincularConfirmTemplate {
                code: code_lookup.code,
                target_username: String::new(),
                target_avatar: String::new(),
                error: Some("La cuenta asociada al código ya no existe.".into()),
            }
            .into_response();
        }
    };
    VincularConfirmTemplate {
        code: code_lookup.code,
        target_username: user.username.clone(),
        target_avatar: user.avatar(),
        error: None,
    }
    .into_response()
}

/// Body del POST de vinculación (form o JSON). Acepta tanto el `code` (token
/// del QR) como el `numeric_code` (6 dígitos): el backend los busca
/// indistintamente en `find_active_link_code`.
#[derive(Debug, Deserialize)]
pub struct ConsumeBody {
    pub code: String,
    #[serde(default)]
    pub device_name: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
}

/// Lógica compartida entre el POST de form y el POST de JSON: valida el
/// código, consume atómicamente, crea la sesión vinculada, setea la cookie.
/// Devuelve Ok(redirect_url) o Err(mensaje_usuario).
async fn do_consume(
    state: &AppState,
    cookies: &Cookies,
    body: ConsumeBody,
) -> Result<String, String> {
    let code_input = body.code.trim().to_string();
    if code_input.is_empty() {
        return Err("Introduce el código de 6 dígitos (o pega el token del QR).".into());
    }
    // Normalizar: quitamos guiones/espacios y pasamos a mayúsculas (los
    // tokens base32 son case-sensitive en mayúsculas por convención, los
    // numéricos no tienen case pero igualan igual).
    let normalized_upper: String = code_input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();

    let link = state
        .db
        .find_active_link_code(&normalized_upper)
        .await
        .map_err(|e| format!("Error interno: {e}"))?
        .ok_or_else(|| {
            "El código no es válido, ya se usó o caducó (dura 60 s). Pídele al \
             principal que genere uno nuevo."
                .to_string()
        })?;

    // Nombre legible del dispositivo (lo elige el secundario; default
    // "Vinculado <fecha>"). Truncado a 80 chars para no romper la UI.
    let device_name = body
        .device_name
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>())
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M");
            format!("Vinculado {}", now)
        });

    let (token, _link_exp) = state
        .db
        .create_linked_session(link.user_id, link.user_id, Some(&device_name))
        .await
        .map_err(|e| format!("Error creando sesión vinculada: {e}"))?;

    // Consume atómicamente: si dos dispositivos compiten por el mismo código
    // a la vez, solo uno gana. Si pierde la carrera, devolvemos error.
    let won = state
        .db
        .consume_link_code(&link.code, &token)
        .await
        .map_err(|e| format!("Error consumiendo código: {e}"))?;
    if !won {
        // Borrar la sesión creada (no debería quedar colgando).
        let _ = state.db.revoke_session_by_token(&token).await;
        return Err(
            "El código ya fue usado por otro dispositivo. Pídele al principal \
             que genere uno nuevo."
                .into(),
        );
    }

    let _ = state
        .db
        .audit(
            Some(link.user_id),
            "device_linked",
            Some(&format!("device_name={} via_code={}", device_name, link.code)),
            None,
        )
        .await;

    // Cookie con remember=false (la ventana real la marca link_expires_at,
    // 3 días rolling; no queremos que el navegador la guarde 30 días si el
    // usuario cierra el navegador — mejor que re-vincule si pasa mucho tiempo).
    set_session_cookie(
        state,
        cookies,
        &token,
        chrono::Utc::now() + chrono::Duration::days(3),
        false,
    );

    let next_safe = body
        .next
        .as_deref()
        .filter(|n| !n.is_empty() && n.starts_with('/'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "/hub".to_string());
    Ok(next_safe)
}

/// `POST /vincular/confirm` — form tradicional (sin JS). El secundario
/// tecleó el código (o vino del botón "Confirmar" de la página ?c=). En
/// caso de error, vuelve a la página adecuada con el mensaje.
pub async fn vincular_confirm_post(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LinkConsumeForm>,
) -> Response {
    let body = ConsumeBody {
        code: form.code.clone(),
        device_name: form.device_name.clone(),
        next: None,
    };
    match do_consume(&state, &cookies, body).await {
        Ok(target) => Redirect::to(&target).into_response(),
        Err(msg) => {
            // Si el código parecía un token QR (largo), mostramos la
            // confirmación con el error; si no, volvemos a /vincular.
            let looks_token = form.code.trim().len() >= 10;
            if looks_token {
                VincularConfirmTemplate {
                    code: form.code,
                    target_username: String::new(),
                    target_avatar: String::new(),
                    error: Some(msg),
                }
                .into_response()
            } else {
                VincularTemplate {
                    mode: "secondary".into(),
                    username: String::new(),
                    avatar: String::new(),
                    next_url: "/hub".into(),
                    error: Some(msg),
                }
                .into_response()
            }
        }
    }
}

/// `POST /api/link/consume` — versión JSON del consumo (para SPA / fetch).
/// Devuelve `{ok, redirect}` o `{ok:false, error}`.
#[derive(Serialize)]
pub struct ConsumeResp {
    pub ok: bool,
    pub redirect: Option<String>,
    pub error: Option<String>,
}

pub async fn api_link_consume(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<ConsumeBody>,
) -> Response {
    match do_consume(&state, &cookies, body).await {
        Ok(target) => Json(ConsumeResp {
            ok: true,
            redirect: Some(target),
            error: None,
        })
        .into_response(),
        Err(msg) => Json(ConsumeResp {
            ok: false,
            redirect: None,
            error: Some(msg),
        })
        .into_response(),
    }
}

/// Respuesta de `GET /api/link/code` — el frontend la pide nada más cargar
/// `/vincular` (modo primary) y cada 50 s para refrescar antes de que
/// caduque (60 s). Contiene el `code` (token del QR), el `numeric_code`
/// (6 dígitos para teclear), `expires_at` (ISO) y `qr_svg` (el QR dibujado
/// como SVG inline, listo para inyectar en un div). El QR codifica la URL
/// `<public_url>/vincular?c=<code>` — al escanearlo, el navegador del
/// secundario abre esa URL y entra al flujo de confirmación.
#[derive(Serialize)]
pub struct LinkCodeResp {
    pub code: String,
    pub numeric_code: String,
    pub expires_at: String,
    pub qr_svg: String,
    pub target_username: String,
}

/// `GET /api/link/code` (auth requerida) — genera un código de vinculación
/// fresco para el usuario actual y devuelve el QR como SVG.
pub async fn api_link_code(AuthUser(user): AuthUser, State(state): State<AppState>) -> Response {
    // GC perezoso: borra códigos caducados/usados hace >24h (best-effort).
    let _ = state.db.cleanup_link_codes().await;

    let (code, numeric, expires) = match state.db.create_link_code(user.id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Error creando link_code para {}: {e}", user.username);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "No se pudo generar el código. Intenta de nuevo.",
            )
                .into_response();
        }
    };

    let qr_url = format!(
        "{}/vincular?c={}",
        state.cfg.public_url.trim_end_matches('/'),
        code
    );
    let qr_svg = match render_qr_svg(&qr_url) {
        Ok(svg) => svg,
        Err(e) => {
            tracing::error!("Error generando QR: {e}");
            // Si el QR falla, seguimos devolviendo el numeric (el usuario
            // puede teclearlo). qr_svg vacío → el JS muestra solo el numeric.
            String::new()
        }
    };

    Json(LinkCodeResp {
        code,
        numeric_code: numeric,
        expires_at: expires.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        qr_svg,
        target_username: user.username.clone(),
    })
    .into_response()
}

/// Renderiza un QR como SVG string (sin deps de imagen, sin assets). El
/// módulo `qrcode::render::svg` dibuja cada módulo como un `<rect>`; le
/// pedimos 200×200 mínimo con quiet zone de 4 módulos. Colores invertidos
/// (fondo dorado, módulos negros) para encajar con la identidad visual.
fn render_qr_svg(data: &str) -> anyhow::Result<String> {
    use qrcode::{QrCode, render::svg::Color};
    let code = QrCode::new(data.as_bytes())?;
    let svg = code
        .render::<Color>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .build();
    Ok(svg)
}

/// Respuesta de `GET /api/link/devices` — lista de dispositivos vinculados
/// al usuario actual (is_primary=0). Para el dashboard.
#[derive(Serialize)]
pub struct LinkedDeviceDto {
    /// Token truncado a 8 chars + "…" — solo para mostrar. La revocación
    /// usa el token completo (lo pasa el dashboard desde el template, no
    /// desde este DTO).
    pub token: String,
    pub device_name: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub link_expires_at: Option<String>,
    /// True si la sesión sigue activa (link_expires_at en el futuro).
    pub active: bool,
}

/// `GET /api/link/devices` (auth requerida) — lista de dispositivos
/// vinculados a la cuenta del usuario actual.
pub async fn api_link_devices(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Response {
    let devices = state.db.list_linked_devices(user.id).await.unwrap_or_default();
    let now = chrono::Utc::now();
    let dtos: Vec<LinkedDeviceDto> = devices
        .into_iter()
        .map(|d| LinkedDeviceDto {
            token: truncate_token(&d.token),
            device_name: d.device_name.unwrap_or_else(|| "(sin nombre)".into()),
            created_at: d.created_at.format("%Y-%m-%d %H:%M").to_string(),
            last_seen_at: d.last_seen_at.format("%Y-%m-%d %H:%M").to_string(),
            link_expires_at: d
                .link_expires_at
                .map(|t| t.format("%Y-%m-%d %H:%M").to_string()),
            active: d.link_expires_at.map(|t| t > now).unwrap_or(false),
        })
        .collect();
    Json(dtos).into_response()
}

/// `POST /api/link/revoke/:token` (auth requerida) — revoca una sesión
/// vinculada. El `:token` debe ser el token COMPLETO de la sesión a
/// revocar (el dashboard del principal lo envía desde el template, donde
/// sí está disponible completo).
pub async fn api_link_revoke(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Response {
    let ok = state
        .db
        .revoke_linked_device(user.id, &token)
        .await
        .unwrap_or(false);
    if ok {
        let _ = state
            .db
            .audit(
                Some(user.id),
                "linked_device_revoked",
                Some(&format!("token={}…", &token.get(..8).unwrap_or(&token))),
                None,
            )
            .await;
        Json(serde_json::json!({"ok": true})).into_response()
    } else {
        (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"ok": false, "error": "No existe ese dispositivo vinculado o no es tuyo."})),
        )
            .into_response()
    }
}

fn truncate_token(t: &str) -> String {
    let head = t.get(..8).unwrap_or(t);
    format!("{}…", head)
}

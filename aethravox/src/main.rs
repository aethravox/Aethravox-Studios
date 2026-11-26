// askama referencia fields de structs en templates, pero el compilador
// no lo sabe y emite "field never read" warnings. Los silenciamos aquí.
#![allow(dead_code)]

mod auth_mw;
mod config;
mod db;
mod docs;
mod handlers;
mod helpers;
mod models;
mod notifications;
mod role_color;
mod templates;
mod aethramail_client; // Sección 6: cliente mTLS contra AethraMail
mod client_mw; // Sección 7: detección web/nativo (cookie aethravox_client)

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    middleware::from_fn_with_state,
    routing::{delete, get, post},
    Router,
};
use tokio::sync::mpsc;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub cfg: config::Config,
    pub db: db::Db,
    /// Caché de colores de rol ({ role_key → {color, text, name} }). Se refresca
    /// al arrancar y tras cada create/update/delete de rol. Sirve
    /// /api/role_colors (JSON para role_colors.js) y /static/role-colors.css
    /// (CSS en <head> para evitar FOUC de los badges).
    pub role_colors: Arc<role_color::RoleColorCache>,
    pub ws_connections: Arc<tokio::sync::Mutex<HashMap<i64, mpsc::UnboundedSender<String>>>>,
    /// Tokens de desbloqueo del panel admin (idea 1: re-auth).
    /// Clave: token aleatorio de 32 bytes hex. Valor: (user_id, expires_at epoch secs).
    /// Se usa memoria en vez de DB porque al reiniciar el binario los tokens
    /// deben invalidarse automáticamente (más seguro).
    pub admin_unlocks: Arc<tokio::sync::Mutex<HashMap<String, AdminUnlock>>>,
    /// Salas de "Transmitir" (screen sharing WebRTC). En memoria porque son
    /// efímeras: si el server reinicia, las transmisiones se cortan y los
    /// clientes simplemente reconectan. Clave: nombre de la sala (lowercase).
    pub transmit_rooms: Arc<tokio::sync::Mutex<HashMap<String, handlers::transmit::TransmitRoom>>>,
    /// Contador atómico para asignar IDs únicos a cada conexión WS de /transmitir/ws.
    /// Cada pestaña/ventana abierta recibe un ID distinto, así el mismo usuario
    /// puede tener múltiples sesiones (ej: una para enviar, otra para recibir).
    pub transmit_conn_counter: Arc<std::sync::atomic::AtomicI64>,
    /// Gestor de Web Push (VAPID + envío). None si no se pudo inicializar
    /// (falta config, error generando claves…). La app sigue funcionando sin
    /// push; simplemente no se envían notificaciones cuando la pestaña está
    /// cerrada. Envuelto en Arc para reutilizar el cliente HTTP caro.
    pub push: Arc<Option<notifications::PushManager>>,
    /// Caché del id del rol "Invitado" (sembrado en migración 036). Lo usa
    /// `auth_mw::resolve_user_or_guest` para construir el User invitado
    /// sintético cuando no hay sesión. OnceCell: se resuelve de la DB una
    /// sola vez (primer request sin sesión) y se reutiliza sin tocar la base.
    /// `None` interno si la migración 036 no corrió (el invitado no tendrá
    /// perms, pero el routing igual lo deja ver lo público).
    pub invitado_role_id: Arc<tokio::sync::OnceCell<Option<i64>>>,
    /// Rate limiter por IP para acciones de invitado (Sección 3.6). En memoria,
    /// ventana deslizante. Lo usa el handler de /support/create para frenar el
    /// spam de tickets anónimos.
    pub guest_rate_limiter: Arc<auth_mw::GuestRateLimiter>,
    /// Cliente mTLS contra la API interna de AethraMail (Sección 6). None si no
    /// había certs al arrancar (dev sin `aethramail/scripts/gen_certs.sh`); los
    /// flujos que mandan correo degradan a modo dev (link inline / aviso).
    pub aethramail: Arc<Option<aethramail_client::AethraMailClient>>,
}

#[derive(Clone, Debug)]
pub struct AdminUnlock {
    pub user_id: i64,
    /// Expiración en segundos desde epoch (UTC).
    pub expires_at: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let log_level = std::env::var("AETHRAVOX_LOG_LEVEL").unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_level))
        .init();

    let cfg = config::Config::from_env();
    tracing::info!("⚔️  Aethravox Studios arrancando…");
    tracing::info!("📅 Fundación digital: 24/01/2026");
    tracing::info!("🌐 Escuchando en http://{}", cfg.listen_addr);

    let db = db::Db::connect("data/aethravox.db").await?;
    let _ = std::fs::create_dir_all("data/aextube");
    db.ensure_owner(&cfg.owner_user, &cfg.owner_email, &cfg.owner_pass)
        .await
        .unwrap_or_else(|e| tracing::warn!("No se pudo crear Owner inicial: {e}"));

    // Caché de colores de rol: puebla desde la DB para servir /api/role_colors
    // (JSON) y /static/role-colors.css (CSS en <head>, evita FOUC de badges).
    let role_colors = Arc::new(role_color::RoleColorCache::new());
    role_colors.refresh(&db).await;

    // Web Push: genera claves VAPID si no existen y construye el cliente. Si
    // falla, la app arranca sin push (None) — el frontend lo detecta y no
    // intenta suscribir.
    let push = match notifications::PushManager::init(
        std::path::Path::new("data"),
        &cfg.owner_email,
    ) {
        Ok(pm) => {
            tracing::info!("🔔 Web Push listo (clave pública VAPID cargada)");
            Some(pm)
        }
        Err(e) => {
            tracing::warn!("⚠️  Web Push no disponible: {e:#}. Las notificaciones solo funcionarán con la pestaña abierta.");
            None
        }
    };

    // Cliente mTLS contra AethraMail (Sección 6). Best-effort: si faltan certs,
    // el hub arranca sin mail y los flujos degradan a modo dev.
    let aethramail = match aethramail_client::AethraMailClient::new(&cfg) {
        Ok(c) => {
            tracing::info!("📧 Cliente AethraMail mTLS listo (API {})", cfg.aethramail_api_url);
            Some(c)
        }
        Err(e) => {
            tracing::warn!("⚠️  AethraMail no disponible (flujos de correo degradan a dev): {e:#}");
            None
        }
    };

    let state = AppState {
        cfg: cfg.clone(),
        db,
        role_colors: role_colors.clone(),
        ws_connections: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        admin_unlocks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        transmit_rooms: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        transmit_conn_counter: Arc::new(std::sync::atomic::AtomicI64::new(1)),
        push: Arc::new(push),
        invitado_role_id: Arc::new(tokio::sync::OnceCell::new()),
        guest_rate_limiter: Arc::new(auth_mw::GuestRateLimiter::new()),
        aethramail: Arc::new(aethramail),
    };

    let public_routes = Router::new()
        .route("/", get(handlers::auth::index))
        .route("/login", get(handlers::auth::login_get).post(handlers::auth::login_post))
        .route("/register", get(handlers::auth::register_get).post(handlers::auth::register_post))
        .route("/recover", get(handlers::auth::recover_get).post(handlers::auth::recover_post))
        .route("/reset", get(handlers::auth::reset_get).post(handlers::auth::reset_post))
        .route("/reset/cancel", get(handlers::auth::reset_cancel_get)) // 6.5 "No soy yo"
        .route("/logout", post(handlers::auth::logout))
        .route("/d/:uuid", get(handlers::docs::public_view))
        // ===== Colores de rol (públicos) =====
        // /api/role_colors: JSON { role_key: { color, text, name } } para role_colors.js.
        // /role-colors.css: CSS para <head> (evita FOUC de los badges). Va fuera de
        // /static porque /static es un nest_service(ServeDir) y una ruta explícita
        // bajo ese prefijo haría chocar el router de axum al arrancar.
        .route("/api/role_colors", get(role_color::api_role_colors))
        .route("/role-colors.css", get(role_color::role_colors_css))
        // /title-colors.css: CSS dinámico con los colores de los títulos
        // (badges cosméticos). Mismo patrón que /role-colors.css: enlazado en
        // <head> para evitar FOUC de los badges de título.
        .route("/title-colors.css", get(handlers::titles::title_colors_css))
        // Documentos legales públicos (sin sesión): privacidad y términos.
        // Enlazados desde el checkbox "acepto términos" del registro.
        .route("/privacidad", get(handlers::auth::legal_privacidad))
        .route("/terminos", get(handlers::auth::legal_terminos))
        // ===== Vinculación de dispositivos (V4.5) =====
        // /vincular es pública: el dispositivo secundario NO tiene sesión.
        // Si la tiene (es el principal), muestra el generador de QR/código.
        // /vincular/confirm recibe el código del form (tecleteado o token QR).
        // /api/link/consume es la versión JSON para SPA / fetch.
        .route("/vincular", get(handlers::link::vincular_page))
        .route("/vincular/confirm", post(handlers::link::vincular_confirm_post))
        .route("/api/link/consume", post(handlers::link::api_link_consume))
        // Visor PDF (plantilla Askama) + PDF.js como vendor JS en static/pdf/
        // (servido por ServeDir). Públicos porque el visor se carga desde /d/:uuid.
        .route("/pdf-viewer/viewer.html", get(handlers::assets::viewer));

    // ===== Rutas guest-allowed (Sección 3.2/3.4) =====
    // El invitado sin sesión puede ver: hub, docs (solo view, no descargar),
    // comunidades (públicas, view) y soporte (filear ticket). El middleware
    // resolve_user_or_guest resuelve el usuario real o, si no hay sesión,
    // asigna un User invitado sintético (stateless) — NUNCA redirige a /login.
    // El gating fuerte (chat/friends/dashboard/transmitir/admin/notifications/
    // perfil/u/docs-download) queda en protected_routes con require_auth.
    let guest_routes = Router::new()
        .route("/hub", get(handlers::hub::hub))
        .route("/api/stats", get(handlers::api::hub_stats))
        .route("/api/docs/list", get(handlers::api::docs_list_api))
        .route("/docs", get(handlers::docs::list))
        .route("/docs/:id", get(handlers::docs::view))
        .route("/docs/:id/raw", get(handlers::docs::serve_raw))
        .route("/docs/:id/preview", get(handlers::docs::serve_preview))
        .route("/communities", get(handlers::communities::communities_page))
        .route("/community/:id", get(handlers::communities::community_view))
        .route("/api/communities/:id/members", get(handlers::communities::api_community_members))
        .route("/api/communities/:id/inviteable", get(handlers::communities::api_community_inviteable))
        .route("/support", get(handlers::support::support_page))
        .route("/support/create", post(handlers::support::api_create_ticket))
        // ===== Sección 4: categorías del hub (guest-allowed) =====
        .route("/categorias", get(handlers::categories::categorias_page))
        .route("/descargas", get(handlers::descargas::descargas_page))
        .route("/categoria/:slug", get(handlers::categories::categoria_detalle))
        .route("/explorar", get(handlers::categories::explorar_page))
        .route("/buscar", get(handlers::categories::buscar_page))
        .route("/correo", get(handlers::correo::correo_redirect))
        .route("/api/aris/core/status", get(handlers::aris::core::api_core_status))
        .layer(from_fn_with_state(state.clone(), auth_mw::resolve_user_or_guest));

    let protected_routes = Router::new()
        .route("/dashboard", get(handlers::dashboard::dashboard))
        .route("/dashboard/profile", post(handlers::dashboard::update_profile))
        .route("/dashboard/password", post(handlers::dashboard::change_password))
        .route("/dashboard/avatar", post(handlers::dashboard::upload_avatar))
        .route("/dashboard/sessions/:token/revoke", post(handlers::dashboard::revoke_session))
        .route("/dashboard/devices/:token/revoke", post(handlers::dashboard::revoke_linked_device))
        .route("/dashboard/logout_all", post(handlers::dashboard::logout_all))
        // Sección 6.2 — Correo interno (vinculación con código de 8 dígitos).
        .route("/cuenta/correo", get(handlers::correo::correo_get))
        .route("/cuenta/correo/solicitar", post(handlers::correo::correo_solicitar_post))
        .route("/cuenta/correo/verificar", post(handlers::correo::correo_verificar_post))
        .route("/cuenta/correo/primary", post(handlers::correo::correo_primary_post))
        .route("/cuenta/correo/remove", post(handlers::correo::correo_remove_post))
        .route("/u/:username", get(handlers::profile::profile))
        // /perfil = perfil PROPIO del usuario logueado (web nueva). Distinto
        // de /dashboard (ajustes de cuenta) y de /u/:username (perfil público).
        .route("/perfil", get(handlers::profile::perfil_get))
        .route("/perfil/titles", post(handlers::profile::perfil_titles_post))
        // /admin/unlock NO requiere desbloqueo previo (es la puerta de entrada).
        // Sí requiere auth, por eso va en protected_routes.
        .route("/admin/unlock", get(handlers::admin::unlock_get).post(handlers::admin::unlock_post))
        // ===== API =====
        .route("/api/me", get(handlers::api::me))
        .route("/api/docs/:id/views", get(handlers::api::doc_views))
        .route("/api/users/online", get(handlers::api::users_online))
        .route("/api/tags", get(handlers::api::list_tags))
        // ===== Chat API =====
        .route("/api/chat/conversations", get(handlers::chat::api_conversations))
        .route("/api/chat/communities", get(handlers::chat::api_chat_communities))
        .route("/api/chat/messages/:id", get(handlers::chat::api_messages))
        .route("/api/chat/read/:id", post(handlers::chat::api_mark_read))
        .route("/api/chat/message/:id/edit", post(handlers::chat::api_edit_message))
        .route("/api/chat/message/:id/delete", post(handlers::chat::api_delete_message))
        .route("/api/chat/messages/bulk-delete", post(handlers::chat::api_bulk_delete_messages))
        .route("/api/chat/message/:id/info", get(handlers::chat::api_message_info))
        .route("/api/chat/new_dm", post(handlers::chat::api_new_dm))
        .route("/api/chat/new_group", post(handlers::chat::api_new_group))
        .route("/api/chat/users", get(handlers::chat::api_users))
        // ===== Notificaciones Web Push + Service Worker =====
        .route("/api/notifications/vapid-public-key", get(handlers::notifications::vapid_public_key))
        .route("/api/notifications/subscribe", post(handlers::notifications::subscribe))
        .route("/api/notifications/unsubscribe", post(handlers::notifications::unsubscribe))
        // ===== Vinculación de dispositivos (V4.5) — endpoints autenticados =====
        // El principal genera códigos y lista/revoca vinculados desde aquí.
        // /vincular y /api/link/consume son públicas (van en public_routes).
        .route("/api/link/code", get(handlers::link::api_link_code))
        .route("/api/link/devices", get(handlers::link::api_link_devices))
        .route("/api/link/revoke/:token", post(handlers::link::api_link_revoke))
        // SW en raíz para que su scope sea '/' (controla todo el sitio).
        .route("/sw.js", get(handlers::notifications::serve_sw))
        // ===== Docs =====
        .route("/docs/:id/download", get(handlers::docs::download))
        .route("/docs/:id/download/:format", get(handlers::docs::download_as))
        // ===== Chat =====
        .route("/chat", get(handlers::chat::chat_page))
        .route("/chat/ws", get(handlers::chat::ws_handler))
        .route("/api/chat/:id/upload", post(handlers::chat::upload_chat_image))
        .route("/api/chat/:id/upload_file", post(handlers::chat::upload_chat_file))
        // ===== Group management (Fase 3) =====
        .route("/chat/group/:id/settings", get(handlers::chat::group_settings))
        .route("/api/chat/group/:id/ranks", get(handlers::chat::api_list_ranks))
        .route("/api/chat/group/:id/members", get(handlers::chat::api_list_members))
        .route("/api/chat/group/:id/rank", post(handlers::chat::api_set_member_rank))
        .route("/api/chat/group/:id/ranks/create", post(handlers::chat::api_create_rank))
        .route("/api/chat/group/:id/ranks/:rank_id/update", post(handlers::chat::api_update_rank))
        .route("/api/chat/group/:id/ranks/:rank_id/delete", post(handlers::chat::api_delete_rank))
        .route("/api/chat/group/:id/transfer", post(handlers::chat::api_transfer_ownership))
        .route("/api/chat/group/:id/image", post(handlers::chat::upload_group_image))
        // ===== Friends (Web 5) =====
        .route("/friends", get(handlers::friends::friends_page))
        .route("/friends/send", post(handlers::friends::send_request_form))
        .route("/friends/accept/:id", post(handlers::friends::accept_request_form))
        .route("/friends/reject/:id", post(handlers::friends::reject_request_form))
        .route("/friends/remove/:id", post(handlers::friends::remove_friend_form))
        // ===== Friends API (JSON) =====
        .route("/api/friends/list", get(handlers::friends::api_friends_list))
        .route("/api/friends/pending", get(handlers::friends::api_pending_requests))
        .route("/api/friends/request", post(handlers::friends::api_send_request))
        .route("/api/friends/accept/:id", post(handlers::friends::api_accept_request))
        .route("/api/friends/reject/:id", post(handlers::friends::api_reject_request))
        .route("/api/friends/remove/:id", post(handlers::friends::api_remove_friend))
        .route("/api/friends/search", get(handlers::friends::api_search_users))
        // ===== Playground (idea 3) =====
        .route("/playground", get(handlers::playground::playground_page))
        .route("/juegos/neon-canvas", get(handlers::juegos::neon_canvas_page))
        .route("/embed/:slug", get(handlers::admin::embed_external))
        // ===== Communities (Fase 5) =====
        .route("/api/communities/create", post(handlers::communities::api_create_community))
        .route("/api/communities/:id/join", post(handlers::communities::api_join_community))
        .route("/api/communities/:id/leave", post(handlers::communities::api_leave_community))
        .route("/api/communities/:id/invite", post(handlers::communities::api_invite_community))
        .route("/api/communities/:id/group/create", post(handlers::communities::api_create_community_group))
        .route("/api/communities/:id/moderate_delete", post(handlers::communities::api_community_moderate_delete))
        // ===== Support (Fase 6) =====
        .route("/support/ticket/:id", get(handlers::support::ticket_view))
        .route("/support/ticket/:id/reply", post(handlers::support::api_ticket_reply))
        // Tiempo real: polling de mensajes + envío AJAX (sin recargar).
        .route("/api/support/ticket/:id/messages", get(handlers::support::api_ticket_messages))
        .route("/api/support/ticket/:id/send", post(handlers::support::api_ticket_send))
        // Ciclo de vida del ticket (admin): reclamar / terminar / rechazar / cerrar.
        .route("/support/ticket/:id/claim", post(handlers::support::api_ticket_claim))
        .route("/support/ticket/:id/resolve", post(handlers::support::api_ticket_resolve))
        .route("/support/ticket/:id/reject", post(handlers::support::api_ticket_reject))
        .route("/support/ticket/:id/close", post(handlers::support::api_ticket_close))
        // ===== Transmitir (Fase 7 — Screen sharing P2P WebRTC) =====
        .route("/transmitir", get(handlers::transmit::panel_page))
        .route("/transmitir/enviar", get(handlers::transmit::sender_page))
        .route("/transmitir/recibir", get(handlers::transmit::receiver_page))
        .route("/transmitir/ws", get(handlers::transmit::ws_handler))
        // ===== Aris (Asistente IA + Núcleo) =====
        .route("/aris", get(handlers::aris::aris_page))
        .route("/api/aris/credits", get(handlers::aris::api_get_credits))
        .route("/api/aris/core/analytics", get(handlers::aris::core::api_core_analytics))
        .route("/api/aris/playground/generate", post(handlers::aris::core::api_playground_generate))
        .route("/api/aris/chats", get(handlers::aris::api_list_chats).post(handlers::aris::api_create_chat))
        .route("/api/aris/chats/:id", delete(handlers::aris::api_delete_chat))
        .route("/api/aris/chats/:id/messages", get(handlers::aris::api_get_messages))
        .route("/api/aris/chats/:id/settings", post(handlers::aris::api_update_chat))
        .route("/api/aris/chats/:id/pin", post(handlers::aris::api_pin_chat))
        .route("/api/aris/chats/:id/send", post(handlers::aris::api_send_message))
        .route("/api/aris/chats/:id/invite", post(handlers::aris::api_invite_member))
        .route("/api/aris/chats/:id/members", get(handlers::aris::api_list_members))
        .route("/api/aris/chats/:id/members/:user_id", delete(handlers::aris::api_remove_member))
        // ===== aexOS + aexTube =====
        .route("/aexos", get(handlers::aexos::aexos_page))
        .route("/api/aexos/profiles", get(handlers::aexos::api_list_profiles).post(handlers::aexos::api_create_profile))
        .route("/api/aexos/profiles/:id/delete", post(handlers::aexos::api_delete_profile))
        .route("/api/aexos/switch", post(handlers::aexos::api_switch_profile))
        .route("/api/aexos/prefs", post(handlers::aexos::api_save_prefs))
        .route("/api/aexos/exit", post(handlers::aexos::api_exit_aexos))
        .route("/aextube", get(handlers::aextube::aextube_page))
        .route("/api/aextube/videos", get(handlers::aextube::api_list_videos))
        .route("/api/aextube/upload", post(handlers::aextube::api_upload_video))
        .route("/api/aextube/videos/:id/react", post(handlers::aextube::api_react_video))
        .route("/api/aextube/videos/:id/view", post(handlers::aextube::api_view_video))
        .route("/api/aextube/videos/:id", delete(handlers::aextube::api_delete_video))
        .route("/api/aextube/channels/:author/subscribe", post(handlers::aextube::api_toggle_subscribe))
        // ===== Sistema de Temas + Onboarding =====
        .route("/tema", get(handlers::theme::tema_page))
        .route("/api/theme/select", post(handlers::theme::api_select_theme))
        .route("/api/theme/status", get(handlers::theme::api_theme_status))
        .layer(from_fn_with_state(state.clone(), auth_mw::require_auth));

    // ===== Rutas de admin que requieren re-desbloqueo (idea 1) =====
    // Todas las /admin/* excepto /admin/unlock (que está en protected_routes).
    // Se aplican DOS layers: require_auth (comprueba sesión) + require_admin_unlock
    // (comprueba cookie de desbloqueo válida).
    let admin_unlocked_routes = Router::new()
        .route("/admin", get(handlers::admin::index))
        .route("/admin/users", get(handlers::admin::users))
        .route("/admin/users/new", get(handlers::admin::new_user_get).post(handlers::admin::new_user_post))
        .route("/admin/users/:id/delete", post(handlers::admin::delete_user))
        .route("/admin/users/:id/force_logout", post(handlers::admin::force_logout))
        .route("/admin/users/:id/ban", post(handlers::admin::ban_user))
        .route("/admin/users/:id/suspend", post(handlers::admin::suspend_user))
        .route("/admin/users/:id/unban", post(handlers::admin::unban_user))
        .route("/admin/users/:id/role", post(handlers::roles::assign_user_role))
        .route("/admin/webs", get(handlers::admin::webs))
        .route("/admin/webs/create", post(handlers::admin::create_web))
        .route("/admin/webs/:id/delete", post(handlers::admin::delete_web))
        .route("/admin/webs/:id/perms", post(handlers::admin::set_web_perms))
        .route("/admin/webs/:id/tag", post(handlers::admin::set_web_tag))
        .route("/admin/webs/:id/toggle", post(handlers::admin::toggle_web_active))
        .route("/admin/webs/:id/html", post(handlers::admin::update_web_html))
        // Roles configurables + permisos granulares por rol.
        .route("/admin/roles", get(handlers::roles::roles))
        .route("/admin/roles/create", post(handlers::roles::create_role))
        .route("/admin/roles/:id/update", post(handlers::roles::update_role))
        .route("/admin/roles/:id/delete", post(handlers::roles::delete_role))
        .route("/admin/roles/:id/perms", post(handlers::roles::set_role_perms))
        .route("/admin/roles/:id/web_perms", post(handlers::roles::set_role_web_perms))
        .route("/admin/audit", get(handlers::admin::audit))
        .route("/admin/config", get(handlers::admin::config_get).post(handlers::admin::config_post))
        .route("/admin/ip_bans", get(handlers::admin::ip_bans))
        .route("/admin/ip_bans/create", post(handlers::admin::create_ip_ban))
        .route("/admin/ip_bans/:id/revoke", post(handlers::admin::revoke_ip_ban))
        .route("/admin/users/:id/ips", get(handlers::admin::api_user_ips))
        .route("/admin/tickets", get(handlers::support::admin_tickets))
        .route("/admin/docs", get(handlers::docs::admin_list))
        .route("/admin/docs/new", get(handlers::docs::admin_new_get).post(handlers::docs::admin_new_post))
        .route("/admin/docs/:id/edit", get(handlers::docs::admin_edit_get).post(handlers::docs::admin_edit_post))
        .route("/admin/docs/:id/regenerate", post(handlers::docs::admin_regenerate))
        .route("/admin/docs/:id/delete", post(handlers::docs::admin_delete))
        // Títulos (badges cosméticos). Solo Owner (permiso titles.manage).
        .route("/admin/titles", get(handlers::titles::admin_titles))
        .route("/admin/titles/create", post(handlers::titles::create_title))
        .route("/admin/titles/:id/update", post(handlers::titles::update_title))
        .route("/admin/titles/:id/delete", post(handlers::titles::delete_title))
        .route("/admin/titles/:id/assign", post(handlers::titles::assign_title))
        .route("/admin/titles/:id/revoke", post(handlers::titles::revoke_title))
        // ===== Descargas nativas (Sección 7) =====
        .route("/admin/downloads", get(handlers::descargas::admin_downloads))
        .route("/admin/downloads/create", post(handlers::descargas::create_download))
        .route("/admin/downloads/:id/delete", post(handlers::descargas::delete_download))
        // ===== Gestión de APIs (Aris) =====
        .route("/admin/apis", get(handlers::aris::keys::admin_apis_page))
        .route("/admin/apis/keys", get(handlers::aris::keys::api_list_keys).post(handlers::aris::keys::api_add_key))
        .route("/admin/apis/keys/reset-cooldowns", post(handlers::aris::keys::api_reset_cooldowns))
        .route("/admin/apis/keys/:id/toggle", post(handlers::aris::keys::api_toggle_key))
        .route("/admin/apis/keys/:id/cap", post(handlers::aris::keys::api_update_key_cap))
        .route("/admin/apis/keys/:id/test", post(handlers::aris::keys::api_test_key))
        .route("/admin/apis/keys/:id", delete(handlers::aris::keys::api_delete_key))
        .route("/admin/apis/providers", get(handlers::aris::keys::api_list_providers).post(handlers::aris::keys::api_add_provider))
        .route("/admin/apis/providers/:id/update", post(handlers::aris::keys::api_update_provider))
        .route("/admin/apis/providers/:id/toggle", post(handlers::aris::keys::api_toggle_provider))
        .route("/admin/apis/providers/:id/priority", post(handlers::aris::keys::api_reorder_provider))
        .route("/admin/apis/providers/:id", delete(handlers::aris::keys::api_delete_provider))
        .route("/admin/apis/config", post(handlers::aris::keys::api_save_aris_config))
        .route("/admin/apis/logs", get(handlers::aris::keys::api_list_rotation_logs).delete(handlers::aris::keys::api_clear_rotation_logs))
        .layer(from_fn_with_state(state.clone(), auth_mw::require_admin_unlock))
        .layer(from_fn_with_state(state.clone(), auth_mw::require_auth));

    let app = Router::new()
        .merge(public_routes)
        .merge(guest_routes)
        .merge(protected_routes)
        .merge(admin_unlocked_routes)
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/chat/images", ServeDir::new("data/chat_images"))
        .nest_service("/chat/files", ServeDir::new("data/chat_files"))
        .nest_service("/avatars", ServeDir::new("data/avatars"))
        .nest_service("/group_images", ServeDir::new("data/group_images"))
        .nest_service("/descargas/files", ServeDir::new("data/downloads"))
        .nest_service("/aextube/media", ServeDir::new("data/aextube"))
        // Fallback: cualquier ruta no matched → intentar servir HTML custom de webs.
        // Si no existe → 404. Esto permite que /slug funcione para secciones custom.
        .fallback(handlers::admin::serve_custom_web)
        .layer(from_fn_with_state(state.clone(), track_web_visits))
        .with_state(state.clone())
        .layer(CookieManagerLayer::new())
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024))
        .layer(from_fn_with_state(state.clone(), client_mw::detect_client))
        .layer(from_fn_with_state(state, auth_mw::require_no_ip_ban));

    let addr: SocketAddr = cfg.listen_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("🛡️  Listo para conexiones…");
    axum::serve(listener, app).await?;
    Ok(())
}

fn trackable_slug_from_path(path: &str) -> Option<String> {
    if path.starts_with("/static/")
        || path.starts_with("/api/")
        || path.starts_with("/chat/images")
        || path.starts_with("/chat/files")
        || path.starts_with("/avatars")
        || path.starts_with("/group_images")
        || path.starts_with("/descargas/files")
        || path.starts_with("/aextube/media")
        || path.starts_with("/pdf-viewer")
        || path == "/role-colors.css"
        || path == "/title-colors.css"
        || path == "/sw.js"
        || path == "/chat/ws"
        || path == "/transmitir/ws"
        || path.contains('.')
    {
        return None;
    }

    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() || trimmed == "hub" {
        return Some("hub".to_string());
    }
    if trimmed == "juegos/neon-canvas" {
        return Some("juegos/neon-canvas".to_string());
    }
    let first = trimmed.split('/').next().unwrap_or("");
    if matches!(
        first,
        "login" | "register" | "recover" | "reset" | "logout" | "vincular" | "privacidad" | "terminos" | "embed" | "d"
    ) {
        return None;
    }
    if first == "community" {
        return Some("communities".to_string());
    }
    if first == "categoria" {
        return Some("categorias".to_string());
    }
    if !first.is_empty() {
        return Some(first.to_string());
    }
    None
}

async fn track_web_visits(
    axum::extract::State(state): axum::extract::State<AppState>,
    cookies: tower_cookies::Cookies,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let is_get = *req.method() == axum::http::Method::GET;
    let accept_html = req
        .headers()
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html") || s.contains("*/*"))
        .unwrap_or(true);
    let slug = if is_get && accept_html {
        trackable_slug_from_path(req.uri().path())
    } else {
        None
    };

    let res = next.run(req).await;

    if let Some(slug) = slug {
        if res.status().is_success() {
            let db = state.db.clone();
            let cookie_name = state.cfg.cookie_name.clone();
            let token = cookies.get(&cookie_name).map(|c| c.value().to_string());
            tokio::spawn(async move {
                let (uid, role_key) = if let Some(tok) = token {
                    if let Ok(Some(u)) = db.get_session_user(&tok).await {
                        (Some(u.id), u.role)
                    } else {
                        (None, "invitado".to_string())
                    }
                } else {
                    (None, "invitado".to_string())
                };
                let _ = db.record_web_visit(&slug, uid, &role_key).await;
            });
        }
    }

    res
}

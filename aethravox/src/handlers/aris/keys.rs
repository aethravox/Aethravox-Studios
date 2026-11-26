//! Gestión de API keys y proveedores ilimitados (solo admin).
//!
//! - Proveedores predeterminados editables + creación ilimitada de nuevos proveedores.
//! - Keys ilimitadas por proveedor (alta individual o por lote), con tope diario preventivo.
//! - Estrategias de rotación configurables (round_robin, least_used, provider_order).
//! - Health check directo por key, liberación de cooldowns, configuración de Aris y bitácora.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Form, Json,
};
use serde::Deserialize;

use crate::auth_mw::AuthUser;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct AddKeyForm {
    pub provider_id: i64,
    pub api_key: String,
    pub label: Option<String>,
    pub daily_cap: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleForm {
    pub active: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyCapForm {
    pub daily_cap: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderForm {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub default_model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderProviderForm {
    pub delta: i64,
}

#[derive(Debug, Deserialize)]
pub struct ArisConfigForm {
    pub rotation_enabled: Option<String>,
    pub rotation_strategy: Option<String>,
    pub cooldown_minutes: Option<String>,
    pub auto_failover: Option<String>,
    pub ollama_url: Option<String>,
    pub hourly_limit: Option<String>,
    pub daily_limit: Option<String>,
    pub daily_credits_limit: Option<String>,
    pub aethramail_url: Option<String>,
    pub system_prompt: Option<String>,
}

fn parse_opt_positive_i64(val: Option<&str>) -> Option<i64> {
    val.and_then(|s| s.trim().parse::<i64>().ok())
        .filter(|&n| n > 0)
}

/// GET /admin/apis — Página de gestión de APIs.
pub async fn admin_apis_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return axum::response::Redirect::to("/").into_response();
    }

    let keys = state.db.list_api_keys().await.unwrap_or_default();
    let providers = state.db.list_api_providers_full().await.unwrap_or_default();
    let logs = state.db.list_rotation_logs(40).await.unwrap_or_default();

    let rotation_enabled = state.db.aris_get_config("rotation_enabled").await
        .ok().flatten().map(|v| v.trim() != "0" && v.trim() != "false").unwrap_or(true);
    let rotation_strategy = state.db.aris_get_config("rotation_strategy").await
        .ok().flatten().unwrap_or_else(|| "round_robin".to_string());
    let cooldown_minutes = state.db.aris_get_config("cooldown_minutes").await
        .ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(60);
    let auto_failover = state.db.aris_get_config("auto_failover").await
        .ok().flatten().map(|v| v.trim() != "0" && v.trim() != "false").unwrap_or(true);

    let ollama_url = state.db.aris_get_config("ollama_url").await
        .ok().flatten().unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let hourly_limit = state.db.aris_get_config("hourly_limit").await
        .ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(30);
    let daily_limit = state.db.aris_get_config("daily_limit").await
        .ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(150);
    let daily_credits_limit = state.db.aris_get_config("daily_credits_limit").await
        .ok().flatten().and_then(|v| v.parse::<i64>().ok()).map(|n| n.max(10)).unwrap_or(120);
    let aethramail_url = state.db.aris_get_config("aethramail_url").await
        .ok().flatten().unwrap_or_default();
    let system_prompt = state.db.aris_get_config("system_prompt").await
        .ok().flatten().unwrap_or_default();

    crate::templates::AdminApisPage {
        user,
        keys,
        providers,
        logs,
        rotation_enabled,
        rotation_strategy,
        cooldown_minutes,
        auto_failover,
        ollama_url,
        hourly_limit,
        daily_limit,
        daily_credits_limit,
        aethramail_url,
        system_prompt,
    }
    .into_response()
}

/// POST /admin/apis/keys — Agregar una o varias keys (separadas por salto de línea).
pub async fn api_add_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<AddKeyForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    let lines: Vec<&str> = form.api_key
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Json(serde_json::json!({"error": "key_vacia"})).into_response();
    }

    let base_label = form.label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .unwrap_or("key")
        .to_string();

    let daily_cap = parse_opt_positive_i64(form.daily_cap.as_deref());

    if lines.len() == 1 {
        match state.db.add_api_key(form.provider_id, lines[0], &base_label, daily_cap).await {
            Ok(id) => Json(serde_json::json!({"ok": true, "id": id, "added": 1})).into_response(),
            Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
        }
    } else {
        let mut added = 0i64;
        for (idx, line) in lines.iter().enumerate() {
            let lbl = format!("{} #{}", base_label, idx + 1);
            if state.db.add_api_key(form.provider_id, line, &lbl, daily_cap).await.is_ok() {
                added += 1;
            }
        }
        if added > 0 {
            Json(serde_json::json!({"ok": true, "added": added})).into_response()
        } else {
            Json(serde_json::json!({"error": "server"})).into_response()
        }
    }
}

/// POST /admin/apis/keys/:id/toggle — Activar/desactivar key.
pub async fn api_toggle_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(key_id): Path<i64>,
    Form(form): Form<ToggleForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let active = matches!(form.active.trim(), "1" | "true" | "on");
    let _ = state.db.toggle_api_key(key_id, active).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

/// POST /admin/apis/keys/:id/cap — Actualizar tope diario preventivo de una key.
pub async fn api_update_key_cap(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(key_id): Path<i64>,
    Form(form): Form<UpdateKeyCapForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let daily_cap = parse_opt_positive_i64(form.daily_cap.as_deref());
    match state.db.update_api_key_daily_cap(key_id, daily_cap).await {
        Ok(_) => Json(serde_json::json!({"ok": true, "daily_cap": daily_cap})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// POST /admin/apis/keys/:id/test — Health Check real contra el endpoint del proveedor.
pub async fn api_test_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(key_id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    match crate::handlers::aris::external::test_api_key(&state.db.pool, key_id).await {
        Ok((healthy, detail, latency_ms)) => Json(serde_json::json!({
            "ok": true,
            "healthy": healthy,
            "detail": detail,
            "latency_ms": latency_ms,
        }))
        .into_response(),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})).into_response(),
    }
}

/// POST /admin/apis/keys/reset-cooldowns — Libera todas las keys en cooldown 429.
pub async fn api_reset_cooldowns(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let _ = state.db.reset_all_rate_limits().await;
    Json(serde_json::json!({"ok": true})).into_response()
}

/// DELETE /admin/apis/keys/:id — Borrar key.
pub async fn api_delete_key(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(key_id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let _ = state.db.delete_api_key(key_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

/// GET /admin/apis/keys — JSON con estado de las keys.
pub async fn api_list_keys(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let keys = state.db.list_api_keys().await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "keys": keys})).into_response()
}

// ============================================================
// Gestión ilimitada de proveedores
// ============================================================

/// GET /admin/apis/providers
pub async fn api_list_providers(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let providers = state.db.list_api_providers_full().await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "providers": providers})).into_response()
}

/// POST /admin/apis/providers — Crear proveedor nuevo (sin límite por tipo).
pub async fn api_add_provider(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<ProviderForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    let name = form.name.trim();
    let provider_type = form.provider_type.trim();
    let base_url = form.base_url.trim();
    let default_model = form.default_model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("default");

    if name.is_empty() || base_url.is_empty() || provider_type.is_empty() {
        return Json(serde_json::json!({"error": "campos_incompletos"})).into_response();
    }

    match state.db.add_api_provider(name, provider_type, base_url, default_model).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// POST /admin/apis/providers/:id/update — Editar nombre, tipo, base_url y modelo.
pub async fn api_update_provider(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(provider_id): Path<i64>,
    Form(form): Form<ProviderForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    let name = form.name.trim();
    let provider_type = form.provider_type.trim();
    let base_url = form.base_url.trim();
    let default_model = form.default_model
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("default");

    if name.is_empty() || base_url.is_empty() || provider_type.is_empty() {
        return Json(serde_json::json!({"error": "campos_incompletos"})).into_response();
    }

    match state.db.update_api_provider(provider_id, name, provider_type, base_url, default_model).await {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// POST /admin/apis/providers/:id/toggle — Pausar/activar proveedor.
pub async fn api_toggle_provider(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(provider_id): Path<i64>,
    Form(form): Form<ToggleForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let active = matches!(form.active.trim(), "1" | "true" | "on");
    let _ = state.db.toggle_api_provider(provider_id, active).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

/// POST /admin/apis/providers/:id/priority — Subir/bajar prioridad.
pub async fn api_reorder_provider(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(provider_id): Path<i64>,
    Form(form): Form<ReorderProviderForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let delta = if form.delta < 0 { -1 } else { 1 };
    let _ = state.db.move_api_provider_priority(provider_id, delta).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

/// DELETE /admin/apis/providers/:id — Eliminar proveedor personalizado.
pub async fn api_delete_provider(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(provider_id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let providers = state.db.list_api_providers_full().await.unwrap_or_default();
    if let Some(p) = providers.into_iter().find(|p| p.id == provider_id) {
        if p.is_preset == 1 {
            return Json(serde_json::json!({"error": "preset_protegido"})).into_response();
        }
    }
    let _ = state.db.delete_api_provider(provider_id).await;
    Json(serde_json::json!({"ok": true})).into_response()
}

// ============================================================
// Configuración de Aris & Rotación + Bitácora
// ============================================================

/// POST /admin/apis/config — Guarda parámetros de rotación y límites de Aris.
pub async fn api_save_aris_config(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<ArisConfigForm>,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }

    if let Some(v) = form.rotation_enabled {
        let norm = if matches!(v.trim(), "0" | "false" | "off") { "0" } else { "1" };
        let _ = state.db.aris_set_config("rotation_enabled", norm).await;
    }
    if let Some(v) = form.rotation_strategy {
        let s = match v.trim() {
            "least_used" => "least_used",
            "provider_order" => "provider_order",
            _ => "round_robin",
        };
        let _ = state.db.aris_set_config("rotation_strategy", s).await;
    }
    if let Some(v) = form.cooldown_minutes {
        if let Ok(mins) = v.trim().parse::<i64>() {
            let clamped = mins.clamp(1, 1440);
            let _ = state.db.aris_set_config("cooldown_minutes", &clamped.to_string()).await;
        }
    }
    if let Some(v) = form.auto_failover {
        let norm = if matches!(v.trim(), "0" | "false" | "off") { "0" } else { "1" };
        let _ = state.db.aris_set_config("auto_failover", norm).await;
    }
    if let Some(v) = form.ollama_url {
        let t = v.trim();
        if !t.is_empty() {
            let _ = state.db.aris_set_config("ollama_url", t).await;
        }
    }
    if let Some(v) = form.hourly_limit {
        if let Ok(n) = v.trim().parse::<i64>() {
            let _ = state.db.aris_set_config("hourly_limit", &n.max(1).to_string()).await;
        }
    }
    if let Some(v) = form.daily_limit {
        if let Ok(n) = v.trim().parse::<i64>() {
            let _ = state.db.aris_set_config("daily_limit", &n.max(1).to_string()).await;
        }
    }
    if let Some(v) = form.daily_credits_limit {
        if let Ok(n) = v.trim().parse::<i64>() {
            let _ = state.db.aris_set_config("daily_credits_limit", &n.max(10).to_string()).await;
        }
    }
    if let Some(v) = form.aethramail_url {
        let t = v.trim();
        let _ = state.db.aris_set_config("aethramail_url", t).await;
    }
    if let Some(v) = form.system_prompt {
        let t = v.trim();
        if !t.is_empty() {
            let _ = state.db.aris_set_config("system_prompt", t).await;
        }
    }

    Json(serde_json::json!({"ok": true})).into_response()
}

/// GET /admin/apis/logs — JSON con últimas entradas de bitácora.
pub async fn api_list_rotation_logs(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let logs = state.db.list_rotation_logs(50).await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "logs": logs})).into_response()
}

/// DELETE /admin/apis/logs — Vacía la bitácora de rotación.
pub async fn api_clear_rotation_logs(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    if !state.db.user_has_perm(&user, "admin.apis").await {
        return Json(serde_json::json!({"error": "no_autorizado"})).into_response();
    }
    let _ = state.db.clear_rotation_logs().await;
    Json(serde_json::json!({"ok": true})).into_response()
}

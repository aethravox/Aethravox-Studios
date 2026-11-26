#![allow(unused_imports)]
// Web 2 — Panel de administración (Owner + Admin).

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use tower_cookies::Cookies;

use crate::{
    auth_mw::AuthUser,
    models::{
        ActionOnUser, AdminCreateUserForm, BanForm, ChangeRoleForm, CreateWebForm,
        SuspendForm, UpdateConfigForm, User, WebPermsForm,
    },
    AppState,
};

// ===== Idea 1: re-autenticación para /admin/* =====


use super::audit_view::parse_local_datetime_to_utc;

pub async fn serve_custom_web(
    State(state): State<AppState>,
    cookies: Cookies,
    OriginalUri(uri): OriginalUri,
) -> Response {
    // El fallback no tiene ruta con `:param`, así que extraemos el slug del
    // primer segmento del path a mano. Sin esto, `Path<String>` revienta con
    // "se esperaba 1, se obtuvo 0" para CUALQUIER URL desconocida.
    let slug = uri.path().trim_start_matches('/').split('/').next().unwrap_or("");

    // Resolver usuario logueado (None si no hay sesión válida). El fallback
    // corre FUERA de las capas de auth de protected/admin_routes, así que
    // leemos la sesión a mano igual que `auth_mw::require_auth`. Sin esto,
    // saber la URL de una sección privada (aexOS, etc.) te daba acceso directo
    // sin importar el tag ni siquiera si habías iniciado sesión.
    let user = crate::helpers::current_user(&state.db, &cookies, &state.cfg.cookie_name).await;

    // Buscar la web (cualquiera activa por slug, no solo custom).
    let web = match state.db.get_web_by_slug(slug).await {
        Ok(Some(w)) => w,
        _ => return (axum::http::StatusCode::NOT_FOUND, "Página no encontrada").into_response(),
    };

    // ===== Control de acceso (igual que el hub: list_webs_for_user) =====
    // is_public=1 → cualquiera (logueado o no). Si no, necesita permiso de
    // rol para esta web. Sin permiso y sin sesión → login (con
    // retorno); con sesión pero sin permiso → 404 (no revelar existencia).
    let has_access = web.is_public != 0
        || match &user {
            Some(u) => state
                .db
                .user_obj_can_access_web(u, web.id)
                .await
                .unwrap_or(false),
            None => false,
        };
    if !has_access {
        return match &user {
            None => Redirect::to(&format!("/login?next=/{}", slug)).into_response(),
            Some(_) => (axum::http::StatusCode::NOT_FOUND, "Página no encontrada").into_response(),
        };
    }

    // ===== Tiene acceso. ¿Puede "entrar" o ve "en construcción"? =====
    // is_clickable_for(role) ya codifica tag + rol: Próximamente/Mantenimiento
    // → no clickable para nadie; "En proceso" → solo owner/admin/staff. Si no
    // es clickable, mostramos la página de construcción (equivalente al modal
    // del hub), nunca el contenido real. Así ni sabiendo la URL se entra a una
    // sección que aún no debe abrirse.
    let role = user.as_ref().map(|u| u.role.as_str()).unwrap_or("");
    if !web.is_clickable_for(role) {
        return web_construction_response(&web);
    }

    // Clickable + web custom con HTML propio → servir el HTML.
    if web.is_custom != 0 {
        if let Some(html) = web.html_content.as_deref() {
            if !html.is_empty() {
                let mut resp = axum::response::Html(html.to_owned()).into_response();
                let headers = resp.headers_mut();
                headers.insert(
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
                );
                return resp;
            }
        }
    }

    // Clickable pero sin HTML propio ni ruta dedicada → página "en desarrollo"
    // graceful (no 404).
    web_construction_response(&web)
}

/// Construye la página "en desarrollo" para una web sin contenido propio.
fn web_construction_response(web: &crate::models::Web) -> Response {
    let opens_at = web
        .tag_opens_at
        .as_ref()
        .map(|d| d.format("%d/%m/%Y a las %H:%M").to_string());
    WebConstructionTemplate {
        name: web.name.clone(),
        icon_emoji: web.icon_emoji(),
        message: web.tag_display_message(),
        has_opens_at: opens_at.is_some(),
        opens_at: opens_at.unwrap_or_default(),
    }
    .into_response()
}

/// Plantilla para secciones que existen en el hub pero todavía no tienen
/// página propia (tag "En proceso" / "Próximamente" / "Mantenimiento").
/// Evita el 404/crash cuando alguien entra directo a su URL.
#[derive(Template)]
#[template(path = "admin/webs/web_construction.html")]
pub struct WebConstructionTemplate {
    pub name: String,
    pub icon_emoji: String,
    pub message: String,
    pub has_opens_at: bool,
    pub opens_at: String,
}

pub async fn delete_web(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.delete").await {
        return Redirect::to("/admin").into_response();
    }
    // PROTECCIÓN: slugs del sistema NO se pueden borrar, ni siquiera el owner.
    // Evita que un owner (comprometido o por error) borre 'login', 'hub', etc.
    // y rompa toda la app.
    if let Ok(Some(web)) = state.db.get_web_by_id(id).await {
        if is_system_slug(&web.slug) {
            let _ = state.db.audit(
                Some(actor.id),
                "web_delete_blocked_system",
                Some(&format!("id={} slug={} (SYSTEM, refused)", id, web.slug)),
                None,
            ).await;
            return Redirect::to("/admin/webs?err=system_undeletable").into_response();
        }
    }
    let _ = state.db.delete_web(id).await;
    let detail = format!("id={}", id);
    let _ = state
        .db
        .audit(Some(actor.id), "web_delete", Some(&detail), None)
        .await;
    Redirect::to("/admin/webs?ok=deleted").into_response()
}

/// Slugs CORE del sistema: NO se pueden desactivar NI eliminar.
/// Sin estos la app no funciona (autenticación, navegación, admin).
/// Siempre están activos.
pub fn is_core_slug(slug: &str) -> bool {
    matches!(slug,
        "login" | "register" | "recover" | "reset" |
        "hub" | "dashboard" | "admin"
    )
}

/// Slugs de FEATURES: se pueden desactivar (ocultar del hub) pero NO eliminar
/// (los insertan las migraciones y se recrearían al reiniciar).
pub fn is_feature_slug(slug: &str) -> bool {
    matches!(slug,
        "chat" | "friends" | "playground" | "docs"
    )
}

/// Slugs que NO se pueden eliminar (core + feature).
/// Custom HTML sí se puede eliminar.
pub fn is_system_slug(slug: &str) -> bool {
    is_core_slug(slug) || is_feature_slug(slug)
}

pub async fn set_web_perms(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<WebPermsForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.perms").await {
        return Redirect::to("/admin").into_response();
    }
    let mut roles: Vec<&str> = Vec::new();
    if form.owner.is_some() {
        roles.push("owner");
    }
    if form.admin.is_some() {
        roles.push("admin");
    }
    if form.staff.is_some() {
        roles.push("staff");
    }
    if form.usuario.is_some() {
        roles.push("usuario");
    }
    let _ = state.db.set_web_perms(id, &roles).await;
    let detail = format!("id={} perms={:?}", id, roles);
    let _ = state
        .db
        .audit(Some(actor.id), "web_perms", Some(&detail), None)
        .await;
    Redirect::to("/admin/webs?ok=perms").into_response()
}

// ===== POST /admin/webs/:id/tag =====
#[derive(Deserialize)]
pub struct WebTagForm {
    pub tag: Option<String>,
    #[serde(default)]
    pub tag_message: Option<String>,
    /// ISO datetime-local (ej: "2026-11-15T18:00"). Se parsea a UTC.
    #[serde(default)]
    pub tag_opens_at: Option<String>,
}

pub async fn set_web_tag(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<WebTagForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.tag").await {
        return Redirect::to("/admin").into_response();
    }
    let tag = form.tag.as_deref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty() && s != "activo");

    // Mensaje: trim, si queda vacío → NULL.
    let tag_message = form.tag_message.as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Fecha: tiene sentido para "próximamente", "mantenimiento" y "en_proceso"
    // (estimación de entrega). Para otros tags, ignoramos la fecha (NULL).
    let tag_opens_at: Option<chrono::DateTime<chrono::Utc>> = match tag.as_deref() {
        Some(t) if t == "próximamente" || t == "proximamente" || t == "mantenimiento" || t == "en_proceso" || t == "proceso" => {
            form.tag_opens_at.as_deref()
                .filter(|s| !s.is_empty())
                .and_then(|s| parse_local_datetime_to_utc(s))
        }
        _ => None,
    };

    // Actualizar las 3 columnas en una sola query.
    let _ = sqlx::query(
        "UPDATE webs SET tag = ?, tag_message = ?, tag_opens_at = ? WHERE id = ?",
    )
    .bind(tag.as_deref())
    .bind(&tag_message)
    .bind(tag_opens_at)
    .bind(id)
    .execute(&state.db.pool)
    .await;

    let _ = state.db.audit(
        Some(actor.id),
        "web_tag_change",
        Some(&format!("id={} tag={:?} msg={:?} opens_at={:?}",
            id, tag, tag_message, tag_opens_at)),
        None,
    ).await;
    Redirect::to("/admin/webs?ok=tag").into_response()
}

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


use super::web_custom::{is_core_slug, is_feature_slug, is_system_slug};
use super::config::urlencoding;

pub async fn webs(State(state): State<AppState>, AuthUser(actor): AuthUser) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.view").await {
        return Redirect::to("/admin").into_response();
    }

    let webs = state.db.list_all_webs().await.unwrap_or_default();
    let mut perms_map: HashMap<i64, Vec<String>> = HashMap::new();
    for w in &webs {
        if let Ok(roles) = state.db.web_perms(w.id).await {
            perms_map.insert(w.id, roles);
        }
    }

    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    AdminWebsTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        webs: webs
            .iter()
            .map(|w| WebRow {
                id: w.id,
                slug: w.slug.clone(),
                name: w.name.clone(),
                description: w.description.clone().unwrap_or_default(),
                icon: w.icon.clone().unwrap_or_else(|| "📄".into()),
                url: w.url.clone(),
                is_public: w.is_public != 0,
                created_at: w.created_at.format("%d/%m/%Y").to_string(),
                perms: perms_map.remove(&w.id).unwrap_or_default(),
                tag: w.tag.clone(),
                tag_message: w.tag_message.clone(),
                tag_opens_at: w.tag_opens_at.map(|dt| dt.format("%Y-%m-%dT%H:%M").to_string()),
                is_system: is_system_slug(&w.slug),
                is_core: is_core_slug(&w.slug),
                is_feature: is_feature_slug(&w.slug),
                is_active: w.is_active != 0,
                is_custom: w.is_custom != 0,
                has_html: w.html_content.is_some(),
                has_owner_perm: perms_map.get(&w.id).map_or(false, |p| p.contains(&"owner".to_string())),
                has_admin_perm: perms_map.get(&w.id).map_or(false, |p| p.contains(&"admin".to_string())),
                has_staff_perm: perms_map.get(&w.id).map_or(false, |p| p.contains(&"staff".to_string())),
                has_usuario_perm: perms_map.get(&w.id).map_or(false, |p| p.contains(&"usuario".to_string())),
            })
            .collect(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "admin/webs/admin_webs.html")]
pub struct AdminWebsTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub webs: Vec<WebRow>,
}

pub struct WebRow {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub url: String,
    pub is_public: bool,
    pub created_at: String,
    pub perms: Vec<String>,
    pub tag: Option<String>,
    pub tag_message: Option<String>,
    /// Fecha en formato HTML datetime-local (ej: "2026-11-15T18:00").
    pub tag_opens_at: Option<String>,
    /// True si es sección del sistema (CORE o FEATURE) que NO se puede borrar.
    pub is_system: bool,
    /// True si es sección CORE (login, hub, admin) — NO se puede desactivar.
    pub is_core: bool,
    /// True si es FEATURE (chat, friends, docs, playground) — se puede desactivar, no borrar.
    pub is_feature: bool,
    /// True si está activa (visible en el hub), False si está desactivada.
    pub is_active: bool,
    /// True si es custom (creada por el owner, se puede eliminar), False si es del sistema.
    pub is_custom: bool,
    /// True si tiene HTML custom (se sirve en /slug). False si usa URL externa o ruta hardcoded.
    pub has_html: bool,
    // Permisos pre-calculados (para no llamar has_perm() con string args en el template).
    pub has_owner_perm: bool,
    pub has_admin_perm: bool,
    pub has_staff_perm: bool,
    pub has_usuario_perm: bool,
}

impl WebRow {
    pub fn has_perm(&self, role: &str) -> bool {
        self.perms.iter().any(|p| p == role)
    }
    pub fn tag_normalized(&self) -> String {
        self.tag.as_ref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).unwrap_or_else(|| "activo".to_string())
    }
}

pub async fn create_web(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<CreateWebForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.create").await {
        return Redirect::to("/admin").into_response();
    }
    if let Err(e) = crate::models::validate_slug(&form.slug) {
        return Redirect::to(&format!("/admin/webs?err={}", urlencoding(&e))).into_response();
    }
    // No se puede crear con un slug del sistema (login, hub, etc.) — conflict con rutas hardcoded.
    if is_system_slug(form.slug.trim()) {
        return Redirect::to("/admin/webs?err=system_slug").into_response();
    }
    if form.name.trim().is_empty() {
        return Redirect::to("/admin/webs?err=campos_vacios").into_response();
    }
    let is_public = form.is_public.is_some();

    // Determinar tipo de sección: external (URL externa) o custom (HTML propio).
    // IMPORTANTE: TODAS las secciones creadas por el owner son "custom" (is_custom=true),
    // independientemente de si son HTML o URL externa. Solo el sistema (insertado por
    // migraciones) tiene is_custom=false. Sin esto, las URL externas no se pueden eliminar
    // desde el admin porque la UI las trata como "sistema".
    let url_type = form.url_type.as_deref().unwrap_or("external").trim();
    let is_custom = true;
    let (html_content, url_to_save) = match url_type {
        "custom" => {
            let html = form.html_content.as_deref().unwrap_or("").trim();
            if html.is_empty() {
                return Redirect::to("/admin/webs?err=html_empty").into_response();
            }
            // Para custom HTML, la URL es siempre /slug (la ruta donde se sirve el HTML).
            (Some(html), format!("/{}", form.slug.trim()))
        }
        _ => {
            // External: validar que URL no esté vacía.
            if form.url.trim().is_empty() {
                return Redirect::to("/admin/webs?err=campos_vacios").into_response();
            }
            (None, form.url.trim().to_string())
        }
    };

    let res = state
        .db
        .create_web(
            form.slug.trim(),
            form.name.trim(),
            form.description.as_deref().map(|s| s.trim()),
            form.icon.as_deref().map(|s| s.trim()),
            &url_to_save,
            is_public,
            is_custom,
            html_content,
        )
        .await;
    match res {
        Ok(_id) => {
            let detail = format!("{} ({}) [{}]", form.name, form.slug, url_type);
            let _ = state
                .db
                .audit(Some(actor.id), "web_create", Some(&detail), None)
                .await;
            Redirect::to("/admin/webs?ok=created").into_response()
        }
        Err(e) => {
            tracing::error!("Error creando web: {e}");
            Redirect::to("/admin/webs?err=db").into_response()
        }
    }
}

/// POST /admin/webs/:id/toggle — Activa o desactiva una sección.
/// El body envía `active=1` (activar) o nada (desactivar).
/// PROTECCIÓN: las secciones CORE (login, hub, admin, etc.) NO se pueden desactivar.
/// Las FEATURE (chat, friends, docs, playground) SÍ se pueden desactivar.
/// Las CUSTOM se pueden activar/desactivar libremente.
pub async fn toggle_web_active(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<ToggleActiveForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.toggle").await {
        return Redirect::to("/admin").into_response();
    }
    let active = form.active.is_some();

    // Cargar la web para comprobar su slug.
    if let Ok(Some(web)) = state.db.get_web_by_id(id).await {
        // Las CORE no se pueden desactivar (pero sí "activar" — no-op si ya activa).
        if !active && is_core_slug(&web.slug) {
            let _ = state.db.audit(
                Some(actor.id),
                "web_toggle_blocked_core",
                Some(&format!("id={} slug={} (CORE, refused)", id, web.slug)),
                None,
            ).await;
            return Redirect::to("/admin/webs?err=core_undeactivatable").into_response();
        }
    } else {
        return Redirect::to("/admin/webs?err=not_found").into_response();
    }

    let _ = state.db.set_web_active(id, active).await;
    let _ = state.db.audit(
        Some(actor.id),
        "web_toggle_active",
        Some(&format!("id={} active={}", id, active)),
        None,
    ).await;
    let qs = if active { "ok=activated" } else { "ok=deactivated" };
    Redirect::to(&format!("/admin/webs?{}", qs)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct ToggleActiveForm {
    /// Si está presente → activar (1). Si no → desactivar (0).
    #[serde(default)]
    pub active: Option<String>,
}

/// POST /admin/webs/:id/html — Actualiza el HTML content de una sección custom.
pub async fn update_web_html(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<crate::models::UpdateWebHtmlForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "webs.html.edit").await {
        return Redirect::to("/admin").into_response();
    }
    // Solo secciones custom pueden tener HTML editado.
    if let Ok(Some(web)) = state.db.get_web_by_id(id).await {
        if web.is_custom == 0 {
            return Redirect::to("/admin/webs?err=not_custom").into_response();
        }
    } else {
        return Redirect::to("/admin/webs?err=not_found").into_response();
    }
    let _ = state.db.update_web_html(id, form.html_content.trim()).await;
    let _ = state.db.audit(
        Some(actor.id),
        "web_html_updated",
        Some(&format!("id={}", id)),
        None,
    ).await;
    Redirect::to("/admin/webs?ok=html_updated").into_response()
}

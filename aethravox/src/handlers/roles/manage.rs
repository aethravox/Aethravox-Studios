#![allow(unused_imports)]
// handlers/roles.rs — gestión de roles configurables + permisos granulares.
//
// Reemplaza la noción de "4 roles fijos hardcoded" por roles configurables
// desde el admin. Cada rol agrupa:
//   (a) capabilities atómicas (role_perms) — ban, ban-ip, entrar a /admin,
//       ver usuarios, borrar usuarios, etc. CADA acción del sistema es un
//       permiso individual: si querés dar a alguien "solo banear IP", creás
//       un rol con solo ip_bans.create + ip_bans.view activados y nada más.
//   (b) secciones visibles (role_web_perms) — qué webs del hub ve.
//   (c) posición jerárquica (position) — para can_act_on.
//
// Los 4 roles base (Owner/Admin/Staff/Usuario) son is_system=1: no se pueden
// borrar (sí editar). El Owner además es is_protected=1: no se puede cambiar
// name/role_key/position ni quitarle permisos.
//
// Rangos (`ranks` + `rank_perms`) SIGUEN existiendo como bundle ADITIVO
// extra opcional. Un usuario = 1 rol (obligatorio) + 0 o 1 rango (opcional).
//
// Todos los handlers requieren `roles.manage`. Excepto assign_user_role que
// requiere `users.role.change`.

use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Path, RawForm, State},
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;

use crate::auth_mw::AuthUser;
use crate::models::{PermissionDef, PERMISSIONS, Role, RoleForm, User};
use crate::AppState;

// ===== Estructuras para el template =====


use super::assign::{parse_form_array, percent_decode};

#[derive(Template)]
#[template(path = "admin/roles/admin_roles.html")]
pub struct AdminRolesTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub roles: Vec<RoleRow>,
    pub all_perms: Vec<PermGroup>,
    pub all_webs: Vec<WebRow>,
}

/// Una entrada del catálogo PERMISSIONS con flag `checked` precomputado
/// para un rol concreto (evita llamar `contains` desde el template askama,
/// que no soporta closures ni expresiones con `&`).
pub struct PermCheck {
    pub key: &'static str,
    pub label: &'static str,
    pub checked: bool,
}

/// Una sección (web) con flag `checked` precomputado para un rol concreto.
pub struct WebCheck {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub icon: String,
    pub checked: bool,
}

pub struct RoleRow {
    pub id: i64,
    pub name: String,
    pub role_key: String,
    pub color: Option<String>,
    pub position: i64,
    pub description: Option<String>,
    pub is_system: bool,
    pub is_protected: bool,
    pub perm_count: usize,
    pub web_count: usize,
    pub user_count: i64,
    /// Permisos del catálogo con flag `checked` precomputado.
    pub perm_checks: Vec<PermGroupChecked>,
    /// Secciones con flag `checked` precomputado.
    pub web_checks: Vec<WebCheck>,
}

/// Grupo de permisos (por categoría) con cada permiso + su flag checked.
pub struct PermGroupChecked {
    pub category: String,
    pub perms: Vec<PermCheck>,
}

pub struct PermGroup {
    pub category: String,
    pub perms: Vec<PermissionDef>,
}

pub struct WebRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub icon: String,
}

// ===== Form extra (DeleteRoleForm, no está en models.rs) =====

#[derive(Deserialize)]
pub struct DeleteRoleForm {
    pub reassign_to: Option<String>,
}

// ===== Helper: ¿el actor puede actuar sobre el target? =====
//
// Replica la lógica de can_act_on pero usando Role.position. El owner actúa
// sobre todos (salvo borrarse a sí mismo). Otros: no sobre sí mismos; sobre
// cualquier usuario común (position <= 100) o sobre alguien de position
// estrictamente inferior.

pub fn can_act_on_role(
    actor: &User,
    actor_role: &Role,
    target: &User,
    target_role: &Role,
) -> bool {
    if actor_role.is_owner() {
        return actor.id != target.id;
    }
    actor.id != target.id
        && (target_role.position <= 100 || target_role.position < actor_role.position)
}

// ===== GET /admin/roles =====

pub async fn roles(State(state): State<AppState>, AuthUser(actor): AuthUser) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    let all_roles = state.db.list_roles().await.unwrap_or_default();
    let all_webs = state.db.list_all_webs().await.unwrap_or_default();

    let mut rows = Vec::with_capacity(all_roles.len());
    for r in all_roles {
        let granted_perms: std::collections::HashSet<String> = state
            .db
            .role_perms(r.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();
        let granted_webs: std::collections::HashSet<i64> = state
            .db
            .role_web_perms(r.id)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();
        let user_count = state.db.count_users_with_role(r.id).await.unwrap_or(0);

        // Permisos agrupados por categoría, con flag checked precomputado.
        let mut groups: HashMap<String, Vec<PermissionDef>> = HashMap::new();
        for p in PERMISSIONS.iter() {
            groups.entry(p.category.to_string()).or_default().push(*p);
        }
        let mut perm_checks: Vec<PermGroupChecked> = groups
            .into_iter()
            .map(|(category, perms)| PermGroupChecked {
                category: category.clone(),
                perms: perms
                    .iter()
                    .map(|p| PermCheck {
                        key: p.key,
                        label: p.label,
                        checked: granted_perms.contains(p.key),
                    })
                    .collect(),
            })
            .collect();
        perm_checks.sort_by(|a, b| a.category.cmp(&b.category));

        // Secciones con flag checked precomputado.
        let web_checks: Vec<WebCheck> = all_webs
            .iter()
            .map(|w| WebCheck {
                id: w.id,
                name: w.name.clone(),
                slug: w.slug.clone(),
                icon: w.icon.clone().unwrap_or_default(),
                checked: granted_webs.contains(&w.id),
            })
            .collect();

        rows.push(RoleRow {
            id: r.id,
            name: r.name.clone(),
            role_key: r.role_key.clone(),
            color: r.color.clone(),
            position: r.position,
            description: r.description.clone(),
            is_system: r.is_system(),
            is_protected: r.is_protected(),
            perm_count: granted_perms.len(),
            web_count: granted_webs.len(),
            user_count,
            perm_checks,
            web_checks,
        });
    }

    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    AdminRolesTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        roles: rows,
        all_perms: Vec::new(),
        all_webs: Vec::new(),
    }
    .into_response()
}

// ===== POST /admin/roles/create =====

pub async fn create_role(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<RoleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    let name = form.name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Redirect::to("/admin/roles").into_response();
    }
    // El color puede ser un hex suelto ("#8b5cf6") o el composite del color
    // picker ("#hex|text|opacity" o "#hex~#hex2|text|opacity"). Aceptamos
    // cualquier string no vacío que empiece por '#' (el picker siempre produce
    // '#…'); role_color::parse_role_color lo resuelve a CSS al servir
    // /role-colors.css y /api/role_colors.
    let color = form
        .color
        .as_deref()
        .map(str::trim)
        .filter(|c| c.starts_with('#') && !c.is_empty() && c.len() <= 64)
        .map(String::from);
    let position = form.position.unwrap_or(100).clamp(0, 999);
    let description = form
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(String::from);

    match state
        .db
        .create_role(name, color.as_deref(), position, description.as_deref())
        .await
    {
        Ok(_) => {
            let _ = state
                .db
                .audit(Some(actor.id), "role_create", Some(&format!("name={}", name)), None)
                .await;
            // Refrescar colores de rol: el nuevo rol puede tener color y debe
            // aparecer en /api/role_colors y /role-colors.css sin reiniciar.
            state.role_colors.refresh(&state.db).await;
        }
        Err(e) => {
            tracing::warn!("role_create error: {e}");
        }
    }
    Redirect::to("/admin/roles").into_response()
}

// ===== POST /admin/roles/:id/update =====

pub async fn update_role(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<RoleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    let role = match state.db.get_role(id).await {
        Ok(Some(r)) => r,
        _ => return Redirect::to("/admin/roles").into_response(),
    };

    let name = form.name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Redirect::to("/admin/roles").into_response();
    }
    // El color puede ser un hex suelto ("#8b5cf6") o el composite del color
    // picker ("#hex|text|opacity" o "#hex~#hex2|text|opacity"). Aceptamos
    // cualquier string no vacío que empiece por '#' (el picker siempre produce
    // '#…'); role_color::parse_role_color lo resuelve a CSS al servir
    // /role-colors.css y /api/role_colors.
    let color = form
        .color
        .as_deref()
        .map(str::trim)
        .filter(|c| c.starts_with('#') && !c.is_empty() && c.len() <= 64)
        .map(String::from);
    // Si el rol es protected, position se ignora (se queda con la actual).
    let position = if role.is_protected() {
        role.position
    } else {
        form.position.unwrap_or(100).clamp(0, 999)
    };
    let description = form
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(String::from);

    let _ = state
        .db
        .update_role(id, name, color.as_deref(), position, description.as_deref())
        .await;
    let _ = state
        .db
        .audit(Some(actor.id), "role_update", Some(&format!("id={}", id)), None)
        .await;
    // Refrescar colores de rol: el color (o nombre) puede haber cambiado.
    state.role_colors.refresh(&state.db).await;
    Redirect::to("/admin/roles").into_response()
}

// ===== POST /admin/roles/:id/delete =====

pub async fn delete_role(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<DeleteRoleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    // No se puede borrar el rol propio del actor (sería degradarse).
    if actor.role_id == Some(id) {
        return Redirect::to("/admin/roles").into_response();
    }

    let role = match state.db.get_role(id).await {
        Ok(Some(r)) => r,
        _ => return Redirect::to("/admin/roles").into_response(),
    };
    if role.is_system() {
        return Redirect::to("/admin/roles").into_response();
    }

    // Si hay usuarios con este rol, reasignar al destino indicado.
    let user_count = state.db.count_users_with_role(id).await.unwrap_or(0);
    if user_count > 0 {
        let target = form
            .reassign_to
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<i64>().ok());
        let target = match target {
            Some(t) => t,
            None => return Redirect::to("/admin/roles").into_response(),
        };
        if target == id {
            return Redirect::to("/admin/roles").into_response();
        }
        match state.db.get_role(target).await {
            Ok(Some(t)) if t.role_key != "owner" => {}
            _ => return Redirect::to("/admin/roles").into_response(),
        }
        let _ = state.db.reassign_users_role(id, target).await;
    }

    let _ = state.db.delete_role(id).await;
    let _ = state
        .db
        .audit(Some(actor.id), "role_delete", Some(&format!("id={}", id)), None)
        .await;
    // El rol borrado desaparece del caché de colores (y los usuarios
    // reasignados ya pintan con el rol destino, que sí está en caché).
    state.role_colors.refresh(&state.db).await;
    Redirect::to("/admin/roles").into_response()
}

// ===== POST /admin/roles/:id/perms =====
//
// Reemplaza TODOS los permisos atómicos del rol. Valida contra el catálogo
// PERMISSIONS: claves inventadas se descartan.
//
// Usa `RawForm` en vez de `Form<RolePermsForm>` porque axum 0.7 `Form` usa
// `serde_urlencoded` que NO soporta arrays multi-valuados (los checkboxes
// `name="perms"` envían `perms=a&perms=b`). Aquí parseamos el body raw a mano
// para recolectar todos los valores de `perms`.

pub async fn set_role_perms(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    RawForm(body): RawForm,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    // Parsear el body urlencoded a mano para soportar `perms=X&perms=Y`.
    let perms = parse_form_array(&body, "perms");

    let valid: std::collections::HashSet<&str> = PERMISSIONS.iter().map(|p| p.key).collect();
    let perms: Vec<String> = perms
        .into_iter()
        .filter(|p| valid.contains(p.as_str()))
        .collect();

    let _ = state.db.set_role_perms(id, &perms).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "role_perms",
            Some(&format!("id={},n={}", id, perms.len())),
            None,
        )
        .await;
    Redirect::to("/admin/roles").into_response()
}

// ===== POST /admin/roles/:id/web_perms =====

pub async fn set_role_web_perms(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    RawForm(body): RawForm,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "roles.manage").await {
        return Redirect::to("/hub").into_response();
    }

    let web_ids: Vec<i64> = parse_form_array(&body, "web_ids")
        .into_iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();

    let _ = state.db.set_role_web_perms(id, &web_ids).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "role_web_perms",
            Some(&format!("id={},n={}", id, web_ids.len())),
            None,
        )
        .await;
    Redirect::to("/admin/roles").into_response()
}

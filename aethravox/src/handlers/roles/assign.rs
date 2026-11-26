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


use super::manage::can_act_on_role;

pub fn parse_form_array(body: &[u8], field: &str) -> Vec<String> {
    let s = match std::str::from_utf8(body) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for pair in s.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            // El campo puede venir como `perms` o `perms[]` (algunos clientes).
            let key = k.trim_end_matches("[]");
            if key == field {
                out.push(percent_decode(v));
            }
        }
    }
    out
}

/// Percent-decode minimal para valores de form (solo %XX y +).
pub fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '+' => out.push(' '),
            '%' => {
                let h1 = chars.next();
                let h2 = chars.next();
                if let (Some(a), Some(b)) = (h1, h2) {
                    if let (Some(av), Some(bv)) = (a.to_digit(16), b.to_digit(16)) {
                        out.push(((av << 4) | bv) as u8 as char);
                        continue;
                    }
                    out.push('%');
                    out.push(a);
                    out.push(b);
                } else {
                    out.push('%');
                }
            }
            _ => out.push(c),
        }
    }
    out
}

// ===== POST /admin/users/:id/role =====
//
// Asigna el ROL (obligatorio, 1 por usuario) a un usuario. Valida jerarquía
// (no actuar sobre alguien de nivel superior) y bloquea asignar el rol owner
// (el Owner se crea en bootstrap, no se asigna vía UI).

pub async fn assign_user_role(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<crate::models::AssignRoleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "users.role.change").await {
        return Redirect::to("/hub").into_response();
    }

    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users").into_response(),
    };

    let actor_role = match actor.role_id {
        Some(rid) => match state.db.get_role(rid).await {
            Ok(Some(r)) => r,
            _ => return Redirect::to("/admin/users").into_response(),
        },
        None => return Redirect::to("/admin/users").into_response(),
    };
    let target_role = match target.role_id {
        Some(rid) => match state.db.get_role(rid).await {
            Ok(Some(r)) => r,
            _ => return Redirect::to("/admin/users").into_response(),
        },
        None => return Redirect::to("/admin/users").into_response(),
    };

    if !can_act_on_role(&actor, &actor_role, &target, &target_role) {
        return Redirect::to("/admin/users").into_response();
    }

    let role_id = form
        .role_id
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|&i| i > 0);
    let role_id = match role_id {
        Some(rid) => rid,
        None => return Redirect::to("/admin/users").into_response(),
    };

    let _ = state.db.assign_user_role(id, role_id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "user_role_assign",
            Some(&format!("user={},role={}", id, role_id)),
            None,
        )
        .await;
    // Antes redirigía a /admin/users/{id}, pero esa ruta GET NO existe (solo
    // está /admin/users, la lista). Caía al fallback de webs custom y parecía
    // que "no se podía asignar el rol". La lista /admin/users sí existe y
    // muestra el rol ya actualizado.
    Redirect::to("/admin/users").into_response()
}

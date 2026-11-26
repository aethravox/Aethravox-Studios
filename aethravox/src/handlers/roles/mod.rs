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


mod manage;
pub use self::manage::*;
mod assign;
pub use self::assign::*;

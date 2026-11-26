#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

/// Rol configurable creado por el Owner desde /admin/roles. Reemplaza la
/// noción de "4 roles fijos" (owner/admin/staff/usuario). Cada rol agrupa:
///   (a) capabilities atómicas (role_perms) — ban, ban-ip, admin.access, etc.
///   (b) secciones visibles (role_web_perms) — qué webs del hub ve.
///   (c) posición jerárquica (position) — para can_act_on.
///
/// Los 4 roles base (Owner/Admin/Staff/Usuario) son is_system=1 (no se
/// pueden borrar, sí editar). El Owner además es is_protected=1 (no se
/// puede cambiar name/role_key/position ni quitarle permisos — aunque los
/// tuviera todos vía bypass en user_has_perm).
#[derive(Debug, Clone, FromRow)]
pub struct Role {
    pub id: i64,
    pub name: String,
    /// Clave interna inmutable tras crear. 'owner' identifica al Owner del
    /// sistema (que tiene bypass total). Custom roles usan 'custom_<id>'.
    pub role_key: String,
    pub color: Option<String>,
    pub position: i64,
    pub description: Option<String>,
    pub is_system: i64,
    pub is_protected: i64,
}

impl Role {
    pub fn is_owner(&self) -> bool {
        self.role_key == "owner"
    }
    pub fn is_protected(&self) -> bool {
        self.is_protected != 0
    }
    pub fn is_system(&self) -> bool {
        self.is_system != 0
    }
}

/// Una capability atómica que el Owner puede conceder a un rol.
/// `key` es el identificador estable (p.ej. "users.ban.temporary"),
/// `category` agrupa permisos en la UI, `label` es lo que se muestra.
#[derive(Debug, Clone, Copy)]
pub struct PermissionDef {
    pub key: &'static str,
    pub category: &'static str,
    pub label: &'static str,
}

/// Catálogo completo de permisos atómicos. Añadir aquí cualquier capability
/// nueva que se quiera conceder por rol. El Owner del sistema tiene bypass
/// total (todos estos permisos, sin necesidad de rol).
pub const PERMISSIONS: &[PermissionDef] = &[
    // — Panel —
    PermissionDef { key: "admin.access",          category: "Panel",       label: "Entrar al panel de administración" },
    // — Usuarios —
    PermissionDef { key: "users.view",            category: "Usuarios",    label: "Ver lista de usuarios" },
    PermissionDef { key: "users.create",          category: "Usuarios",    label: "Crear usuarios nuevos" },
    PermissionDef { key: "users.role.change",     category: "Usuarios",    label: "Cambiar el rol de un usuario" },
    PermissionDef { key: "users.delete",          category: "Usuarios",    label: "Eliminar usuarios" },
    PermissionDef { key: "users.force_logout",    category: "Usuarios",    label: "Cerrar sesión remota de un usuario" },
    PermissionDef { key: "users.ban.temporary",   category: "Usuarios",    label: "Banear temporalmente" },
    PermissionDef { key: "users.ban.permanent",   category: "Usuarios",    label: "Banear permanentemente" },
    PermissionDef { key: "users.suspend",         category: "Usuarios",    label: "Suspender usuarios" },
    PermissionDef { key: "users.unban",           category: "Usuarios",    label: "Quitar baneos / suspensiones" },
    PermissionDef { key: "users.ip.view",         category: "Usuarios",    label: "Ver historial de IPs de un usuario" },
    // — Secciones (webs) —
    PermissionDef { key: "webs.view",             category: "Secciones",   label: "Ver gestión de secciones" },
    PermissionDef { key: "webs.create",          category: "Secciones",   label: "Crear secciones" },
    PermissionDef { key: "webs.delete",          category: "Secciones",   label: "Borrar secciones" },
    PermissionDef { key: "webs.toggle",           category: "Secciones",   label: "Activar / desactivar secciones" },
    PermissionDef { key: "webs.html.edit",        category: "Secciones",   label: "Editar HTML de secciones custom" },
    PermissionDef { key: "webs.perms",            category: "Secciones",   label: "Cambiar permisos de rol de una sección" },
    PermissionDef { key: "webs.tag",              category: "Secciones",   label: "Cambiar el tag de una sección" },
    // — Roles —
    PermissionDef { key: "roles.manage",          category: "Roles",       label: "Crear, editar y borrar roles" },
    // — Títulos — (badges cosméticos que el Owner otorga a usuarios)
    PermissionDef { key: "titles.manage",         category: "Títulos",     label: "Crear, editar y asignar títulos" },
    // — Configuración —
    PermissionDef { key: "config.edit",           category: "Config",      label: "Editar configuración global" },
    // — Auditoría —
    PermissionDef { key: "audit.view",            category: "Auditoría",   label: "Ver registros de auditoría" },
    // — Baneos de IP —
    PermissionDef { key: "ip_bans.view",          category: "Baneos IP",   label: "Ver baneos de IP" },
    PermissionDef { key: "ip_bans.create",        category: "Baneos IP",   label: "Crear baneos de IP" },
    PermissionDef { key: "ip_bans.revoke",        category: "Baneos IP",   label: "Revocar baneos de IP" },
    // — Tickets / soporte —
    PermissionDef { key: "tickets.view",          category: "Soporte",     label: "Ver tickets de otros usuarios" },
    PermissionDef { key: "tickets.respond",       category: "Soporte",     label: "Responder tickets como staff" },
    PermissionDef { key: "tickets.manage",        category: "Soporte",     label: "Reclamar / cerrar / rechazar tickets" },
    // — Documentos —
    PermissionDef { key: "docs.view",             category: "Documentos",  label: "Ver gestión de documentos" },
    PermissionDef { key: "docs.create",           category: "Documentos",  label: "Crear documentos" },
    PermissionDef { key: "docs.edit",             category: "Documentos",  label: "Editar documentos" },
    PermissionDef { key: "docs.delete",           category: "Documentos",  label: "Borrar documentos" },
    // — Perfiles —
    PermissionDef { key: "profile.view_suspended",category: "Perfiles",    label: "Ver perfiles suspendidos / baneados" },
    // — Moderación de chat —
    PermissionDef { key: "chat.moderate",         category: "Chat",        label: "Borrar mensajes ajenos en chat" },
    PermissionDef { key: "communities.moderate",  category: "Comunidades", label: "Moderar comunidades" },
    // — Aris / APIs —
    PermissionDef { key: "admin.apis",            category: "Aris / IA",   label: "Gestionar API keys de Aris" },
    // — Aris · Carriles IA (núcleo) —
    PermissionDef { key: "aris.lane.chat",         category: "Aris · Carriles IA", label: "Conversar en /aris (vista completa)" },
    PermissionDef { key: "aris.lane.quick_drawer", category: "Aris · Carriles IA", label: "Usar Aris rápido contextual en cualquier web" },
    PermissionDef { key: "aris.lane.playground",   category: "Aris · Carriles IA", label: "Generar HTML/CSS/JS con Aris en Playground" },
    PermissionDef { key: "aris.lane.studio",       category: "Aris · Carriles IA", label: "Usar Aris en aexTube, Docs y Soporte" },
    PermissionDef { key: "aris.lane.social",       category: "Aris · Carriles IA", label: "Enviar DMs a amigos aceptados vía Aris" },
    PermissionDef { key: "aris.lane.analytics",    category: "Aris · Carriles IA", label: "Consultar métricas de visitas, usuarios y webs usadas" },
    PermissionDef { key: "aris.lane.admin_ops",    category: "Aris · Carriles IA", label: "Consultar auditoría, tickets globales y salud de APIs" },
];

/// Permisos que cada rol del sistema tiene por defecto (baseline). Codifica
/// el comportamiento actual (hardcoded) para que nada se rompa al migrar a
/// `user_has_perm`: admin/staff siguen haciendo lo mismo. El Owner tiene
/// bypass total (no entra aquí; lo trata `user_has_perm` aparte).
pub fn baseline_has_perm(role: &str, perm: &str) -> bool {
    match role {
        "admin" => matches!(
            perm,
            "admin.access"
                | "admin.apis"
                | "aris.lane.chat"
                | "aris.lane.quick_drawer"
                | "aris.lane.playground"
                | "aris.lane.studio"
                | "aris.lane.social"
                | "aris.lane.analytics"
                | "aris.lane.admin_ops"
                | "users.view"
                | "users.create"
                | "users.role.change"
                | "users.delete"
                | "users.force_logout"
                | "users.ban.temporary"
                | "users.ban.permanent"
                | "users.suspend"
                | "users.unban"
                | "users.ip.view"
                | "audit.view"
                | "ip_bans.view"
                | "ip_bans.create"
                | "ip_bans.revoke"
                | "tickets.view"
                | "tickets.respond"
                | "tickets.manage"
                | "profile.view_suspended"
        ),
        "staff" => matches!(
            perm,
            "admin.access"
                | "users.ban.temporary"
                | "users.suspend"
                | "aris.lane.chat"
                | "aris.lane.quick_drawer"
                | "aris.lane.studio"
                | "aris.lane.social"
                | "aris.lane.analytics"
        ),
        "usuario" | "user" => matches!(perm, "aris.lane.chat" | "aris.lane.social"),
        _ => false, // invitado y desconocido: sin baseline.
    }
}

#[derive(Debug, Deserialize)]
pub struct RoleForm {
    pub name: String,
    pub color: Option<String>,
    pub position: Option<i64>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RolePermsForm {
    #[serde(default)]
    pub perms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoleWebPermsForm {
    #[serde(default)]
    pub web_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AssignRoleForm {
    pub role_id: String,
}


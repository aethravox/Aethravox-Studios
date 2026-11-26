#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub total_seconds: i64,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub status: String,
    /// Última IP desde la que se logueó (NULL si nunca ha iniciado sesión).
    pub last_ip: Option<String>,
    /// Rol configurable (FK a roles.id). Tras la migración 031, todos los
    /// usuarios quedan con un role_id válido. El campo `role` (TEXT) se
    /// mantiene por retrocompatibilidad y se sincroniza con role_key.
    pub role_id: Option<i64>,
    /// Título principal (FK a titles.id). Se muestra junto al nombre del
    /// usuario en todo el sitio (chat, amigos, admin, perfil). Nullable.
    pub primary_title_id: Option<i64>,
    /// Los 3 títulos visibles en el perfil público (slots 1..3). Nullable.
    pub visible_title_1: Option<i64>,
    pub visible_title_2: Option<i64>,
    pub visible_title_3: Option<i64>,
}

/// Jerarquía numérica de roles. Mayor = más permisos.
pub fn role_level(role: &str) -> i32 {
    match role {
        "owner" => 100,
        "admin" => 50,
        "staff" => 20,
        "usuario" => 10,
        _ => 0,
    }
}

pub fn is_role(s: &str) -> bool {
    matches!(s, "owner" | "admin" | "staff" | "usuario")
}

impl User {
    /// Construye un User invitado sintético (stateless, sin fila en `users`).
    /// Lo usa `auth_mw::resolve_user_or_guest` cuando no hay sesión: el
    /// invitado fluye por la app con rol "invitado" y los permisos sembrados
    /// en la DB (role_id apunta al rol Invitado, así `user_has_perm` y
    /// `role_display` funcionan como con cualquier usuario real).
    /// `id = 0` es un centinela "no es un usuario real" — nunca se persiste,
    /// nunca se loguea. Sirve para que los handlers guest-allowed reciban un
    /// `AuthUser` válido sin tocar la DB por visita.
    /// `invitado_role_id = None` solo si la migración 036 no corrió; en ese
    /// caso el invitado no tiene perms (cae al baseline, que no concede nada)
    /// pero igual puede ver lo público por el routing.
    pub fn guest(invitado_role_id: Option<i64>) -> User {
        User {
            id: 0,
            username: "Invitado".into(),
            email: String::new(),
            password_hash: String::new(),
            role: "invitado".into(),
            avatar_url: None,
            created_at: Utc::now(),
            last_login_at: None,
            total_seconds: 0,
            display_name: None,
            bio: None,
            status: "active".into(),
            last_ip: None,
            role_id: invitado_role_id,
            primary_title_id: None,
            visible_title_1: None,
            visible_title_2: None,
            visible_title_3: None,
        }
    }

    /// Formatea el tiempo de uso acumulado en "1d 4h 12m".
    pub fn usage_string(&self) -> String {
        let total = self.total_seconds;
        let days = total / 86400;
        let hours = (total % 86400) / 3600;
        let mins = (total % 3600) / 60;
        if days > 0 {
            format!("{}d {}h {}m", days, hours, mins)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }

    /// Nombre a mostrar: display_name si está seteado, si no username.
    pub fn display(&self) -> String {
        self.display_name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.username.clone())
    }

    /// Avatar: avatar_url si está seteado, si no placeholder SVG con inicial.
    /// El color de fondo del placeholder varía según el username (hash → matiz
    /// HSL), así cada usuario tiene un color distinto y los avatares "por
    /// defecto" no se ven todos iguales en negro con letra dorada.
    pub fn avatar(&self) -> String {
        if let Some(url) = &self.avatar_url {
            if !url.is_empty() {
                return url.clone();
            }
        }
        let initial_upper: String = self
            .username
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .collect();
        let escaped: String = initial_upper
            .chars()
            .map(|c| match c {
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '&' => "&amp;".to_string(),
                _ => c.to_string(),
            })
            .collect();
        // Hash simple del username → 0..360 (matiz)
        let mut hue: u32 = 0;
        for b in self.username.as_bytes() {
            hue = hue.wrapping_mul(31).wrapping_add(*b as u32) % 360;
        }
        format!(
            "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200'>\
             <rect width='200' height='200' fill='hsl({hue}%2C%2030%25%2C%2018%25)'/>\
             <text x='100' y='125' font-size='100' fill='hsl({hue}%2C%2060%25%2C%2085%25)' text-anchor='middle' \
             font-family='sans-serif'>{}</text></svg>",
            escaped.replace(' ', "%20")
        )
    }

    /// ¿El usuario puede ejecutar `action` sobre el usuario `target`?
    ///
    /// Esta función comprueba SOLO la **jerarquía** (¿puedes tocar a alguien
    /// de ese nivel?). La **autorización** (¿tienes el permiso para esa
    /// acción?) la da `Db::user_has_perm` y se comprueba aparte en el handler.
    /// Regla: el Owner puede actuar sobre todos (salvo borrarse a sí mismo);
    /// cualquier otro puede actuar sobre usuarios de nivel estrictamente
    /// inferior, o sobre cualquier `usuario` (nivel ≤ 10) que no sea él mismo.
    /// Nunca se puede actuar sobre el Owner (nivel 100) si no se es Owner.
    pub fn can_act_on(&self, target: &User, action: ActionOnUser) -> bool {
        if self.role == "owner" {
            // El Owner puede todo, salvo borrarse a sí mismo.
            return !(action == ActionOnUser::Delete && target.id == self.id);
        }
        let my_lvl = role_level(&self.role);
        let tgt_lvl = role_level(&target.role);
        // No actuar sobre uno mismo, ni sobre Owner/Admin (>=50) si no eres Owner.
        target.id != self.id && (tgt_lvl <= 10 || tgt_lvl < my_lvl)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionOnUser {
    BanTemporary,
    BanPermanent,
    Suspend,
    CreateUser,
    ChangeRole,
    Delete,
    ForceLogout,
    ViewList,
    ViewAudit,
}

#[derive(Debug, Clone, FromRow)]
pub struct SessionInfo {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub remember: i64,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    // Vinculación de dispositivos (migración 024). NULL/None para sesiones
    // creadas antes de la migración o sesiones primary sin etiqueta.
    pub device_name: Option<String>,
    pub is_primary: i64,
    pub linked_by_user_id: Option<i64>,
    pub link_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordForm {
    pub current: String,
    pub new: String,
    #[serde(rename = "new_confirm")]
    pub new_confirm: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileForm {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    #[serde(default)]
    pub remove_avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminCreateUserForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleForm {
    pub role: String,
}


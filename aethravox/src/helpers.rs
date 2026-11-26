// Helpers compartidos entre handlers.
use tower_cookies::Cookies;

use crate::{db::Db, models::User};

/// Devuelve el usuario autenticado actual leyendo la cookie de sesión.
/// Si no hay cookie, o la sesión no existe / expiró, devuelve None.
pub async fn current_user(db: &Db, cookies: &Cookies, cookie_name: &str) -> Option<User> {
    let token = cookies.get(cookie_name)?.value().to_string();
    db.get_session_user(&token).await.ok().flatten()
}

/// Sanitiza texto para inyectarlo en HTML de forma segura.
/// (Reservado para uso en Web 2/3 donde necesitemos escapar valores
/// que NO vengan de templates askama, que ya escapan por defecto.)
#[allow(dead_code)]
pub fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

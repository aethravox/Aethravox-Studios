// Juegos 2D — Neon Canvas Ultimate
// Juego canvas 2D (dibujo neón con modos libre/desafío/zen/VS IA/diario,
// powerups, logros, multijugador WiFi). Toda la lógica es client-side y vive
// en /static/js/juegos/neon_canvas.js + /static/css/juegos/neon_canvas.css;
// este handler solo renderiza la plantilla Askama que monta el canvas/HUD y
// enlaza esos assets. Ruta /juegos/neon-canvas detrás de require_auth
// (protected_routes), igual que /playground.
use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{auth_mw::AuthUser, AppState};

pub async fn neon_canvas_page(
    State(_): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    NeonCanvasTemplate {
        username: user.username.clone(),
        role: user.role.clone(),
    }
}

#[derive(Template)]
#[template(path = "juegos_2d/neon_canvas/neon_canvas.html")]
pub struct NeonCanvasTemplate {
    pub username: String,
    pub role: String,
}

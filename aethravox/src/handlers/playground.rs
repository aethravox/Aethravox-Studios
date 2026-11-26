// Web 6 — Playground (idea 3)
// Editor de HTML/CSS/JS con preview en vivo usando iframe srcdoc.
// Todo client-side; este handler solo renderiza la plantilla.
use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{auth_mw::AuthUser, AppState};

pub async fn playground_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> impl IntoResponse {
    let (_, role_display) = state.db.role_display(&user).await;
    let can_use_aris_playground = state.db.user_has_perm(&user, "aris.lane.playground").await;
    PlaygroundTemplate {
        username: user.username.clone(),
        role: user.role.clone(),
        role_display,
        can_use_aris_playground,
    }
}

#[derive(Template)]
#[template(path = "creatividad/playground/playground.html")]
pub struct PlaygroundTemplate {
    pub username: String,
    pub role: String,
    pub role_display: String,
    pub can_use_aris_playground: bool,
}

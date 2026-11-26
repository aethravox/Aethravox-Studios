//! Sistema de temas: onboarding (primer ingreso) + selector en /tema.
//!
//! El tema se guarda en user_themes (migración 041). Si el usuario no
//! tiene fila en user_onboarding (043), el frontend muestra el panel.
//! El CSS del tema se carga en base.html según user.theme.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Form, Json,
};
use serde::Deserialize;

use crate::auth_mw::AuthUser;
use crate::AppState;

pub const THEMES: &[(&str, &str)] = &[
    ("aethravox", "Aethravox Studios"),
    ("shadow", "Shadow Studios"),
    ("commoncents", "CommonCents"),
];

#[derive(Debug, Deserialize)]
pub struct SelectThemeForm {
    pub theme: String,
}

/// GET /tema — Página de ajustes del tema.
pub async fn tema_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let current = state.db.get_user_theme(user.id).await
        .unwrap_or_else(|| "aethravox".to_string());

    crate::templates::ThemePage {
        user,
        current,
        themes: THEMES.to_vec(),
    }
    .into_response()
}

/// POST /api/theme/select — Guardar tema (desde onboarding o ajustes).
pub async fn api_select_theme(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<SelectThemeForm>,
) -> Response {
    if !THEMES.iter().any(|(id, _)| *id == form.theme) {
        return Json(serde_json::json!({"error": "tema_invalido"})).into_response();
    }

    match state.db.set_user_theme(user.id, &form.theme).await {
        Ok(_) => {
            // Marcar onboarding completado (por si venía del panel)
            let _ = sqlx::query(
                "INSERT INTO user_onboarding (user_id, theme_completed, completed_at)
                 VALUES (?, 1, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
                 ON CONFLICT(user_id) DO UPDATE SET
                     theme_completed = 1, completed_at = excluded.completed_at"
            )
            .bind(user.id)
            .execute(&state.db.pool)
            .await;

            Json(serde_json::json!({"ok": true, "theme": form.theme})).into_response()
        }
        Err(_) => Json(serde_json::json!({"error": "server"})).into_response(),
    }
}

/// GET /api/theme/status — ¿Debe mostrarse el panel de onboarding?
/// El frontend lo pregunta al cargar base.html y muestra el modal si hace falta.
pub async fn api_theme_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let completed: Option<(i64,)> = sqlx::query_as(
        "SELECT theme_completed FROM user_onboarding WHERE user_id = ?"
    )
    .bind(user.id)
    .fetch_optional(&state.db.pool)
    .await
    .ok()
    .flatten();

    let theme = state.db.get_user_theme(user.id).await;

    Json(serde_json::json!({
        "show_onboarding": completed.map(|c| c.0 == 0).unwrap_or(true),
        "theme": theme.unwrap_or_else(|| "aethravox".to_string()),
        "themes": THEMES.iter().map(|(id, name)| serde_json::json!({
            "id": id, "name": name
        })).collect::<Vec<_>>(),
    }))
    .into_response()
}

// handlers/titles.rs — sistema de títulos (badges cosméticos).
//
// Un título es un badge público que el Owner otorga a un usuario (p.ej.
// "Creador", "Streamer", "Dueño de X"). Es COSMÉTICO: no da permisos.
// Reglas: el usuario puede tener todos los que le den; en su perfil elige
// hasta 3 visibles y 1 principal (el principal se ve junto al nombre en
// todo el sitio).
//
// Todas las rutas de gestión (CRUD + assign) requieren `titles.manage`,
// que por defecto SOLO el Owner tiene (migración 034).

use askama::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

use crate::{
    auth_mw::AuthUser,
    models::{Title, TitleView},
    role_color::parse_role_color,
    AppState,
};

// ===== Helper: Title → TitleView (resuelve color CSS) =====

pub fn title_view(t: &Title) -> TitleView {
    let (bg, text) = parse_role_color(t.color.as_deref());
    let color_css = if bg.is_empty() {
        // Sin color configurado: default sutil.
        "background:rgba(255,255,255,0.08);color:#e0e0e0".to_string()
    } else {
        format!("background:{};color:{}", bg, text)
    };
    TitleView {
        id: t.id,
        name: t.name.clone(),
        color_css,
        effect: t.effect.clone(),
        description: t.description.clone(),
    }
}

// ===== GET /admin/titles — panel de gestión de títulos =====

#[derive(Template)]
#[template(path = "admin/roles/admin_titles.html")]
pub struct AdminTitlesTemplate {
    pub actor_username: String,
    pub actor_role: String,
    pub actor_role_display: String,
    pub actor_role_css_key: String,
    pub titles: Vec<TitleRow>,
    /// Usuarios elegibles para asignar títulos (todos los activos). El modal
    /// de "Asignar a usuario" los muestra en una lista con barra de búsqueda
    /// (filtrado en vivo por JS) en vez de pedir un ID a mano, que era
    /// confuso. El form de asignación manda el user_id del elegido.
    pub users: Vec<AssignUserRow>,
    pub can_manage: bool,
}

pub struct AssignUserRow {
    pub id: i64,
    pub username: String,
    pub display: String,
}

pub struct TitleRow {
    pub id: i64,
    pub name: String,
    pub color_css: String,
    // Composite crudo "#hex|#text|opacidad" (o "#hex1~#hex2|...") tal como
    // está en la tabla titles.color. Lo usa admin_titles.html para rellenar
    // el data-edit-color del botón "Editar" y que el color picker muestre el
    // color actual del título al editarlo.
    pub color: Option<String>,
    pub effect: String,
    pub effect_label: String,
    pub description: Option<String>,
    pub user_count: i64,
}

pub async fn admin_titles(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
) -> impl IntoResponse {
    let can_manage = state.db.user_has_perm(&actor, "titles.manage").await;
    if !can_manage {
        return Redirect::to("/admin").into_response();
    }

    let all = state.db.list_titles().await.unwrap_or_default();
    let rows: Vec<TitleRow> = all
        .iter()
        .map(|t| {
            let v = title_view(t);
            TitleRow {
                id: t.id,
                name: t.name.clone(),
                color_css: v.color_css,
                color: t.color.clone(),
                effect: t.effect.clone(),
                effect_label: effect_label(&t.effect).to_string(),
                description: t.description.clone(),
                user_count: 0, // se setea abajo
            }
        })
        .collect();

    // Contar usuarios por título (un query por título; hay pocos).
    let mut rows = rows;
    for r in rows.iter_mut() {
        r.user_count = count_users_with_title(&state, r.id).await;
    }

    // Lista de usuarios activos para el modal de "Asignar a usuario".
    // La filtración en vivo la hace el JS del modal; aquí mandamos todos.
    let users: Vec<AssignUserRow> = state
        .db
        .list_users()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|u| u.status == "active")
        .map(|u| AssignUserRow {
            id: u.id,
            username: u.username.clone(),
            display: u.display(),
        })
        .collect();

    let (actor_role_css_key, actor_role_display) = state.db.role_display(&actor).await;
    AdminTitlesTemplate {
        actor_username: actor.username.clone(),
        actor_role: actor.role.clone(),
        actor_role_display,
        actor_role_css_key,
        titles: rows,
        users,
        can_manage,
    }
    .into_response()
}

async fn count_users_with_title(state: &AppState, title_id: i64) -> i64 {
    let row: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) FROM user_titles WHERE title_id = ?")
        .bind(title_id)
        .fetch_optional(&state.db.pool)
        .await
        .ok()
        .flatten();
    row.map(|(n,)| n).unwrap_or(0)
}

fn effect_label(e: &str) -> &'static str {
    match e {
        "glow" => "✨ Glow",
        "shimmer" => "💫 Shimmer",
        "gradient" => "🌈 Degradado",
        _ => "—",
    }
}

// ===== Form de crear/editar título =====

#[derive(Deserialize)]
pub struct TitleForm {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

fn clean_form(form: &TitleForm) -> (String, Option<String>, String, Option<String>) {
    let name = form.name.trim().to_string();
    let color = form
        .color
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty() && c.starts_with('#') && c.len() <= 64)
        .map(String::from);
    let effect = form
        .effect
        .as_deref()
        .map(str::trim)
        .filter(|e| matches!(*e, "none" | "glow" | "shimmer" | "gradient"))
        .unwrap_or("none")
        .to_string();
    let description = form
        .description
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(String::from);
    (name, color, effect, description)
}

// ===== POST /admin/titles/create =====

pub async fn create_title(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Form(form): Form<TitleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "titles.manage").await {
        return Redirect::to("/admin").into_response();
    }
    let (name, color, effect, description) = clean_form(&form);
    if name.is_empty() || name.chars().count() > 32 {
        return Redirect::to("/admin/titles").into_response();
    }
    match state
        .db
        .create_title(&name, color.as_deref(), &effect, description.as_deref())
        .await
    {
        Ok(_) => {
            let _ = state
                .db
                .audit(Some(actor.id), "title_create", Some(&format!("name={}", name)), None)
                .await;
        }
        Err(e) => tracing::warn!("title_create error: {e}"),
    }
    Redirect::to("/admin/titles").into_response()
}

// ===== POST /admin/titles/:id/update =====

pub async fn update_title(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<TitleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "titles.manage").await {
        return Redirect::to("/admin").into_response();
    }
    let (name, color, effect, description) = clean_form(&form);
    if name.is_empty() || name.chars().count() > 32 {
        return Redirect::to("/admin/titles").into_response();
    }
    // Si el color viene vacío, conservar el actual (para no perderlo al editar
    // otros campos desde el modal, que no precarga el color).
    let color = match color {
        Some(c) => Some(c),
        None => state
            .db
            .get_title(id)
            .await
            .ok()
            .flatten()
            .and_then(|t| t.color),
    };
    let _ = state
        .db
        .update_title(id, &name, color.as_deref(), &effect, description.as_deref())
        .await;
    let _ = state
        .db
        .audit(Some(actor.id), "title_update", Some(&format!("id={}", id)), None)
        .await;
    Redirect::to("/admin/titles").into_response()
}

// ===== POST /admin/titles/:id/delete =====

pub async fn delete_title(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "titles.manage").await {
        return Redirect::to("/admin").into_response();
    }
    let _ = state.db.delete_title(id).await;
    let _ = state
        .db
        .audit(Some(actor.id), "title_delete", Some(&format!("id={}", id)), None)
        .await;
    Redirect::to("/admin/titles").into_response()
}

// ===== POST /admin/titles/:id/assign =====
// Asigna un título a un usuario. Body: user_id=N

#[derive(Deserialize)]
pub struct AssignTitleForm {
    pub user_id: i64,
}

pub async fn assign_title(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(title_id): Path<i64>,
    Form(form): Form<AssignTitleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "titles.manage").await {
        return Redirect::to("/admin").into_response();
    }
    let _ = state.db.assign_title(form.user_id, title_id, actor.id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "title_assign",
            Some(&format!("user={},title={}", form.user_id, title_id)),
            None,
        )
        .await;
    Redirect::to("/admin/titles").into_response()
}

// ===== POST /admin/titles/:id/revoke =====
// Quita un título a un usuario. Body: user_id=N

#[derive(Deserialize)]
pub struct RevokeTitleForm {
    pub user_id: i64,
}

pub async fn revoke_title(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(title_id): Path<i64>,
    Form(form): Form<RevokeTitleForm>,
) -> impl IntoResponse {
    if !state.db.user_has_perm(&actor, "titles.manage").await {
        return Redirect::to("/admin").into_response();
    }
    let _ = state.db.revoke_title(form.user_id, title_id).await;
    let _ = state
        .db
        .audit(
            Some(actor.id),
            "title_revoke",
            Some(&format!("user={},title={}", form.user_id, title_id)),
            None,
        )
        .await;
    Redirect::to("/admin/titles").into_response()
}

// ===== GET /title-colors.css =====
// CSS dinámico con los colores de todos los títulos. Lo enlazamos en base.html
// para que los badges de título se pinten en el primer render (sin FOUC),
// igual que /role-colors.css. Clases: .title-badge.title-<id>{...}
// más los efectos (.title-badge.fx-glow, etc.).

pub async fn title_colors_css(State(state): State<AppState>) -> Response {
    let titles = state.db.list_titles().await.unwrap_or_default();
    let mut css = String::from("/* Generado server-side desde la tabla titles. No editar. */\n");
    for t in &titles {
        let v = title_view(t);
        if v.color_css.is_empty() {
            continue;
        }
        css.push_str(&format!(".title-badge.title-{}{{{}}}\n", t.id, v.color_css));
    }
    // Efectos globales por clase .fx-<effect>.
    css.push_str("\n/* Efectos visuales de título */\n");
    css.push_str(".title-badge.fx-glow{box-shadow:0 0 6px 1px currentColor;}\n");
    css.push_str(".title-badge.fx-shimmer{background-size:200% 100%;animation:title-shimmer 2.4s linear infinite;}\n");
    css.push_str("@keyframes title-shimmer{0%{background-position:200% 0}100%{background-position:-200% 0}}\n");
    css.push_str(".title-badge.fx-gradient{background-size:200% 100%;animation:title-grad 4s ease infinite;}\n");
    css.push_str("@keyframes title-grad{0%,100%{background-position:0 50%}50%{background-position:100% 50%}}\n");
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8".to_string()),
            (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        axum::body::Body::from(css),
    )
        .into_response()
}

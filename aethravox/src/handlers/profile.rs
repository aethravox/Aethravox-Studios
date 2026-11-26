// Web 3 — Perfil público de un usuario.
// /u/:username — visible para cualquiera logueado.
// /perfil      — perfil PROPIO del usuario logueado (web nueva, con editor
//                de títulos visibles + principal). Distinta de /dashboard
//                (que es ajustes de cuenta) y de /u/:username (que es el
//                perfil público de cualquiera).

use askama::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};

use crate::{
    auth_mw::AuthUser,
    handlers::titles::title_view,
    models::TitleView,
    AppState,
};

pub async fn profile(
    State(state): State<AppState>,
    AuthUser(viewer): AuthUser,
    Path(username): Path<String>,
) -> impl IntoResponse {
    let user = match state.db.find_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => {
            return ProfileNotFoundTemplate { username }.into_response();
        }
    };

    if user.status != "active" && !state.db.user_has_perm(&viewer, "profile.view_suspended").await {
        return ProfileNotFoundTemplate { username }.into_response();
    }

    // Título principal del usuario (se muestra junto al nombre).
    let primary_title: Option<TitleView> = state
        .db
        .get_primary_title(user.id)
        .await
        .ok()
        .flatten()
        .map(|t| title_view(&t));

    // Los 3 visibles en el perfil público.
    let mut visible_titles: Vec<TitleView> = Vec::new();
    for tid in [user.visible_title_1, user.visible_title_2, user.visible_title_3]
        .into_iter()
        .flatten()
    {
        if let Ok(Some(t)) = state.db.get_title(tid).await {
            visible_titles.push(title_view(&t));
        }
    }

    // Badge de rol del usuario cuyo perfil se ve (puede ser custom).
    let (role_css_key, role_display) = state.db.role_display(&user).await;

    ProfileTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: role_display.clone(),
        role_css_key,
        bio: user.bio.clone().unwrap_or_default(),
        created_at: user.created_at.format("%d/%m/%Y").to_string(),
        is_self: user.id == viewer.id,
        viewer_role: viewer.role.clone(),
        primary_title,
        visible_titles,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "auth/profile.html")]
pub struct ProfileTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    pub role_css_key: String,
    pub bio: String,
    pub created_at: String,
    pub is_self: bool,
    pub viewer_role: String,
    /// Título principal (se ve junto al nombre en todo el sitio).
    pub primary_title: Option<TitleView>,
    /// Hasta 3 títulos visibles en el perfil.
    pub visible_titles: Vec<TitleView>,
}

#[derive(Template)]
#[template(path = "auth/profile_not_found.html")]
pub struct ProfileNotFoundTemplate {
    pub username: String,
}

// ===== GET /perfil — perfil propio + editor de títulos =====

/// Fila de título en el editor de /perfil: el TitleView + flags
/// precomputados (selected = está entre los visibles, is_primary = es el
/// principal). Precomputar aquí evita comparar Option<i64> en el template
/// askama, que no acepta `Some(t.id) == visible_title_1` cuando t va por ref.
pub struct MyTitleRow {
    pub id: i64,
    pub name: String,
    pub color_css: String,
    pub effect: String,
    pub description: Option<String>,
    pub selected: bool,
    pub is_primary: bool,
}

pub async fn perfil_get(State(state): State<AppState>, AuthUser(user): AuthUser) -> impl IntoResponse {
    let rows = build_my_title_rows(&state, &user).await;
    // Badge de rol: si el usuario tiene un rol custom, mostrar su nombre real.
    let (role_css_key, role_display) = state.db.role_display(&user).await;
    PerfilTemplate {
        username: user.username.clone(),
        display: user.display(),
        avatar: user.avatar(),
        role: role_display.clone(),
        role_css_key,
        bio: user.bio.clone().unwrap_or_default(),
        created_at: user.created_at.format("%d/%m/%Y").to_string(),
        my_titles: rows,
        error: None,
        info: None,
    }
    .into_response()
}

async fn build_my_title_rows(state: &AppState, user: &crate::models::User) -> Vec<MyTitleRow> {
    let my_titles = state.db.list_user_titles(user.id).await.unwrap_or_default();
    my_titles
        .iter()
        .map(|t| {
            let v = title_view(t);
            let selected = user.visible_title_1 == Some(t.id)
                || user.visible_title_2 == Some(t.id)
                || user.visible_title_3 == Some(t.id);
            let is_primary = user.primary_title_id == Some(t.id);
            MyTitleRow {
                id: v.id,
                name: v.name,
                color_css: v.color_css,
                effect: v.effect,
                description: v.description,
                selected,
                is_primary,
            }
        })
        .collect()
}

#[derive(Template)]
#[template(path = "auth/perfil.html")]
pub struct PerfilTemplate {
    pub username: String,
    pub display: String,
    pub avatar: String,
    pub role: String,
    pub role_css_key: String,
    pub bio: String,
    pub created_at: String,
    /// Títulos asignados al usuario con flags precomputados.
    pub my_titles: Vec<MyTitleRow>,
    pub error: Option<String>,
    pub info: Option<String>,
}

// ===== POST /perfil/titles — guarda selección de visibles + principal =====
//
// OJO: NO usamos `Form<PerfilTitlesForm>` aquí. El form manda varios
// checkboxes `visible=N` (uno por título marcado). serde_urlencoded (que
// usa axum::Form) NO soporta secuencias cuando hay un SOLO valor: si el
// usuario marca un único título, el body es `visible=2&primary=2` y
// serde_urlencoded ve "2" como un string escalar, no como una secuencia de
// un elemento → error "invalid type: string \"2\", expected a sequence".
//
// En su lugar capturamos el body crudo (application/x-www-form-urlencoded)
// y lo partimos a mano, recolectando TODOS los valores de `visible`. Así
// funciona con 0, 1, 2 o 3 títulos marcados. Los valores son IDs numéricos,
// así que no necesitan URL-decode.

pub async fn perfil_titles_post(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    body: String,
) -> impl IntoResponse {
    let mut visible: Vec<i64> = Vec::new();
    let mut primary: Option<i64> = None;
    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        if key == "visible" {
            if let Ok(id) = val.parse::<i64>() {
                visible.push(id);
            }
        } else if key == "primary" {
            if let Ok(id) = val.parse::<i64>() {
                primary = Some(id);
            }
        }
    }
    // Tomar solo los primeros 3 (por si el cliente manda más).
    let vids: Vec<Option<i64>> = visible.into_iter().take(3).map(Some).collect();
    let v1 = vids.first().copied().flatten();
    let v2 = vids.get(1).copied().flatten();
    let v3 = vids.get(2).copied().flatten();

    match state
        .db
        .set_user_title_selection(user.id, primary, v1, v2, v3)
        .await
    {
        Ok(_) => Redirect::to("/perfil").into_response(),
        Err(e) => {
            tracing::warn!("perfil_titles error: {e}");
            let rows = build_my_title_rows(&state, &user).await;
            let (role_css_key, role_display) = state.db.role_display(&user).await;
            PerfilTemplate {
                username: user.username.clone(),
                display: user.display(),
                avatar: user.avatar(),
                role: role_display.clone(),
                role_css_key,
                bio: user.bio.clone().unwrap_or_default(),
                created_at: user.created_at.format("%d/%m/%Y").to_string(),
                my_titles: rows,
                error: Some(format!("No se pudo guardar: {e}")),
                info: None,
            }
            .into_response()
        }
    }
}

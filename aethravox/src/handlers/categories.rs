// Sección 4 — Categorías del hub: handlers para /categorias, /categoria/:slug,
// /explorar y /buscar. Todas guest-allowed (el invitado ve las apps públicas
// que su rol permite, filtradas por web_role_perms como el hub).
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::{auth_mw::AuthUser, models::Web, AppState};

/// Filtra `apps` a las visibles para el rol del viewer (intersecta con
/// list_webs_for_role, que usa web_role_perms). Así un invitado solo ve
/// docs/communities/support aunque la categoría tenga más apps.
fn filter_visible(apps: Vec<Web>, visible: &[Web]) -> Vec<Web> {
    let ids: HashSet<i64> = visible.iter().map(|w| w.id).collect();
    apps.into_iter()
        .filter(|w| ids.contains(&w.id) && w.slug != "correo" && w.slug != "hub" && w.slug != "login" && w.slug != "dashboard")
        .collect()
}

// ============================================================
// /categorias — lista de categorías (con conteo de apps visibles).
// ============================================================
pub async fn categorias_page(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let cats = state.db.list_categories().await.unwrap_or_default();
    let visible: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.slug != "correo" && w.slug != "hub" && w.slug != "login" && w.slug != "dashboard")
        .collect();
    let visible_ids: HashSet<i64> = visible.iter().map(|w| w.id).collect();
    // Contar apps visibles por categoría (no totales: el invitado no debe ver
    // "Comunicación (3)" si solo puede acceder a 1).
    let pairs = state.db.list_app_category_pairs().await.unwrap_or_default();
    let mut count_map: HashMap<i64, i64> = HashMap::new();
    for (app_id, cat_id) in &pairs {
        if visible_ids.contains(app_id) {
            *count_map.entry(*cat_id).or_insert(0) += 1;
        }
    }
    CategoriasTemplate {
        username: user.username.clone(),
        is_guest: user.id == 0,
        categories: cats
            .iter()
            .map(|c| CategoryCard {
                name: c.name.clone(),
                slug: c.slug.clone(),
                count: *count_map.get(&c.id).unwrap_or(&0),
            })
            .collect(),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "categories/categorias.html")]
pub struct CategoriasTemplate {
    pub username: String,
    pub is_guest: bool,
    pub categories: Vec<CategoryCard>,
}

pub struct CategoryCard {
    pub name: String,
    pub slug: String,
    pub count: i64,
}

// ============================================================
// /categoria/:slug — todas las apps de una categoría (lista compartida).
// ============================================================
pub async fn categoria_detalle(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(slug): Path<String>,
) -> Response {
    let cat = state.db.get_category_by_slug(&slug).await.ok().flatten();
    let Some(cat) = cat else {
        return Redirect::to("/categorias").into_response();
    };
    let visible = state.db.list_webs_for_user_obj(&user).await.unwrap_or_default();
    let apps = filter_visible(
        state.db.list_apps_in_category(cat.id).await.unwrap_or_default(),
        &visible,
    );
    AppListTemplate {
        title: cat.name.clone(),
        subtitle: format!("Apps en «{}»", cat.name),
        is_guest: user.id == 0,
        role: user.role.clone(),
        apps,
    }
    .into_response()
}

// ============================================================
// /explorar — todas las apps visibles (sin filtro de categoría).
// Reusa el MISMO AppListTemplate/template que /categoria/:slug (Sección 4.9).
// ============================================================
pub async fn explorar_page(State(state): State<AppState>, AuthUser(user): AuthUser) -> Response {
    let apps: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.slug != "correo" && w.slug != "hub" && w.slug != "login" && w.slug != "dashboard")
        .collect();
    AppListTemplate {
        title: "Explorar".into(),
        subtitle: "Todas las apps y secciones disponibles".into(),
        is_guest: user.id == 0,
        role: user.role.clone(),
        apps,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "categories/app_list.html")]
pub struct AppListTemplate {
    pub title: String,
    pub subtitle: String,
    pub is_guest: bool,
    pub role: String,
    pub apps: Vec<Web>,
}

// ============================================================
// /buscar?q= — búsqueda de apps por nombre (filtrada por visibilidad).
// ============================================================
#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: Option<String>,
}

pub async fn buscar_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<SearchQuery>,
) -> Response {
    let raw_q = q.q.unwrap_or_default();
    let visible: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| w.slug != "correo" && w.slug != "hub" && w.slug != "login" && w.slug != "dashboard")
        .collect();
    let apps = if raw_q.trim().is_empty() {
        Vec::new()
    } else {
        filter_visible(state.db.search_webs(&raw_q).await.unwrap_or_default(), &visible)
    };
    BuscarTemplate {
        title: if raw_q.trim().is_empty() {
            "Buscar".to_string()
        } else {
            format!("«{}»", raw_q)
        },
        is_guest: user.id == 0,
        role: user.role.clone(),
        q: raw_q,
        apps,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "categories/buscar.html")]
pub struct BuscarTemplate {
    pub title: String,
    pub is_guest: bool,
    pub role: String,
    pub q: String,
    pub apps: Vec<Web>,
}

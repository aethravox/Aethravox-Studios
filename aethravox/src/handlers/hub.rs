use askama::Template;
use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use std::collections::HashMap;

use crate::{auth_mw::AuthUser, models::Web, AppState};

// Slugs de webs que NO se muestran como cards en el hub
const HIDDEN_SLUGS: &[&str] = &["hub", "login", "dashboard", "correo"];
// Máximo de apps por tira de categoría en el hub (Sección 4.4). Si hay más,
// el botón "Ver más" lleva a /categoria/:slug con todas.
const HUB_CARDS_PER_CATEGORY: usize = 18;

// ===== /hub — panel principal (rediseño Sección 4: tiras por categoría) =====
pub async fn hub(State(state): State<AppState>, AuthUser(user): AuthUser) -> impl IntoResponse {
    // Apps visibles para el rol del viewer (web_role_perms + is_active).
    // Se filtran las ocultas (hub/login/dashboard no son "apps").
    // list_webs_for_user: owner → todas; resto → list_webs_for_role (que para
    // 'invitado' lee role_web_perms por role_key, ver db/webs.rs).
    let visible: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !HIDDEN_SLUGS.contains(&w.slug.as_str()))
        .collect();

    // Categorías + mapeo app_id → category_id para agrupar en Rust.
    let cats = state.db.list_categories().await.unwrap_or_default();
    let pairs = state.db.list_app_category_pairs().await.unwrap_or_default();
    let app_to_cat: HashMap<i64, i64> = pairs.into_iter().collect();

    // category_id → apps visibles en esa categoría.
    let mut by_cat: HashMap<i64, Vec<Web>> = HashMap::new();
    for w in &visible {
        if let Some(&cid) = app_to_cat.get(&w.id) {
            by_cat.entry(cid).or_default().push(w.clone());
        }
    }

    // Tiras: solo categorías con ≥1 app visible, ordenadas por position (list_categories
    // ya viene ordenada). Cada tira limita a HUB_CARDS_PER_CATEGORY apps; el "Ver más"
    // del template lleva a /categoria/:slug con el resto.
    let categories: Vec<CategoryRow> = cats
        .iter()
        .filter_map(|c| {
            let items = by_cat.remove(&c.id).unwrap_or_default();
            if items.is_empty() {
                return None;
            }
            let items = if items.len() > HUB_CARDS_PER_CATEGORY {
                items.into_iter().take(HUB_CARDS_PER_CATEGORY).collect()
            } else {
                items
            };
            Some(CategoryRow {
                name: c.name.clone(),
                slug: c.slug.clone(),
                items,
            })
        })
        .collect();

    let (owner_count, admin_count, staff_count, user_count) =
        state.db.count_users_by_role().await.unwrap_or((0, 0, 0, 0));

    // Para el badge: si el usuario tiene un rol custom, mostrar su nombre real
    // (p.ej. "Moderador") en vez del legacy "admin". css_key = role_key real
    // (custom_N) para que role-{{ css_key }} coja el color de /role-colors.css.
    let (role_css_key, role_display) = state.db.role_display(&user).await;

    // ¿Puede entrar a /admin? El botón del panel admin vive abajo del hub
    // (Sección 4.10), fuera del panel inferior, solo visible si can_access_admin.
    let can_access_admin = state.db.user_has_perm(&user, "admin.access").await;

    HubTemplate {
        username: user.username.clone(),
        role: user.role.clone(),
        role_css_key,
        role_display,
        can_access_admin,
        display: user.display(),
        avatar: user.avatar(),
        is_guest: user.id == 0,
        categories,
        owner_count,
        admin_count,
        staff_count,
        user_count,
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "hub/hub.html")]
pub struct HubTemplate {
    pub username: String,
    pub role: String,
    pub role_css_key: String,
    pub role_display: String,
    pub can_access_admin: bool,
    pub display: String,
    pub avatar: String,
    /// true si el viewer es invitado sin sesión (User sintético id=0).
    pub is_guest: bool,
    /// Tiras de categorías con sus apps visibles (hasta 18 c/u).
    pub categories: Vec<CategoryRow>,
    pub owner_count: i64,
    pub admin_count: i64,
    pub staff_count: i64,
    pub user_count: i64,
}

/// Una tira del hub: nombre de categoría + slug (para "Ver más") + apps.
pub struct CategoryRow {
    pub name: String,
    pub slug: String,
    pub items: Vec<Web>,
}

#[allow(dead_code)]
fn _silence_redirect() -> Redirect {
    Redirect::to("/")
}

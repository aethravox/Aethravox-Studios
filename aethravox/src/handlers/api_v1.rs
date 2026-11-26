use std::collections::HashMap;

use aethravox_core::{
    ApiError, AuthSuccess, CategoryRowDto, HubPayload, HubStatsDto, LoginRequest, RegisterRequest,
    ThemeKind, ThemeSelectRequest, UserSummary, WebAppDto,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use crate::{
    db::verify_password,
    models::{validate_email, validate_password, validate_username, User, Web},
    AppState,
};

const HIDDEN_SLUGS: &[&str] = &["hub", "login", "dashboard", "correo"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/me", get(me))
        .route("/api/v1/hub", get(hub))
        .route("/api/v1/theme", post(select_theme))
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ApiError { error: msg.into() })).into_response()
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let trimmed = raw.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn resolve_bearer_or_guest(state: &AppState, headers: &HeaderMap) -> User {
    if let Some(tok) = extract_bearer_token(headers) {
        if let Ok(Some(u)) = state.db.get_session_user(&tok).await {
            return u;
        }
    }
    let rid = state
        .invitado_role_id
        .get_or_try_init(|| async { state.db.role_id_by_key("invitado").await })
        .await
        .ok()
        .copied()
        .flatten();
    User::guest(rid)
}

async fn require_bearer_user(state: &AppState, headers: &HeaderMap) -> Result<User, Response> {
    let Some(tok) = extract_bearer_token(headers) else {
        return Err(json_err(StatusCode::UNAUTHORIZED, "Token de sesión requerido."));
    };
    match state.db.get_session_user(&tok).await {
        Ok(Some(u)) => Ok(u),
        _ => Err(json_err(StatusCode::UNAUTHORIZED, "Sesión inválida o expirada.")),
    }
}

async fn build_user_summary(state: &AppState, user: &User) -> UserSummary {
    let (role_css_key, role_display) = state.db.role_display(user).await;
    let role_colors = state.role_colors.snapshot();
    let (role_color, role_text_color) = role_colors
        .get(&role_css_key)
        .map(|e| (e.color.clone(), e.text.clone()))
        .unwrap_or_else(|| ("#6b7280".to_string(), "#ffffff".to_string()));

    let can_access_admin = if user.id == 0 {
        false
    } else {
        state.db.user_has_perm(user, "admin.access").await
    };

    let theme = if user.id == 0 {
        ThemeKind::Aethravox
    } else {
        state
            .db
            .get_user_theme(user.id)
            .await
            .map(|t| ThemeKind::from_key(&t))
            .unwrap_or(ThemeKind::Aethravox)
    };

    UserSummary {
        id: user.id,
        username: user.username.clone(),
        display: user.display(),
        role_key: role_css_key,
        role_display,
        role_color,
        role_text_color,
        is_guest: user.id == 0,
        can_access_admin,
        bio: user.bio.clone().unwrap_or_default(),
        theme,
    }
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> Response {
    let user = match state.db.find_user_by_username(req.username.trim()).await {
        Ok(Some(u)) => u,
        _ => return json_err(StatusCode::UNAUTHORIZED, "Usuario no encontrado."),
    };

    if state.db.is_aexos_only_user(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            "Esta cuenta es exclusiva de aexOS y no tiene contraseña externa.",
        );
    }

    if !verify_password(&req.password, &user.password_hash) {
        return json_err(StatusCode::UNAUTHORIZED, "Contraseña incorrecta.");
    }

    if let Ok(Some(ban)) = state.db.is_user_banned(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            ban.reason
                .unwrap_or_else(|| "Cuenta baneada.".to_string()),
        );
    }

    if let Ok(Some(susp)) = state.db.is_user_suspended(user.id).await {
        return json_err(
            StatusCode::FORBIDDEN,
            susp.reason
                .unwrap_or_else(|| "Cuenta suspendida temporalmente.".to_string()),
        );
    }

    let _ = state.db.touch_last_login(user.id).await;
    let Ok((token, _)) = state.db.create_session(user.id, req.remember).await else {
        return json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "No se pudo crear la sesión.",
        );
    };

    let summary = build_user_summary(&state, &user).await;
    Json(AuthSuccess {
        token,
        user: summary,
    })
    .into_response()
}

async fn register(State(state): State<AppState>, Json(req): Json<RegisterRequest>) -> Response {
    if let Err(e) = validate_username(&req.username) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    if let Err(e) = validate_email(&req.email) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }
    if let Err(e) = validate_password(&req.password) {
        return json_err(StatusCode::BAD_REQUEST, e);
    }

    if state
        .db
        .find_user_by_username(&req.username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return json_err(StatusCode::CONFLICT, "Ese nombre de usuario ya está en uso.");
    }

    if state
        .db
        .find_user_by_email(&req.email)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return json_err(StatusCode::CONFLICT, "Ese email ya está registrado.");
    }

    let uid = match state
        .db
        .create_user(&req.username, &req.email, &req.password)
        .await
    {
        Ok(id) => id,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, format!("Error creando cuenta: {e}")),
    };

    let Ok(Some(user)) = state.db.find_user_by_id(uid).await else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "Usuario no encontrado tras crear.");
    };

    let Ok((token, _)) = state.db.create_session(uid, true).await else {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, "No se pudo crear sesión.");
    };

    let summary = build_user_summary(&state, &user).await;
    Json(AuthSuccess {
        token,
        user: summary,
    })
    .into_response()
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(tok) = extract_bearer_token(&headers) {
        let _ = state.db.delete_session(&tok).await;
    }
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match require_bearer_user(&state, &headers).await {
        Ok(user) => Json(build_user_summary(&state, &user).await).into_response(),
        Err(resp) => resp,
    }
}

async fn select_theme(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ThemeSelectRequest>,
) -> Response {
    let user = match require_bearer_user(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let theme = ThemeKind::from_key(&req.theme);
    let _ = state.db.set_user_theme(user.id, theme.as_str()).await;
    Json(serde_json::json!({ "ok": true, "theme": theme.as_str() })).into_response()
}

async fn hub(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = resolve_bearer_or_guest(&state, &headers).await;
    let summary = build_user_summary(&state, &user).await;

    let visible: Vec<Web> = state
        .db
        .list_webs_for_user_obj(&user)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !HIDDEN_SLUGS.contains(&w.slug.as_str()))
        .collect();

    let cats = state.db.list_categories().await.unwrap_or_default();
    let pairs = state.db.list_app_category_pairs().await.unwrap_or_default();
    let app_to_cat: HashMap<i64, i64> = pairs.into_iter().collect();

    let mut by_cat: HashMap<i64, Vec<WebAppDto>> = HashMap::new();
    for w in &visible {
        if let Some(&cid) = app_to_cat.get(&w.id) {
            let dto = WebAppDto {
                id: w.id,
                slug: w.slug.clone(),
                name: w.name.clone(),
                description: w.description_or_empty(),
                icon: w.icon_emoji(),
                url: w.effective_url(),
                tag: w.tag_normalized(),
                tag_label: w.tag_label().to_string(),
                tag_color: w.tag_color().to_string(),
                clickable: w.is_clickable_for(&user.role),
                lock_message: w.tag_display_message(),
            };
            by_cat.entry(cid).or_default().push(dto);
        }
    }

    let categories: Vec<CategoryRowDto> = cats
        .into_iter()
        .filter_map(|c| {
            let items = by_cat.remove(&c.id)?;
            if items.is_empty() {
                None
            } else {
                Some(CategoryRowDto {
                    name: c.name,
                    slug: c.slug,
                    items,
                })
            }
        })
        .collect();

    let (owner_count, admin_count, staff_count, user_count) =
        state.db.count_users_by_role().await.unwrap_or((0, 0, 0, 0));
    let online_count = state
        .db
        .list_online_user_ids()
        .await
        .unwrap_or_default()
        .len() as i64;
    let doc_count = state.db.list_documents().await.unwrap_or_default().len() as i64;
    let chat_unread = if user.id == 0 {
        0
    } else {
        state
            .db
            .count_total_unread_for_user(user.id)
            .await
            .unwrap_or(0)
    };
    let community_unread = if user.id == 0 {
        0
    } else {
        state
            .db
            .count_community_unread_for_user(user.id)
            .await
            .unwrap_or(0)
    };

    Json(HubPayload {
        user: summary,
        categories,
        stats: HubStatsDto {
            online_count,
            doc_count,
            chat_unread,
            community_unread,
            owner_count,
            admin_count,
            staff_count,
            user_count,
        },
    })
    .into_response()
}

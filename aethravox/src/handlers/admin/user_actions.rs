#![allow(unused_imports)]
// Web 2 — Panel de administración (Owner + Admin).

use askama::Template;
use axum::{
    extract::{OriginalUri, Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use tower_cookies::Cookies;

use crate::{
    auth_mw::AuthUser,
    models::{
        ActionOnUser, AdminCreateUserForm, BanForm, ChangeRoleForm, CreateWebForm,
        SuspendForm, UpdateConfigForm, User, WebPermsForm,
    },
    AppState,
};

// ===== Idea 1: re-autenticación para /admin/* =====



pub async fn change_role(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<ChangeRoleForm>,
) -> impl IntoResponse {
    if !crate::models::is_role(&form.role) {
        return Redirect::to("/admin/users?err=bad_role").into_response();
    }

    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };

    if target.id == actor.id && target.role == "owner" && form.role != "owner" {
        return Redirect::to("/admin/users?err=owner_self_demote").into_response();
    }

    if !state.db.user_has_perm(&actor, "users.role.change").await
        || !actor.can_act_on(&target, ActionOnUser::ChangeRole) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }

    if form.role == "owner" {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE role='owner' AND id<>?",
        )
        .bind(id)
        .fetch_optional(&state.db.pool)
        .await
        .ok()
        .flatten();
        if matches!(row, Some((c,)) if c > 0) {
            return Redirect::to("/admin/users?err=double_owner").into_response();
        }
    }

    let _ = state.db.set_user_role(id, &form.role).await;
    let detail = format!("{} → {} (id={})", target.username, form.role, id);
    let _ = state
        .db
        .audit(Some(actor.id), "role_change", Some(&detail), None)
        .await;
    let _ = state.db.revoke_all_user_sessions(id).await;

    Redirect::to("/admin/users?ok=role").into_response()
}

// ===== POST /admin/users/:id/delete =====
pub async fn delete_user(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };

    if target.role == "owner" {
        return Redirect::to("/admin/users?err=cannot_delete_owner").into_response();
    }

    if !state.db.user_has_perm(&actor, "users.delete").await
        || !actor.can_act_on(&target, ActionOnUser::Delete) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }

    let _ = state.db.delete_user(id).await;
    let detail = format!("{} (id={})", target.username, id);
    let _ = state
        .db
        .audit(Some(actor.id), "user_delete", Some(&detail), None)
        .await;
    Redirect::to("/admin/users?ok=deleted").into_response()
}

// ===== POST /admin/users/:id/force_logout =====
pub async fn force_logout(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };
    if !state.db.user_has_perm(&actor, "users.force_logout").await
        || !actor.can_act_on(&target, ActionOnUser::ForceLogout) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }
    let _ = state.db.revoke_all_user_sessions(id).await;
    let detail = format!("{} (id={})", target.username, id);
    let _ = state
        .db
        .audit(Some(actor.id), "force_logout", Some(&detail), None)
        .await;
    Redirect::to("/admin/users?ok=forced").into_response()
}

// ===== POST /admin/users/:id/ban =====
pub async fn ban_user(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<BanForm>,
) -> impl IntoResponse {
    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };
    let permanent = form.permanent.is_some();
    let action = if permanent {
        ActionOnUser::BanPermanent
    } else {
        ActionOnUser::BanTemporary
    };
    let ban_perm = if permanent { "users.ban.permanent" } else { "users.ban.temporary" };
    if !state.db.user_has_perm(&actor, ban_perm).await || !actor.can_act_on(&target, action) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }

    let reason = form.reason.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // Convertir duration_value + duration_unit a horas (para create_ban que recibe horas)
    let duration_hours: Option<i64> = if permanent {
        None
    } else {
        let value = form.duration_value.unwrap_or(24);
        let unit = form.duration_unit.as_deref().unwrap_or("hours");
        let hours = match unit {
            "minutes" => value.max(1) / 60,  // mínimo 1 minuto (redondea a entero)
            "hours"   => value,
            "days"    => value * 24,
            "weeks"   => value * 24 * 7,
            "months"  => value * 24 * 30,    // aproximado
            _ => value,                      // default treat as hours
        };
        Some(hours.max(1))
    };

    let _ = state
        .db
        .create_ban(id, actor.id, reason, permanent, duration_hours)
        .await;
    let _ = state.db.set_user_status(id, "banned").await;
    let _ = state.db.revoke_all_user_sessions(id).await;

    let detail = format!(
        "{} (id={}) {} — {}",
        target.username,
        id,
        if permanent { "BAN_PERMANENT" } else { "BAN_TEMP" },
        reason.unwrap_or("(sin razón)")
    );
    let _ = state
        .db
        .audit(Some(actor.id), "user_ban", Some(&detail), None)
        .await;
    Redirect::to("/admin/users?ok=banned").into_response()
}

// ===== POST /admin/users/:id/suspend =====
pub async fn suspend_user(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
    Form(form): Form<SuspendForm>,
) -> impl IntoResponse {
    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };
    if !state.db.user_has_perm(&actor, "users.suspend").await
        || !actor.can_act_on(&target, ActionOnUser::Suspend) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }

    let reason = form.reason.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // Convertir duration_value + duration_unit a horas
    let value = form.duration_value.max(1);
    let unit = form.duration_unit.as_deref().unwrap_or("hours");
    let duration_hours = match unit {
        "minutes" => value.max(1) / 60,    // mínimo 1 minuto
        "hours"   => value,
        "days"    => value * 24,
        "weeks"   => value * 24 * 7,
        "months"  => value * 24 * 30,
        _ => value,
    }.max(1);

    let _ = state
        .db
        .create_suspension(id, actor.id, reason, duration_hours)
        .await;
    let _ = state.db.set_user_status(id, "suspended").await;
    let _ = state.db.revoke_all_user_sessions(id).await;

    let detail = format!(
        "{} (id={}) SUSPEND {}h ({} {}) — {}",
        target.username, id, duration_hours, value, unit, reason.unwrap_or("(sin razón)")
    );
    let _ = state
        .db
        .audit(Some(actor.id), "user_suspend", Some(&detail), None)
        .await;
    Redirect::to("/admin/users?ok=suspended").into_response()
}

// ===== POST /admin/users/:id/unban =====
pub async fn unban_user(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let target = match state.db.find_user_by_id(id).await {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/admin/users?err=not_found").into_response(),
    };
    if !state.db.user_has_perm(&actor, "users.unban").await
        || !actor.can_act_on(&target, ActionOnUser::BanTemporary) {
        return Redirect::to("/admin/users?err=forbidden").into_response();
    }
    let _ = sqlx::query(
        "UPDATE user_bans SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(Utc::now())
    .bind(id)
    .execute(&state.db.pool)
    .await;
    let _ = sqlx::query(
        "UPDATE user_suspensions SET revoked_at = ? WHERE user_id = ? AND revoked_at IS NULL",
    )
    .bind(Utc::now())
    .bind(id)
    .execute(&state.db.pool)
    .await;
    let _ = state.db.set_user_status(id, "active").await;
    let detail = format!("{} (id={}) UNBAN/UNSUSPEND", target.username, id);
    let _ = state
        .db
        .audit(Some(actor.id), "user_unban", Some(&detail), None)
        .await;
    Redirect::to("/admin/users?ok=unbanned").into_response()
}

// ===== /admin/webs =====

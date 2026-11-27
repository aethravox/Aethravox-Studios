//! Aethravox Studios — Capa de datos de Aris + API keys.
//!
//! Queries de chats, mensajes, miembros, rate limits y gestión
//! de API keys con failover automático.

use sqlx::SqlitePool;

use super::Db;
use crate::handlers::aris::{
    ApiKeyRow, ApiProviderRow, ApiRotationLogRow,
    ArisChat, ArisChatMember, ArisChatMemberView, ArisCreditLogRow, ArisCreditsStatus, ArisMessage,
};

// ============================================================
// Chats
// ============================================================

/// Crea un chat nuevo. Devuelve el id.
pub async fn create_aris_chat(
    pool: &SqlitePool,
    owner_id: i64,
    title: &str,
    history_limit: i64,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query_scalar(
        "INSERT INTO aris_chats (owner_id, title, history_limit)
         VALUES (?, ?, ?)
         RETURNING id",
    )
    .bind(owner_id)
    .bind(title)
    .bind(history_limit)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Chat por id (con verificación de acceso implícita hecha por el caller).
pub async fn get_aris_chat(pool: &SqlitePool, chat_id: i64) -> Result<Option<ArisChat>, sqlx::Error> {
    let row = sqlx::query_as::<_, ArisChat>(
        "SELECT id, owner_id, title, history_limit, pinned,
                strftime('%Y-%m-%dT%H:%M:%SZ', created_at) as created_at,
                strftime('%Y-%m-%dT%H:%M:%SZ', updated_at) as updated_at
         FROM aris_chats WHERE id = ?",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lista los chats donde el usuario es dueño O miembro invitado.
pub async fn list_aris_chats_for_user(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<ArisChat>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ArisChat>(
        "SELECT DISTINCT c.id, c.owner_id, c.title, c.history_limit, c.pinned,
                strftime('%Y-%m-%dT%H:%M:%SZ', c.created_at) as created_at,
                strftime('%Y-%m-%dT%H:%M:%SZ', c.updated_at) as updated_at
         FROM aris_chats c
         LEFT JOIN aris_chat_members m ON m.chat_id = c.id
         WHERE c.owner_id = ? OR m.user_id = ?
         ORDER BY c.pinned DESC, c.updated_at DESC",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Actualiza título / límite de historial. Solo el dueño (verificado en handler).
pub async fn update_aris_chat_settings(
    pool: &SqlitePool,
    chat_id: i64,
    title: Option<&str>,
    history_limit: Option<i64>,
) -> Result<(), sqlx::Error> {
    if let Some(t) = title {
        sqlx::query("UPDATE aris_chats SET title = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?")
            .bind(t).bind(chat_id).execute(pool).await?;
    }
    if let Some(h) = history_limit {
        if [20, 50, 100, 200].contains(&h) {
            sqlx::query("UPDATE aris_chats SET history_limit = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?")
                .bind(h).bind(chat_id).execute(pool).await?;
        }
    }
    Ok(())
}

/// Alterna el estado fijado (pinned) de un chat. Devuelve el nuevo estado.
pub async fn toggle_aris_chat_pin(pool: &SqlitePool, chat_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query("UPDATE aris_chats SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END WHERE id = ?")
        .bind(chat_id)
        .execute(pool)
        .await?;
    let pinned: i64 = sqlx::query_scalar("SELECT pinned FROM aris_chats WHERE id = ?")
        .bind(chat_id)
        .fetch_one(pool)
        .await?;
    Ok(pinned != 0)
}

/// Borra un chat entero (solo dueño, verificado en handler).
pub async fn delete_aris_chat(pool: &SqlitePool, chat_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM aris_chats WHERE id = ?").bind(chat_id).execute(pool).await?;
    Ok(())
}

// ============================================================
// Miembros / permisos
// ============================================================

/// Invita a un usuario. invited_by debe ser el dueño (verificado en handler).
pub async fn add_aris_member(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: i64,
    can_read_history: bool,
    can_send: bool,
    invited_by: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO aris_chat_members (chat_id, user_id, can_read_history, can_send, invited_by)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(chat_id, user_id) DO UPDATE SET
             can_read_history = excluded.can_read_history,
             can_send = excluded.can_send",
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(can_read_history as i64)
    .bind(can_send as i64)
    .bind(invited_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Miembro con sus permisos, si existe.
pub async fn get_aris_member(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: i64,
) -> Result<Option<ArisChatMember>, sqlx::Error> {
    let row = sqlx::query_as::<_, ArisChatMember>(
        "SELECT chat_id, user_id, can_read_history, can_send, invited_by
         FROM aris_chat_members WHERE chat_id = ? AND user_id = ?",
    )
    .bind(chat_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Lista miembros de un chat (para el modal de gestión).
pub async fn list_aris_members(
    pool: &SqlitePool,
    chat_id: i64,
) -> Result<Vec<ArisChatMemberView>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ArisChatMemberView>(
        "SELECT m.chat_id, m.user_id, m.can_read_history, m.can_send, m.invited_by, u.username
         FROM aris_chat_members m JOIN users u ON u.id = m.user_id
         WHERE m.chat_id = ? ORDER BY m.created_at",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Expulsa a un miembro.
pub async fn remove_aris_member(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM aris_chat_members WHERE chat_id = ? AND user_id = ?")
        .bind(chat_id).bind(user_id).execute(pool).await?;
    Ok(())
}

// ============================================================
// Mensajes
// ============================================================

/// Guarda un mensaje (user o aris). Devuelve el id.
pub async fn add_aris_message(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: Option<i64>,
    role: &str,
    content: &str,
    model: Option<&str>,
    provider: Option<&str>,
) -> Result<i64, sqlx::Error> {
    add_aris_message_with_meta(pool, chat_id, user_id, role, content, model, provider, None).await
}

/// Guarda un mensaje con metadatos JSON opcionales (adjuntos, modo, pensamiento, créditos).
pub async fn add_aris_message_with_meta(
    pool: &SqlitePool,
    chat_id: i64,
    user_id: Option<i64>,
    role: &str,
    content: &str,
    model: Option<&str>,
    provider: Option<&str>,
    meta_json: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query_scalar(
        "INSERT INTO aris_messages (chat_id, user_id, role, content, model, provider, meta_json)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(chat_id)
    .bind(user_id)
    .bind(role)
    .bind(content)
    .bind(model)
    .bind(provider)
    .bind(meta_json)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Últimos N mensajes del chat (para el contexto del modelo).
pub async fn list_aris_messages(
    pool: &SqlitePool,
    chat_id: i64,
    limit: i64,
) -> Result<Vec<ArisMessage>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ArisMessage>(
        "SELECT id, chat_id, user_id, role, content, model, provider, meta_json,
                strftime('%Y-%m-%dT%H:%M:%SZ', created_at) as created_at
         FROM aris_messages WHERE chat_id = ?
         ORDER BY id DESC LIMIT ?",
    )
    .bind(chat_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    let mut rows = rows;
    rows.reverse(); // orden cronológico para el contexto
    Ok(rows)
}

/// Purgado: borra mensajes que exceden history_limit del chat.
/// Conserva siempre los últimos N. Corre tras cada mensaje nuevo.
pub async fn purge_aris_history(pool: &SqlitePool, chat_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM aris_messages WHERE chat_id = ? AND id NOT IN (
            SELECT id FROM aris_messages WHERE chat_id = ?
            ORDER BY id DESC LIMIT (SELECT history_limit FROM aris_chats WHERE id = ?)
        )",
    )
    .bind(chat_id).bind(chat_id).bind(chat_id)
    .execute(pool).await?;
    Ok(())
}

/// Toca updated_at del chat (para ordenar la lista).
pub async fn touch_aris_chat(pool: &SqlitePool, chat_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE aris_chats SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id = ?")
        .bind(chat_id).execute(pool).await?;
    Ok(())
}

// ============================================================
// Rate limiting
// ============================================================

/// Estado del rate limit del usuario. Si no existe, lo crea.
/// Devuelve (hourly_count, daily_count).
pub async fn aris_rate_limit_check(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<(i64, i64), sqlx::Error> {
    // Reset de ventana horaria si pasó más de 1 hora
    sqlx::query(
        "UPDATE aris_rate_limits
         SET hourly_count = 0,
             hourly_start = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE user_id = ?
           AND hourly_start < strftime('%Y-%m-%dT%H:%M:%SZ','now', '-1 hour')",
    )
    .bind(user_id).execute(pool).await?;

    // Reset del contador diario si cambió el día
    sqlx::query(
        "UPDATE aris_rate_limits
         SET daily_count = 0, daily_start = date('now')
         WHERE user_id = ? AND daily_start < date('now')",
    )
    .bind(user_id).execute(pool).await?;

    sqlx::query(
        "INSERT INTO aris_rate_limits (user_id) VALUES (?)
         ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(user_id).execute(pool).await?;

    let row: (i64, i64) = sqlx::query_as(
        "SELECT hourly_count, daily_count FROM aris_rate_limits WHERE user_id = ?",
    )
    .bind(user_id).fetch_one(pool).await?;
    Ok(row)
}

/// Incrementa el contador tras un mensaje enviado.
pub async fn aris_rate_limit_increment(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE aris_rate_limits SET hourly_count = hourly_count + 1, daily_count = daily_count + 1
         WHERE user_id = ?",
    )
    .bind(user_id).execute(pool).await?;
    Ok(())
}

// ============================================================
// Créditos diarios de Aris (modos, pensamiento y multimodal)
// ============================================================

pub async fn get_aris_daily_limit(pool: &SqlitePool) -> i64 {
    aris_get_config(pool, "daily_credits_limit")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .map(|n| n.max(10))
        .unwrap_or(120)
}

pub async fn get_aris_credits_status(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<ArisCreditsStatus, sqlx::Error> {
    let daily_limit = get_aris_daily_limit(pool).await;
    if user_id <= 0 {
        return Ok(ArisCreditsStatus {
            daily_limit,
            daily_used: 0,
            remaining: daily_limit,
        });
    }

    sqlx::query(
        "INSERT INTO aris_daily_credits (user_id) VALUES (?)
         ON CONFLICT(user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE aris_daily_credits
         SET daily_used = 0,
             last_reset_date = date('now'),
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE user_id = ? AND last_reset_date < date('now')",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    let daily_used: i64 = sqlx::query_scalar(
        "SELECT daily_used FROM aris_daily_credits WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let remaining = (daily_limit - daily_used).max(0);
    Ok(ArisCreditsStatus {
        daily_limit,
        daily_used,
        remaining,
    })
}

pub async fn consume_aris_daily_credits(
    pool: &SqlitePool,
    user_id: i64,
    chat_id: Option<i64>,
    mode: &str,
    thinking_enabled: bool,
    thinking_level: &str,
    attachments_summary: &str,
    cost: i64,
) -> Result<ArisCreditsStatus, String> {
    let status = get_aris_credits_status(pool, user_id)
        .await
        .map_err(|_| "server".to_string())?;
    let clean_cost = cost.max(1);
    if status.remaining < clean_cost {
        return Err(format!(
            "No tienes suficientes créditos diarios (necesitas {}, disponibles {}). Reduce el nivel de pensamiento o adjuntos, o espera al reinicio diario.",
            clean_cost, status.remaining
        ));
    }

    if user_id > 0 {
        sqlx::query(
            "UPDATE aris_daily_credits
             SET daily_used = daily_used + ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE user_id = ?",
        )
        .bind(clean_cost)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|_| "server".to_string())?;

        let _ = sqlx::query(
            "INSERT INTO aris_credit_logs
                (user_id, chat_id, mode, thinking_enabled, thinking_level, attachments_summary, credits_cost)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(chat_id)
        .bind(mode)
        .bind(if thinking_enabled { 1i64 } else { 0i64 })
        .bind(thinking_level)
        .bind(attachments_summary)
        .bind(clean_cost)
        .execute(pool)
        .await;

        let _ = sqlx::query(
            "DELETE FROM aris_credit_logs
             WHERE user_id = ?
               AND id NOT IN (
                   SELECT id FROM aris_credit_logs WHERE user_id = ? ORDER BY id DESC LIMIT 100
               )",
        )
        .bind(user_id)
        .bind(user_id)
        .execute(pool)
        .await;
    }

    get_aris_credits_status(pool, user_id)
        .await
        .map_err(|_| "server".to_string())
}

pub async fn list_aris_credit_logs(
    pool: &SqlitePool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<ArisCreditLogRow>, sqlx::Error> {
    if user_id <= 0 {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, ArisCreditLogRow>(
        "SELECT id, user_id, chat_id, mode, thinking_enabled, thinking_level,
                attachments_summary, credits_cost,
                strftime('%Y-%m-%dT%H:%M:%SZ', created_at) as created_at
         FROM aris_credit_logs
         WHERE user_id = ?
         ORDER BY id DESC LIMIT ?",
    )
    .bind(user_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ============================================================
// Config de Aris
// ============================================================

pub async fn aris_get_config(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let v: Option<(String,)> = sqlx::query_as("SELECT value FROM aris_config WHERE key = ?")
        .bind(key).fetch_optional(pool).await?;
    Ok(v.map(|r| r.0))
}

pub async fn aris_set_config(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO aris_config (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key).bind(value).execute(pool).await?;
    Ok(())
}

// ============================================================
// API KEYS & PROVEEDORES con rotación ilimitada + failover
// ============================================================

async fn reset_stale_daily_key_counts(pool: &SqlitePool) {
    let _ = sqlx::query(
        "UPDATE api_keys SET daily_uses = 0, daily_start = date('now') WHERE daily_start < date('now')",
    )
    .execute(pool)
    .await;
}

/// Lista TODAS las keys (admin: gestión).
pub async fn list_api_keys(pool: &SqlitePool) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    reset_stale_daily_key_counts(pool).await;
    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.provider_id, k.api_key, k.label, k.is_active,
                k.total_uses, k.daily_uses, k.daily_cap, k.rate_limited_until, k.last_error, k.last_used_at,
                p.name as provider_name, p.provider_type, p.base_url, p.default_model, p.priority as provider_priority
         FROM api_keys k JOIN api_providers p ON p.id = k.provider_id
         ORDER BY p.priority ASC, p.name ASC, k.id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Obtiene una sola key por id (para health check).
pub async fn get_api_key_by_id(pool: &SqlitePool, key_id: i64) -> Result<Option<ApiKeyRow>, sqlx::Error> {
    reset_stale_daily_key_counts(pool).await;
    let row = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.provider_id, k.api_key, k.label, k.is_active,
                k.total_uses, k.daily_uses, k.daily_cap, k.rate_limited_until, k.last_error, k.last_used_at,
                p.name as provider_name, p.provider_type, p.base_url, p.default_model, p.priority as provider_priority
         FROM api_keys k JOIN api_providers p ON p.id = k.provider_id
         WHERE k.id = ?",
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Keys ACTIVAS y DISPONIBLES de un provider específico.
pub async fn get_available_api_keys(
    pool: &SqlitePool,
    provider_type: &str,
) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    reset_stale_daily_key_counts(pool).await;
    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.provider_id, k.api_key, k.label, k.is_active,
                k.total_uses, k.daily_uses, k.daily_cap, k.rate_limited_until, k.last_error, k.last_used_at,
                p.name as provider_name, p.provider_type, p.base_url, p.default_model, p.priority as provider_priority
         FROM api_keys k
         JOIN api_providers p ON p.id = k.provider_id
         WHERE p.provider_type = ? AND p.is_active = 1 AND k.is_active = 1
           AND (k.rate_limited_until IS NULL
                OR k.rate_limited_until < strftime('%Y-%m-%dT%H:%M:%SZ','now'))
           AND (k.daily_cap IS NULL OR k.daily_cap <= 0 OR k.daily_uses < k.daily_cap)
         ORDER BY k.last_used_at ASC NULLS FIRST, k.id ASC",
    )
    .bind(provider_type)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// TODAS las keys disponibles de TODOS los proveedores activos, ordenadas según estrategia:
/// - `round_robin`: least-recently-used global entre todas las keys.
/// - `least_used`: menor cantidad de usos diarios/totales primero.
/// - `provider_order`: prioridad del proveedor (`p.priority ASC`) y luego round-robin interno.
pub async fn get_all_available_api_keys(
    pool: &SqlitePool,
    strategy: &str,
) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
    reset_stale_daily_key_counts(pool).await;
    let order_clause = match strategy {
        "least_used" => "ORDER BY k.daily_uses ASC, k.total_uses ASC, k.id ASC",
        "provider_order" => "ORDER BY p.priority ASC, k.last_used_at ASC NULLS FIRST, k.id ASC",
        _ => "ORDER BY k.last_used_at ASC NULLS FIRST, k.id ASC",
    };

    let sql = format!(
        "SELECT k.id, k.provider_id, k.api_key, k.label, k.is_active,
                k.total_uses, k.daily_uses, k.daily_cap, k.rate_limited_until, k.last_error, k.last_used_at,
                p.name as provider_name, p.provider_type, p.base_url, p.default_model, p.priority as provider_priority
         FROM api_keys k
         JOIN api_providers p ON p.id = k.provider_id
         WHERE p.is_active = 1 AND k.is_active = 1
           AND (k.rate_limited_until IS NULL
                OR k.rate_limited_until < strftime('%Y-%m-%dT%H:%M:%SZ','now'))
           AND (k.daily_cap IS NULL OR k.daily_cap <= 0 OR k.daily_uses < k.daily_cap)
         {}",
        order_clause
    );

    let rows = sqlx::query_as::<_, ApiKeyRow>(&sql)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Marca una key como rate-limited hasta `minutes` minutos.
pub async fn mark_key_rate_limited(
    pool: &SqlitePool,
    key_id: i64,
    minutes: i64,
    error: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE api_keys SET
            rate_limited_until = strftime('%Y-%m-%dT%H:%M:%SZ','now', ?),
            last_error = ?
         WHERE id = ?",
    )
    .bind(format!("+{} minutes", minutes.max(1)))
    .bind(error)
    .bind(key_id)
    .execute(pool).await?;
    Ok(())
}

/// Registra error no-rate-limit en la key (por ej. fallo en health check).
pub async fn set_key_last_error(
    pool: &SqlitePool,
    key_id: i64,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET last_error = ? WHERE id = ?")
        .bind(error)
        .bind(key_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Registra uso exitoso de una key (contadores + last_used_at round-robin).
pub async fn record_key_success(pool: &SqlitePool, key_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE api_keys SET
            total_uses = total_uses + 1,
            daily_uses = CASE WHEN daily_start < date('now') THEN 1 ELSE daily_uses + 1 END,
            daily_start = date('now'),
            last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ','now'),
            rate_limited_until = NULL,
            last_error = NULL
         WHERE id = ?",
    )
    .bind(key_id).execute(pool).await?;
    Ok(())
}

/// Agrega una key nueva (admin) con daily_cap opcional.
pub async fn add_api_key(
    pool: &SqlitePool,
    provider_id: i64,
    api_key: &str,
    label: &str,
    daily_cap: Option<i64>,
) -> Result<i64, sqlx::Error> {
    let cap = daily_cap.filter(|&v| v > 0);
    let id = sqlx::query_scalar(
        "INSERT INTO api_keys (provider_id, api_key, label, daily_cap) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(provider_id).bind(api_key).bind(label).bind(cap)
    .fetch_one(pool).await?;
    Ok(id)
}

/// Actualiza el tope diario preventivo de una key.
pub async fn update_api_key_daily_cap(
    pool: &SqlitePool,
    key_id: i64,
    daily_cap: Option<i64>,
) -> Result<(), sqlx::Error> {
    let cap = daily_cap.filter(|&v| v > 0);
    sqlx::query("UPDATE api_keys SET daily_cap = ? WHERE id = ?")
        .bind(cap)
        .bind(key_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Libera todas las keys en cooldown (rate_limited_until = NULL).
pub async fn reset_all_rate_limits(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET rate_limited_until = NULL, last_error = NULL")
        .execute(pool)
        .await?;
    Ok(())
}

/// Activa/desactiva o borra una key (admin).
pub async fn toggle_api_key(pool: &SqlitePool, key_id: i64, active: bool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_keys SET is_active = ? WHERE id = ?")
        .bind(active as i64).bind(key_id).execute(pool).await?;
    Ok(())
}

pub async fn delete_api_key(pool: &SqlitePool, key_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM api_keys WHERE id = ?").bind(key_id).execute(pool).await?;
    Ok(())
}

/// Lista providers activos (legacy tupla para compatibilidad).
pub async fn list_api_providers(
    pool: &SqlitePool,
) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, provider_type FROM api_providers WHERE is_active = 1 ORDER BY priority ASC, name ASC",
    )
    .fetch_all(pool).await?;
    Ok(rows)
}

/// Lista TODOS los providers con todos sus campos (para gestión ilimitada).
pub async fn list_api_providers_full(
    pool: &SqlitePool,
) -> Result<Vec<ApiProviderRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ApiProviderRow>(
        "SELECT id, name, provider_type, base_url, default_model, is_preset, priority, is_active
         FROM api_providers
         ORDER BY priority ASC, id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Agrega un provider nuevo (sin límite por tipo).
pub async fn add_api_provider(
    pool: &SqlitePool,
    name: &str,
    provider_type: &str,
    base_url: &str,
    default_model: &str,
) -> Result<i64, sqlx::Error> {
    let next_priority: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(priority), 0) + 1 FROM api_providers",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(1);

    let id = sqlx::query_scalar(
        "INSERT INTO api_providers (name, provider_type, base_url, default_model, is_preset, priority, is_active)
         VALUES (?, ?, ?, ?, 0, ?, 1) RETURNING id",
    )
    .bind(name)
    .bind(provider_type)
    .bind(base_url)
    .bind(default_model)
    .bind(next_priority)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Edita nombre, tipo, endpoint base y modelo de un provider.
pub async fn update_api_provider(
    pool: &SqlitePool,
    provider_id: i64,
    name: &str,
    provider_type: &str,
    base_url: &str,
    default_model: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE api_providers
         SET name = ?, provider_type = ?, base_url = ?, default_model = ?
         WHERE id = ?",
    )
    .bind(name)
    .bind(provider_type)
    .bind(base_url)
    .bind(default_model)
    .bind(provider_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Activa o pausa un provider completo.
pub async fn toggle_api_provider(
    pool: &SqlitePool,
    provider_id: i64,
    active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE api_providers SET is_active = ? WHERE id = ?")
        .bind(active as i64)
        .bind(provider_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mueve un provider arriba (`delta = -1`) o abajo (`delta = 1`) en prioridad.
pub async fn move_api_provider_priority(
    pool: &SqlitePool,
    provider_id: i64,
    delta: i64,
) -> Result<(), sqlx::Error> {
    let mut providers = list_api_providers_full(pool).await?;
    if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
        let target = if delta < 0 {
            if pos == 0 { return Ok(()); }
            pos - 1
        } else {
            if pos + 1 >= providers.len() { return Ok(()); }
            pos + 1
        };
        providers.swap(pos, target);
        for (idx, p) in providers.iter().enumerate() {
            sqlx::query("UPDATE api_providers SET priority = ? WHERE id = ?")
                .bind((idx + 1) as i64)
                .bind(p.id)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Borra un provider (solo permitido si no es preset o si el caller lo valida; CASCADE borra sus keys).
pub async fn delete_api_provider(pool: &SqlitePool, provider_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM api_providers WHERE id = ?")
        .bind(provider_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ============================================================
// Bitácora de rotación y fallbacks
// ============================================================

pub async fn add_rotation_log(
    pool: &SqlitePool,
    source: &str,
    channel_label: &str,
    provider_name: &str,
    key_label: &str,
    model: &str,
    status: &str,
    latency_ms: i64,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO api_rotation_logs (source, channel_label, provider_name, key_label, model, status, latency_ms, detail)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(source)
    .bind(channel_label)
    .bind(provider_name)
    .bind(key_label)
    .bind(model)
    .bind(status)
    .bind(latency_ms)
    .bind(detail)
    .execute(pool)
    .await?;

    // Conservar últimas 300 entradas para evitar crecimiento ilimitado
    let _ = sqlx::query(
        "DELETE FROM api_rotation_logs WHERE id NOT IN (SELECT id FROM api_rotation_logs ORDER BY id DESC LIMIT 300)",
    )
    .execute(pool)
    .await;

    Ok(())
}

pub async fn list_rotation_logs(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<ApiRotationLogRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ApiRotationLogRow>(
        "SELECT id, source, channel_label, provider_name, key_label, model, status, latency_ms, detail, created_at
         FROM api_rotation_logs
         ORDER BY id DESC LIMIT ?",
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn clear_rotation_logs(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM api_rotation_logs").execute(pool).await?;
    Ok(())
}

// ============================================================
// Wrappers sobre Db (para llamar state.db.<método> directamente)
// ============================================================

impl Db {
    pub async fn create_aris_chat(
        &self,
        owner_id: i64,
        title: &str,
        history_limit: i64,
    ) -> Result<i64, sqlx::Error> {
        create_aris_chat(&self.pool, owner_id, title, history_limit).await
    }

    pub async fn get_aris_chat(&self, chat_id: i64) -> Result<Option<ArisChat>, sqlx::Error> {
        get_aris_chat(&self.pool, chat_id).await
    }

    pub async fn list_aris_chats_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<ArisChat>, sqlx::Error> {
        list_aris_chats_for_user(&self.pool, user_id).await
    }

    pub async fn update_aris_chat_settings(
        &self,
        chat_id: i64,
        title: Option<&str>,
        history_limit: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        update_aris_chat_settings(&self.pool, chat_id, title, history_limit).await
    }

    pub async fn toggle_aris_chat_pin(&self, chat_id: i64) -> Result<bool, sqlx::Error> {
        toggle_aris_chat_pin(&self.pool, chat_id).await
    }

    pub async fn delete_aris_chat(&self, chat_id: i64) -> Result<(), sqlx::Error> {
        delete_aris_chat(&self.pool, chat_id).await
    }

    pub async fn add_aris_member(
        &self,
        chat_id: i64,
        user_id: i64,
        can_read_history: bool,
        can_send: bool,
        invited_by: i64,
    ) -> Result<(), sqlx::Error> {
        add_aris_member(&self.pool, chat_id, user_id, can_read_history, can_send, invited_by).await
    }

    pub async fn get_aris_member(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<Option<ArisChatMember>, sqlx::Error> {
        get_aris_member(&self.pool, chat_id, user_id).await
    }

    pub async fn list_aris_members(
        &self,
        chat_id: i64,
    ) -> Result<Vec<ArisChatMemberView>, sqlx::Error> {
        list_aris_members(&self.pool, chat_id).await
    }

    pub async fn remove_aris_member(
        &self,
        chat_id: i64,
        user_id: i64,
    ) -> Result<(), sqlx::Error> {
        remove_aris_member(&self.pool, chat_id, user_id).await
    }

    pub async fn add_aris_message(
        &self,
        chat_id: i64,
        user_id: Option<i64>,
        role: &str,
        content: &str,
        model: Option<&str>,
        provider: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        add_aris_message(&self.pool, chat_id, user_id, role, content, model, provider).await
    }

    pub async fn add_aris_message_with_meta(
        &self,
        chat_id: i64,
        user_id: Option<i64>,
        role: &str,
        content: &str,
        model: Option<&str>,
        provider: Option<&str>,
        meta_json: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        add_aris_message_with_meta(&self.pool, chat_id, user_id, role, content, model, provider, meta_json).await
    }

    pub async fn list_aris_messages(
        &self,
        chat_id: i64,
        limit: i64,
    ) -> Result<Vec<ArisMessage>, sqlx::Error> {
        list_aris_messages(&self.pool, chat_id, limit).await
    }

    pub async fn touch_aris_chat(&self, chat_id: i64) -> Result<(), sqlx::Error> {
        touch_aris_chat(&self.pool, chat_id).await
    }

    pub async fn aris_rate_limit_check(&self, user_id: i64) -> Result<(i64, i64), sqlx::Error> {
        aris_rate_limit_check(&self.pool, user_id).await
    }

    pub async fn aris_rate_limit_increment(&self, user_id: i64) -> Result<(), sqlx::Error> {
        aris_rate_limit_increment(&self.pool, user_id).await
    }

    pub async fn get_aris_credits_status(&self, user_id: i64) -> Result<ArisCreditsStatus, sqlx::Error> {
        get_aris_credits_status(&self.pool, user_id).await
    }

    pub async fn consume_aris_daily_credits(
        &self,
        user_id: i64,
        chat_id: Option<i64>,
        mode: &str,
        thinking_enabled: bool,
        thinking_level: &str,
        attachments_summary: &str,
        cost: i64,
    ) -> Result<ArisCreditsStatus, String> {
        consume_aris_daily_credits(
            &self.pool,
            user_id,
            chat_id,
            mode,
            thinking_enabled,
            thinking_level,
            attachments_summary,
            cost,
        )
        .await
    }

    pub async fn list_aris_credit_logs(
        &self,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<ArisCreditLogRow>, sqlx::Error> {
        list_aris_credit_logs(&self.pool, user_id, limit).await
    }

    pub async fn aris_get_config(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        aris_get_config(&self.pool, key).await
    }

    pub async fn aris_set_config(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        aris_set_config(&self.pool, key, value).await
    }

    pub async fn list_api_keys(&self) -> Result<Vec<ApiKeyRow>, sqlx::Error> {
        list_api_keys(&self.pool).await
    }

    pub async fn get_api_key_by_id(&self, key_id: i64) -> Result<Option<ApiKeyRow>, sqlx::Error> {
        get_api_key_by_id(&self.pool, key_id).await
    }

    pub async fn add_api_key(
        &self,
        provider_id: i64,
        api_key: &str,
        label: &str,
        daily_cap: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        add_api_key(&self.pool, provider_id, api_key, label, daily_cap).await
    }

    pub async fn update_api_key_daily_cap(
        &self,
        key_id: i64,
        daily_cap: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        update_api_key_daily_cap(&self.pool, key_id, daily_cap).await
    }

    pub async fn reset_all_rate_limits(&self) -> Result<(), sqlx::Error> {
        reset_all_rate_limits(&self.pool).await
    }

    pub async fn toggle_api_key(&self, key_id: i64, active: bool) -> Result<(), sqlx::Error> {
        toggle_api_key(&self.pool, key_id, active).await
    }

    pub async fn delete_api_key(&self, key_id: i64) -> Result<(), sqlx::Error> {
        delete_api_key(&self.pool, key_id).await
    }

    pub async fn list_api_providers(&self) -> Result<Vec<(i64, String, String)>, sqlx::Error> {
        list_api_providers(&self.pool).await
    }

    pub async fn list_api_providers_full(&self) -> Result<Vec<ApiProviderRow>, sqlx::Error> {
        list_api_providers_full(&self.pool).await
    }

    pub async fn add_api_provider(
        &self,
        name: &str,
        provider_type: &str,
        base_url: &str,
        default_model: &str,
    ) -> Result<i64, sqlx::Error> {
        add_api_provider(&self.pool, name, provider_type, base_url, default_model).await
    }

    pub async fn update_api_provider(
        &self,
        provider_id: i64,
        name: &str,
        provider_type: &str,
        base_url: &str,
        default_model: &str,
    ) -> Result<(), sqlx::Error> {
        update_api_provider(&self.pool, provider_id, name, provider_type, base_url, default_model).await
    }

    pub async fn toggle_api_provider(&self, provider_id: i64, active: bool) -> Result<(), sqlx::Error> {
        toggle_api_provider(&self.pool, provider_id, active).await
    }

    pub async fn move_api_provider_priority(&self, provider_id: i64, delta: i64) -> Result<(), sqlx::Error> {
        move_api_provider_priority(&self.pool, provider_id, delta).await
    }

    pub async fn delete_api_provider(&self, provider_id: i64) -> Result<(), sqlx::Error> {
        delete_api_provider(&self.pool, provider_id).await
    }

    pub async fn list_rotation_logs(&self, limit: i64) -> Result<Vec<ApiRotationLogRow>, sqlx::Error> {
        list_rotation_logs(&self.pool, limit).await
    }

    pub async fn clear_rotation_logs(&self) -> Result<(), sqlx::Error> {
        clear_rotation_logs(&self.pool).await
    }
    pub async fn reset_api_cooldowns(&self) -> anyhow::Result<()> {
        sqlx::query("UPDATE api_keys SET rate_limited_until = NULL, last_error = NULL")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}



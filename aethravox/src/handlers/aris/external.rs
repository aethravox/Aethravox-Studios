//! Rotación ilimitada y failover de APIs externas:
//!   - Sin límite de proveedores ni de keys por proveedor.
//!   - Estrategias configurables en aris_config:
//!       * round_robin: la key menos usada recientemente va primero.
//!       * least_used: la de menor consumo diario/total va primero.
//!       * provider_order: respeta la prioridad de proveedores (priority ASC).
//!   - Tope preventivo diario (daily_cap) por key: se salta antes de llegar al 429 real.
//!   - Adaptadores incluidos: Gemini nativo, Anthropic Messages (/v1/messages) y
//!     cualquier endpoint OpenAI-compatible (Groq, OpenRouter, OpenAI, Together, Mistral, DeepSeek, Custom).
//!   - Bitácora persistente en api_rotation_logs y Health Check directo por key.

use sqlx::SqlitePool;
use std::time::Instant;

use crate::db::aris as db;
use crate::handlers::aris::ollama::OllamaMessage;
use crate::handlers::aris::ApiKeyRow;

#[derive(Debug)]
pub enum FailoverError {
    NoKeys,
    AllFailed(String),
    Http(String),
}

impl std::fmt::Display for FailoverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailoverError::NoKeys => write!(f, "no hay API keys disponibles"),
            FailoverError::AllFailed(e) => write!(f, "todas las keys fallaron: {}", e),
            FailoverError::Http(e) => write!(f, "http: {}", e),
        }
    }
}
impl std::error::Error for FailoverError {}

/// Orquestador principal con rotación global + failover.
/// Devuelve (respuesta, nombre_modelo, nombre_proveedor).
pub async fn chat_with_failover(
    pool: &SqlitePool,
    messages: &[OllamaMessage],
    system_prompt: &str,
    channel_label: &str,
) -> Result<(String, String, String), FailoverError> {
    let rotation_enabled = db::aris_get_config(pool, "rotation_enabled")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim() != "0" && v.trim() != "false")
        .unwrap_or(true);

    let strategy = db::aris_get_config(pool, "rotation_strategy")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "round_robin".to_string());

    let cooldown_minutes: i64 = db::aris_get_config(pool, "cooldown_minutes")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(60)
        .clamp(1, 1440);

    let auto_failover = db::aris_get_config(pool, "auto_failover")
        .await
        .ok()
        .flatten()
        .map(|v| v.trim() != "0" && v.trim() != "false")
        .unwrap_or(true);

    let keys = db::get_all_available_api_keys(pool, &strategy)
        .await
        .map_err(|e| FailoverError::Http(format!("db: {}", e)))?;

    if keys.is_empty() {
        let _ = db::add_rotation_log(
            pool,
            "aris_chat",
            channel_label,
            "Pool Externo",
            "—",
            "—",
            "exhausted",
            0,
            "No hay API keys activas o todas están en cooldown/tope diario",
        )
        .await;
        return Err(FailoverError::NoKeys);
    }

    let candidates: &[ApiKeyRow] = if rotation_enabled || auto_failover {
        &keys
    } else {
        &keys[..1]
    };

    let mut last_error = String::new();

    for key in candidates {
        let started = Instant::now();
        let resolved_model = resolve_model(key).to_string();

        match call_one(key, messages, system_prompt, 120).await {
            Ok((text, model)) => {
                let latency_ms = started.elapsed().as_millis() as i64;
                let _ = db::record_key_success(pool, key.id).await;
                let _ = db::add_rotation_log(
                    pool,
                    "aris_chat",
                    channel_label,
                    &key.provider_name,
                    &key.label,
                    &model,
                    "ok",
                    latency_ms,
                    "Respuesta generada correctamente",
                )
                .await;
                return Ok((text, model, key.provider_name.clone()));
            }
            Err(CallError::RateLimited(msg)) => {
                let latency_ms = started.elapsed().as_millis() as i64;
                let _ = db::mark_key_rate_limited(pool, key.id, cooldown_minutes, &msg).await;
                let _ = db::add_rotation_log(
                    pool,
                    "aris_chat",
                    channel_label,
                    &key.provider_name,
                    &key.label,
                    &resolved_model,
                    "429",
                    latency_ms,
                    &msg,
                )
                .await;
                last_error = msg;
                if !auto_failover {
                    break;
                }
            }
            Err(CallError::Auth(msg)) => {
                let latency_ms = started.elapsed().as_millis() as i64;
                let _ = db::mark_key_rate_limited(pool, key.id, 24 * 60, &msg).await;
                let _ = db::add_rotation_log(
                    pool,
                    "aris_chat",
                    channel_label,
                    &key.provider_name,
                    &key.label,
                    &resolved_model,
                    "401",
                    latency_ms,
                    &msg,
                )
                .await;
                last_error = msg;
                if !auto_failover {
                    break;
                }
            }
            Err(CallError::Other(msg)) => {
                let latency_ms = started.elapsed().as_millis() as i64;
                let _ = db::set_key_last_error(pool, key.id, Some(&msg)).await;
                let _ = db::add_rotation_log(
                    pool,
                    "aris_chat",
                    channel_label,
                    &key.provider_name,
                    &key.label,
                    &resolved_model,
                    "error",
                    latency_ms,
                    &msg,
                )
                .await;
                last_error = msg;
                if !auto_failover {
                    break;
                }
            }
        }
    }

    Err(FailoverError::AllFailed(if last_error.is_empty() {
        "ollama caído y pool externo agotado".to_string()
    } else {
        last_error
    }))
}

/// Prueba en vivo una sola API key (Health Check desde Admin).
/// No incrementa contadores de uso productivo; actualiza estado/error y registra bitácora.
pub async fn test_api_key(pool: &SqlitePool, key_id: i64) -> Result<(bool, String, i64), String> {
    let key = db::get_api_key_by_id(pool, key_id)
        .await
        .map_err(|e| format!("db: {}", e))?
        .ok_or_else(|| "key_no_encontrada".to_string())?;

    let resolved_model = resolve_model(&key).to_string();

    if let Some(cap) = key.daily_cap {
        if cap > 0 && key.daily_uses >= cap {
            let msg = format!("Tope diario preventivo alcanzado ({}/{})", key.daily_uses, cap);
            let _ = db::add_rotation_log(
                pool,
                "health_check",
                "Health Check",
                &key.provider_name,
                &key.label,
                &resolved_model,
                "daily_cap",
                0,
                &msg,
            )
            .await;
            return Ok((false, msg, 0));
        }
    }

    let ping = [OllamaMessage {
        role: "user".to_string(),
        content: "Responde solo OK".to_string(),
        images: None,
    }];
    let started = Instant::now();

    match call_one(&key, &ping, "Responde únicamente OK.", 20).await {
        Ok((_, model)) => {
            let latency_ms = started.elapsed().as_millis() as i64;
            let _ = sqlx::query(
                "UPDATE api_keys SET rate_limited_until = NULL, last_error = NULL WHERE id = ?",
            )
            .bind(key.id)
            .execute(pool)
            .await;

            let detail = format!("200 OK · modelo {} respondió en {}ms", model, latency_ms);
            let _ = db::add_rotation_log(
                pool,
                "health_check",
                "Health Check",
                &key.provider_name,
                &key.label,
                &model,
                "ok",
                latency_ms,
                &detail,
            )
            .await;
            Ok((true, detail, latency_ms))
        }
        Err(CallError::RateLimited(msg)) => {
            let latency_ms = started.elapsed().as_millis() as i64;
            let cooldown_minutes: i64 = db::aris_get_config(pool, "cooldown_minutes")
                .await
                .ok()
                .flatten()
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60)
                .clamp(1, 1440);
            let _ = db::mark_key_rate_limited(pool, key.id, cooldown_minutes, &msg).await;
            let _ = db::add_rotation_log(
                pool,
                "health_check",
                "Health Check",
                &key.provider_name,
                &key.label,
                &resolved_model,
                "429",
                latency_ms,
                &msg,
            )
            .await;
            Ok((false, msg, latency_ms))
        }
        Err(CallError::Auth(msg)) => {
            let latency_ms = started.elapsed().as_millis() as i64;
            let _ = db::mark_key_rate_limited(pool, key.id, 24 * 60, &msg).await;
            let _ = db::add_rotation_log(
                pool,
                "health_check",
                "Health Check",
                &key.provider_name,
                &key.label,
                &resolved_model,
                "401",
                latency_ms,
                &msg,
            )
            .await;
            Ok((false, msg, latency_ms))
        }
        Err(CallError::Other(msg)) => {
            let latency_ms = started.elapsed().as_millis() as i64;
            let _ = db::set_key_last_error(pool, key.id, Some(&msg)).await;
            let _ = db::add_rotation_log(
                pool,
                "health_check",
                "Health Check",
                &key.provider_name,
                &key.label,
                &resolved_model,
                "error",
                latency_ms,
                &msg,
            )
            .await;
            Ok((false, msg, latency_ms))
        }
    }
}

#[derive(Debug)]
enum CallError {
    RateLimited(String),
    Auth(String),
    Other(String),
}

fn resolve_model(key: &ApiKeyRow) -> &str {
    let m = key.default_model.trim();
    if !m.is_empty() && m != "default" {
        return m;
    }
    match key.provider_type.as_str() {
        "gemini" => "gemini-2.5-flash",
        "groq" => "llama-3.1-8b-instant",
        "anthropic" => "claude-3-5-sonnet-latest",
        _ => "google/gemma-2-9b-it:free",
    }
}

/// Una sola llamada con una sola key. Devuelve (texto, modelo_usado).
async fn call_one(
    key: &ApiKeyRow,
    messages: &[OllamaMessage],
    system_prompt: &str,
    timeout_secs: u64,
) -> Result<(String, String), CallError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| CallError::Other(format!("client: {}", e)))?;

    match key.provider_type.as_str() {
        "gemini" => call_gemini(&client, key, messages, system_prompt).await,
        "anthropic" => call_anthropic(&client, key, messages, system_prompt).await,
        _ => call_openai_compat(&client, key, messages, system_prompt).await,
    }
}

/// Gemini nativo (generateContent).
async fn call_gemini(
    client: &reqwest::Client,
    key: &ApiKeyRow,
    messages: &[OllamaMessage],
    system_prompt: &str,
) -> Result<(String, String), CallError> {
    let model = resolve_model(key);
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        key.base_url.trim_end_matches('/'),
        model,
        key.api_key
    );

    let contents: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let role = if m.role == "assistant" { "model" } else { "user" };
            serde_json::json!({
                "role": role,
                "parts": [{ "text": m.content }]
            })
        })
        .collect();

    let body = serde_json::json!({
        "system_instruction": {
            "parts": [{ "text": system_prompt }]
        },
        "contents": contents,
        "generationConfig": { "temperature": 0.7 }
    });

    let res = client.post(&url).json(&body).send().await
        .map_err(|e| CallError::Other(format!("{} connect: {}", key.provider_name, e)))?;

    match res.status().as_u16() {
        200 => {}
        429 => return Err(CallError::RateLimited(format!("{} 429: cuota agotada", key.provider_name))),
        401 | 403 => return Err(CallError::Auth(format!("{} 401/403: key inválida", key.provider_name))),
        s => return Err(CallError::Other(format!("{} status {}", key.provider_name, s))),
    }

    let json: serde_json::Value = res.json().await
        .map_err(|e| CallError::Other(format!("gemini parse: {}", e)))?;

    let text = json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        return Err(CallError::Other(format!("{}: respuesta vacía", key.provider_name)));
    }
    Ok((text, model.to_string()))
}

/// Anthropic nativo (/messages con x-api-key y anthropic-version).
async fn call_anthropic(
    client: &reqwest::Client,
    key: &ApiKeyRow,
    messages: &[OllamaMessage],
    system_prompt: &str,
) -> Result<(String, String), CallError> {
    let model = resolve_model(key);
    let url = format!("{}/messages", key.base_url.trim_end_matches('/'));

    let api_messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| {
            let role = if m.role == "assistant" { "assistant" } else { "user" };
            serde_json::json!({
                "role": role,
                "content": m.content,
            })
        })
        .collect();

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 2048,
        "system": system_prompt,
        "messages": api_messages,
    });

    let res = client
        .post(&url)
        .header("x-api-key", &key.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| CallError::Other(format!("{} connect: {}", key.provider_name, e)))?;

    match res.status().as_u16() {
        200 => {}
        429 => return Err(CallError::RateLimited(format!("{} 429: cuota agotada", key.provider_name))),
        401 | 403 => return Err(CallError::Auth(format!("{} 401/403: key inválida", key.provider_name))),
        s => return Err(CallError::Other(format!("{} status {}", key.provider_name, s))),
    }

    let json: serde_json::Value = res.json().await
        .map_err(|e| CallError::Other(format!("anthropic parse: {}", e)))?;

    let text = json["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        return Err(CallError::Other(format!("{}: respuesta vacía", key.provider_name)));
    }
    Ok((text, model.to_string()))
}

/// OpenAI-compatible (Groq, OpenRouter, OpenAI, Together, Mistral, DeepSeek, Custom). /chat/completions.
async fn call_openai_compat(
    client: &reqwest::Client,
    key: &ApiKeyRow,
    messages: &[OllamaMessage],
    system_prompt: &str,
) -> Result<(String, String), CallError> {
    let model = resolve_model(key);
    let url = format!("{}/chat/completions", key.base_url.trim_end_matches('/'));

    let mut api_messages = vec![serde_json::json!({
        "role": "system",
        "content": system_prompt,
    })];
    for m in messages {
        api_messages.push(serde_json::json!({
            "role": m.role,
            "content": m.content,
        }));
    }

    let body = serde_json::json!({
        "model": model,
        "messages": api_messages,
        "temperature": 0.7,
    });

    let res = client.post(&url)
        .bearer_auth(&key.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| CallError::Other(format!("{} connect: {}", key.provider_name, e)))?;

    match res.status().as_u16() {
        200 => {}
        429 => return Err(CallError::RateLimited(format!("{} 429: cuota agotada", key.provider_name))),
        401 | 403 => return Err(CallError::Auth(format!("{} 401/403: key inválida", key.provider_name))),
        s => return Err(CallError::Other(format!("{} status {}", key.provider_name, s))),
    }

    let json: serde_json::Value = res.json().await
        .map_err(|e| CallError::Other(format!("parse: {}", e)))?;

    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        return Err(CallError::Other(format!("{}: respuesta vacía", key.provider_name)));
    }
    Ok((text, model.to_string()))
}

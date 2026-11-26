//! Núcleo unificado de Aris conectado al backend de Aethravox:
//!   - Exposición de carriles activos según el rol del usuario (`aris.lane.*`).
//!   - Analíticas reales de visitas (`web_visits`) por rango de días.
//!   - Copiloto generador HTML/CSS/JS para Playground (`aris.lane.playground`).
//!   - Constructor de prompt del núcleo con snapshot de datos permitidos.
//!   - Ejecutor seguro de acciones (`send_friend_dm`) validando amistad y rol.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::{
    auth_mw::AuthUser,
    db::analytics::UserArisLanes,
    handlers::aris::{
        external::chat_with_failover,
        ollama::{chat_stream_ollama, OllamaMessage},
    },
    models::User,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PlaygroundGenPayload {
    pub prompt: String,
    #[serde(default)]
    pub current_html: String,
}

#[derive(Debug, Serialize)]
pub struct PlaygroundGenResponse {
    pub ok: bool,
    pub html: String,
    pub css: String,
    pub js: String,
    pub note: String,
    pub model: String,
    pub provider: String,
}

/// GET /api/aris/core/status (guest-allowed: devuelve estado sin redirigir a /login)
pub async fn api_core_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Response {
    let lanes = state.db.user_aris_lanes(&user).await;
    let (role_css_key, role_display) = if user.id == 0 {
        ("invitado".to_string(), "Invitado".to_string())
    } else {
        state.db.role_display(&user).await
    };

    let top_webs_7d = if lanes.analytics {
        state.db.top_webs_in_days(7, 6).await.unwrap_or_default()
    } else {
        Vec::new()
    };

    Json(serde_json::json!({
        "ok": true,
        "is_guest": user.id == 0,
        "username": user.username,
        "display": user.display(),
        "role_css_key": role_css_key,
        "role_display": role_display,
        "lanes": lanes,
        "top_webs_7d": top_webs_7d,
    }))
    .into_response()
}

/// GET /api/aris/core/analytics?days=7 (requiere aris.lane.analytics)
pub async fn api_core_analytics(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Response {
    if !state.db.user_has_perm(&user, "aris.lane.analytics").await {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "carril_bloqueado",
                "lane": "aris.lane.analytics",
                "message": "Tu rol no tiene acceso al carril de Analíticas."
            })),
        )
            .into_response();
    }

    let days = q.days.unwrap_or(7).clamp(1, 90);
    let total_visits = state.db.total_visits_in_days(days).await.unwrap_or(0);
    let top_webs = state.db.top_webs_in_days(days, 15).await.unwrap_or_default();

    Json(serde_json::json!({
        "ok": true,
        "days": days,
        "total_visits": total_visits,
        "top_webs": top_webs,
    }))
    .into_response()
}

/// POST /api/aris/playground/generate (requiere aris.lane.playground)
pub async fn api_playground_generate(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(payload): Json<PlaygroundGenPayload>,
) -> Response {
    if !state.db.user_has_perm(&user, "aris.lane.playground").await {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "carril_bloqueado",
                "lane": "aris.lane.playground",
                "message": "Tu rol no tiene permiso para usar Aris en Playground (aris.lane.playground)."
            })),
        )
            .into_response();
    }

    let prompt = payload.prompt.trim();
    if prompt.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Describe qué quieres construir en Playground." })),
        )
            .into_response();
    }

    let system_prompt = "Eres Aris, copiloto de código web en Aethravox Playground. \
Responde ÚNICAMENTE con un objeto JSON válido con las claves exactas: \
\"html\" (solo contenido dentro de <body>), \"css\", \"js\", \"note\" (breve resumen en español). \
No uses fetch externos ni imports; código puro HTML/CSS/JS limpio y funcional.";

    let user_prompt = if payload.current_html.trim().is_empty() {
        format!("Genera una experiencia web interactiva para Playground sobre: {}", prompt)
    } else {
        format!(
            "Solicitud: {}\n\nHTML actual en el editor (mejora o reemplaza según corresponda):\n{}",
            prompt,
            payload.current_html.chars().take(1800).collect::<String>()
        )
    };

    match generate_single_completion(&state, system_prompt, &user_prompt, "playground").await {
        Ok((raw, model, provider)) => {
            let (html, css, js, note) = parse_playground_output(&raw, prompt);
            Json(PlaygroundGenResponse {
                ok: true,
                html,
                css,
                js,
                note,
                model,
                provider,
            })
            .into_response()
        }
        Err(_) => {
            let (html, css, js, note) = fallback_playground_template(prompt);
            Json(PlaygroundGenResponse {
                ok: true,
                html,
                css,
                js,
                note,
                model: "Aris-Local-Scaffold".to_string(),
                provider: "aethravox-core".to_string(),
            })
            .into_response()
        }
    }
}

async fn generate_single_completion(
    state: &AppState,
    system_prompt: &str,
    user_prompt: &str,
    channel_label: &str,
) -> Result<(String, String, String), String> {
    let ollama_url = state
        .db
        .aris_get_config("ollama_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());

    let msgs = vec![OllamaMessage {
        role: "user".to_string(),
        content: user_prompt.to_string(),
        images: None,
    }];

    if let Ok(mut stream) =
        chat_stream_ollama(&ollama_url, "Aris", system_prompt, msgs.clone()).await
    {
        let mut out = String::new();
        while let Some(chunk) = stream.next().await {
            if let Ok(txt) = chunk {
                out.push_str(&txt);
            }
        }
        if !out.trim().is_empty() {
            return Ok((out, "Aris".to_string(), "ollama".to_string()));
        }
    }

    chat_with_failover(&state.db.pool, &msgs, system_prompt, channel_label)
        .await
        .map_err(|e| e.to_string())
}

fn parse_playground_output(raw: &str, prompt: &str) -> (String, String, String, String) {
    // 1) Intentar extraer JSON directo o dentro de ```json ... ```
    let trimmed = raw.trim();
    let json_candidate = if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if end > start {
            Some(&trimmed[start..=end])
        } else {
            None
        }
    } else {
        None
    };

    if let Some(candidate) = json_candidate {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate) {
            let html = val["html"].as_str().unwrap_or("").trim().to_string();
            let css = val["css"].as_str().unwrap_or("").trim().to_string();
            let js = val["js"].as_str().unwrap_or("").trim().to_string();
            let note = val["note"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Código generado por Aris e inyectado en el editor.")
                .to_string();
            if !html.is_empty() || !css.is_empty() || !js.is_empty() {
                return (html, css, js, note);
            }
        }
    }

    // 2) Fallback: extraer bloques ```html / ```css / ```js
    let html = extract_code_fence(raw, "html").unwrap_or_default();
    let css = extract_code_fence(raw, "css").unwrap_or_default();
    let js = extract_code_fence(raw, "js")
        .or_else(|| extract_code_fence(raw, "javascript"))
        .unwrap_or_default();

    if !html.is_empty() || !css.is_empty() || !js.is_empty() {
        return (
            html,
            css,
            js,
            "Bloques HTML/CSS/JS extraídos por Aris.".to_string(),
        );
    }

    fallback_playground_template(prompt)
}

fn extract_code_fence(text: &str, lang: &str) -> Option<String> {
    let marker = format!("```{}", lang);
    let lower = text.to_lowercase();
    let start = lower.find(&marker)?;
    let after = &text[start + marker.len()..];
    let body_start = after.find('\n').map(|i| i + 1).unwrap_or(0);
    let rest = &after[body_start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
}

fn fallback_playground_template(prompt: &str) -> (String, String, String, String) {
    let safe_title: String = prompt
        .chars()
        .filter(|&c| c != '<' && c != '>')
        .take(64)
        .collect();
    let html = format!(
        "<main class=\"card\">\n  <span class=\"tag\">ARiS · PLAYGROUND</span>\n  <h1>{}</h1>\n  <p>Prototipo interactivo listo para personalizar en HTML, CSS y JS.</p>\n  <div class=\"counter-box\">\n    <button id=\"btn-action\">Ejecutar acción</button>\n    <span id=\"counter\">0 interacciones</span>\n  </div>\n</main>",
        if safe_title.is_empty() { "Proyecto Aethravox".to_string() } else { safe_title }
    );
    let css = "body {\n  margin: 0;\n  min-height: 100vh;\n  display: grid;\n  place-items: center;\n  background: #06030C;\n  color: #F4F4F5;\n  font-family: system-ui, sans-serif;\n}\n.card {\n  width: min(440px, 90vw);\n  padding: 28px;\n  border-radius: 16px;\n  background: #0D0818;\n  border: 1px solid #2A2140;\n  box-shadow: 0 20px 50px rgba(0,0,0,.45);\n}\n.tag {\n  font-size: 11px;\n  letter-spacing: .12em;\n  color: #e0a308;\n  font-weight: 700;\n}\nh1 { margin: 10px 0 8px; font-size: 1.4rem; }\np { color: #b0b0c0; line-height: 1.5; }\n.counter-box { display: flex; align-items: center; gap: 12px; margin-top: 18px; }\nbutton {\n  padding: 10px 16px;\n  border: none;\n  border-radius: 10px;\n  background: #e0a308;\n  color: #06030C;\n  font-weight: 700;\n  cursor: pointer;\n}".to_string();
    let js = "let count = 0;\ndocument.getElementById('btn-action').addEventListener('click', () => {\n  count += 1;\n  document.getElementById('counter').textContent = `${count} ${count === 1 ? 'interacción' : 'interacciones'}`;\n});".to_string();
    let note = "Plantilla base generada (configura Ollama o una API key en /admin/apis para generación libre con IA).".to_string();
    (html, css, js, note)
}

/// Detecta cuántos días solicita el usuario en su consulta de analítica.
pub fn detect_requested_days(text: &str) -> i64 {
    let lower = text.to_lowercase();
    if lower.contains("hoy") {
        return 1;
    }
    if lower.contains("ayer") {
        return 2;
    }
    if lower.contains("semana") {
        return 7;
    }
    if lower.contains("mes") {
        return 30;
    }
    // Buscar "<numero> dia" / "<numero> día"
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    for i in 0..tokens.len() {
        if let Ok(n) = tokens[i].trim_matches(|c: char| !c.is_ascii_digit()).parse::<i64>() {
            if let Some(next) = tokens.get(i + 1) {
                if next.starts_with("dia") || next.starts_with("día") {
                    return n.clamp(1, 90);
                }
            }
        }
    }
    7
}

/// Construye el system prompt enriquecido con los datos reales permitidos para el rol.
pub async fn build_core_system_prompt(
    state: &AppState,
    user: &User,
    lanes: &UserArisLanes,
    base_prompt: &str,
    user_msg: &str,
) -> String {
    let days = detect_requested_days(user_msg);
    let snapshot = state.db.build_aris_core_snapshot(user, lanes, days).await;

    format!(
        "{}\n\n\
=== NÚCLEO AETHRAVOX (CONTEXTO REAL Y PERMISOS POR ROL) ===\n\
{}\n\n\
REGLAS ESTRICTAS DE SEGURIDAD Y CARRILES:\n\
1. Responde siempre como Aris (nunca digas 'Aris IA').\n\
2. Para conteos de usuarios, visitas, secciones más usadas o analíticas, usa EXCLUSIVAMENTE los datos reales del bloque ANALÍTICA EN VIVO anterior. Nunca inventes cifras.\n\
3. Si el usuario pide analíticas/visitas y el carril `aris.lane.analytics` está bloqueado para su rol, indícale amablemente que su rol no tiene acceso al carril de Analíticas.\n\
4. Si el usuario pide enviar un mensaje directo (DM) a un amigo y `aris.lane.social` está activo:\n\
   - Verifica que el destinatario esté en la lista de `amigos aceptados` de SOCIAL DEL USUARIO.\n\
   - Si SÍ es su amigo aceptado, al FINAL de tu respuesta incluye una línea exacta con formato:\n\
     [[ARIS_ACTION:send_friend_dm|to=username_exacto|text=mensaje exacto a enviar]]\n\
   - Si NO es amigo aceptado o falta el contenido, NO incluyas la etiqueta y explica por qué.\n\
5. Si `aris.lane.social` está bloqueado y pide enviar mensajes por chat, explica que su rol no tiene habilitado el carril Social.",
        base_prompt.trim(),
        snapshot
    )
}

/// Ejecuta acciones seguras emitidas por Aris (`[[ARIS_ACTION:send_friend_dm|to=...|text=...]]`)
/// y limpia cualquier etiqueta interna antes de guardar/mostrar al usuario.
pub async fn process_aris_core_actions(
    state: &AppState,
    user: &User,
    lanes: &UserArisLanes,
    raw_reply: &str,
) -> String {
    let mut clean_lines: Vec<&str> = Vec::new();
    let mut actions: Vec<(String, String)> = Vec::new();

    for line in raw_reply.lines() {
        let trimmed = line.trim();
        if let Some(inner) = trimmed
            .strip_prefix("[[ARIS_ACTION:")
            .and_then(|s| s.strip_suffix("]]"))
        {
            if let Some(rest) = inner.strip_prefix("send_friend_dm|") {
                let mut to = String::new();
                let mut text = String::new();
                for part in rest.splitn(2, '|') {
                    if let Some(v) = part.strip_prefix("to=") {
                        to = v.trim().to_string();
                    } else if let Some(v) = part.strip_prefix("text=") {
                        text = v.trim().to_string();
                    }
                }
                if !to.is_empty() && !text.is_empty() {
                    actions.push((to, text));
                }
            }
            continue;
        }
        clean_lines.push(line);
    }

    let mut clean_text = clean_lines.join("\n").trim().to_string();

    for (to, body) in actions {
        if !lanes.social {
            clean_text.push_str("\n\n🔒 Tu rol no tiene permiso en el carril Social (`aris.lane.social`).");
            continue;
        }
        match state.db.send_dm_via_aris(user, &to, &body).await {
            Ok((conv_id, target, msg_id)) => {
                // Notificar en tiempo real por WebSocket o Web Push al amigo destinatario
                let created_at = chrono::Utc::now().format("%H:%M").to_string();
                let created_iso = chrono::Utc::now().to_rfc3339();
                let sender_name = user.display();
                let sender_avatar = user.avatar();

                let target_online = {
                    let conns = state.ws_connections.lock().await;
                    if let Some(tx) = conns.get(&target.id) {
                        let _ = tx.send(
                            serde_json::json!({
                                "type": "message",
                                "id": msg_id,
                                "conversation_id": conv_id,
                                "sender_id": user.id,
                                "sender_name": sender_name,
                                "sender_avatar": sender_avatar,
                                "body": body,
                                "time": created_at,
                                "created_iso": created_iso,
                                "status": "delivered"
                            })
                            .to_string(),
                        );
                        true
                    } else {
                        false
                    }
                };

                if !target_online {
                    let push_arc = state.push.clone();
                    let db = state.db.clone();
                    let target_id = target.id;
                    let preview: String = body.chars().take(120).collect();
                    let payload = serde_json::json!({
                        "title": sender_name,
                        "body": preview,
                        "tag": format!("msg-{}", conv_id),
                        "type": "message",
                        "conversation_id": conv_id,
                        "url": "/chat",
                        "requireInteraction": false
                    });
                    tokio::spawn(async move {
                        if let Some(pm) = push_arc.as_ref() {
                            pm.send_to_user(&db, target_id, &payload).await;
                        }
                    });
                }

                clean_text.push_str(&format!(
                    "\n\n✅ Mensaje enviado por chat directo a @{}.",
                    target.username
                ));
            }
            Err(err) => {
                clean_text.push_str(&format!("\n\n⚠️ No se pudo enviar el DM: {}", err));
            }
        }
    }

    clean_text
}

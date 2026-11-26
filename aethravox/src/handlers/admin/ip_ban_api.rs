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



pub async fn api_user_ips(
    State(state): State<AppState>,
    AuthUser(actor): AuthUser,
    Path(user_id): Path<i64>,
) -> Response {
    if !state.db.user_has_perm(&actor, "users.ip.view").await {
        return axum::Json(serde_json::json!({"error": "forbidden"})).into_response();
    }
    let ips = state.db.list_user_ips(user_id).await.unwrap_or_default();
    let result: Vec<_> = ips.iter().map(|h| {
        serde_json::json!({
            "ip": h.ip,
            "first_seen": h.first_seen.format("%d/%m/%Y %H:%M").to_string(),
            "last_seen": h.last_seen.format("%d/%m/%Y %H:%M").to_string(),
            "times_used": h.times_used,
            "is_first_time": h.times_used == 1,
        })
    }).collect();
    axum::Json(result).into_response()
}

// ===== Pantalla de ban (la ve el usuario baneado) =====

/// Helper para escapar HTML en la pantalla de ban (no usar askama aquí porque
/// es standalone y queremos control total del layout).
fn html_escape(s: &str) -> String {
    s.chars().map(|c| match c {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '&' => "&amp;".to_string(),
        '"' => "&quot;".to_string(),
        '\'' => "&#39;".to_string(),
        _ => c.to_string(),
    }).collect()
}

// ===== Pantalla de ban unificada (IP ban, user ban, user suspension) =====

/// Tipo de ban. Determina el icono, título, mensaje y labels de la pantalla.
#[derive(Debug, Clone, Copy)]
pub enum BanKind {
    /// Ban de IP (require_no_ip_ban middleware)
    IpBan,
    /// Ban permanente de cuenta (user_bans, status='banned')
    UserBan,
    /// Suspensión temporal de cuenta (user_suspensions, status='suspended')
    UserSuspension,
}

impl BanKind {
    fn icon(self) -> &'static str {
        match self {
            BanKind::IpBan           => "🔒",
            BanKind::UserBan          => "⛔",
            BanKind::UserSuspension   => "⏸️",
        }
    }
    fn title(self) -> &'static str {
        match self {
            BanKind::IpBan           => "Acceso restringido",
            BanKind::UserBan         => "Cuenta baneada",
            BanKind::UserSuspension  => "Cuenta suspendida",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            BanKind::IpBan =>
                "Tu dirección IP ha sido bloqueada por un administrador.",
            BanKind::UserBan =>
                "Tu cuenta de usuario ha sido baneada permanentemente de Aethravox Studios.",
            BanKind::UserSuspension =>
                "Tu acceso a Aethravox Studios ha sido limitado temporalmente.",
        }
    }
    fn identifier_label(self) -> &'static str {
        match self {
            BanKind::IpBan           => "IP afectada",
            BanKind::UserBan |
            BanKind::UserSuspension  => "Cuenta afectada",
        }
    }
    fn reason_label(self) -> &'static str {
        match self {
            BanKind::IpBan           => "Motivo del ban de IP",
            BanKind::UserBan         => "Motivo del ban",
            BanKind::UserSuspension  => "Motivo de la suspensión",
        }
    }
    /// Mensaje del footer según el tipo (explica qué hacer).
    fn footer_msg(self) -> &'static str {
        match self {
            BanKind::IpBan =>
                "Si crees que esto es un error, contacta al equipo de Aethravox Studios por otros medios. Los intentos de acceso se registran en el log de auditoría.",
            BanKind::UserBan =>
                "Si crees que este ban es injustificado, puedes contactar al equipo por otros medios para apelar. Los bans permanentes no expiran automáticamente.",
            BanKind::UserSuspension =>
                "La suspensión se levantará automáticamente al expirar el tiempo. Mientras tanto, no puedes acceder a tu cuenta.",
        }
    }
}

/// Construye la pantalla de ban con countdown real (años/meses/semanas/días/horas/min/seg).
/// STANDALONE: no hereda de base.html. CSS y JS inline para no depender de /static/
/// (que también está bloqueado por el middleware require_no_ip_ban).
pub fn ban_screen_response(
    kind: BanKind,
    ban_id: i64,
    identifier: String,    // IP para IpBan, username para UserBan/UserSuspension
    reason: String,
    applied_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Response {
    let is_permanent = expires_at.is_none();
    let expires_iso = expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
    let expires_display = expires_at
        .map(|dt| dt.format("%d/%m/%Y a las %H:%M UTC").to_string())
        .unwrap_or_default();
    let applied_display = applied_at.format("%d/%m/%Y a las %H:%M UTC").to_string();

    let html = format!(r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} — Aethravox Studios</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: 'Inter', system-ui, -apple-system, sans-serif;
            background: radial-gradient(circle at center, #1a1a1a 0%, #050505 80%);
            color: #fff;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .ban-card {{
            background: linear-gradient(135deg, #111113 0%, #1a1a1c 100%);
            border: 2px solid {border_color};
            border-radius: 18px;
            box-shadow: 0 0 0 1px {border_glow},
                        0 20px 60px rgba(0,0,0,0.7),
                        0 0 80px {border_glow};
            max-width: 580px;
            width: 100%;
            padding: 36px 30px;
            text-align: center;
        }}
        .ban-icon {{
            font-size: 4rem;
            margin-bottom: 14px;
            animation: pulse 2s ease-in-out infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ transform: scale(1); opacity: 1; }}
            50% {{ transform: scale(1.08); opacity: 0.85; }}
        }}
        .ban-title {{
            color: {accent_color};
            font-size: 1.7rem;
            letter-spacing: 1px;
            text-transform: uppercase;
            font-weight: 800;
            margin-bottom: 8px;
            text-shadow: 0 0 18px {border_glow};
        }}
        .ban-subtitle {{
            color: #b0b0b0;
            font-size: 0.95rem;
            margin-bottom: 26px;
            line-height: 1.5;
        }}
        .ban-section {{
            text-align: left;
            background: rgba(0,0,0,0.3);
            border: 1px solid #222;
            border-radius: 10px;
            padding: 14px 16px;
            margin: 12px 0;
        }}
        .ban-label {{
            color: {accent_color};
            font-size: 0.75rem;
            letter-spacing: 1px;
            text-transform: uppercase;
            font-weight: 700;
            margin-bottom: 6px;
        }}
        .ban-value {{
            color: #fff;
            font-size: 1rem;
            line-height: 1.4;
            word-wrap: break-word;
        }}
        .countdown {{
            color: #00d4ff;
            font-size: 1.4rem;
            font-weight: 700;
            font-family: 'Menlo', 'Monaco', monospace;
            letter-spacing: 1px;
            text-shadow: 0 0 12px rgba(0, 212, 255, 0.4);
        }}
        .countdown-units {{
            display: flex;
            flex-wrap: wrap;
            gap: 8px;
            justify-content: center;
            margin-top: 8px;
        }}
        .countdown-unit {{
            background: rgba(0,212,255,0.08);
            border: 1px solid rgba(0,212,255,0.3);
            border-radius: 8px;
            padding: 6px 10px;
            min-width: 60px;
            text-align: center;
        }}
        .countdown-num {{
            color: #00d4ff;
            font-size: 1.3rem;
            font-weight: 700;
            display: block;
        }}
        .countdown-unit-label {{
            color: #888;
            font-size: 0.7rem;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        .ban-info {{
            color: #888;
            font-size: 0.85rem;
            margin-top: 20px;
            line-height: 1.6;
        }}
        .ban-info strong {{ color: #b0b0b0; }}
        .ban-footer {{
            color: #555;
            font-size: 0.75rem;
            margin-top: 24px;
            padding-top: 18px;
            border-top: 1px solid #222;
            line-height: 1.6;
        }}
        .permanent-badge {{
            display: inline-block;
            background: rgba(255, 62, 62, 0.12);
            color: #ff3e3e;
            border: 1px solid rgba(255, 62, 62, 0.4);
            border-radius: 12px;
            padding: 4px 14px;
            font-size: 0.85rem;
            font-weight: 700;
            letter-spacing: 1px;
            margin-top: 8px;
        }}
        .permanent-extra {{
            margin-top: 8px;
            color: #888;
            font-size: 0.85rem;
        }}
        .reactivate-hint {{
            margin-top: 6px;
            color: #888;
            font-size: 0.8rem;
            font-style: italic;
        }}
    </style>
</head>
<body>
    <div class="ban-card">
        <div class="ban-icon">{icon}</div>
        <h1 class="ban-title">{title}</h1>
        <p class="ban-subtitle">{subtitle}</p>

        <div class="ban-section">
            <div class="ban-label">{reason_label}</div>
            <div class="ban-value">{reason}</div>
        </div>

        {expires_section}

        <div class="ban-info">
            <p><strong>{identifier_label}:</strong> {identifier}</p>
            <p><strong>Ban ID:</strong> #{ban_id}</p>
            <p><strong>Aplicado el:</strong> {applied_display}</p>
            {expires_info}
        </div>

        <div class="ban-footer">
            {footer_msg}
        </div>
    </div>

    {js_section}
</body>
</html>"#,
        title = html_escape(kind.title()),
        icon = kind.icon(),
        subtitle = html_escape(kind.subtitle()),
        reason_label = html_escape(kind.reason_label()),
        reason = html_escape(&reason),
        identifier_label = kind.identifier_label(),
        identifier = html_escape(&identifier),
        ban_id = ban_id,
        applied_display = applied_display,
        accent_color = match kind {
            BanKind::IpBan          => "#e0a308",  // dorado
            BanKind::UserBan        => "#ff3e3e",  // rojo
            BanKind::UserSuspension => "#ff8c42",  // naranja
        },
        border_color = match kind {
            BanKind::IpBan          => "#e0a308",
            BanKind::UserBan        => "#ff3e3e",
            BanKind::UserSuspension => "#ff8c42",
        },
        border_glow = match kind {
            BanKind::IpBan          => "rgba(224, 163, 8, 0.25)",
            BanKind::UserBan        => "rgba(255, 62, 62, 0.25)",
            BanKind::UserSuspension => "rgba(255, 140, 66, 0.25)",
        },
        expires_section = if is_permanent {
            r#"<div class="ban-section">
                <div class="ban-label">Duración</div>
                <div class="ban-value">
                    <span class="permanent-badge">PERMANENTE</span>
                    <p class="permanent-extra">Este ban no expira automáticamente.</p>
                </div>
            </div>"#.to_string()
        } else {
            r#"<div class="ban-section">
                <div class="ban-label">Tiempo restante</div>
                <div class="countdown" id="countdown-text">Calculando…</div>
                <div class="countdown-units" id="countdown-units"></div>
                <p class="reactivate-hint">La página se recargará sola cuando expire.</p>
            </div>"#.to_string()
        },
        expires_info = if is_permanent {
            String::new()
        } else {
            format!("<p><strong>Se restablece automáticamente el:</strong> {}</p>", expires_display)
        },
        js_section = if is_permanent {
            String::new()
        } else {
            format!(r#"<script>
            (function() {{
                const expiresAt = new Date("{expires_iso}");
                function update() {{
                    const now = new Date();
                    let diff = Math.max(0, expiresAt - now);
                    if (diff <= 0) {{
                        document.getElementById('countdown-text').textContent = '⏰ Tiempo expirado';
                        document.getElementById('countdown-units').innerHTML = '<p style="color:#888;">Recargando…</p>';
                        setTimeout(() => location.reload(), 2000);
                        return;
                    }}
                    const totalSeconds = Math.floor(diff / 1000);
                    const years   = Math.floor(totalSeconds / (365 * 86400));
                    const rem1    = totalSeconds - years * 365 * 86400;
                    const months  = Math.floor(rem1 / (30 * 86400));
                    const rem2    = rem1 - months * 30 * 86400;
                    const weeks   = Math.floor(rem2 / (7 * 86400));
                    const rem3    = rem2 - weeks * 7 * 86400;
                    const days    = Math.floor(rem3 / 86400);
                    const rem4    = rem3 - days * 86400;
                    const hours   = Math.floor(rem4 / 3600);
                    const rem5    = rem4 - hours * 3600;
                    const minutes = Math.floor(rem5 / 60);
                    const seconds = rem5 - minutes * 60;
                    document.getElementById('countdown-text').textContent = '⏳ Tiempo restante';
                    const units = [
                        {{n: years,   l: 'años'}},
                        {{n: months,  l: 'meses'}},
                        {{n: weeks,   l: 'sem'}},
                        {{n: days,    l: 'días'}},
                        {{n: hours,   l: 'hrs'}},
                        {{n: minutes, l: 'min'}},
                        {{n: seconds, l: 'seg'}}
                    ];
                    document.getElementById('countdown-units').innerHTML = units
                        .filter(u => u.n > 0 || u.l === 'seg')
                        .map(u => '<div class="countdown-unit"><span class="countdown-num">' + u.n + '</span><span class="countdown-unit-label">' + u.l + '</span></div>')
                        .join('');
                }}
                update();
                setInterval(update, 1000);
            }})();
            </script>"#,
                expires_iso = expires_iso
            )
        },
        footer_msg = html_escape(kind.footer_msg())
    );

    axum::response::Html(html).into_response()
}

// ===== Embed de URLs externas =====

#[derive(askama::Template)]
#[template(path = "herramientas/embed/embed.html")]
pub struct EmbedTemplate {
    pub title: String,
    pub external_url: String,
}

/// GET /embed/:slug — Muestra una URL externa dentro de Aethravox (iframe).
pub async fn embed_external(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Response {
    // Buscar la web por slug
    let web = sqlx::query_as::<_, crate::models::Web>(
        "SELECT id, slug, name, description, icon, url, is_public, created_at, tag, tag_message, tag_opens_at, is_active, is_custom, html_content
         FROM webs WHERE slug = ? AND is_active = 1"
    )
    .bind(&slug)
    .fetch_optional(&state.db.pool)
    .await
    .ok()
    .flatten();

    match web {
        Some(w) if w.url.starts_with("http://") || w.url.starts_with("https://") => {
            EmbedTemplate {
                title: w.name.clone(),
                external_url: w.url.clone(),
            }.into_response()
        }
        _ => (axum::http::StatusCode::NOT_FOUND, "Sección no encontrada o no es externa").into_response(),
    }
}

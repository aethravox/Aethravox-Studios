// Fase 6 — Panel de asistencia/sugerencias
use askama::Template;
use axum::{
    extract::{Form, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{auth_mw::{AuthUser, get_client_ip}, AppState};

// ===== Models =====

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Ticket {
    pub id: i64,
    pub user_id: i64,
    pub ticket_type: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub answered_at: Option<chrono::DateTime<chrono::Utc>>,
    // Migración 022: ciclo de vida del ticket.
    pub reclamado_por: Option<i64>,
    pub resolucion: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TicketMessage {
    pub id: i64,
    pub ticket_id: i64,
    pub from_user_id: i64,
    pub is_admin: i64,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ===== User-facing =====

#[derive(Deserialize)]
pub struct CreateTicketForm {
    pub ticket_type: String,
    pub subject: String,
    pub body: String,
}

/// Query params de /support (?ok=... / ?err=...) para mensajes flash.
#[derive(Deserialize)]
pub struct SupportQuery {
    pub ok: Option<String>,
    pub err: Option<String>,
}

pub async fn support_page(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<SupportQuery>,
) -> Response {
    let tickets = state.db.list_user_tickets(user.id).await.unwrap_or_default();
    let flash = match (q.ok.as_deref(), q.err.as_deref()) {
        (_, Some("empty")) => "⚠️ El asunto y el mensaje no pueden estar vacíos.".to_string(),
        (_, Some("cat")) => "⚠️ Categoría no permitida para invitado.".to_string(),
        (_, Some("rate")) => "⏳ Demasiados tickets desde tu IP. Espera un rato e inténtalo de nuevo.".to_string(),
        (_, Some("system")) => "⚠️ No se pudo procesar el ticket ahora. Inténtalo más tarde.".to_string(),
        (Some("created_guest"), _) => "✅ Ticket enviado. El equipo lo revisará.".to_string(),
        (Some("created"), _) => "✅ Ticket creado.".to_string(),
        _ => String::new(),
    };
    SupportTemplate {
        username: user.username.clone(),
        is_guest: user.id == 0,
        flash,
        tickets: tickets.iter().map(|t| TicketRow {
            id: t.id,
            ticket_type: t.ticket_type.clone(),
            subject: t.subject.clone(),
            status: t.status.clone(),
            resolucion: t.resolucion.clone().unwrap_or_default(),
            created_at: t.created_at.format("%d/%m/%Y %H:%M").to_string(),
        }).collect(),
    }.into_response()
}

#[derive(Template)]
#[template(path = "asistencia/support/support.html")]
pub struct SupportTemplate {
    pub username: String,
    /// true si el viewer es invitado sin sesión. Restringe las categorías del
    /// form a 'errores de inicio de sesión' + 'sugerencia' y oculta "Mis tickets".
    pub is_guest: bool,
    /// Mensaje flash desde query params (?ok / ?err).
    pub flash: String,
    pub tickets: Vec<TicketRow>,
}

pub struct TicketRow {
    pub id: i64,
    pub ticket_type: String,
    pub subject: String,
    pub status: String,
    pub resolucion: String,
    pub created_at: String,
}

pub async fn ticket_view(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    let ticket = state.db.get_ticket(id).await.ok().flatten();
    let Some(ticket) = ticket else {
        return Redirect::to("/support").into_response();
    };
    // Only ticket owner or admin can view
    if ticket.user_id != user.id && !state.db.user_has_perm(&user, "tickets.view").await {
        return Redirect::to("/support").into_response();
    }
    let messages = state.db.list_ticket_messages(id).await.unwrap_or_default();
    let ticket_owner = state.db.find_user_by_id(ticket.user_id).await.ok().flatten();
    let owner_name = ticket_owner.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "desconocido".into());

    // Nombre del admin que reclamó el ticket (si hay).
    let reclamado_nombre = match ticket.reclamado_por {
        Some(aid) => state.db.find_user_by_id(aid).await.ok().flatten()
            .map(|u| u.display()).unwrap_or_else(|| "Admin".into()),
        None => String::new(),
    };

    let is_admin = state.db.user_has_perm(&user, "tickets.respond").await;

    let mut msg_rows = Vec::new();
    for m in &messages {
        let sender = state.db.find_user_by_id(m.from_user_id).await.ok().flatten();
        let sender_name = sender.as_ref().map(|u| u.display()).unwrap_or_else(|| "Usuario".into());
        msg_rows.push(TicketMsgRow {
            id: m.id,
            body: m.body.clone(),
            sender_name,
            is_admin: m.is_admin != 0,
            created_at: m.created_at.format("%d/%m/%Y %H:%M").to_string(),
            created_at_iso: m.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        });
    }

    TicketViewTemplate {
        ticket_id: ticket.id,
        ticket_type: ticket.ticket_type.clone(),
        subject: ticket.subject.clone(),
        body: ticket.body.clone(),
        status: ticket.status.clone(),
        resolucion: ticket.resolucion.clone().unwrap_or_default(),
        created_at: ticket.created_at.format("%d/%m/%Y %H:%M").to_string(),
        owner_name,
        is_admin,
        reclamado_por_nombre: reclamado_nombre,
        messages: msg_rows,
    }.into_response()
}

#[derive(Template)]
#[template(path = "asistencia/support/ticket_view.html")]
pub struct TicketViewTemplate {
    pub ticket_id: i64,
    pub ticket_type: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub resolucion: String,
    pub created_at: String,
    pub owner_name: String,
    pub is_admin: bool,
    pub reclamado_por_nombre: String,
    pub messages: Vec<TicketMsgRow>,
}

pub struct TicketMsgRow {
    pub id: i64,
    pub body: String,
    pub sender_name: String,
    pub is_admin: bool,
    pub created_at: String,
    pub created_at_iso: String,
}

pub async fn api_create_ticket(State(state): State<AppState>, AuthUser(user): AuthUser, headers: HeaderMap, Form(form): Form<CreateTicketForm>) -> Response {
    let subject = form.subject.trim();
    let body = form.body.trim();
    if subject.is_empty() || body.is_empty() {
        return Redirect::to("/support?err=empty").into_response();
    }
    let is_guest = user.id == 0;
    if is_guest {
        // Sección 3.5: el invitado solo puede filear 'errores de inicio de
        // sesión' (mapeado a ticket_type='bug' por el CHECK de la tabla) o
        // 'sugerencia'. Cualquier otra categoría → rechazo server-side.
        let tt = form.ticket_type.as_str();
        if tt != "bug" && tt != "sugerencia" {
            return Redirect::to("/support?err=cat").into_response();
        }
        // Filear bajo la cuenta sistema "invitado" (sembrada en migración 036)
        // como dueño nominal + registrar la IP real del invitado (guest_ip).
        let owner = match state.db.find_user_by_username("invitado").await.ok().flatten() {
            Some(o) => o,
            None => return Redirect::to("/support?err=system").into_response(),
        };
        let ip = get_client_ip(&headers).unwrap_or_default();
        // Sección 3.6: rate limiting por IP para guests en lo poco que pueden
        // hacer (filear ticket). Si la IP superó el límite (5/hora), denegar.
        if !state.guest_rate_limiter.allow(&ip).await {
            return Redirect::to("/support?err=rate").into_response();
        }
        let _ = state.db.create_guest_ticket(owner.id, tt, subject, body, &ip).await;
        Redirect::to("/support?ok=created_guest").into_response()
    } else {
        let _ = state.db.create_ticket(user.id, &form.ticket_type, subject, body).await;
        Redirect::to("/support?ok=created").into_response()
    }
}

#[derive(Deserialize)]
pub struct TicketReplyForm {
    pub body: String,
}

/// POST /support/ticket/:id/reply — fallback sin JS (form POST → redirect).
pub async fn api_ticket_reply(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>, Form(form): Form<TicketReplyForm>) -> Response {
    let body = form.body.trim();
    if body.is_empty() { return Redirect::to(&format!("/support/ticket/{}", id)).into_response(); }
    let is_admin = if state.db.user_has_perm(&user, "tickets.respond").await { 1 } else { 0 };
    let _ = state.db.add_ticket_message(id, user.id, is_admin, body).await;
    // Si responde el admin y nadie había reclamado, el admin queda como
    // responsable automáticamente.
    if is_admin == 1 {
        let _ = state.db.reclamar_ticket_if_unclaimed(id, user.id).await;
    }
    let _ = state.db.update_ticket_status(id, if is_admin == 1 { "answered" } else { "open" }).await;
    Redirect::to(&format!("/support/ticket/{}", id)).into_response()
}

// ===== Tiempo real: endpoints JSON para polling + envío AJAX =====
// El cliente (ticket_view.html) hace GET a /api/support/ticket/:id/messages cada
// ~3s y pinta los mensajes nuevos sin recargar. Para enviar una respuesta sin
// recargar, hace POST a /api/support/ticket/:id/send y recibe el mensaje creado.

/// GET /api/support/ticket/:id/messages — lista de mensajes + estado en JSON.
pub async fn api_ticket_messages(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    let ticket = match state.db.get_ticket(id).await.ok().flatten() {
        Some(t) => t,
        None => return Json(json!({"error":"no_encontrado"})).into_response(),
    };
    if ticket.user_id != user.id && !state.db.user_has_perm(&user, "tickets.view").await {
        return Json(json!({"error":"no_autorizado"})).into_response();
    }
    let messages = state.db.list_ticket_messages(id).await.unwrap_or_default();
    let mut msgs = Vec::new();
    for m in messages {
        let sender = state.db.find_user_by_id(m.from_user_id).await.ok().flatten();
        let sender_name = sender.as_ref().map(|u| u.display()).unwrap_or_else(|| "Usuario".into());
        msgs.push(json!({
            "id": m.id,
            "body": m.body,
            "sender": sender_name,
            "is_admin": m.is_admin != 0,
            "created_at": m.created_at.format("%d/%m/%Y %H:%M").to_string(),
        }));
    }
    let reclamado_nombre = match ticket.reclamado_por {
        Some(aid) => state.db.find_user_by_id(aid).await.ok().flatten()
            .map(|u| u.display()).unwrap_or_else(|| "Admin".into()),
        None => String::new(),
    };
    Json(json!({
        "status": ticket.status,
        "resolucion": ticket.resolucion,
        "reclamado_por": reclamado_nombre,
        "messages": msgs,
    })).into_response()
}

/// POST /api/support/ticket/:id/send — enviar respuesta y devolverla en JSON
/// (para que el cliente la pinte sin recargar). Acepta form-encoded (body=...).
pub async fn api_ticket_send(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>, Form(form): Form<TicketReplyForm>) -> Response {
    let body = form.body.trim();
    if body.is_empty() {
        return Json(json!({"error":"vacio"})).into_response();
    }
    // Solo el dueño o un admin pueden escribir en el ticket.
    let ticket = match state.db.get_ticket(id).await.ok().flatten() {
        Some(t) => t,
        None => return Json(json!({"error":"no_encontrado"})).into_response(),
    };
    if ticket.user_id != user.id && !state.db.user_has_perm(&user, "tickets.respond").await {
        return Json(json!({"error":"no_autorizado"})).into_response();
    }
    if ticket.status == "closed" {
        return Json(json!({"error":"cerrado"})).into_response();
    }
    let is_admin = if state.db.user_has_perm(&user, "tickets.respond").await { 1 } else { 0 };
    let _ = state.db.add_ticket_message(id, user.id, is_admin, body).await;
    if is_admin == 1 {
        let _ = state.db.reclamar_ticket_if_unclaimed(id, user.id).await;
    }
    let _ = state.db.update_ticket_status(id, if is_admin == 1 { "answered" } else { "open" }).await;
    let now = chrono::Utc::now();
    Json(json!({
        "ok": true,
        "message": {
            "body": body,
            "sender": user.display(),
            "is_admin": is_admin == 1,
            "created_at": now.format("%d/%m/%Y %H:%M").to_string(),
        }
    })).into_response()
}

// ===== Ciclo de vida del ticket (admin) =====


/// POST /support/ticket/:id/claim — admin reclama el ticket.
pub async fn api_ticket_claim(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    if !state.db.user_has_perm(&user, "tickets.manage").await { return Redirect::to("/support").into_response(); }
    let _ = state.db.reclamar_ticket(id, user.id).await;
    Redirect::to(&format!("/support/ticket/{}", id)).into_response()
}

/// POST /support/ticket/:id/resolve — admin marca como terminado.
pub async fn api_ticket_resolve(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    if !state.db.user_has_perm(&user, "tickets.manage").await { return Redirect::to("/support").into_response(); }
    let _ = state.db.terminar_ticket(id).await;
    Redirect::to(&format!("/support/ticket/{}", id)).into_response()
}

/// POST /support/ticket/:id/reject — admin rechaza el ticket.
pub async fn api_ticket_reject(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    if !state.db.user_has_perm(&user, "tickets.manage").await { return Redirect::to("/support").into_response(); }
    let _ = state.db.rechazar_ticket(id).await;
    Redirect::to(&format!("/support/ticket/{}", id)).into_response()
}

/// POST /support/ticket/:id/close — cierra el ticket (admin o dueño).
pub async fn api_ticket_close(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<i64>) -> Response {
    if let Some(ticket) = state.db.get_ticket(id).await.ok().flatten() {
        if ticket.user_id == user.id || state.db.user_has_perm(&user, "tickets.manage").await {
            let _ = state.db.cerrar_ticket(id).await;
        }
    }
    Redirect::to(&format!("/support/ticket/{}", id)).into_response()
}

// ===== Admin-facing =====

pub async fn admin_tickets(State(state): State<AppState>, AuthUser(actor): AuthUser) -> Response {
    if !state.db.user_has_perm(&actor, "tickets.view").await {
        return Redirect::to("/admin").into_response();
    }
    let tickets = state.db.list_all_tickets().await.unwrap_or_default();
    AdminTicketsTemplate {
        actor_username: actor.username.clone(),
        tickets: tickets.iter().map(|t| TicketRow {
            id: t.id,
            ticket_type: t.ticket_type.clone(),
            subject: t.subject.clone(),
            status: t.status.clone(),
            resolucion: t.resolucion.clone().unwrap_or_default(),
            created_at: t.created_at.format("%d/%m/%Y %H:%M").to_string(),
        }).collect(),
    }.into_response()
}

#[derive(Template)]
#[template(path = "admin/tickets/admin_tickets.html")]
pub struct AdminTicketsTemplate {
    pub actor_username: String,
    pub tickets: Vec<TicketRow>,
}

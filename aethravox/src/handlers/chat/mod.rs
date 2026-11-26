#![allow(unused_imports)]
// Web 2 — Aethravox Chat (estilo WhatsApp con WebSocket)
use askama::Template;
use futures::stream::StreamExt;
use futures::sink::SinkExt;
use axum::{
    extract::{
        ws::{self, WebSocketUpgrade, Message as WsMessage},
        Form, Multipart, Path, State,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::{
    auth_mw::AuthUser,
    models::{NewDmForm, NewGroupForm},
    AppState,
};

// ===== WebSocket connections registry =====
pub type WsConnections = Arc<Mutex<HashMap<i64, mpsc::UnboundedSender<String>>>>;

// ===== GET /chat — Chat page (Askama template, responsive) =====

mod page;
pub use self::page::*;
mod websocket;
pub use self::websocket::*;
mod api_list;
pub use self::api_list::*;
mod api_create;
pub use self::api_create::*;
mod uploads;
pub use self::uploads::*;
mod groups;
pub use self::groups::*;

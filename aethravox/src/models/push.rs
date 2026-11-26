#![allow(unused_imports)]
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::FromRow;

use super::*;

/// Suscripción Push de un usuario (Web Push API). Cada navegador/dispositivo
/// que pide recibir push genera una suscripción con un endpoint único (en
/// FCM/Mozilla/APNS) y dos claves (p256dh + auth) que el server usa para
/// cifrar el payload. Se guarda en `push_subscriptions`.
#[derive(Debug, Clone, FromRow)]
pub struct PushSubscription {
    pub id: i64,
    pub user_id: i64,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: String,
}


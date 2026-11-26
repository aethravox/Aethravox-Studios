//! Web Push (VAPID + RFC 8291) — envío de notificaciones push a navegadores
//! que se han suscrito, para avisar de mensajes/llamadas incluso cuando la
//! pestaña del chat está cerrada.
//!
//! El ciclo:
//! 1. `PushManager::init` genera (una sola vez) un par de claves P-256 VAPID
//!    en `data/vapid_private.pem` y deriva la pública (base64url, 65 bytes
//!    uncompressed) que el navegador necesita como `applicationServerKey`.
//! 2. El navegador suscribe push con esa clave y POSTea la subscription info
//!    (endpoint + p256dh + auth) a `/api/notifications/subscribe`, que la
//!    guarda en `push_subscriptions`.
//! 3. Cuando el backend detecta que un destinatario NO tiene WS activo,
//!    llama a `PushManager::send_to_user(user_id, payload)`, que recupera sus
//!    suscripciones y envía un push cifrado a cada una.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
// Generación de claves VAPID (P-256). Llenamos 32 bytes con OS randomness vía
// getrandom (API estable, sin depender de las versiones cambiantes de
// rand_core/OsRng) y construimos la SigningKey. Si los bytes caen fuera del
// rango válido del escalar (caso ~2^-128) reintentamos.
use p256::ecdsa::SigningKey;
use p256::pkcs8::{EncodePrivateKey, LineEnding};
use std::path::Path;
use web_push::{
    ContentEncoding, IsahcWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushError, WebPushMessageBuilder,
};

use crate::db::Db;
use crate::models::PushSubscription;

/// Resultado del envío de un push a una suscripción concreta.
pub enum SendOutcome {
    /// El push se entregó al push service (FCM/Mozilla/APNS). El navegador
    /// mostrará la notificación cuando el SW reciba el evento `push`.
    Sent,
    /// La suscripción ya no es válida (410 Gone / 404). El caller debe
    /// borrarla de la DB para no seguir intentándolo.
    SubscriptionDead,
    /// Error transitorio (red, servidor 5xx). No se borra la suscripción.
    Failed,
}

pub struct PushManager {
    /// Bytes del PEM PKCS8 de la clave privada VAPID (en memoria para no
    /// releer el fichero en cada envío).
    pem_bytes: Vec<u8>,
    /// Clave pública uncompressed (65 bytes) en base64url sin padding — lista
    /// para entregar al navegador como `applicationServerKey`.
    pub public_key_b64: String,
    /// Cliente HTTP reutilizable (isahc/curl). Crearlo es caro.
    client: IsahcWebPushClient,
    /// Claim `sub` del JWT VAPID (mailto: del contacto).
    sub_claim: String,
}

impl PushManager {
    /// Genera las claves VAPID si no existen, las carga y construye el
    /// cliente. Si algo falla, devuelve error; el caller lo envuelve en
    /// `Option` para que la app arranque igualmente (sin push).
    pub fn init(data_dir: &Path, contact_email: &str) -> Result<Self> {
        std::fs::create_dir_all(data_dir).ok();
        let pem_path = data_dir.join("vapid_private.pem");
        if !pem_path.exists() {
            // Par P-256 nuevo → PEM PKCS8. Se hace UNA sola vez por
            // despliegue; persistir la misma clave es imprescindible: si
            // rotara, las suscripciones existentes dejarían de validarse.
            // Generamos 32 bytes aleatorios vía getrandom y construimos la
            // SigningKey; reintentando si el escalar queda fuera de rango.
            let sk = {
                let mut bytes = [0u8; 32];
                let mut tries = 0;
                loop {
                    getrandom::fill(&mut bytes)
                        .map_err(|e| anyhow!("getrandom: {e}"))?;
                    match SigningKey::from_slice(&bytes) {
                        Ok(k) => break k,
                        Err(_) => {
                            tries += 1;
                            if tries > 8 {
                                return Err(anyhow!(
                                    "no se pudo generar un escalar P-256 válido tras 8 intentos"
                                ));
                            }
                            continue;
                        }
                    }
                }
            };
            let pem = sk
                .to_pkcs8_pem(LineEnding::LF)
                .context("serializar clave VAPID a PEM PKCS8")?;
            std::fs::write(&pem_path, pem.as_str())
                .with_context(|| format!("escribir {}", pem_path.display()))?;
            tracing::info!("🔑 Claves VAPID generadas en {}", pem_path.display());
        }
        let pem_bytes = std::fs::read(&pem_path)
            .with_context(|| format!("leer {}", pem_path.display()))?;
        // Derivar la pública sin suscripción para obtener los bytes
        // uncompressed que el navegador espera como applicationServerKey.
        let partial = VapidSignatureBuilder::from_pem_no_sub(&pem_bytes[..])
            .map_err(|e| anyhow!("parsear PEM VAPID: {e:?}"))?;
        let pub_bytes = partial.get_public_key();
        let public_key_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&pub_bytes);
        let client = IsahcWebPushClient::new()
            .map_err(|e| anyhow!("crear IsahcWebPushClient: {e:?}"))?;
        Ok(Self {
            pem_bytes,
            public_key_b64,
            client,
            sub_claim: format!("mailto:{}", contact_email),
        })
    }

    /// Envía un push (payload JSON ya serializado a bytes) a UNA suscripción.
    /// Devuelve el outcome para que el caller decida si borrarla.
    pub async fn send(&self, sub: &PushSubscription, payload: &[u8]) -> SendOutcome {
        let sub_info = SubscriptionInfo::new(&sub.endpoint, &sub.p256dh, &sub.auth);
        // Firma VAPID para esta suscripción. Clonamos el builder parcial (la
        // clave privada es la misma, solo cambia la subscription info).
        let sig = match VapidSignatureBuilder::from_pem_no_sub(&self.pem_bytes[..]) {
            Ok(p) => {
                // add_sub_info consume el partial y devuelve el builder completo;
                // add_claim va después (mut) sobre el builder devuelto.
                let mut builder = p.add_sub_info(&sub_info);
                builder.add_claim("sub", self.sub_claim.clone());
                match builder.build() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("VAPID build error: {e:?}");
                        return SendOutcome::Failed;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("VAPID PEM error: {e:?}");
                return SendOutcome::Failed;
            }
        };
        let mut builder = WebPushMessageBuilder::new(&sub_info);
        builder.set_payload(ContentEncoding::Aes128Gcm, payload);
        builder.set_vapid_signature(sig);
        let msg = match builder.build() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("push build error: {e:?}");
                return SendOutcome::Failed;
            }
        };
        match self.client.send(msg).await {
            Ok(()) => SendOutcome::Sent,
            Err(WebPushError::EndpointNotValid(_)) | Err(WebPushError::EndpointNotFound(_)) => {
                tracing::info!(
                    "Suscripción push muerta (endpoint {}), se borrará",
                    sub.endpoint
                );
                SendOutcome::SubscriptionDead
            }
            Err(e) => {
                tracing::warn!("push send error a {}: {e:?}", sub.endpoint);
                SendOutcome::Failed
            }
        }
    }

    /// Envía el mismo payload a TODAS las suscripciones de un usuario.
    /// Borra las que devuelvan SubscriptionDead. No falla si no hay
    /// suscripciones (silencio).
    pub async fn send_to_user(&self, db: &Db, user_id: i64, payload: &serde_json::Value) {
        let subs = match db.get_push_subscriptions(user_id).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("get_push_subscriptions({user_id}): {e}");
                return;
            }
        };
        if subs.is_empty() {
            return;
        }
        let payload_bytes = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
        for sub in &subs {
            match self.send(sub, &payload_bytes).await {
                SendOutcome::SubscriptionDead => {
                    let _ = db.remove_push_subscription(&sub.endpoint).await;
                }
                _ => {}
            }
        }
    }
}

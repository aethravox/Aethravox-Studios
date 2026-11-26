//! Cliente mTLS del hub contra la API interna de AethraMail (Sección 6).
//!
//! El hub presenta `client.pem`/`client-key.pem` (firmados por la CA interna
//! `ca.pem`, generados por `aethramail/scripts/gen_certs.sh`) y solo así
//! completa el handshake TLS contra `https://127.0.0.1:8449`. Sin ese cert,
//! AethraMail descarta la conexión a nivel TLS (ver aethramail/src/tls.rs).
//!
//! Stack: `rustls 0.23` (provider `ring`, sin aws-lc-rs/cmake) + `hyper 1`
//! cliente low-level (`client::conn::http1`) + `http-body-util`. Una conexión
//! por llamada: el volumen es bajo (recovery / verificación de correo).

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request};
use hyper_util::rt::TokioIo;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{debug, warn};

use crate::config::Config;

/// Respuesta de `POST /api/internal/send` (AethraMail).
#[derive(Deserialize)]
struct SendResp {
    ok: bool,
    id: Option<i64>,
    error: Option<String>,
}

#[derive(Clone)]
pub struct AethraMailClient {
    connector: TlsConnector,
    addr: SocketAddr,
    server_name: ServerName<'static>,
    sender: String,
    timeout: std::time::Duration,
}

impl AethraMailClient {
    /// Construye el cliente cargando los certs. Falla si los archivos no
    /// existen o no parsean; el llamador decide si arrancar sin mail (None).
    pub fn new(cfg: &Config) -> Result<Self> {
        // ring como crypto provider por defecto (idempotente).
        let _ = rustls::crypto::ring::default_provider().install_default();

        let client_certs = load_certs(&cfg.aethramail_client_cert)
            .with_context(|| format!("cargar client cert {}", cfg.aethramail_client_cert))?;
        let client_key = load_key(&cfg.aethramail_client_key)
            .with_context(|| format!("cargar client key {}", cfg.aethramail_client_key))?;

        let mut roots = RootCertStore::empty();
        let ca_certs = load_certs(&cfg.aethramail_ca_cert)
            .with_context(|| format!("cargar CA {}", cfg.aethramail_ca_cert))?;
        for ca in &ca_certs {
            roots.add(ca.clone())?;
        }

        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| anyhow!("build client config: {e}"))?;

        let (addr, server_name) = parse_api_url(&cfg.aethramail_api_url)
            .with_context(|| format!("parsear AETHRAMAIL_API_URL={}", cfg.aethramail_api_url))?;

        Ok(Self {
            connector: TlsConnector::from(Arc::new(config)),
            addr,
            server_name,
            sender: cfg.aethramail_sender.clone(),
            timeout: std::time::Duration::from_secs(cfg.aethramail_timeout_secs),
        })
    }

    /// Envía un correo interno vía la API mTLS de AethraMail.
    /// `to` debe ser una dirección interna existente (p.ej. "alice@shadow.sac").
    /// Devuelve Ok(id) si AethraMail lo aceptó, Err con el mensaje en caso contrario.
    pub async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<i64> {
        let payload = serde_json::json!({
            "from": self.sender,
            "to": to,
            "subject": subject,
            "body": body,
        });
        let json = serde_json::to_vec(&payload)?;

        // Una conexión TCP+TLS por llamada (volumen bajo).
        let tcp = tokio::time::timeout(self.timeout, TcpStream::connect(self.addr))
            .await
            .map_err(|_| anyhow!("timeout conectando a AethraMail"))??;
        let _ = tcp.set_nodelay(true);

        let tls = tokio::time::timeout(
            self.timeout,
            self.connector.connect(self.server_name.clone(), tcp),
        )
        .await
        .map_err(|_| anyhow!("timeout en handshake TLS con AethraMail"))??;

        let io = TokioIo::new(tls);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        // Drive la conexión en background hasta que se cierre.
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                debug!("aethramail conn cerrada: {e}");
            }
        });

        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/internal/send")
            .header("host", "aethramail-internal")
            .header("content-type", "application/json")
            .header("content-length", json.len())
            .body(Full::new(Bytes::from(json)))?;

        let resp = tokio::time::timeout(self.timeout, sender.send_request(req))
            .await
            .map_err(|_| anyhow!("timeout esperando respuesta de AethraMail"))??;
        let status = resp.status();
        let body_bytes = resp.into_body().collect().await?.to_bytes();

        let parsed: SendResp = serde_json::from_slice(&body_bytes)
            .with_context(|| {
                format!(
                    "parsear respuesta AethraMail (status={status}, body={})",
                    String::from_utf8_lossy(&body_bytes)
                )
            })?;

        if parsed.ok {
            Ok(parsed.id.unwrap_or(0))
        } else {
            let msg = parsed.error.unwrap_or_else(|| "error desconocido".into());
            warn!("AethraMail rechazó envío a {to}: {msg}");
            Err(anyhow!("AethraMail: {msg}"))
        }
    }
}

// ── helpers de carga (mismo patrón que aethramail/src/tls.rs) ─────────

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<_, _>>()?;
    if certs.is_empty() {
        return Err(anyhow!("no se encontraron certs en {path}"));
    }
    Ok(certs)
}

fn load_key(path: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = std::io::BufReader::new(std::fs::File::open(path)?);
    rustls_pemfile::private_key(&mut reader)?
        .ok_or_else(|| anyhow!("no se encontró private key en {path}"))
}

/// "https://127.0.0.1:8449" → (SocketAddr, ServerName).
fn parse_api_url(url: &str) -> Result<(SocketAddr, ServerName<'static>)> {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let stripped = stripped.split('/').next().unwrap_or(stripped);
    let (host, port) = match stripped.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(8449)),
        None => (stripped, 8449),
    };
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let server_name: ServerName<'static> = match host.parse::<IpAddr>() {
        Ok(ip) => ServerName::from(ip),
        Err(_) => ServerName::try_from(host.to_string())
            .map_err(|e| anyhow!("server name inválido '{host}': {e}"))?,
    };
    Ok((addr, server_name))
}

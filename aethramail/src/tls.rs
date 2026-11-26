use std::fs::File;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::Router;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
    service::TowerToHyperService,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ServerConfig, WebPkiClientVerifier};
use rustls::RootCertStore;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// Instala ring como crypto provider por defecto (rustls 0.23 lo exige cuando
/// no se usa aws-lc-rs). Idempotente.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = BufReader::new(File::open(path).with_context(|| format!("abrir {path}"))?);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<_, _>>()
        .with_context(|| format!("parsear certs en {path}"))?;
    if certs.is_empty() {
        return Err(anyhow!("no se encontraron certs en {path}"));
    }
    Ok(certs)
}

pub fn load_key(path: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(File::open(path).with_context(|| format!("abrir {path}"))?);
    rustls_pemfile::private_key(&mut reader)
        .with_context(|| format!("parsear key en {path}"))?
        .ok_or_else(|| anyhow!("no se encontró private key en {path}"))
}

/// Construye la ServerConfig con client auth OBLIGATORIO (mTLS, 5.7–5.8).
/// Solo conexiones que presenten un cert firmado por `client_ca` completan el
/// handshake; el resto se descarta a nivel TLS antes de tocar la app.
pub fn build_mtls_server_config(
    server_cert_path: &str,
    server_key_path: &str,
    client_ca_path: &str,
) -> Result<ServerConfig> {
    install_crypto_provider();

    let certs = load_certs(server_cert_path)?;
    let key = load_key(server_key_path)?;

    let mut client_roots = RootCertStore::empty();
    let ca_certs = load_certs(client_ca_path)?;
    for ca in &ca_certs {
        client_roots.add(ca.clone())?;
    }

    let verifier =
        WebPkiClientVerifier::builder(Arc::new(client_roots))
            .build()
            .map_err(|e| anyhow!("build client verifier: {e}"))?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|e| anyhow!("build server config: {e}"))?;

    Ok(config)
}

/// Sirve `app` sobre HTTPS + mTLS (client cert obligatorio) en `addr`.
///
/// axum 0.7 `serve` solo acepta `tokio::net::TcpListener`, así que replicamos
/// su loop interno: aceptamos TCP, envolvemos en TLS (con client auth) y
/// servimos cada conexión con `hyper_util` + `TowerToHyperService` (exactamente
/// como hace axum internamente).
pub async fn serve_mtls(app: Router, addr: SocketAddr, config: ServerConfig) -> Result<()> {
    let tcp = TcpListener::bind(addr).await?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let hyper_service = TowerToHyperService::new(app);

    loop {
        let (stream, peer) = tcp.accept().await?;
        let acceptor = acceptor.clone();
        let svc = hyper_service.clone();
        tokio::spawn(async move {
            // mTLS: si el cliente no presenta un cert válido, el handshake
            // falla y la conexión se descarta silenciosamente.
            let tls = match acceptor.accept(stream).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::debug!(%peer, err=%e, "mTLS handshake rechazado");
                    return;
                }
            };
            let io = TokioIo::new(tls);
            if let Err(e) = Builder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await
            {
                tracing::debug!(%peer, err=%e, "conn cerrada");
            }
        });
    }
}

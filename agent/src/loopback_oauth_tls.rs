//! Self-signed TLS for loopback OAuth callbacks (browser → agent).

use axum::Router;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

fn cert_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("Library/Application Support/TradeAutopsy/loopback-oauth")
}

fn load_or_create_pem() -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let dir = cert_dir();
    let cert_path = dir.join("loopback-oauth.crt");
    let key_path = dir.join("loopback-oauth.key");
    if cert_path.is_file() && key_path.is_file() {
        return Ok((fs::read(&cert_path)?, fs::read(&key_path)?));
    }
    fs::create_dir_all(&dir)?;

    let key_pair = KeyPair::generate()?;
    let mut params = CertificateParams::new(vec!["127.0.0.1".to_string()])?;
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "TradeAutopsy Loopback OAuth");
    let cert = params.self_signed(&key_pair)?;
    let cert_pem = cert.pem().into_bytes();
    let key_pem = key_pair.serialize_pem().into_bytes();
    fs::write(&cert_path, &cert_pem)?;
    fs::write(&key_path, &key_pem)?;
    Ok((cert_pem, key_pem))
}

/// Serve `router` on `127.0.0.1:{port}` with a persisted loopback self-signed cert.
pub fn spawn(router: Router, port: u16) -> anyhow::Result<()> {
    crate::rustls_crypto::ensure_installed();
    let (cert_pem, key_pem) = load_or_create_pem()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tokio::spawn(async move {
        let tls = match axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("loopback oauth tls config: {e}");
                return;
            }
        };
        if let Err(e) = axum_server::bind_rustls(addr, tls)
            .serve(router.into_make_service())
            .await
        {
            tracing::error!(%addr, "loopback oauth tls server: {e}");
        }
    });
    tracing::info!(%addr, "loopback oauth tls listening (browser callbacks)");
    Ok(())
}

//! Browser OAuth redirect targets on loopback HTTPS (Kite portal requires `https://` URIs).
//! Daemon wire traffic stays on plain HTTP [`AGENT_PORT`] (default 9137).

pub const DEFAULT_OAUTH_TLS_PORT: u16 = 9140;

/// Port for `https://127.0.0.1:{port}/api/daemon/broker/*/callback` only.
pub fn oauth_tls_port() -> u16 {
    std::env::var("AGENT_OAUTH_TLS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_OAUTH_TLS_PORT)
}

/// Registered redirect/postback URI shape for broker OAuth callbacks.
pub fn https_oauth_callback_url(path: &str) -> String {
    let trimmed = path.trim();
    let path = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    format!("https://127.0.0.1:{}{}", oauth_tls_port(), path)
}

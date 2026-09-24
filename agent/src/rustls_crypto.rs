//! Rustls 0.23 requires an explicit process-wide crypto provider before TLS (OAuth loopback, etc.).

use std::sync::Once;

static INSTALL: Once = Once::new();

/// Safe to call multiple times; only the first install runs.
pub fn ensure_installed() {
    INSTALL.call_once(|| {
        if rustls::crypto::ring::default_provider()
            .install_default()
            .is_err()
        {
            tracing::debug!("rustls CryptoProvider already installed");
        }
    });
}

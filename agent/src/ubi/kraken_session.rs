//! Kraken spot REST signing (host-side attach only).
//!
//! ADR 0017 · B6 `kraken` rows 10/22. Production host `api.kraken.com` only;
//! futures/demo-futures hosts are refused on book `kraken-com-spot`.

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ubi::http::PreparedHttpRequest;

type HmacSha512 = Hmac<Sha512>;

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const KRAKEN_API_HOST: &str = "api.kraken.com";
pub const KRAKEN_BOOK_ID: &str = "kraken-com-spot";

const REFUSED_HOSTS: &[&str] = &["futures.kraken.com", "demo-futures.kraken.com"];

/// Live monotonic nonce (nanoseconds). Tests pass `Some(now_unix_ms)` via `prepare_kraken_signed_post`.
pub fn next_kraken_nonce(test_clock_ms: Option<i64>) -> u64 {
    if let Some(ms) = test_clock_ms {
        return ms as u64 * 1_000_000;
    }
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    loop {
        let prev = NONCE_COUNTER.load(Ordering::Relaxed);
        let next = base.max(prev.saturating_add(1));
        if NONCE_COUNTER
            .compare_exchange(prev, next, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}

pub fn kraken_host_refused(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    REFUSED_HOSTS.contains(&h.as_str())
}

/// Spot private POST paths allowed for book `kraken-com-spot` (B6 row 2).
pub fn kraken_path_allowed(method: &str, path_norm: &str) -> bool {
    let upper = method.to_ascii_uppercase();
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    match upper.as_str() {
        "GET" if p.starts_with("/0/public/") => {
            p == "/0/public/time"
                || p == "/0/public/systemstatus"
                || p == "/0/public/ticker"
                || p == "/0/public/ohlc"
                || p == "/0/public/depth"
                || p == "/0/public/trades"
                || p == "/0/public/assetpairs"
        }
        "POST" if p.starts_with("/0/private/") => {
            p == "/0/private/tradeshistory" || p == "/0/private/balance"
        }
        _ => false,
    }
}

/// Build `nonce=…&…` body (nonce always first).
pub fn kraken_post_data(nonce: u64, extra_form: &str) -> String {
    let nonce_part = format!("nonce={nonce}");
    let extra = extra_form.trim().trim_start_matches('&');
    if extra.is_empty() {
        nonce_part
    } else {
        format!("{nonce_part}&{extra}")
    }
}

/// Kraken spot sign: HMAC-SHA512(secret, path + SHA256(nonce + post_data)).
pub fn kraken_sign_spot(api_secret_b64: &str, path: &str, post_data: &str) -> Result<String, String> {
    let secret = BASE64_STANDARD
        .decode(api_secret_b64.trim())
        .map_err(|e| format!("kraken secret base64 decode: {e}"))?;
    let nonce_str = post_data
        .strip_prefix("nonce=")
        .and_then(|rest| rest.split('&').next())
        .ok_or("kraken post_data missing nonce")?;
    let sha_input = format!("{nonce_str}{post_data}");
    let hash = Sha256::digest(sha_input.as_bytes());
    let mut message = path.as_bytes().to_vec();
    message.extend_from_slice(&hash);
    let mut mac =
        HmacSha512::new_from_slice(&secret).map_err(|e| format!("kraken hmac key: {e}"))?;
    mac.update(&message);
    Ok(BASE64_STANDARD.encode(mac.finalize().into_bytes()))
}

pub fn prepare_kraken_signed_post(
    host: &str,
    path: &str,
    extra_form: &str,
    api_key: &str,
    api_secret: &str,
    test_clock_ms: Option<i64>,
) -> Result<PreparedHttpRequest, String> {
    let host = host.trim().to_ascii_lowercase();
    if host != KRAKEN_API_HOST {
        return Err(format!("kraken signing refused on {host}"));
    }
    if kraken_host_refused(&host) {
        return Err(format!("kraken refused host {host}"));
    }
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    let nonce = next_kraken_nonce(test_clock_ms);
    let post_data = kraken_post_data(nonce, extra_form);
    let sign = kraken_sign_spot(api_secret, &path, &post_data)?;
    Ok(PreparedHttpRequest {
        method: "POST".to_string(),
        url: format!("https://{host}{path}"),
        headers: vec![
            ("API-Key".to_string(), api_key.to_string()),
            ("API-Sign".to_string(), sign),
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
        ],
        body: Some(post_data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refused_hosts_include_futures_siblings() {
        assert!(kraken_host_refused("futures.kraken.com"));
        assert!(kraken_host_refused("demo-futures.kraken.com"));
        assert!(!kraken_host_refused("api.kraken.com"));
    }

    #[test]
    fn post_data_puts_nonce_first() {
        assert_eq!(kraken_post_data(42, ""), "nonce=42");
        assert_eq!(
            kraken_post_data(42, "type=trade&ofs=0"),
            "nonce=42&type=trade&ofs=0"
        );
    }

    #[test]
    fn sign_is_deterministic_for_fixture_secret() {
        // secret = base64("secret")
        let secret_b64 = BASE64_STANDARD.encode(b"secret");
        let post = "nonce=1700000000000000&type=trade";
        let sig = kraken_sign_spot(&secret_b64, "/0/private/TradesHistory", post).unwrap();
        assert!(!sig.is_empty());
        assert_eq!(
            kraken_sign_spot(&secret_b64, "/0/private/TradesHistory", post).unwrap(),
            sig
        );
    }
}

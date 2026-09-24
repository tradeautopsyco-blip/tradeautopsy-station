//! Coinbase Advanced Trade CDP JWT session (host-side attach only).
//!
//! ADR 0018 · B6 `coinbase_advanced` rows 10/22. Production host `api.coinbase.com`
//! only; `api-sandbox.coinbase.com` is refused. PEM EC private key lives in Keychain
//! (vault); the Wasm component never sees it.

use crate::ubi::http::PreparedHttpRequest;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::DecodePrivateKey;
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

pub const COINBASE_API_HOST: &str = "api.coinbase.com";
pub const COINBASE_BOOK_ID: &str = "coinbase-advanced-spot";
pub const COINBASE_REST_PREFIX: &str = "/api/v3/brokerage";
pub const COINBASE_FILLS_PATH: &str = "/api/v3/brokerage/orders/historical/fills";
pub const COINBASE_ACCOUNTS_PATH: &str = "/api/v3/brokerage/accounts";
pub const COINBASE_PRODUCTS_PATH: &str = "/api/v3/brokerage/products";

pub const JWT_ISSUER: &str = "cdp";
pub const JWT_EXPIRY_SECS: u64 = 120;

const REFUSED_HOSTS: &[&str] = &["api-sandbox.coinbase.com"];

/// Read-only Advanced Trade paths for book `coinbase-advanced-spot` (B6 row 2).
pub fn coinbase_path_allowed(method: &str, path_norm: &str) -> bool {
    if method.to_ascii_uppercase() != "GET" {
        return false;
    }
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    p == COINBASE_FILLS_PATH
        || p == COINBASE_ACCOUNTS_PATH
        || p == COINBASE_PRODUCTS_PATH
        || p.starts_with(&format!("{COINBASE_REST_PREFIX}/products/"))
        || p.starts_with("/api/v3/brokerage/market/")
}

pub fn coinbase_host_refused(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    REFUSED_HOSTS.contains(&h.as_str())
}

/// REST JWT `uri` claim: `"{METHOD} {host}{path}"` (no scheme).
pub fn coinbase_rest_uri(method: &str, host: &str, path: &str) -> String {
    let method = method.trim().to_ascii_uppercase();
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{method} {host}{path}")
}

fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn normalize_pem(pem: &str) -> String {
    pem.trim().replace("\\n", "\n")
}

fn parse_signing_key_pem(pem: &str) -> Result<SigningKey, String> {
    if let Ok(key) = SigningKey::from_pkcs8_pem(pem) {
        return Ok(key);
    }
    let doc = pem::parse(pem).map_err(|e| format!("coinbase PEM parse: {e}"))?;
    let bytes = doc.contents();
    if bytes.len() != 32 {
        return Err("coinbase EC private key must be 32 bytes".into());
    }
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "coinbase EC private key length")?;
    SigningKey::from_bytes(&arr.into())
        .map_err(|e| format!("coinbase SEC1 key: {e}"))
}

/// Build ES256 JWT for one REST call (`kid` = API key name, `nonce` = random hex).
pub fn build_rest_jwt(
    api_key: &str,
    pem_private_key: &str,
    uri: &str,
    now_unix_secs: u64,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("coinbase api_key is required".into());
    }
    let pem = normalize_pem(pem_private_key);
    let signing_key = parse_signing_key_pem(&pem)?;

    let mut nonce_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = hex::encode(nonce_bytes);

    let header = serde_json::json!({
        "alg": "ES256",
        "typ": "JWT",
        "kid": api_key.trim(),
        "nonce": nonce,
    });
    let payload = serde_json::json!({
        "sub": api_key.trim(),
        "iss": JWT_ISSUER,
        "nbf": now_unix_secs,
        "exp": now_unix_secs + JWT_EXPIRY_SECS,
        "uri": uri,
    });

    let header_b64 = base64url_encode(header.to_string().as_bytes());
    let payload_b64 = base64url_encode(payload.to_string().as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");

    let signature: Signature = signing_key
        .sign(signing_input.as_bytes());
    let sig_b64 = base64url_encode(&signature.to_bytes());

    Ok(format!("{signing_input}.{sig_b64}"))
}

pub fn coinbase_bearer_authorization(jwt: &str) -> String {
    format!("Bearer {}", jwt.trim())
}

pub fn prepare_coinbase_authenticated_request(
    method: &str,
    host: &str,
    path: &str,
    query: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&str>,
    api_key: &str,
    pem_private_key: &str,
    now_unix_ms: i64,
) -> Result<PreparedHttpRequest, String> {
    let host_norm = host.trim().to_ascii_lowercase();
    if host_norm != COINBASE_API_HOST {
        return Err(format!("coinbase signing refused on {host_norm}"));
    }
    if coinbase_host_refused(&host_norm) {
        return Err("coinbase sandbox host refused".into());
    }
    let now_secs = (now_unix_ms / 1000).max(0) as u64;
    let uri = coinbase_rest_uri(method, &host_norm, path);
    let jwt = build_rest_jwt(api_key, pem_private_key, &uri, now_secs)?;

    let canonical = query
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let url = if canonical.is_empty() {
        format!("https://{host_norm}{path}")
    } else {
        format!("https://{host_norm}{path}?{canonical}")
    };

    let mut out_headers: Vec<(String, String)> = headers
        .iter()
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();
    out_headers.push((
        "Authorization".to_string(),
        coinbase_bearer_authorization(&jwt),
    ));
    out_headers.push(("Accept".to_string(), "application/json".to_string()));

    Ok(PreparedHttpRequest {
        method: method.to_ascii_uppercase(),
        url,
        headers: out_headers,
        body: body.map(|b| b.to_string()),
    })
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePrivateKey;

    fn test_signing_key_pem() -> String {
        let key = SigningKey::random(&mut rand::thread_rng());
        key.to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
            .expect("pkcs8 pem")
            .to_string()
    }

    #[test]
    fn uri_claim_matches_advanced_trade_shape() {
        let uri = coinbase_rest_uri("GET", COINBASE_API_HOST, COINBASE_FILLS_PATH);
        assert_eq!(
            uri,
            "GET api.coinbase.com/api/v3/brokerage/orders/historical/fills"
        );
    }

    #[test]
    fn sandbox_host_refused() {
        assert!(coinbase_host_refused("api-sandbox.coinbase.com"));
        assert!(!coinbase_host_refused(COINBASE_API_HOST));
    }

    #[test]
    fn jwt_has_three_segments_and_bearer_prefix() {
        let pem = test_signing_key_pem();
        let jwt = build_rest_jwt(
            "organizations/test/apiKeys/key-id",
            &pem,
            "GET api.coinbase.com/api/v3/brokerage/accounts",
            1_700_000_000,
        )
        .expect("jwt");
        assert_eq!(jwt.matches('.').count(), 2);
        assert!(coinbase_bearer_authorization(&jwt).starts_with("Bearer "));
    }

    #[test]
    fn fills_path_allowed_execution_refused() {
        assert!(coinbase_path_allowed("GET", COINBASE_FILLS_PATH));
        assert!(coinbase_path_allowed("GET", COINBASE_ACCOUNTS_PATH));
        assert!(!coinbase_path_allowed("POST", "/api/v3/brokerage/orders"));
    }
}

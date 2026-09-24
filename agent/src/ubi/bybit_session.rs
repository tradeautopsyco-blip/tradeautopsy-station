//! Bybit v5 API-key HMAC session (host-side attach only).
//!
//! ADR 0015 · B6 `bybit` rows 10/22. Production host `api.bybit.com` only;
//! testnet/demo hosts are refused. Same Keychain `HmacApiKeySecret` blob as Binance COM.

use crate::ubi::http::{PreparedHttpRequest, sign_query};

pub const BYBIT_API_HOST: &str = "api.bybit.com";
pub const BYBIT_BOOK_ID: &str = "bybit-com-spot";
pub const BYBIT_RECV_WINDOW: &str = "5000";

const REFUSED_HOSTS: &[&str] = &["api-testnet.bybit.com", "api-demo.bybit.com"];

/// Read-only v5 paths allowed for book `bybit-com-spot` (B6 row 2).
pub fn bybit_path_allowed(path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    p == "/v5/execution/list"
        || p == "/v5/account/wallet-balance"
        || p == "/v5/market/instruments-info"
        || p == "/v5/market/kline"
        || p == "/v5/market/tickers"
        || p == "/v5/market/time"
        || p == "/v5/user/query-api"
}

pub fn bybit_host_refused(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    REFUSED_HOSTS.contains(&h.as_str())
}

/// Official GET sign string: `timestamp + api_key + recv_window + queryString` (no separators).
pub fn bybit_sign_payload(timestamp: &str, api_key: &str, recv_window: &str, query_string: &str) -> String {
    format!("{timestamp}{api_key}{recv_window}{query_string}")
}

pub fn bybit_signature(api_secret: &str, payload: &str) -> String {
    sign_query(api_secret, payload)
}

/// Attach Bybit v5 private GET auth (headers only; query unchanged).
pub fn prepare_bybit_signed_get(
    host: &str,
    path: &str,
    query: &[(String, String)],
    api_key: &str,
    api_secret: &str,
    now_unix_ms: i64,
) -> PreparedHttpRequest {
    let host = host.trim().to_ascii_lowercase();
    assert_eq!(host, BYBIT_API_HOST, "bybit signing refused on {host}");
    let timestamp = now_unix_ms.to_string();
    let canonical = query
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let payload = bybit_sign_payload(&timestamp, api_key, BYBIT_RECV_WINDOW, &canonical);
    let signature = bybit_signature(api_secret, &payload);
    let url = if canonical.is_empty() {
        format!("https://{host}{path}")
    } else {
        format!("https://{host}{path}?{canonical}")
    };
    PreparedHttpRequest {
        method: "GET".to_string(),
        url,
        headers: vec![
            ("X-BAPI-API-KEY".to_string(), api_key.to_string()),
            ("X-BAPI-TIMESTAMP".to_string(), timestamp),
            ("X-BAPI-RECV-WINDOW".to_string(), BYBIT_RECV_WINDOW.to_string()),
            ("X-BAPI-SIGN".to_string(), signature),
        ],
        body: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_payload_matches_official_concat_rule() {
        let payload = bybit_sign_payload(
            "1658384314791",
            "XXXXXXXXXX",
            "5000",
            "category=option&symbol=BTC-29JUL22-25000-C",
        );
        assert_eq!(
            payload,
            "1658384314791XXXXXXXXXX5000category=option&symbol=BTC-29JUL22-25000-C"
        );
    }

    #[test]
    fn refused_hosts_include_testnet_and_demo() {
        assert!(bybit_host_refused("api-testnet.bybit.com"));
        assert!(bybit_host_refused("api-demo.bybit.com"));
        assert!(!bybit_host_refused("api.bybit.com"));
    }
}

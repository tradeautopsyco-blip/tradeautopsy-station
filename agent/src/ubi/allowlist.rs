//! Host allowlist for `broker_http_call` targets (R6 / R8).
//! Mirrored by Kill DNS (`dns_block::hosts_for_broker`) — a test asserts every host
//! here is sinkholed by an L3 block.

/// Broker API hosts the Enforcer may contact on behalf of a component.
pub const ALLOWED_BROKER_HOSTS: &[&str] = &[
    // binance_com (never binance.us — B6 refuse list)
    "api.binance.com",
    "eapi.binance.com",
    "fapi.binance.com",
    "dapi.binance.com",
    // kotak_neo — trading base comes from login validate `baseUrl` (SDK hosts).
    "cis.kotaksecurities.com",
    "neo.kotaksecurities.com",
    "mis.kotaksecurities.com",
    "gw-napi.kotaksecurities.com",
    "mnapi.kotaksecurities.com",
    "cnapi.kotaksecurities.com",
    "napi.kotaksecurities.com",
    // v2 data centers: Kotak FAQ “which API version” (fetched 2026-08-27) —
    // validate `baseUrl` is one of e21 / e22 / e41 / e43. Also SDK `urls.py`
    // `ORDER_FEED_URL_E21`…`E43`. Cash CSV host `lapi` is HTTP-only (file-paths
    // sample); mlhsm stays refused.
    "e21.kotaksecurities.com",
    "e22.kotaksecurities.com",
    "e41.kotaksecurities.com",
    "e43.kotaksecurities.com",
    "lapi.kotaksecurities.com",
    // zerodha_kite — Kite Connect v3 REST (B6 row 22; login host is browser-only).
    "api.kite.trade",
];

pub const ZERODHA_KITE_BOOK_ID: &str = "zerodha-nse-bse-cash";
pub const KITE_API_HOST: &str = "api.kite.trade";

/// Read-only Kite REST path prefixes allowed for book `zerodha-nse-bse-cash`.
pub fn zerodha_kite_path_allowed(path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() || p.contains("/gtt") {
        return false;
    }
    p.starts_with("/session")
        || p.starts_with("/user")
        || p == "/orders"
        || p.starts_with("/orders/")
        || p == "/trades"
        || p.starts_with("/portfolio")
        || p == "/instruments"
        || p.starts_with("/instruments/")
        || p.starts_with("/quote")
}

/// Refuse execution surfaces (orders write verbs, GTT).
pub fn zerodha_kite_path_refused(method: &str, path_norm: &str) -> bool {
    zerodha_kite_path_refused_impl(method, path_norm)
}

fn zerodha_kite_path_refused_impl(method: &str, path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.contains("/gtt") {
        return true;
    }
    let upper = method.to_ascii_uppercase();
    matches!(upper.as_str(), "POST" | "PUT" | "DELETE" | "PATCH")
        && (p == "/orders" || p.starts_with("/orders/"))
}

pub fn host_allowed(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    ALLOWED_BROKER_HOSTS
        .iter()
        .any(|allowed| normalized == *allowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_includes_binance_com_and_kotak() {
        assert!(host_allowed("api.binance.com"));
        assert!(host_allowed("eapi.binance.com"));
        assert!(host_allowed("fapi.binance.com"));
        assert!(host_allowed("dapi.binance.com"));
        assert!(host_allowed("API.Binance.COM"));
        assert!(host_allowed("cis.kotaksecurities.com"));
        assert!(host_allowed("e21.kotaksecurities.com"));
        assert!(host_allowed("e22.kotaksecurities.com"));
        assert!(host_allowed("e41.kotaksecurities.com"));
        assert!(host_allowed("e43.kotaksecurities.com"));
        assert!(host_allowed("lapi.kotaksecurities.com"));
        assert!(host_allowed("api.kite.trade"));
        assert!(!host_allowed("evil.example.com"));
        assert!(!host_allowed("mlhsm.kotaksecurities.com"));
        assert!(!host_allowed("api.binance.us"));
    }

    #[test]
    fn zerodha_kite_read_paths_allowed_execution_refused() {
        assert!(zerodha_kite_path_allowed("/orders"));
        assert!(zerodha_kite_path_allowed("/trades"));
        assert!(zerodha_kite_path_allowed("/portfolio/holdings"));
        assert!(zerodha_kite_path_allowed("/quote/ltp"));
        assert!(zerodha_kite_path_allowed("/instruments/NSE"));
        assert!(!zerodha_kite_path_allowed("/gtt/triggers"));
        assert!(zerodha_kite_path_refused("POST", "/orders"));
        assert!(zerodha_kite_path_refused("PUT", "/orders/123"));
        assert!(zerodha_kite_path_refused("DELETE", "/orders/123"));
        assert!(!zerodha_kite_path_refused("GET", "/orders"));
    }
}

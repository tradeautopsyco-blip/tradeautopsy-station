//! Host allowlist for `broker_http_call` targets (R6 / R8).
//! Mirrored by Kill DNS (`dns_block::hosts_for_broker`) — a test asserts every host
//! here is sinkholed by an L3 block.

/// Broker API hosts the Enforcer may contact on behalf of a component.
pub const ALLOWED_BROKER_HOSTS: &[&str] = &[
    // binance_com (never binance.us — B6 refuse list)
    "api.binance.com",
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
];

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
        assert!(host_allowed("API.Binance.COM"));
        assert!(host_allowed("cis.kotaksecurities.com"));
        assert!(host_allowed("e21.kotaksecurities.com"));
        assert!(host_allowed("e22.kotaksecurities.com"));
        assert!(host_allowed("e41.kotaksecurities.com"));
        assert!(host_allowed("e43.kotaksecurities.com"));
        assert!(host_allowed("lapi.kotaksecurities.com"));
        assert!(!host_allowed("evil.example.com"));
        assert!(!host_allowed("mlhsm.kotaksecurities.com"));
        assert!(!host_allowed("api.binance.us"));
    }
}

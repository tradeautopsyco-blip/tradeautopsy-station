//! Host allowlist for `broker_http_call` targets (R6 / R8).
//! Align with Kill DNS host sets; Binance hosts are listed here even though
//! `dns_block.rs` still lacks them (known gap — track for Kill dogfood).

/// Broker API hosts the Enforcer may contact on behalf of a component.
pub const ALLOWED_BROKER_HOSTS: &[&str] = &[
    // binance_com (Kill DNS gap in dns_block.rs — R8)
    "api.binance.com",
    // kotak_neo (already in dns_block)
    "cis.kotaksecurities.com",
    "neo.kotaksecurities.com",
    "mis.kotaksecurities.com",
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
        assert!(!host_allowed("evil.example.com"));
        assert!(!host_allowed("api.binance.us"));
    }
}

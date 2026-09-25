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
    // upstox — REST + BOD instrument gzips (B6 row 22; api-hft refused).
    "api.upstox.com",
    "assets.upstox.com",
    // fyers — v3 REST reads + public sym master (B6 row 22; siblings refused in Kill DNS).
    "api-t1.fyers.in",
    "public.fyers.in",
    // groww — REST v1 reads + mint host (B6 row 22; assets CSV host separate).
    "api.groww.in",
    "growwapi-assets.groww.in",
    // dhan — REST v2 + consent auth (B6 row 22; WS/postback refused as sync).
    "api.dhan.co",
    "auth.dhan.co",
    "api-feed.dhan.co",
    "api-order-update.dhan.co",
    // bybit — v5 REST prod only (B6 row 22; testnet/demo refused).
    "api.bybit.com",
    // okx_com — global REST only (B6 row 0; us/eea refused).
    "www.okx.com",
    // coinbase_advanced — Advanced Trade REST (B6 row 22; sandbox refused).
    "api.coinbase.com",
    // kraken — spot REST prod only (B6 row 22; futures hosts refused on kraken-com-spot).
    "api.kraken.com",
];

pub const ZERODHA_KITE_BOOK_ID: &str = "zerodha-nse-bse-cash";
pub const ZERODHA_KITE_NFO_BOOK_ID: &str = "zerodha-nse-nfo";
pub const KITE_API_HOST: &str = "api.kite.trade";

pub const UPSTOX_BOOK_ID: &str = "upstox-nse-bse-cash";
pub const UPSTOX_NFO_BOOK_ID: &str = "upstox-nse-nfo";
pub const UPSTOX_API_HOST: &str = "api.upstox.com";
pub const UPSTOX_ASSETS_HOST: &str = "assets.upstox.com";
pub const UPSTOX_HFT_HOST: &str = "api-hft.upstox.com";

pub const FYERS_BOOK_ID: &str = "fyers-nse-bse-cash";
pub const FYERS_NFO_BOOK_ID: &str = "fyers-nse-nfo";
pub const FYERS_API_HOST: &str = "api-t1.fyers.in";
pub const FYERS_PUBLIC_HOST: &str = "public.fyers.in";

/// Stem from `/market-quote/instruments/exchange/{stem}.json.gz` on Upstox assets host.
pub fn upstox_exchange_bod_stem(path_norm: &str) -> Option<String> {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    let prefix = "/market-quote/instruments/exchange/";
    if !p.starts_with(prefix) || !p.ends_with(".json.gz") {
        return None;
    }
    let stem = &p[prefix.len()..p.len() - 8];
    if stem.is_empty() {
        return None;
    }
    Some(stem.to_string())
}

pub fn is_upstox_complete_bod_path(host: &str, path_norm: &str) -> bool {
    host.trim().trim_end_matches('.').to_ascii_lowercase() == UPSTOX_ASSETS_HOST
        && upstox_exchange_bod_stem(path_norm).as_deref() == Some("complete")
}

pub fn is_upstox_cash_bod_path(host: &str, path_norm: &str) -> bool {
    if host.trim().trim_end_matches('.').to_ascii_lowercase() != UPSTOX_ASSETS_HOST {
        return false;
    }
    matches!(
        upstox_exchange_bod_stem(path_norm).as_deref(),
        Some("nse") | Some("bse")
    )
}

pub fn is_upstox_nfo_bod_path(host: &str, path_norm: &str) -> bool {
    host.trim().trim_end_matches('.').to_ascii_lowercase() == UPSTOX_ASSETS_HOST
        && upstox_exchange_bod_stem(path_norm).as_deref() == Some("nfo")
}

pub fn fyers_sym_details_stem(path_norm: &str) -> Option<String> {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    let prefix = "/sym_details/";
    if !p.starts_with(prefix) || !p.ends_with(".csv") {
        return None;
    }
    let stem = &p[prefix.len()..p.len() - 4];
    if stem.is_empty() {
        return None;
    }
    Some(stem.to_string())
}

pub fn is_fyers_cash_sym_path(host: &str, path_norm: &str) -> bool {
    host.trim().trim_end_matches('.').to_ascii_lowercase() == FYERS_PUBLIC_HOST
        && matches!(
            fyers_sym_details_stem(path_norm).as_deref(),
            Some("nse_cm") | Some("bse_cm")
        )
}

pub fn is_fyers_nfo_sym_path(host: &str, path_norm: &str) -> bool {
    host.trim().trim_end_matches('.').to_ascii_lowercase() == FYERS_PUBLIC_HOST
        && fyers_sym_details_stem(path_norm).as_deref() == Some("nse_fo")
}

pub const GROWW_BOOK_ID: &str = "groww-nse-bse-cash";
pub const GROWW_NFO_BOOK_ID: &str = "groww-nse-nfo";
pub const GROWW_API_HOST: &str = "api.groww.in";
pub const GROWW_ASSETS_HOST: &str = "growwapi-assets.groww.in";
/// Mandatory on every Groww REST call (B6 row 2 / D4).
pub const GROWW_API_VERSION_HEADER: &str = "1.0";

pub const OKX_COM_SPOT_BOOK_ID: &str = "okx-com-spot";
pub const OKX_API_HOST: &str = "www.okx.com";

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

/// Read-only Upstox REST paths allowed for book `upstox-nse-bse-cash` (B6 row 2).
pub fn upstox_path_allowed(host: &str, path_norm: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return false;
    }
    if host == UPSTOX_ASSETS_HOST {
        return p.starts_with("/market-quote/instruments/");
    }
    if host != UPSTOX_API_HOST {
        return false;
    }
    p == "/v2/order/trades/get-trades-for-day"
        || p == "/v2/user/profile"
        || p == "/v2/portfolio/long-term-holdings"
        || p == "/v2/portfolio/short-term-positions"
        || p.starts_with("/v2/order/trades")
        || p == "/v2/order/retrieve-all"
        || p.starts_with("/v2/order/details")
        || p.starts_with("/v2/order/history")
        || p.starts_with("/v2/charges/historical-trades")
        || p.starts_with("/v2/instruments/search")
}

/// Refuse HFT order execution surfaces (B6 row 12).
pub fn upstox_path_refused(host: &str, method: &str, path_norm: &str) -> bool {
    upstox_path_refused_impl(host, method, path_norm)
}

fn upstox_path_refused_impl(host: &str, method: &str, path_norm: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if host == UPSTOX_HFT_HOST {
        return true;
    }
    if host == UPSTOX_API_HOST && p.contains("/gtt") {
        return true;
    }
    let upper = method.to_ascii_uppercase();
    matches!(upper.as_str(), "POST" | "PUT" | "DELETE" | "PATCH")
        && host == UPSTOX_API_HOST
        && (p.starts_with("/v2/order") || p.starts_with("/v3/order"))
}

/// Read-only Fyers v3 paths for book `fyers-nse-bse-cash` (B6 row 2).
pub fn fyers_path_allowed(path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return false;
    }
    p == "/api/v3/tradebook" || p == "/api/v3/profile"
}

/// Refuse Fyers order mutation surfaces.
pub fn fyers_path_refused(method: &str, path_norm: &str) -> bool {
    fyers_path_refused_impl(method, path_norm)
}

fn fyers_path_refused_impl(method: &str, path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    let upper = method.to_ascii_uppercase();
    matches!(upper.as_str(), "POST" | "PUT" | "DELETE" | "PATCH")
        && (p.starts_with("/api/v3/orders") || p.starts_with("/api/v3/order"))
}

/// Read-only Groww v1 paths for book `groww-nse-bse-cash` (B6 row 2 / lock).
pub fn groww_path_allowed(host: &str, method: &str, path_norm: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if method.to_ascii_uppercase() != "GET" {
        return false;
    }
    if host == GROWW_ASSETS_HOST {
        return p == "/instruments/instrument.csv";
    }
    if host != GROWW_API_HOST {
        return false;
    }
    p == "/v1/order/list"
        || p.starts_with("/v1/order/trades/")
        || p.starts_with("/v1/order/status/")
        || p.starts_with("/v1/order/detail/")
        || p.starts_with("/v1/holdings/")
        || p.starts_with("/v1/positions/")
        || p.starts_with("/v1/margins/")
        || p.starts_with("/v1/user/")
}

/// Refuse Groww order mutation surfaces (create/modify/cancel).
pub fn groww_path_refused(method: &str, path_norm: &str) -> bool {
    groww_path_refused_impl(method, path_norm)
}

fn groww_path_refused_impl(method: &str, path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    let upper = method.to_ascii_uppercase();
    matches!(upper.as_str(), "POST" | "PUT" | "DELETE" | "PATCH")
        && p.starts_with("/v1/order")
        && p != "/v1/order/list"
}

/// Read-only OKX v5 paths for book `okx-com-spot` (B6 row 2).
pub fn okx_path_allowed(method: &str, path_norm: &str) -> bool {
    if method.to_ascii_uppercase() != "GET" {
        return false;
    }
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    if p.is_empty() {
        return false;
    }
    p == "/api/v5/trade/fills"
        || p == "/api/v5/account/balance"
        || p == "/api/v5/public/instruments"
        || p == "/api/v5/public/time"
        || p == "/api/v5/market/candles"
        || p == "/api/v5/market/tickers"
}

/// Refuse sandbox header path and trading mutations on the global book.
pub fn okx_path_refused(method: &str, path_norm: &str) -> bool {
    okx_path_refused_impl(method, path_norm)
}

fn okx_path_refused_impl(method: &str, path_norm: &str) -> bool {
    let p = path_norm.trim().trim_end_matches('/').to_ascii_lowercase();
    let upper = method.to_ascii_uppercase();
    matches!(upper.as_str(), "POST" | "PUT" | "DELETE" | "PATCH")
        && p.starts_with("/api/v5/trade")
        && p != "/api/v5/trade/fills"
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
        assert!(host_allowed("api.upstox.com"));
        assert!(host_allowed("assets.upstox.com"));
        assert!(host_allowed("api-t1.fyers.in"));
        assert!(host_allowed("api.groww.in"));
        assert!(host_allowed("growwapi-assets.groww.in"));
        assert!(host_allowed("www.okx.com"));
        assert!(!host_allowed("us.okx.com"));
        assert!(!host_allowed("eea.okx.com"));
        assert!(!host_allowed("api-hft.upstox.com"));
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

    #[test]
    fn upstox_read_paths_allowed_hft_and_order_posts_refused() {
        assert!(upstox_path_allowed(
            UPSTOX_API_HOST,
            "/v2/order/trades/get-trades-for-day"
        ));
        assert!(upstox_path_allowed(UPSTOX_API_HOST, "/v2/user/profile"));
        assert!(upstox_path_allowed(
            UPSTOX_API_HOST,
            "/v2/portfolio/long-term-holdings"
        ));
        assert!(upstox_path_allowed(
            UPSTOX_ASSETS_HOST,
            "/market-quote/instruments/exchange/NSE.json.gz"
        ));
        assert!(!upstox_path_allowed(UPSTOX_API_HOST, "/v2/order/place"));
        assert!(upstox_path_refused(
            UPSTOX_HFT_HOST,
            "POST",
            "/v2/order/place"
        ));
        assert!(upstox_path_refused(
            UPSTOX_API_HOST,
            "POST",
            "/v2/order/place"
        ));
        assert!(!upstox_path_refused(
            UPSTOX_API_HOST,
            "GET",
            "/v2/order/trades/get-trades-for-day"
        ));
    }

    #[test]
    fn fyers_read_paths_allowed_order_posts_refused() {
        assert!(fyers_path_allowed("/api/v3/tradebook"));
        assert!(fyers_path_allowed("/api/v3/profile"));
        assert!(!fyers_path_allowed("/api/v3/orders"));
        assert!(fyers_path_refused("POST", "/api/v3/orders"));
        assert!(fyers_path_refused("POST", "/api/v3/order"));
        assert!(!fyers_path_refused("GET", "/api/v3/tradebook"));
    }

    #[test]
    fn groww_read_paths_allowed_order_posts_refused() {
        assert!(host_allowed("api.coinbase.com"));
        assert!(!host_allowed("api-sandbox.coinbase.com"));
        assert!(crate::ubi::coinbase_session::coinbase_path_allowed(
            "GET",
            crate::ubi::coinbase_session::COINBASE_FILLS_PATH
        ));
        assert!(!crate::ubi::coinbase_session::coinbase_path_allowed(
            "POST",
            "/api/v3/brokerage/orders"
        ));
        assert!(groww_path_allowed(GROWW_API_HOST, "GET", "/v1/order/list"));
        assert!(groww_path_allowed(
            GROWW_API_HOST,
            "GET",
            "/v1/order/trades/GWK01"
        ));
        assert!(groww_path_allowed(
            GROWW_ASSETS_HOST,
            "GET",
            "/instruments/instrument.csv"
        ));
        assert!(!groww_path_allowed(GROWW_API_HOST, "GET", "/v1/order/place"));
        assert!(groww_path_refused("POST", "/v1/order/create"));
        assert!(!groww_path_refused("GET", "/v1/order/list"));
    }
}

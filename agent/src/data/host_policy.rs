//! Host-mediated broker HTTP policy (R0). Public calls never receive private credentials.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    Public,
    PrivateRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRefuse {
    HostNotAllowed,
    MutationForbidden,
    PrivateCredentialOnPublicCall,
    PathNotAllowlisted,
    ExecutionEndpoint,
}

impl std::fmt::Display for HostRefuse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl HostRefuse {
    pub fn as_str(self) -> &'static str {
        match self {
            HostRefuse::HostNotAllowed => "host_blocked",
            HostRefuse::MutationForbidden => "mutation_forbidden",
            HostRefuse::PrivateCredentialOnPublicCall => "private_credential_on_public_call",
            HostRefuse::PathNotAllowlisted => "path_not_allowlisted",
            HostRefuse::ExecutionEndpoint => "execution_endpoint",
        }
    }
}

pub const R0_ALLOWED_HOSTS: &[&str] = &[
    "api.binance.com",
    "cis.kotaksecurities.com",
    "neo.kotaksecurities.com",
    "mis.kotaksecurities.com",
    "gw-napi.kotaksecurities.com",
    "mnapi.kotaksecurities.com",
    "cnapi.kotaksecurities.com",
    "napi.kotaksecurities.com",
    // v2 data centers (Kotak FAQ + SDK ORDER_FEED_URL_E21…E43). Keep aligned
    // with `ALLOWED_BROKER_HOSTS` / Kill DNS.
    "e21.kotaksecurities.com",
    "e22.kotaksecurities.com",
    "e41.kotaksecurities.com",
    "e43.kotaksecurities.com",
    // Cash scrip CSV host from official file-paths `filesPaths` sample (guide +
    // REST.md). F&O CSVs and `/quick/user/trades` stay refused on this host.
    "lapi.kotaksecurities.com",
];

pub fn host_allowed(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    R0_ALLOWED_HOSTS
        .iter()
        .any(|allowed| normalized == *allowed)
}

pub fn is_mutation(method: &str, path: &str) -> bool {
    let upper = method.to_ascii_uppercase();
    let lower = path.to_ascii_lowercase();
    if lower.contains("withdraw")
        || lower.contains("transfer")
        || lower.contains("apikey")
        || lower.contains("placeorder")
        || lower.contains("modifyorder")
        || lower.contains("cancelorder")
        || lower.contains("closeposition")
    {
        return true;
    }
    matches!(upper.as_str(), "POST" | "PUT" | "PATCH" | "DELETE") && lower.contains("order")
}

fn path_allowlisted(capability_id: &str, method: &str, path: &str, auth_mode: AuthMode) -> bool {
    let method = method.to_ascii_uppercase();
    let path = path.trim();
    match (capability_id, method.as_str(), auth_mode) {
        ("quote", "GET", AuthMode::Public) if path == "/api/v3/ticker/price" => true,
        ("ohlcv", "GET", AuthMode::Public) if path == "/api/v3/klines" => true,
        ("order_book", "GET", AuthMode::Public) if path == "/api/v3/depth" => true,
        ("instrument_master", "GET", AuthMode::Public)
            if path == "/api/v3/exchangeInfo" || is_kotak_cash_scrip_csv_path(path) =>
        {
            true
        }
        ("fills", "GET", AuthMode::PrivateRead)
            if path == "/api/v3/myTrades" || path.ends_with("/quick/user/trades") =>
        {
            true
        }
        ("funds", "GET", AuthMode::PrivateRead) if path == "/api/v3/account" => true,
        ("instrument_master", "GET", AuthMode::PrivateRead)
            if path.ends_with("/script-details/1.0/masterscrip/file-paths") =>
        {
            true
        }
        ("quote", "GET", AuthMode::PrivateRead) if is_kotak_latest_quote_path(path) => true,
        ("order_book", "GET", AuthMode::PrivateRead) if is_kotak_depth_path(path) => true,
        _ => false,
    }
}

/// Map a host-mediated request to a capability. Mutations never infer as reads.
pub fn infer_capability(method: &str, path: &str) -> Result<(&'static str, AuthMode), HostRefuse> {
    if is_mutation(method, path) {
        return Err(HostRefuse::MutationForbidden);
    }
    let method = method.to_ascii_uppercase();
    let path = path.trim();
    match (method.as_str(), path) {
        ("GET", "/api/v3/ticker/price") => Ok(("quote", AuthMode::Public)),
        ("GET", "/api/v3/klines") => Ok(("ohlcv", AuthMode::Public)),
        ("GET", "/api/v3/depth") => Ok(("order_book", AuthMode::Public)),
        ("GET", "/api/v3/exchangeInfo") => Ok(("instrument_master", AuthMode::Public)),
        ("GET", "/api/v3/myTrades") => Ok(("fills", AuthMode::PrivateRead)),
        ("GET", "/api/v3/account") => Ok(("funds", AuthMode::PrivateRead)),
        ("GET", p) if p.ends_with("/quick/user/trades") => Ok(("fills", AuthMode::PrivateRead)),
        ("GET", p) if p.ends_with("/script-details/1.0/masterscrip/file-paths") => {
            Ok(("instrument_master", AuthMode::PrivateRead))
        }
        ("GET", p) if is_kotak_cash_scrip_csv_path(p) => {
            Ok(("instrument_master", AuthMode::Public))
        }
        ("GET", p) if is_kotak_depth_path(p) => Ok(("order_book", AuthMode::PrivateRead)),
        ("GET", p) if is_kotak_latest_quote_path(p) => Ok(("quote", AuthMode::PrivateRead)),
        _ => Err(HostRefuse::PathNotAllowlisted),
    }
}

pub fn authorize_inferred_call(
    host: &str,
    method: &str,
    path: &str,
    attach_private: bool,
) -> Result<(&'static str, AuthMode), HostRefuse> {
    let (capability_id, auth_mode) = infer_capability(method, path)?;
    authorize_host_call(host, method, path, capability_id, auth_mode, attach_private)?;
    Ok((capability_id, auth_mode))
}

fn is_kotak_depth_path(path: &str) -> bool {
    super::kotak_quotes::is_kotak_depth_path(path)
}

fn is_kotak_latest_quote_path(path: &str) -> bool {
    super::kotak_quotes::is_kotak_latest_quote_path(path)
}

/// Official file-paths sample: `…/wso2-scripmaster/v1/prod/…/transformed/{nse,bse}_cm.csv`.
/// Live cash files: `…/transformed-v1/{nse,bse}_cm-v1.csv` (OpenAlgo unsigned lapi GET).
/// F&O (`_fo.csv`, `-fo.csv`, filenames containing `_fo` / `-fo`) stay refused.
/// Query strings are stripped and a trailing `/` is trimmed before the filename check.
pub fn is_kotak_cash_scrip_csv_path(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    let lower = lower.split('?').next().unwrap_or(&lower);
    let lower = lower.trim_end_matches('/');
    if !lower.contains("/wso2-scripmaster/") {
        return false;
    }
    if lower.contains("_fo.csv") || lower.contains("-fo.csv") {
        return false;
    }
    let filename = lower.rsplit('/').next().unwrap_or(lower);
    if filename.contains("_fo") || filename.contains("-fo") {
        return false;
    }
    is_kotak_cash_csv_filename(filename)
}

fn is_kotak_cash_csv_filename(filename: &str) -> bool {
    for stem in ["nse_cm", "bse_cm"] {
        let Some(rest) = filename.strip_prefix(stem) else {
            continue;
        };
        // SDK sample: `nse_cm.csv` / `bse_cm.csv`.
        // Live / optional: `nse_cm-*.csv`, `nse_cm_*.csv` (and bse).
        let versioned = (rest.starts_with('-') || rest.starts_with('_')) && rest.ends_with(".csv");
        if rest == ".csv" || versioned {
            return true;
        }
    }
    false
}

pub fn authorize_host_call(
    host: &str,
    method: &str,
    path: &str,
    capability_id: &str,
    auth_mode: AuthMode,
    attach_private: bool,
) -> Result<(), HostRefuse> {
    if !host_allowed(host) {
        return Err(HostRefuse::HostNotAllowed);
    }
    let host_norm = host.trim().trim_end_matches('.').to_ascii_lowercase();
    // Public klines / depth are COM-only. Not api.binance.us, not data-api.binance.vision.
    if (path.trim() == "/api/v3/klines" || path.trim() == "/api/v3/depth")
        && host_norm != "api.binance.com"
    {
        return Err(HostRefuse::HostNotAllowed);
    }
    if is_kotak_cash_scrip_csv_path(path) && host_norm != "lapi.kotaksecurities.com" {
        return Err(HostRefuse::HostNotAllowed);
    }
    if host_norm == "lapi.kotaksecurities.com"
        && (path.ends_with("/quick/user/trades")
            || is_kotak_latest_quote_path(path)
            || is_kotak_depth_path(path))
    {
        return Err(HostRefuse::PathNotAllowlisted);
    }
    if is_mutation(method, path) {
        return Err(HostRefuse::MutationForbidden);
    }
    if auth_mode == AuthMode::Public && attach_private {
        return Err(HostRefuse::PrivateCredentialOnPublicCall);
    }
    if !path_allowlisted(capability_id, method, path, auth_mode) {
        return Err(HostRefuse::PathNotAllowlisted);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_quote_cannot_attach_private_credential() {
        let err = authorize_host_call(
            "api.binance.com",
            "GET",
            "/api/v3/ticker/price",
            "quote",
            AuthMode::Public,
            true,
        )
        .unwrap_err();
        assert_eq!(err, HostRefuse::PrivateCredentialOnPublicCall);
    }

    #[test]
    fn place_and_withdraw_are_refused() {
        assert_eq!(
            authorize_host_call(
                "api.binance.com",
                "POST",
                "/api/v3/order",
                "quote",
                AuthMode::PrivateRead,
                true,
            )
            .unwrap_err(),
            HostRefuse::MutationForbidden
        );
        assert_eq!(
            authorize_host_call(
                "api.binance.com",
                "GET",
                "/sapi/v1/capital/withdraw/history",
                "funds",
                AuthMode::PrivateRead,
                true,
            )
            .unwrap_err(),
            HostRefuse::MutationForbidden
        );
    }

    #[test]
    fn private_fills_path_is_allowlisted() {
        authorize_host_call(
            "api.binance.com",
            "GET",
            "/api/v3/myTrades",
            "fills",
            AuthMode::PrivateRead,
            true,
        )
        .expect("signed fills poll remains allowlisted");
    }

    #[test]
    fn infer_refuses_mutations_and_allows_signed_reads() {
        assert_eq!(
            infer_capability("POST", "/api/v3/order").unwrap_err(),
            HostRefuse::MutationForbidden
        );
        let (cap, mode) = infer_capability("GET", "/api/v3/ticker/price").unwrap();
        assert_eq!(cap, "quote");
        assert_eq!(mode, AuthMode::Public);
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/ticker/price", true)
            .expect_err("public quote must not attach private credentials");
        authorize_inferred_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/quick/user/trades",
            true,
        )
        .expect("signed Kotak trade book remains allowlisted");
        authorize_inferred_call("e21.kotaksecurities.com", "GET", "/quick/user/trades", true)
            .expect("v2 data-center trade book is allowlisted");
    }

    #[test]
    fn exchange_info_is_unsigned_public_reference() {
        let (cap, mode) = infer_capability("GET", "/api/v3/exchangeInfo").unwrap();
        assert_eq!(cap, "instrument_master");
        assert_eq!(mode, AuthMode::Public);
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/exchangeInfo", false)
            .expect("unsigned exchangeInfo is allowlisted");
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/exchangeInfo", true)
            .expect_err("public exchangeInfo must not attach private credentials");
    }

    #[test]
    fn kotak_scrip_master_is_private_read() {
        let path = "/script-details/1.0/masterscrip/file-paths";
        let (cap, mode) = infer_capability("GET", path).unwrap();
        assert_eq!(cap, "instrument_master");
        assert_eq!(mode, AuthMode::PrivateRead);
        authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            path,
            "instrument_master",
            AuthMode::PrivateRead,
            true,
        )
        .expect("signed scrip master file-paths remains allowlisted");
        authorize_inferred_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/trading/script-details/1.0/masterscrip/file-paths",
            true,
        )
        .expect("prefixed masterscrip path infers PrivateRead");
    }

    #[test]
    fn kotak_scrip_master_public_with_private_attach_is_refused() {
        let err = authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/script-details/1.0/masterscrip/file-paths",
            "instrument_master",
            AuthMode::Public,
            true,
        )
        .unwrap_err();
        assert_eq!(err, HostRefuse::PrivateCredentialOnPublicCall);
    }

    #[test]
    fn klines_are_unsigned_public_ohlcv_on_com_only() {
        let (cap, mode) = infer_capability("GET", "/api/v3/klines").unwrap();
        assert_eq!(cap, "ohlcv");
        assert_eq!(mode, AuthMode::Public);
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/klines", false)
            .expect("unsigned COM klines are allowlisted");
        assert_eq!(
            authorize_inferred_call("api.binance.com", "GET", "/api/v3/klines", true).unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_host_call(
                "api.binance.com",
                "GET",
                "/api/v3/klines",
                "ohlcv",
                AuthMode::Public,
                true,
            )
            .unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_inferred_call(
                "gw-napi.kotaksecurities.com",
                "GET",
                "/api/v3/klines",
                false
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_host_call(
                "gw-napi.kotaksecurities.com",
                "GET",
                "/api/v3/klines",
                "quote",
                AuthMode::PrivateRead,
                true,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert!(!host_allowed("api.binance.us"));
        assert!(!host_allowed("data-api.binance.vision"));
        assert_eq!(
            authorize_inferred_call("api.binance.us", "GET", "/api/v3/klines", false).unwrap_err(),
            HostRefuse::HostNotAllowed
        );
    }

    #[test]
    fn kotak_place_modify_cancel_are_refused() {
        for path in [
            "/quick/order/place",
            "/quick/order/modify",
            "/quick/order/cancel",
        ] {
            assert_eq!(
                infer_capability("POST", path).unwrap_err(),
                HostRefuse::MutationForbidden
            );
        }
    }

    #[test]
    fn kotak_cash_scrip_csv_is_unsigned_public_on_lapi_only() {
        let path = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv";
        let (cap, mode) = infer_capability("GET", path).unwrap();
        assert_eq!(cap, "instrument_master");
        assert_eq!(mode, AuthMode::Public);
        authorize_inferred_call("lapi.kotaksecurities.com", "GET", path, false)
            .expect("unsigned cash CSV GET is allowlisted");
        let queried = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv?token=1";
        let (q_cap, q_mode) = infer_capability("GET", queried).unwrap();
        assert_eq!(q_cap, "instrument_master");
        assert_eq!(q_mode, AuthMode::Public);
        authorize_inferred_call("lapi.kotaksecurities.com", "GET", queried, false)
            .expect("cash CSV path with query string is allowlisted");
        let fo_q = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv?x=1";
        assert_eq!(
            infer_capability("GET", fo_q).unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_inferred_call("lapi.kotaksecurities.com", "GET", path, true).unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_inferred_call("gw-napi.kotaksecurities.com", "GET", path, false).unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        let fo = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
        assert_eq!(
            infer_capability("GET", fo).unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        let fo_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_fo-v1.csv";
        assert_eq!(
            infer_capability("GET", fo_v1).unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
    }

    #[test]
    fn kotak_live_v1_cash_scrip_csv_is_unsigned_public_on_lapi() {
        let nse_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_cm-v1.csv";
        let bse_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/bse_cm-v1.csv";
        let sample = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv";
        assert!(is_kotak_cash_scrip_csv_path(nse_v1));
        assert!(is_kotak_cash_scrip_csv_path(bse_v1));
        assert!(is_kotak_cash_scrip_csv_path(sample));
        assert!(is_kotak_cash_scrip_csv_path(
            "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_cm-v1.csv/"
        ));
        for path in [nse_v1, bse_v1, sample] {
            let (cap, mode) = infer_capability("GET", path).unwrap();
            assert_eq!(cap, "instrument_master");
            assert_eq!(mode, AuthMode::Public);
            authorize_inferred_call("lapi.kotaksecurities.com", "GET", path, false)
                .expect("unsigned cash CSV GET is allowlisted");
        }
        let queried = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_cm-v1.csv?token=1";
        assert!(is_kotak_cash_scrip_csv_path(queried));
        let (q_cap, q_mode) = infer_capability("GET", queried).unwrap();
        assert_eq!(q_cap, "instrument_master");
        assert_eq!(q_mode, AuthMode::Public);
        authorize_inferred_call("lapi.kotaksecurities.com", "GET", queried, false)
            .expect("live v1 cash CSV path with query string is allowlisted");
        for fo in [
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv",
            "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_fo-v1.csv",
            "/wso2-scripmaster/v1/prod/2026-08-27/transformed/cde_fo.csv",
        ] {
            assert!(!is_kotak_cash_scrip_csv_path(fo), "{fo}");
            assert_eq!(
                infer_capability("GET", fo).unwrap_err(),
                HostRefuse::PathNotAllowlisted,
                "{fo}"
            );
        }
        assert_eq!(
            authorize_inferred_call("lapi.kotaksecurities.com", "GET", nse_v1, true).unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_inferred_call("gw-napi.kotaksecurities.com", "GET", nse_v1, false)
                .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
    }

    #[test]
    fn lapi_refuses_trades_quotes_and_mlhsm_stays_off() {
        assert!(R0_ALLOWED_HOSTS.contains(&"lapi.kotaksecurities.com"));
        assert!(!R0_ALLOWED_HOSTS.contains(&"mlhsm.kotaksecurities.com"));
        assert_eq!(
            authorize_inferred_call(
                "lapi.kotaksecurities.com",
                "GET",
                "/quick/user/trades",
                true
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert!(!host_allowed("mlhsm.kotaksecurities.com"));
        assert!(host_allowed("lapi.kotaksecurities.com"));
    }

    #[test]
    fn kotak_quotes_rest_is_private_read() {
        let path = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp";
        let (cap, mode) = infer_capability("GET", path).unwrap();
        assert_eq!(cap, "quote");
        assert_eq!(mode, AuthMode::PrivateRead);
        authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            path,
            "quote",
            AuthMode::PrivateRead,
            true,
        )
        .expect("documented quotes GET is PrivateRead allowlisted");
        authorize_inferred_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/trading/script-details/1.0/quotes/neosymbol/nse_cm|2885/ohlc",
            true,
        )
        .expect("unencoded neo_symbol + ohlc infers PrivateRead");
        authorize_inferred_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/all",
            true,
        )
        .expect("quote_type=all remains allowlisted");
    }

    #[test]
    fn kotak_quotes_public_with_private_attach_is_refused() {
        let err = authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp",
            "quote",
            AuthMode::Public,
            true,
        )
        .unwrap_err();
        assert_eq!(err, HostRefuse::PrivateCredentialOnPublicCall);
    }

    #[test]
    fn kotak_quick_quotes_and_napi_are_not_allowlisted() {
        assert_eq!(
            infer_capability("GET", "/quick/quotes").unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            infer_capability("GET", "apim/quotes/1.0/quotes/neosymbol/nse_cm%7C2885/ltp")
                .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_host_call(
                "gw-napi.kotaksecurities.com",
                "GET",
                "/quick/quotes",
                "quote",
                AuthMode::PrivateRead,
                true,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
    }

    #[test]
    fn kotak_quote_type_depth_is_order_book_not_quote() {
        let depth = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/depth";
        let ltp = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp";
        let (cap, mode) = infer_capability("GET", depth).unwrap();
        assert_eq!(cap, "order_book");
        assert_eq!(mode, AuthMode::PrivateRead);
        authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            depth,
            "order_book",
            AuthMode::PrivateRead,
            true,
        )
        .expect("quote_type=depth is PrivateRead order_book");
        authorize_host_call(
            "gw-napi.kotaksecurities.com",
            "GET",
            depth,
            "quote",
            AuthMode::PrivateRead,
            true,
        )
        .expect_err("depth must not authorize as quote");
        let (ltp_cap, ltp_mode) = infer_capability("GET", ltp).unwrap();
        assert_eq!(ltp_cap, "quote");
        assert_eq!(ltp_mode, AuthMode::PrivateRead);
        authorize_inferred_call("gw-napi.kotaksecurities.com", "GET", ltp, true)
            .expect("quote_type=ltp stays quote");
        assert_eq!(
            infer_capability(
                "GET",
                "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/market_depth"
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
    }

    #[test]
    fn binance_depth_is_unsigned_public_order_book_on_com_only() {
        let (cap, mode) = infer_capability("GET", "/api/v3/depth").unwrap();
        assert_eq!(cap, "order_book");
        assert_eq!(mode, AuthMode::Public);
        authorize_inferred_call("api.binance.com", "GET", "/api/v3/depth", false)
            .expect("unsigned COM depth is allowlisted");
        assert_eq!(
            authorize_inferred_call("api.binance.com", "GET", "/api/v3/depth", true).unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_host_call(
                "api.binance.com",
                "GET",
                "/api/v3/depth",
                "order_book",
                AuthMode::Public,
                true,
            )
            .unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_inferred_call("gw-napi.kotaksecurities.com", "GET", "/api/v3/depth", false)
                .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_inferred_call("api.binance.us", "GET", "/api/v3/depth", false).unwrap_err(),
            HostRefuse::HostNotAllowed
        );
    }
}

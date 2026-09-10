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
    // Named book `binance-com-options` (slice 1 last = REST lastPrice).
    "eapi.binance.com",
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
    // RMS read POSTs — not place/modify/cancel (PR A5 lock).
    if lower.contains("check-margin") || lower.ends_with("/limits") {
        return false;
    }
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
        ("quote", "GET", AuthMode::Public)
            if path == "/api/v3/ticker/price"
                || normalize_request_path(path) == "/eapi/v1/ticker" =>
        {
            true
        }
        ("ohlcv", "GET", AuthMode::Public)
            if path == "/api/v3/klines" || normalize_request_path(path) == "/eapi/v1/klines" =>
        {
            true
        }
        ("order_book", "GET", AuthMode::Public)
            if path == "/api/v3/depth" || normalize_request_path(path) == "/eapi/v1/depth" =>
        {
            true
        }
        // Venue-published greeks. `greeks` is already on the matrix as
        // Derived/BoundedSnapshot, so no new capability id is minted here. Station
        // copies what /eapi/v1/mark published; it never prices this book.
        // Lock: binance-com-options.md Slice 3.
        ("greeks", "GET", AuthMode::Public) if normalize_request_path(path) == "/eapi/v1/mark" => {
            true
        }
        ("instrument_master", "GET", AuthMode::Public)
            if path == "/api/v3/exchangeInfo"
                || normalize_request_path(path) == "/eapi/v1/exchangeInfo"
                || is_kotak_cash_scrip_csv_path(path)
                || is_kotak_fo_scrip_csv_path(path) =>
        {
            true
        }
        ("open_interest", "GET", AuthMode::Public)
            if normalize_request_path(path) == "/eapi/v1/openInterest" =>
        {
            true
        }
        ("index", "GET", AuthMode::Public) if normalize_request_path(path) == "/eapi/v1/index" => {
            true
        }
        ("fills", "GET", AuthMode::PrivateRead)
            if path == "/api/v3/myTrades"
                || path.ends_with("/quick/user/trades")
                || normalize_request_path(path) == "/eapi/v1/userTrades" =>
        {
            true
        }
        ("funds", "GET", AuthMode::PrivateRead) if path == "/api/v3/account" => true,
        ("funds", "POST", AuthMode::PrivateRead) if path.ends_with("/quick/user/limits") => true,
        ("orders", "GET", AuthMode::PrivateRead)
            if path == "/api/v3/openOrders" || path.ends_with("/quick/user/orders") =>
        {
            true
        }
        ("positions", "GET", AuthMode::PrivateRead) if path.ends_with("/quick/user/positions") => {
            true
        }
        ("holdings", "GET", AuthMode::PrivateRead) if path.ends_with("/portfolio/v1/holdings") => {
            true
        }
        ("margin_estimate", "POST", AuthMode::PrivateRead)
            if path.ends_with("/quick/user/check-margin") =>
        {
            true
        }
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
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/ticker" => {
            Ok(("quote", AuthMode::Public))
        }
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/exchangeInfo" => {
            Ok(("instrument_master", AuthMode::Public))
        }
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/openInterest" => {
            Ok(("open_interest", AuthMode::Public))
        }
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/index" => {
            Ok(("index", AuthMode::Public))
        }
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/mark" => {
            Ok(("greeks", AuthMode::Public))
        }
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/depth" => {
            Ok(("order_book", AuthMode::Public))
        }
        ("GET", "/api/v3/klines") => Ok(("ohlcv", AuthMode::Public)),
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/klines" => {
            Ok(("ohlcv", AuthMode::Public))
        }
        ("GET", "/api/v3/depth") => Ok(("order_book", AuthMode::Public)),
        ("GET", "/api/v3/exchangeInfo") => Ok(("instrument_master", AuthMode::Public)),
        ("GET", "/api/v3/myTrades") => Ok(("fills", AuthMode::PrivateRead)),
        ("GET", p) if normalize_request_path(p) == "/eapi/v1/userTrades" => {
            Ok(("fills", AuthMode::PrivateRead))
        }
        ("GET", "/api/v3/account") => Ok(("funds", AuthMode::PrivateRead)),
        ("GET", "/api/v3/openOrders") => Ok(("orders", AuthMode::PrivateRead)),
        ("GET", p) if p.ends_with("/quick/user/orders") => Ok(("orders", AuthMode::PrivateRead)),
        ("GET", p) if p.ends_with("/quick/user/positions") => {
            Ok(("positions", AuthMode::PrivateRead))
        }
        ("GET", p) if p.ends_with("/portfolio/v1/holdings") => {
            Ok(("holdings", AuthMode::PrivateRead))
        }
        ("POST", p) if p.ends_with("/quick/user/limits") => Ok(("funds", AuthMode::PrivateRead)),
        ("POST", p) if p.ends_with("/quick/user/check-margin") => {
            Ok(("margin_estimate", AuthMode::PrivateRead))
        }
        ("GET", p) if p.ends_with("/quick/user/trades") => Ok(("fills", AuthMode::PrivateRead)),
        ("GET", p) if p.ends_with("/script-details/1.0/masterscrip/file-paths") => {
            Ok(("instrument_master", AuthMode::PrivateRead))
        }
        ("GET", p) if is_kotak_cash_scrip_csv_path(p) || is_kotak_fo_scrip_csv_path(p) => {
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

fn normalize_request_path(path: &str) -> String {
    let trimmed = path.trim();
    let without_query = trimmed.split('?').next().unwrap_or(trimmed);
    without_query.trim_end_matches('/').to_string()
}

fn is_kotak_r0_host(host: &str) -> bool {
    let normalized = host.trim().trim_end_matches('.').to_ascii_lowercase();
    host_allowed(&normalized) && normalized.ends_with(".kotaksecurities.com")
}

/// Fence is (book, host, path prefix). Global R0 still gates which hosts this
/// process may ever dial — do not add fapi/dapi here. `eapi.binance.com` is
/// named for book `binance-com-options` (REST lastPrice only).
///
/// Named book `kotak-nse-nfo` shares Kotak R0 hosts and may fetch FO scrip only.
/// Planned (host: None — never add to R0_ALLOWED_HOSTS until that book is named):
/// binance-com-usdm     fapi.binance.com
/// binance-com-coinm    dapi.binance.com
/// binance-com-stocks   api.binance.com + /sapi/v1/equity/  (same host, blocked by prefix)
///
/// Locks: locks/binance-com-spot.md + locks/kotak-nse-bse-cash.md (fetch 2026-08-22 IST).
pub fn authorize_book_call(
    book_id: &str,
    host: &str,
    method: &str,
    path: &str,
    attach_private: bool,
) -> Result<(&'static str, AuthMode), HostRefuse> {
    if !host_allowed(host) {
        return Err(HostRefuse::HostNotAllowed);
    }
    authorize_book_fence(book_id, host, path)?;
    authorize_inferred_call(host, method, path, attach_private)
}

/// Per-book host/path fence. Callers pass the connection's `book_id`, never a
/// Start-slug lookup (`shipping_book_id_for_slug` stays spot/cash).
pub fn authorize_book_fence(book_id: &str, host: &str, path: &str) -> Result<(), HostRefuse> {
    let host_norm = host.trim().trim_end_matches('.').to_ascii_lowercase();
    let path_norm = normalize_request_path(path);
    let path_lower = path_norm.to_ascii_lowercase();
    match book_id {
        "binance-com-spot" => {
            if host_norm != "api.binance.com" {
                return Err(HostRefuse::HostNotAllowed);
            }
            if !path_lower.starts_with("/api/v3/") {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            Ok(())
        }
        "binance-com-options" => {
            if host_norm != "eapi.binance.com" {
                return Err(HostRefuse::HostNotAllowed);
            }
            if !path_lower.starts_with("/eapi/") {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            Ok(())
        }
        "kotak-nse-bse-cash" => {
            if !is_kotak_r0_host(&host_norm) {
                return Err(HostRefuse::HostNotAllowed);
            }
            if is_kotak_fo_scrip_csv_path(&path_norm) {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            if kotak_quotes_segment(&path_norm).is_some_and(|seg| !is_cash_quotes_segment(&seg)) {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            Ok(())
        }
        "kotak-nse-nfo" => {
            if !is_kotak_r0_host(&host_norm) {
                return Err(HostRefuse::HostNotAllowed);
            }
            if path_lower.starts_with("/api/v3/") || path_lower.starts_with("/eapi/") {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            if is_kotak_fo_scrip_csv_path(&path_norm) && !is_kotak_nse_fo_scrip_csv_path(&path_norm)
            {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            if is_kotak_cash_scrip_csv_path(&path_norm) {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            if kotak_quotes_segment(&path_norm).is_some_and(|seg| !is_nfo_quotes_segment(&seg)) {
                return Err(HostRefuse::PathNotAllowlisted);
            }
            Ok(())
        }
        _ => Err(HostRefuse::PathNotAllowlisted),
    }
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

/// Official sample lists `nse_fo.csv` / `bse_fo.csv` / `cde_fo.csv` / `mcx_fo.csv`.
/// Cash book still refuses these; the named NFO book may fetch `nse_fo` scrip only.
/// Query strings are stripped and a trailing `/` is trimmed before the filename check.
pub fn is_kotak_fo_scrip_csv_path(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    let lower = lower.split('?').next().unwrap_or(&lower);
    let lower = lower.trim_end_matches('/');
    if !lower.contains("/wso2-scripmaster/") {
        return false;
    }
    let filename = lower.rsplit('/').next().unwrap_or(lower);
    is_kotak_fo_csv_filename(filename)
}

/// True only for `/wso2-scripmaster/` filenames `nse_fo.csv` / `nse_fo-v1.csv`
/// (same versioned pattern as other FO CSV stems, but `nse_fo` only).
pub fn is_kotak_nse_fo_scrip_csv_path(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    let lower = lower.split('?').next().unwrap_or(&lower);
    let lower = lower.trim_end_matches('/');
    if !lower.contains("/wso2-scripmaster/") {
        return false;
    }
    let filename = lower.rsplit('/').next().unwrap_or(lower);
    is_kotak_fo_csv_filename_stem(filename, "nse_fo")
}

fn is_kotak_fo_csv_filename(filename: &str) -> bool {
    ["nse_fo", "bse_fo", "cde_fo", "mcx_fo"]
        .into_iter()
        .any(|stem| is_kotak_fo_csv_filename_stem(filename, stem))
}

fn is_kotak_fo_csv_filename_stem(filename: &str, stem: &str) -> bool {
    let Some(rest) = filename.strip_prefix(stem) else {
        return false;
    };
    let versioned = (rest.starts_with('-') || rest.starts_with('_')) && rest.ends_with(".csv");
    rest == ".csv" || versioned
}

fn percent_decode_minimal(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &raw[i + 1..i + 3];
            if let Ok(value) = u8::from_str_radix(hex, 16) {
                out.push(value as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Segment token from a quotes/neosymbol path (`nse_cm|2885` / `nse_fo|…`).
fn kotak_quotes_segment(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    let idx = lower.find(super::kotak_quotes::QUOTES_NEOSYMBOL_MARK)?;
    let rest = &path[idx + super::kotak_quotes::QUOTES_NEOSYMBOL_MARK.len()..];
    let (symbols, _) = rest.rsplit_once('/')?;
    if symbols.is_empty() {
        return None;
    }
    let decoded = percent_decode_minimal(symbols);
    let segment = decoded.split('|').next()?.trim().to_ascii_lowercase();
    if segment.is_empty() {
        None
    } else {
        Some(segment)
    }
}

fn is_cash_quotes_segment(segment: &str) -> bool {
    super::kotak_quotes::is_cash_segment(segment)
}

fn is_nfo_quotes_segment(segment: &str) -> bool {
    super::kotak_quotes::is_nfo_segment(segment)
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
    // Options klines live on eapi.binance.com — a different book, a different path.
    if (path.trim() == "/api/v3/klines" || path.trim() == "/api/v3/depth")
        && host_norm != "api.binance.com"
    {
        return Err(HostRefuse::HostNotAllowed);
    }
    if normalize_request_path(path) == "/eapi/v1/klines" && host_norm != "eapi.binance.com" {
        return Err(HostRefuse::HostNotAllowed);
    }
    if (is_kotak_cash_scrip_csv_path(path) || is_kotak_fo_scrip_csv_path(path))
        && host_norm != "lapi.kotaksecurities.com"
    {
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
    fn open_orders_allowlisted_and_post_order_still_forbidden() {
        let (cap, mode) = infer_capability("GET", "/api/v3/openOrders").unwrap();
        assert_eq!(cap, "orders");
        assert_eq!(mode, AuthMode::PrivateRead);
        authorize_host_call(
            "api.binance.com",
            "GET",
            "/api/v3/openOrders",
            "orders",
            AuthMode::PrivateRead,
            true,
        )
        .expect("signed openOrders poll remains allowlisted");
        authorize_book_call(
            "binance-com-spot",
            "api.binance.com",
            "GET",
            "/api/v3/openOrders",
            true,
        )
        .expect("spot book may GET openOrders");
        assert_eq!(
            infer_capability("POST", "/api/v3/order").unwrap_err(),
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
        let (fo_q_cap, fo_q_mode) = infer_capability("GET", fo_q).unwrap();
        assert_eq!(fo_q_cap, "instrument_master");
        assert_eq!(fo_q_mode, AuthMode::Public);
        assert_eq!(
            authorize_book_call(
                "kotak-nse-bse-cash",
                "lapi.kotaksecurities.com",
                "GET",
                fo_q,
                false
            )
            .unwrap_err(),
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
        let (fo_cap, fo_mode) = infer_capability("GET", fo).unwrap();
        assert_eq!(fo_cap, "instrument_master");
        assert_eq!(fo_mode, AuthMode::Public);
        assert_eq!(
            authorize_book_call(
                "kotak-nse-bse-cash",
                "lapi.kotaksecurities.com",
                "GET",
                fo,
                false
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        let fo_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_fo-v1.csv";
        assert_eq!(
            authorize_book_call(
                "kotak-nse-bse-cash",
                "lapi.kotaksecurities.com",
                "GET",
                fo_v1,
                false
            )
            .unwrap_err(),
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
            assert!(is_kotak_fo_scrip_csv_path(fo), "{fo}");
            assert_eq!(
                authorize_book_call(
                    "kotak-nse-bse-cash",
                    "lapi.kotaksecurities.com",
                    "GET",
                    fo,
                    false
                )
                .unwrap_err(),
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

    #[test]
    fn spot_book_refuses_equity_sapi_and_other_binance_clusters() {
        // Locks/binance-com-spot.md (fetch 2026-08-22 IST): /api/v3 on api.binance.com only.
        assert_eq!(
            authorize_book_call(
                "binance-com-spot",
                "api.binance.com",
                "GET",
                "/sapi/v1/equity/market/quote",
                false,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        for host in ["fapi.binance.com", "dapi.binance.com"] {
            assert!(!host_allowed(host), "{host} must stay off R0_ALLOWED_HOSTS");
            for path in ["/fapi/v1/premiumIndex", "/fapi/v1/forceOrders"] {
                assert_eq!(
                    authorize_book_call("binance-com-spot", host, "GET", path, false).unwrap_err(),
                    HostRefuse::HostNotAllowed,
                    "{host}{path}"
                );
            }
        }
        // eapi is on R0 for the named options book; the spot fence still refuses it.
        assert!(host_allowed("eapi.binance.com"));
        assert_eq!(
            authorize_book_call(
                "binance-com-spot",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/ticker",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        authorize_book_call(
            "binance-com-spot",
            "api.binance.com",
            "GET",
            "/api/v3/ticker/price",
            false,
        )
        .expect("public spot ticker remains allowlisted");
        let fo = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
        assert!(!is_kotak_cash_scrip_csv_path(fo));
        assert!(is_kotak_fo_scrip_csv_path(fo));
        assert_eq!(
            authorize_book_call(
                "kotak-nse-bse-cash",
                "lapi.kotaksecurities.com",
                "GET",
                fo,
                false
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
    }

    /// `/eapi/v1/optionChain` is NOT SPECIFIED on the official MarketDataApi.
    /// Station's `optionchain` noun is a catalog noun served from `exchangeInfo`
    /// — it must never open that path, and blessing the noun does not bless it.
    ///
    /// `/eapi/v1/mark` and `/eapi/v1/depth` are open as of Slice 3
    /// (binance-com-options.md, 2026-08-31): both are public on the official
    /// Market Data page. `mark` infers as `greeks` because Station copies what
    /// the venue published — it is not a licence to price this book.
    #[test]
    fn options_book_refuses_invented_option_chain_path() {
        assert_eq!(
            infer_capability("GET", "/eapi/v1/optionChain").unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/optionChain",
                false,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );

        // Slice 3: both now infer, and both stay public.
        assert_eq!(
            infer_capability("GET", "/eapi/v1/mark").unwrap(),
            ("greeks", AuthMode::Public)
        );
        assert_eq!(
            infer_capability("GET", "/eapi/v1/mark?symbol=BTC-260925-100000-C").unwrap(),
            ("greeks", AuthMode::Public)
        );
        assert_eq!(
            infer_capability("GET", "/eapi/v1/depth").unwrap(),
            ("order_book", AuthMode::Public)
        );
        for path in ["/eapi/v1/mark", "/eapi/v1/depth"] {
            assert!(
                authorize_book_call(
                    "binance-com-options",
                    "eapi.binance.com",
                    "GET",
                    path,
                    false,
                )
                .is_ok(),
                "{path} is public and locked in Slice 3"
            );
            // Public reads never receive private credentials (ADR 0002 / R0).
            assert_eq!(
                authorize_book_call("binance-com-options", "eapi.binance.com", "GET", path, true,)
                    .unwrap_err(),
                HostRefuse::PrivateCredentialOnPublicCall
            );
        }
        // The two paths the chain and OI nouns actually ride stay open.
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/exchangeInfo",
                false,
            )
            .expect("chain snapshot path stays allowlisted"),
            ("instrument_master", AuthMode::Public)
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/openInterest",
                false,
            )
            .expect("OI path stays allowlisted"),
            ("open_interest", AuthMode::Public)
        );
        // The fence is per book, both ways: the spot book may not reach eapi
        // depth, and the options book may not reach the spot depth path.
        assert!(matches!(
            authorize_book_call(
                "binance-com-spot",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/depth",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed { .. }
        ));
        assert!(matches!(
            authorize_book_call(
                "binance-com-options",
                "api.binance.com",
                "GET",
                "/api/v3/depth",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed { .. }
        ));
        // Spot still cannot dial eapi at all, by either path.
        assert_eq!(
            authorize_book_call(
                "binance-com-spot",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/exchangeInfo",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
    }

    #[test]
    fn nfo_book_fence_allows_fo_scrip_and_refuses_cash_and_spot() {
        let fo = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
        let fo_v1 = "/wso2-scripmaster/v1/prod/2026-08-27/transformed-v1/nse_fo-v1.csv";
        authorize_book_fence("kotak-nse-nfo", "lapi.kotaksecurities.com", fo)
            .expect("NFO book may fetch FO scrip");
        authorize_book_call(
            "kotak-nse-nfo",
            "lapi.kotaksecurities.com",
            "GET",
            fo,
            false,
        )
        .expect("NFO FO CSV infers instrument_master");
        authorize_book_call(
            "kotak-nse-nfo",
            "lapi.kotaksecurities.com",
            "GET",
            fo_v1,
            false,
        )
        .expect("NFO live v1 FO CSV is allowlisted");
        assert!(is_kotak_nse_fo_scrip_csv_path(fo));
        assert!(is_kotak_nse_fo_scrip_csv_path(fo_v1));
        for other_fo in [
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/cde_fo.csv",
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_fo.csv",
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/mcx_fo.csv",
        ] {
            assert!(is_kotak_fo_scrip_csv_path(other_fo), "{other_fo}");
            assert!(!is_kotak_nse_fo_scrip_csv_path(other_fo), "{other_fo}");
            assert_eq!(
                authorize_book_call(
                    "kotak-nse-nfo",
                    "lapi.kotaksecurities.com",
                    "GET",
                    other_fo,
                    false,
                )
                .unwrap_err(),
                HostRefuse::PathNotAllowlisted,
                "{other_fo}"
            );
        }
        let cash_csv = "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv";
        assert_eq!(
            authorize_book_fence("kotak-nse-nfo", "lapi.kotaksecurities.com", cash_csv)
                .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_call(
                "kotak-nse-nfo",
                "api.binance.com",
                "GET",
                "/api/v3/ticker/price",
                false
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        let cash_quote = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp";
        assert_eq!(
            authorize_book_fence("kotak-nse-nfo", "gw-napi.kotaksecurities.com", cash_quote)
                .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        let nfo_quote = "/script-details/1.0/quotes/neosymbol/nse_fo%7C12345/ltp";
        authorize_book_fence("kotak-nse-nfo", "gw-napi.kotaksecurities.com", nfo_quote)
            .expect("NFO quotes path stays on this book");
        authorize_book_call(
            "kotak-nse-nfo",
            "gw-napi.kotaksecurities.com",
            "GET",
            nfo_quote,
            true,
        )
        .expect("NFO ltp is PrivateRead quote");
        authorize_book_fence("binance-com-options", "eapi.binance.com", "/eapi/v1/ticker")
            .expect("named options book may GET eapi ticker");
        assert!(host_allowed("eapi.binance.com"));
        assert_eq!(
            authorize_book_fence("kotak-nse-nfo", "eapi.binance.com", "/eapi/v1/ticker")
                .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_book_fence(
                "kotak-nse-nfo",
                "gw-napi.kotaksecurities.com",
                "/eapi/v1/ticker"
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_fence(
                "binance-com-usdm",
                "api.binance.com",
                "/api/v3/ticker/price"
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
    }

    #[test]
    fn options_book_fence_allows_eapi_ticker_and_refuses_spot_cluster() {
        authorize_book_fence("binance-com-options", "eapi.binance.com", "/eapi/v1/ticker")
            .expect("options fence allows eapi ticker");
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/ticker",
            false,
        )
        .expect("options public ticker infers quote");
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/ticker?symbol=BTC-200730-9000-C",
            false,
        )
        .expect("optional symbol query stays quote");
        assert_eq!(
            authorize_book_fence(
                "binance-com-options",
                "api.binance.com",
                "/api/v3/ticker/price"
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "api.binance.com",
                "GET",
                "/api/v3/ticker/price",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        let (cap, mode) = infer_capability("GET", "/eapi/v1/ticker").unwrap();
        assert_eq!(cap, "quote");
        assert_eq!(mode, AuthMode::Public);
        assert_eq!(
            infer_capability("GET", "/eapi/v1/klines").unwrap(),
            ("ohlcv", AuthMode::Public)
        );
        assert_eq!(
            infer_capability("GET", "/eapi/v1/index").unwrap(),
            ("index", AuthMode::Public)
        );
        assert_eq!(
            infer_capability("GET", "/eapi/v1/index?underlying=BTCUSDT").unwrap(),
            ("index", AuthMode::Public)
        );
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/index?underlying=BTCUSDT",
            false,
        )
        .expect("options public index infers index");
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/index",
                true,
            )
            .unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        // OI's query name must not be how this path infers.
        assert_eq!(
            infer_capability("GET", "/eapi/v1/index?underlyingAsset=BTC").unwrap(),
            ("index", AuthMode::Public)
        );
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/klines?symbol=XRP-260911-1.36-C&interval=1m",
            false,
        )
        .expect("options public klines infers ohlcv");
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/klines",
                true,
            )
            .unwrap_err(),
            HostRefuse::PrivateCredentialOnPublicCall
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "api.binance.com",
                "GET",
                "/api/v3/klines",
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_host_call(
                "api.binance.com",
                "GET",
                "/eapi/v1/klines",
                "ohlcv",
                AuthMode::Public,
                false,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            infer_capability("POST", "/eapi/v1/order").unwrap_err(),
            HostRefuse::MutationForbidden
        );
        assert!(is_mutation("POST", "/eapi/v1/order"));
        let (fills_cap, fills_mode) = infer_capability("GET", "/eapi/v1/userTrades").unwrap();
        assert_eq!(fills_cap, "fills");
        assert_eq!(fills_mode, AuthMode::PrivateRead);
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/userTrades",
            true,
        )
        .expect("options userTrades is PrivateRead fills on the options book");
        assert_eq!(
            infer_capability("GET", "/eapi/v1/optionChain").unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-spot",
                "eapi.binance.com",
                "GET",
                "/eapi/v1/userTrades",
                true,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_host_call(
                "eapi.binance.com",
                "GET",
                "/eapi/v1/userTrades",
                "fills",
                AuthMode::Public,
                false,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/fapi/v1/premiumIndex",
                false,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_fence("binance-com-spot", "eapi.binance.com", "/eapi/v1/ticker")
                .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        assert_eq!(
            authorize_book_fence("kotak-nse-bse-cash", "eapi.binance.com", "/eapi/v1/ticker")
                .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
    }

    #[test]
    fn cash_book_refuses_nfo_quote_path() {
        let nfo_quote = "/script-details/1.0/quotes/neosymbol/nse_fo%7C12345/ltp";
        assert_eq!(
            authorize_book_fence(
                "kotak-nse-bse-cash",
                "gw-napi.kotaksecurities.com",
                nfo_quote
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        let cash_quote = "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp";
        authorize_book_fence(
            "kotak-nse-bse-cash",
            "gw-napi.kotaksecurities.com",
            cash_quote,
        )
        .expect("cash quotes remain on the cash book");
    }

    #[test]
    fn options_eapi_public_reads_infer_on_options_book_only() {
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/exchangeInfo",
            false,
        )
        .expect("exchangeInfo is instrument_master on the options book");
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/openInterest?underlyingAsset=BTC&expiration=200730",
            false,
        )
        .expect("openInterest is public on the options book");
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/exchangeInfo",
            true,
        )
        .expect_err("public exchangeInfo must not attach private credentials");
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/openInterest",
            true,
        )
        .expect_err("public openInterest must not attach private credentials");
        // Slice 3: depth is public on this book, query string and all.
        assert_eq!(
            infer_capability("GET", "/eapi/v1/depth").unwrap(),
            ("order_book", AuthMode::Public)
        );
        for path in [
            "/eapi/v1/depth",
            "/eapi/v1/depth?symbol=BTC-200730-9000-C",
            "/eapi/v1/depth?symbol=BTC-200730-9000-C&limit=100",
        ] {
            assert!(
                authorize_book_call(
                    "binance-com-options",
                    "eapi.binance.com",
                    "GET",
                    path,
                    false,
                )
                .is_ok(),
                "{path}"
            );
            authorize_book_call("binance-com-options", "eapi.binance.com", "GET", path, true)
                .expect_err("public depth must not attach private credentials");
        }

        for path in [
            "/eapi/v1/exchangeInfo",
            "/eapi/v1/openInterest",
            "/eapi/v1/depth",
            "/eapi/v1/ticker",
            "/eapi/v1/klines",
            "/eapi/v1/index",
        ] {
            assert_eq!(
                authorize_book_call("binance-com-spot", "eapi.binance.com", "GET", path, false)
                    .unwrap_err(),
                HostRefuse::HostNotAllowed,
                "{path}"
            );
            assert_eq!(
                authorize_book_call("binance-com-spot", "api.binance.com", "GET", path, false)
                    .unwrap_err(),
                HostRefuse::PathNotAllowlisted,
                "{path}"
            );
        }
    }

    #[test]
    fn options_user_trades_is_not_spot_my_trades() {
        let (spot_cap, spot_mode) = infer_capability("GET", "/api/v3/myTrades").unwrap();
        assert_eq!(spot_cap, "fills");
        assert_eq!(spot_mode, AuthMode::PrivateRead);
        authorize_book_call(
            "binance-com-spot",
            "api.binance.com",
            "GET",
            "/api/v3/myTrades",
            true,
        )
        .expect("spot myTrades stays spot fills");
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "eapi.binance.com",
                "GET",
                "/api/v3/myTrades",
                true,
            )
            .unwrap_err(),
            HostRefuse::PathNotAllowlisted
        );
        assert_eq!(
            authorize_book_call(
                "binance-com-options",
                "api.binance.com",
                "GET",
                "/api/v3/myTrades",
                true,
            )
            .unwrap_err(),
            HostRefuse::HostNotAllowed
        );
        authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/userTrades?symbol=BTC-200730-9000-C",
            true,
        )
        .expect("options userTrades with symbol query stays allowlisted");
        let (opts_cap, opts_mode) = authorize_book_call(
            "binance-com-options",
            "eapi.binance.com",
            "GET",
            "/eapi/v1/userTrades",
            false,
        )
        .expect("PrivateRead fills infers without attach_private at policy fence");
        assert_eq!(opts_cap, "fills");
        assert_eq!(opts_mode, AuthMode::PrivateRead);
    }
}

//! Slice C: Kotak Neo REST quotes → TickBook (`adapter_id: kotak_neo`).
//!
//! Path: `GET …/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}`
//! (`PROD_URL["quotes_neo_symbol"]` / `quotes_neo_symbol_api.py`). PrivateRead session
//! attach — not `AuthMode::Public`, not HMAC. Do not call `quotes_neo_symbol_napi`.
//!
//! HSM `wss://mlhsm.kotaksecurities.com` is skipped: WEBSOCKET.md — `mlhsm` is not on
//! the HTTP allowlist; `hsServerId` attach on that socket is unspecified. REST
//! poll/on-select is enough. Do not subscribe `isDepth=true` until an ordered-state
//! replica exists. Slice D stores REST `quote_type=depth` as a bounded snapshot.

use super::descriptor::KOTAK_NEO_ADAPTER_ID;
use super::tick::{QuoteTick, SessionOhlc, Transport};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const QUOTES_NEOSYMBOL_MARK: &str = "/script-details/1.0/quotes/neosymbol/";
pub const QUOTE_TYPE_ALL: &str = "all";
pub const QUOTE_TYPE_LTP: &str = "ltp";
pub const QUOTE_TYPE_OHLC: &str = "ohlc";
pub const QUOTE_TYPE_DEPTH: &str = "depth";

pub const DOCUMENTED_QUOTE_TYPES: &[&str] = &[
    "all",
    "depth",
    "ohlc",
    "ltp",
    "oi",
    "52w",
    "circuit_limits",
    "scrip_details",
];

/// `{segment}|{token}` e.g. `nse_cm|2885` (REST.md / Quotes.md `neo_symbols`).
pub fn kotak_instrument_id(segment: &str, token: &str) -> Option<String> {
    let segment = segment.trim().to_ascii_lowercase();
    let token = token.trim();
    if token.is_empty() || !is_cash_segment(&segment) {
        return None;
    }
    Some(format!("{segment}|{token}"))
}

pub fn is_cash_segment(segment: &str) -> bool {
    matches!(segment, "nse_cm" | "bse_cm")
}

/// SDK `urllib.parse.quote` of `{exchange_segment}|{instrument_token}`.
pub fn encode_neo_symbol(neo_symbol: &str) -> String {
    let mut out = String::with_capacity(neo_symbol.len());
    for byte in neo_symbol.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub fn quotes_neosymbol_path(neo_symbol: &str, quote_type: &str) -> String {
    format!(
        "{QUOTES_NEOSYMBOL_MARK}{}/{quote_type}",
        encode_neo_symbol(neo_symbol)
    )
}

/// Documented GET suffix + `quote_type` token. Does not match napi `apim/quotes/1.0/…`.
pub fn is_kotak_quotes_neosymbol_path(path: &str) -> bool {
    kotak_quotes_neosymbol_quote_type(path).is_some()
}

pub fn kotak_quotes_neosymbol_quote_type(path: &str) -> Option<&str> {
    let path = path.trim();
    let idx = path.find(QUOTES_NEOSYMBOL_MARK)?;
    let rest = &path[idx + QUOTES_NEOSYMBOL_MARK.len()..];
    let (symbols, quote_type) = rest.rsplit_once('/')?;
    if symbols.is_empty() {
        return None;
    }
    DOCUMENTED_QUOTE_TYPES
        .iter()
        .copied()
        .find(|token| quote_type.eq_ignore_ascii_case(token))
}

pub fn is_kotak_depth_path(path: &str) -> bool {
    is_kotak_quotes_neosymbol_path(path)
        && kotak_quotes_neosymbol_quote_type(path) == Some(QUOTE_TYPE_DEPTH)
}

pub fn is_kotak_latest_quote_path(path: &str) -> bool {
    is_kotak_quotes_neosymbol_path(path)
        && kotak_quotes_neosymbol_quote_type(path).is_some_and(|token| token != QUOTE_TYPE_DEPTH)
}

/// Map a Kotak REST quotes JSON body to the first cash LTP tick.
///
/// Envelope: v1 Quotes.md sample is `{ "message": [ { last_traded_price, instrument_token,
/// exchange_segment, ohlc } ] }`. Live GET (founder 2026-08-27, `nse_cm|11536`) is a
/// **root JSON array**. OpenAlgo `broker/kotak/api/data.py` on the same path reads
/// `response[0].ltp`, `.exchange`, `.exchange_token`, `.ohlc`, `.depth` (REST.md).
/// WEBSOCKET.md `ltp` / `op`/`h`/`lo`/`c` remain valid on an object. `quote_type=depth`
/// / `oi` without LTP or session close → `None` (depth lives in [`super::kotak_depth`]).
pub fn quote_tick_from_kotak_json(raw: &str, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    quote_ticks_from_kotak_json(raw, received_at)
        .into_iter()
        .next()
}

pub fn quote_ticks_from_kotak_json(raw: &str, received_at: DateTime<Utc>) -> Vec<QuoteTick> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    quote_objects(&value)
        .into_iter()
        .filter_map(|obj| tick_from_object(obj, received_at))
        .collect()
}

pub(crate) fn quote_objects(value: &Value) -> Vec<&Value> {
    if let Some(arr) = value.as_array() {
        return arr.iter().collect();
    }
    if let Some(arr) = value.get("message").and_then(Value::as_array) {
        return arr.iter().collect();
    }
    if let Some(data) = value.get("data") {
        return quote_objects(data);
    }
    if value.is_object() {
        return vec![value];
    }
    Vec::new()
}

/// Cash `nse_cm|token` from a quote/depth row. v1: `exchange_segment` +
/// `instrument_token`. Live v2 array (OpenAlgo / founder 2026-08-27): `exchange` +
/// `exchange_token`.
pub(crate) fn cash_instrument_id_from_quote_object(value: &Value) -> Option<String> {
    let segment = json_string(
        value
            .get("exchange_segment")
            .or_else(|| value.get("pExchSeg"))
            .or_else(|| value.get("exchSeg"))
            .or_else(|| value.get("exchange"))?,
    )?;
    let token = json_string(
        value
            .get("instrument_token")
            .or_else(|| value.get("pToken"))
            .or_else(|| value.get("token"))
            .or_else(|| value.get("exchange_token"))?,
    )?;
    kotak_instrument_id(&segment, &token)
}

fn tick_from_object(value: &Value, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    let instrument_id = cash_instrument_id_from_quote_object(value)?;
    let session_ohlc = session_ohlc_from_object(value);
    let last = json_string(
        value
            .get("last_traded_price")
            .or_else(|| value.get("ltp"))
            .or_else(|| value.get("last"))
            .or_else(|| value.get("lastPrice"))
            .or_else(|| value.get("LTP"))?,
    )
    .or_else(|| session_ohlc.as_ref().map(|bar| bar.close.clone()))?;
    let last_n: f64 = last.parse().ok()?;
    if last_n <= 0.0 {
        return None;
    }
    Some(QuoteTick {
        instrument_id,
        last,
        as_of: received_at,
        received_at,
        age_unknown: true,
        transport: Transport::Rest,
        adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
        session_ohlc,
    })
}

fn session_ohlc_from_object(value: &Value) -> Option<SessionOhlc> {
    if let Some(ohlc) = value.get("ohlc") {
        return Some(SessionOhlc {
            open: json_string(ohlc.get("open")?)?,
            high: json_string(ohlc.get("high")?)?,
            low: json_string(ohlc.get("low")?)?,
            close: json_string(ohlc.get("close")?)?,
        });
    }
    // WEBSOCKET.md live stock fields — session bar, not historical_series.
    let open = json_string(value.get("op")?)?;
    let high = json_string(value.get("h")?)?;
    let low = json_string(value.get("lo")?)?;
    let close = json_string(value.get("c")?)?;
    Some(SessionOhlc {
        open,
        high,
        low,
        close,
    })
}

pub(crate) fn json_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::apply::apply_quote;
    use crate::data::descriptor::kotak_neo_quote_descriptor;
    use crate::data::registry::Registry;
    use crate::data::tickbook::TickBook;
    use chrono::TimeZone;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
    }

    fn desk_registry() -> Registry {
        Registry::load(&[kotak_neo_quote_descriptor()]).expect("kotak_neo quote loads")
    }

    const FIXTURE: &str = r#"{
        "message": [{
            "last_traded_price": "1400.50",
            "instrument_token": "2885",
            "exchange_segment": "nse_cm",
            "trading_symbol": "RELIANCE-EQ",
            "ohlc": {"open": "1390.00", "high": "1410.25", "low": "1385.10", "close": "1400.50"}
        }]
    }"#;

    #[test]
    fn fixture_quote_applies_to_tickbook_as_kotak_neo() {
        let tick = quote_tick_from_kotak_json(FIXTURE, received()).expect("fixture tick");
        assert_eq!(tick.instrument_id, "nse_cm|2885");
        assert_eq!(tick.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_eq!(tick.transport, Transport::Rest);
        let last: f64 = tick.last.parse().unwrap();
        assert!(last > 0.0);
        assert_eq!(tick.last, "1400.50");
        let ohlc = tick
            .session_ohlc
            .as_ref()
            .expect("session ohlc on envelope");
        assert_eq!(ohlc.open, "1390.00");
        assert_eq!(ohlc.high, "1410.25");
        assert_eq!(ohlc.low, "1385.10");
        assert_eq!(ohlc.close, "1400.50");

        let registry = desk_registry();
        let mut book = TickBook::new();
        apply_quote(&registry, &mut book, tick).unwrap();
        let row = book
            .get(KOTAK_NEO_ADAPTER_ID, "nse_cm|2885")
            .expect("stored");
        let stored_last: f64 = row.last.parse().unwrap();
        assert!(stored_last > 0.0);
        assert_eq!(row.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_ne!(row.adapter_id, "binance_com");
        assert_ne!(row.adapter_id, "kotak_public");
    }

    #[test]
    fn fo_and_depth_without_ltp_are_ignored() {
        let fo_v1 = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
        assert!(quote_tick_from_kotak_json(fo_v1, received()).is_none());
        let fo_v2 = r#"{"exchange_token":"12345","exchange":"nse_fo","ltp":"10.00"}"#;
        assert!(quote_tick_from_kotak_json(fo_v2, received()).is_none());
        let depth = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","depth":{"buy":[],"sell":[]}}"#;
        assert!(quote_tick_from_kotak_json(depth, received()).is_none());
    }

    #[test]
    fn ltp_only_and_ws_field_names_parse() {
        let ltp = r#"{"instrument_token":2885,"exchange_segment":"nse_cm","ltp":"99.50"}"#;
        let tick = quote_tick_from_kotak_json(ltp, received()).unwrap();
        assert_eq!(tick.last, "99.50");
        assert!(tick.session_ohlc.is_none());
        let ws = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","ltp":"100.00","op":"98","h":"101","lo":"97","c":"99"}"#;
        let bar = quote_tick_from_kotak_json(ws, received()).unwrap();
        assert_eq!(bar.session_ohlc.as_ref().unwrap().high, "101");
    }

    #[test]
    fn neo_symbol_path_encodes_pipe_and_keeps_quote_type() {
        assert_eq!(
            quotes_neosymbol_path("nse_cm|2885", QUOTE_TYPE_LTP),
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        );
        assert!(is_kotak_quotes_neosymbol_path(
            "/trading/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/all"
        ));
        assert!(!is_kotak_quotes_neosymbol_path(
            "apim/quotes/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_quotes_neosymbol_path("/quick/quotes"));
        assert!(is_kotak_depth_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/depth"
        ));
        assert!(is_kotak_latest_quote_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_depth_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_quotes_neosymbol_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/market_depth"
        ));
    }

    #[test]
    fn live_root_array_uses_exchange_and_exchange_token() {
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol_live_array.json");
        let tick = quote_tick_from_kotak_json(json, received()).expect("live array tick");
        assert_eq!(tick.instrument_id, "nse_cm|11536");
        assert_eq!(tick.last, "3224.50");
        assert_eq!(tick.session_ohlc.as_ref().unwrap().close, "3210.00");
        assert_eq!(tick.adapter_id, KOTAK_NEO_ADAPTER_ID);
    }
}

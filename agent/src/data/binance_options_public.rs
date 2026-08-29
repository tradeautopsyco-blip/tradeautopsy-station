//! S1 desk: Binance.com European options public last into TickBook.
//!
//! Way 3 last-only: REST `GET /eapi/v1/ticker` → `lastPrice`
//! (`issues/compliance/locks/binance-com-options.md`). Do not map spot
//! `@trade` `p` or invent WS `c`. WS last field is NOT SPECIFIED — URL helper
//! only; do not apply live payloads.

use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID};
use super::tick::{QuoteTick, Transport};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[allow(dead_code)] // REST last lock — URL helpers are not dialed until WS last is specified.
pub const OPTIONS_EAPI_HOST: &str = "eapi.binance.com";
#[allow(dead_code)]
pub const OPTIONS_TICKER_PATH: &str = "/eapi/v1/ticker";
#[allow(dead_code)]
pub const OPTIONS_WS_BASE: &str = "wss://nbstream.binance.com/eoptions";

/// Options contract ids stay mixed-case (`BTC-200730-9000-C`). Trim only.
pub fn normalize_options_instrument(raw: &str) -> String {
    raw.trim().to_string()
}

#[allow(dead_code)]
pub fn binance_options_stream_base_url() -> &'static str {
    OPTIONS_WS_BASE
}

#[allow(dead_code)]
pub fn binance_options_ticker_url(symbol: Option<&str>) -> String {
    match symbol.map(str::trim).filter(|s| !s.is_empty()) {
        Some(symbol) => format!("https://{OPTIONS_EAPI_HOST}{OPTIONS_TICKER_PATH}?symbol={symbol}"),
        None => format!("https://{OPTIONS_EAPI_HOST}{OPTIONS_TICKER_PATH}"),
    }
}

pub fn options_quote_stream_key(symbol: &str) -> String {
    format!(
        "{}\0{}",
        BINANCE_COM_OPTIONS_BOOK_ID,
        normalize_options_instrument(symbol)
    )
}

/// Record `{book_id}\0{symbol}` so options cannot collide with spot `btcusdt`.
/// Does not dial WS (last field unspecified) and must not call the spot `@trade` stream.
pub fn ensure_binance_com_options_quote(spawned: &Arc<Mutex<HashSet<String>>>, symbol: &str) {
    let instrument = normalize_options_instrument(symbol);
    if instrument.is_empty() {
        return;
    }
    let key = options_quote_stream_key(&instrument);
    let mut guard = spawned.lock().expect("quote stream set poisoned");
    let _ = guard.insert(key);
}

fn ticker_objects(value: &Value) -> Vec<&Value> {
    if let Some(arr) = value.as_array() {
        return arr.iter().collect();
    }
    if value.is_object() {
        return vec![value];
    }
    Vec::new()
}

fn tick_from_ticker_object(value: &Value, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    let symbol = value.get("symbol").and_then(Value::as_str)?;
    let instrument_id = normalize_options_instrument(symbol);
    if instrument_id.is_empty() {
        return None;
    }
    let last = value.get("lastPrice").and_then(Value::as_str)?.to_string();
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
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        book_id: BINANCE_COM_OPTIONS_BOOK_ID.to_string(),
        session_ohlc: None,
    })
}

/// Map REST ticker JSON (array or object) to quote ticks. Ignores spot `@trade` / miniTicker.
pub fn quote_ticks_from_options_ticker_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Vec<QuoteTick> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    ticker_objects(&value)
        .into_iter()
        .filter_map(|obj| tick_from_ticker_object(obj, received_at))
        .collect()
}

pub fn quote_tick_from_options_ticker_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Option<QuoteTick> {
    quote_ticks_from_options_ticker_json(raw, received_at)
        .into_iter()
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_public::{
        binance_public_trade_stream_url, quote_tick_from_binance_json,
    };
    use crate::data::descriptor::BINANCE_COM_SPOT_BOOK_ID;
    use chrono::{TimeZone, Utc};

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap()
    }

    #[test]
    fn parses_rest_ticker_array_last_price() {
        let json = include_str!("../../fixtures/binance/options_ticker.json");
        let tick = quote_tick_from_options_ticker_json(json, received()).unwrap();
        assert_eq!(tick.instrument_id, "BTC-200730-9000-C");
        assert_eq!(tick.last, "1.23");
        assert_eq!(tick.transport, Transport::Rest);
        assert_eq!(tick.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(tick.book_id, BINANCE_COM_OPTIONS_BOOK_ID);
        assert!(tick.age_unknown);
        assert_ne!(tick.instrument_id, tick.instrument_id.to_ascii_lowercase());
    }

    #[test]
    fn parses_rest_ticker_object() {
        let json = r#"{"symbol":"BTC-200730-9000-C","lastPrice":"1.23"}"#;
        let tick = quote_tick_from_options_ticker_json(json, received()).unwrap();
        assert_eq!(tick.instrument_id, "BTC-200730-9000-C");
        assert_eq!(tick.last, "1.23");
        assert_eq!(tick.book_id, BINANCE_COM_OPTIONS_BOOK_ID);
    }

    #[test]
    fn ignores_spot_trade_payload_and_does_not_map_p() {
        let trade = r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"100.00","T":1}"#;
        assert!(quote_tick_from_options_ticker_json(trade, received()).is_none());
        let spot = quote_tick_from_binance_json(trade, received()).unwrap();
        assert_eq!(spot.book_id, BINANCE_COM_SPOT_BOOK_ID);
        assert_eq!(spot.last, "100.00");
    }

    #[test]
    fn stream_url_is_eoptions_not_spot_trade() {
        assert_eq!(
            binance_options_stream_base_url(),
            "wss://nbstream.binance.com/eoptions"
        );
        assert_eq!(
            binance_options_ticker_url(None),
            "https://eapi.binance.com/eapi/v1/ticker"
        );
        assert_eq!(
            binance_options_ticker_url(Some("BTC-200730-9000-C")),
            "https://eapi.binance.com/eapi/v1/ticker?symbol=BTC-200730-9000-C"
        );
        assert_ne!(
            binance_options_stream_base_url(),
            binance_public_trade_stream_url("BTCUSDT")
        );
    }

    #[test]
    fn quote_stream_key_cannot_collide_with_spot_instrument() {
        assert_eq!(
            options_quote_stream_key("BTC-200730-9000-C"),
            format!("{BINANCE_COM_OPTIONS_BOOK_ID}\0BTC-200730-9000-C")
        );
        assert_ne!(options_quote_stream_key("btcusdt"), "btcusdt");
        let spawned = Arc::new(Mutex::new(HashSet::new()));
        ensure_binance_com_options_quote(&spawned, "BTC-200730-9000-C");
        let set = spawned.lock().unwrap();
        assert!(set.contains(&options_quote_stream_key("BTC-200730-9000-C")));
        assert!(!set.contains("btcusdt"));
        assert!(!set.contains("BTC-200730-9000-C"));
    }

    #[test]
    fn empty_or_zero_last_is_not_a_tick() {
        assert!(quote_tick_from_options_ticker_json(
            r#"{"symbol":"BTC-200730-9000-C","lastPrice":"0"}"#,
            received()
        )
        .is_none());
        assert!(quote_tick_from_options_ticker_json("[]", received()).is_none());
    }
}

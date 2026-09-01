//! Options order book from `GET /eapi/v1/depth` — a REST **bounded snapshot**.
//!
//! This is not spot's managed `@depth` loop. There is no `U`/`u`/`pu`
//! reconstruction on this book, no `synced` phase, and no gap machine: one GET
//! answers with `bids` / `asks` as positional `[price, quantity]` pairs plus a
//! named `lastUpdateId`, and that is the whole physics —
//! `market/order_book/bounded_snapshot`.
//!
//! Levels are **2-element arrays, not objects**. Kotak's cash ladder is
//! `{"price":…,"quantity":…,"orders":…}`; an `orders` count on an eapi level is
//! NOT SPECIFIED IN SOURCE. The two parsers must never be pointed at each other's
//! bodies, which is what the refuse tests at the bottom of this file prove.
//!
//! Lock: `locks/binance-com-options.md` Slice 3 ·
//! `docs/reference/crypto/binance-global/options/REST.md` Slice 3.

use super::binance_options_public::normalize_options_instrument;
use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID};
use super::kotak_depth::{DepthLevel, DepthSnapshot};
use super::tick::Transport;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Official host for the named options book. Never `api.binance.com`.
pub const OPTIONS_DEPTH_HOST: &str = "eapi.binance.com";
/// Official path (public, `Security: None`).
pub const OPTIONS_DEPTH_PATH: &str = "/eapi/v1/depth";
/// Weight 1 band (REST.md: limit 5/10/20/50 → weight 1). Do **not** copy spot's
/// 5000 / weight 250: that is a different host, a different book, and a
/// reconstruction loop this slice does not run.
pub const OPTIONS_DEPTH_LIMIT: u32 = 50;

/// `symbol` is **mandatory** on this endpoint, and mixed-case: Binance option
/// symbols are dated contracts (`BTC-200730-9000-C`), never spot pairs. Nothing
/// here may `to_ascii_lowercase` and nothing may omit the symbol.
pub fn options_depth_query(symbol: &str) -> String {
    format!(
        "symbol={}&limit={OPTIONS_DEPTH_LIMIT}",
        normalize_options_instrument(symbol)
    )
}

fn string_scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let s = s.trim();
            (!s.is_empty()).then(|| s.to_string())
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// One positional `[price, quantity]` level.
///
/// An object level (Kotak's shape) is **skipped**, not coerced: this endpoint
/// publishes arrays, so an object here means the body is not an eapi depth body.
/// `orders` is always `None` — the venue publishes no order count, and inventing
/// one would be a number that looks real and is not.
fn level_from_pair(value: &Value) -> Option<DepthLevel> {
    let pair = value.as_array()?;
    if pair.len() < 2 {
        return None;
    }
    let price = string_scalar(&pair[0])?;
    let quantity = string_scalar(&pair[1])?;
    let price_n: f64 = price.parse().ok()?;
    let qty_n: f64 = quantity.parse().ok()?;
    // Same as the spot REST snapshot: a non-positive level is dropped. This is
    // not the WS qty-0-removes-a-level rule — there is no delta stream here.
    if !price_n.is_finite() || !qty_n.is_finite() || price_n <= 0.0 || qty_n <= 0.0 {
        return None;
    }
    Some(DepthLevel {
        price,
        quantity,
        orders: None,
    })
}

fn levels_from(value: Option<&Value>) -> Vec<DepthLevel> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter().filter_map(level_from_pair).collect()
}

/// The bounded snapshot in an `/eapi/v1/depth` body, keyed to the contract that
/// was asked for.
///
/// `None` — never an empty success — when the body will not parse, when
/// `lastUpdateId` is missing (the snapshot has no identity), or when neither side
/// holds a usable level. Obtain then stays Unavailable rather than serving an
/// empty ladder as a Success.
///
/// `as_of` is Station's `received_at`. `T` (venue transaction time) is read only
/// to stamp `as_of` when it parses as a ms epoch; it never reaches the obtain
/// JSON, because the spot/cash obtain shape has no venue-time field and this book
/// does not get to invent one.
pub fn depth_snapshot_from_eapi_json(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<DepthSnapshot> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let instrument_id = normalize_options_instrument(symbol);
    if instrument_id.is_empty() {
        return None;
    }
    // Required: a bounded snapshot without its update id is unusable, not empty.
    let sequence = value.get("lastUpdateId")?.as_u64()?;
    let bids = levels_from(value.get("bids"));
    let asks = levels_from(value.get("asks"));
    if bids.is_empty() && asks.is_empty() {
        return None;
    }
    let bound_levels = bids.len().max(asks.len());
    let as_of = value
        .get("T")
        .and_then(Value::as_i64)
        .and_then(DateTime::from_timestamp_millis)
        .unwrap_or(received_at);
    Some(DepthSnapshot {
        instrument_id,
        // Login identity stays the slug shared with spot.
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        // Slot prefix is the **named book**, never
        // `shipping_book_id_for_slug("binance_com")` — that is spot.
        book_id: BINANCE_COM_OPTIONS_BOOK_ID.to_string(),
        bids,
        asks,
        completeness: true,
        bound_levels,
        as_of,
        transport: Transport::Rest,
        sequence: Some(sequence),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::kotak_depth::depth_snapshot_from_kotak_json;
    use chrono::TimeZone;

    /// Official response example, verbatim from the Order Book docs page.
    const OFFICIAL: &str = include_str!("../../fixtures/binance/options_depth.json");

    /// Kotak's cash ladder: object levels with an order count. Same noun, other book.
    const KOTAK_BODY: &str = r#"{
        "message": [{
            "instrument_token": "2885",
            "exchange_segment": "nse_cm",
            "trading_symbol": "RELIANCE-EQ",
            "depth": {
                "buy": [{"price": "1400.00", "quantity": "120", "orders": "3"}],
                "sell": [{"price": "1400.50", "quantity": "90", "orders": "4"}]
            }
        }]
    }"#;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 31, 9, 0, 0).unwrap()
    }

    #[test]
    fn official_fixture_is_a_named_options_book_snapshot() {
        let snap =
            depth_snapshot_from_eapi_json(OFFICIAL, "BTC-200730-9000-C", received()).expect("snap");
        assert_eq!(snap.instrument_id, "BTC-200730-9000-C");
        assert_eq!(snap.adapter_id, BINANCE_COM_ADAPTER_ID);
        // The load-bearing assertion: options rows land in the options slot, not spot's.
        assert_eq!(snap.book_id, BINANCE_COM_OPTIONS_BOOK_ID);
        assert_ne!(snap.book_id, "binance-com-spot");
        assert_eq!(snap.transport, Transport::Rest);
        assert_eq!(snap.sequence, Some(361));
        assert!(snap.completeness);
        assert_eq!(snap.bids[0].price, "1000.000");
        assert_eq!(snap.bids[0].quantity, "0.1000");
        assert_eq!(snap.asks[0].price, "1900.000");
        assert_eq!(snap.bound_levels, 1);
        // No order count is published on an eapi level; none may be invented.
        assert!(snap.bids.iter().all(|l| l.orders.is_none()));
        assert!(snap.asks.iter().all(|l| l.orders.is_none()));
    }

    #[test]
    fn the_two_depth_parsers_do_not_share_a_body() {
        // An eapi body through the Kotak parser: no cash instrument, no object levels.
        assert!(depth_snapshot_from_kotak_json(OFFICIAL, received()).is_none());
        // A Kotak object ladder through the eapi parser: no lastUpdateId, and the
        // levels are objects, not `[price, quantity]` pairs.
        assert!(
            depth_snapshot_from_eapi_json(KOTAK_BODY, "BTC-200730-9000-C", received()).is_none()
        );
    }

    #[test]
    fn object_levels_are_skipped_not_coerced() {
        // lastUpdateId present, but the levels are Kotak-shaped → nothing usable.
        let body =
            r#"{"lastUpdateId":9,"bids":[{"price":"1","quantity":"1","orders":"2"}],"asks":[]}"#;
        assert!(depth_snapshot_from_eapi_json(body, "BTC-200730-9000-C", received()).is_none());
    }

    #[test]
    fn empty_both_sides_is_none_not_an_empty_success() {
        let body = r#"{"bids":[],"asks":[],"T":1762780909676,"lastUpdateId":361}"#;
        assert!(depth_snapshot_from_eapi_json(body, "BTC-200730-9000-C", received()).is_none());
    }

    #[test]
    fn a_missing_last_update_id_is_unusable() {
        let body =
            r#"{"bids":[["1000.000","0.1000"]],"asks":[["1900.000","0.1000"]],"T":1762780909676}"#;
        assert!(depth_snapshot_from_eapi_json(body, "BTC-200730-9000-C", received()).is_none());
    }

    #[test]
    fn non_positive_levels_are_dropped() {
        let body = r#"{"lastUpdateId":7,"bids":[["0","1"],["1000.000","0"],["999.5","2"]],"asks":[["-1","1"]]}"#;
        let snap =
            depth_snapshot_from_eapi_json(body, "BTC-200730-9000-C", received()).expect("snap");
        assert_eq!(snap.bids.len(), 1);
        assert_eq!(snap.bids[0].price, "999.5");
        assert!(snap.asks.is_empty());
        // One live side is still a bounded snapshot.
        assert!(snap.completeness);
    }

    #[test]
    fn a_dated_contract_keeps_its_case_and_a_lowercased_one_is_a_different_id() {
        let mixed =
            depth_snapshot_from_eapi_json(OFFICIAL, "BTC-200730-9000-C", received()).expect("snap");
        assert_eq!(mixed.instrument_id, "BTC-200730-9000-C");
        let smashed =
            depth_snapshot_from_eapi_json(OFFICIAL, "btc-200730-9000-c", received()).expect("snap");
        // Two ids, not one: the parser never normalizes case for this book.
        assert_ne!(mixed.instrument_id, smashed.instrument_id);
    }

    #[test]
    fn a_spot_pair_passed_as_symbol_is_still_not_lowercased() {
        // The dial gate refuses `BTCUSDT` upstream; if one arrives anyway the
        // parser must not quietly turn it into spot's `btcusdt` key.
        let snap = depth_snapshot_from_eapi_json(OFFICIAL, "BTCUSDT", received()).expect("snap");
        assert_eq!(snap.instrument_id, "BTCUSDT");
        assert_ne!(snap.instrument_id, "btcusdt");
        assert_eq!(snap.book_id, BINANCE_COM_OPTIONS_BOOK_ID);
    }

    #[test]
    fn query_always_names_the_symbol_and_the_weight_one_limit() {
        assert_eq!(
            options_depth_query("BTC-200730-9000-C"),
            "symbol=BTC-200730-9000-C&limit=50"
        );
        // Not spot's 5000.
        assert_eq!(OPTIONS_DEPTH_LIMIT, 50);
        assert_eq!(OPTIONS_DEPTH_HOST, "eapi.binance.com");
        assert_eq!(OPTIONS_DEPTH_PATH, "/eapi/v1/depth");
    }

    #[test]
    fn an_empty_symbol_never_parses() {
        assert!(depth_snapshot_from_eapi_json(OFFICIAL, "   ", received()).is_none());
    }

    #[test]
    fn junk_bodies_are_none_not_a_panic() {
        for body in ["", "null", "{}", "[]", "not json", "[1,2,3]", "0"] {
            assert!(
                depth_snapshot_from_eapi_json(body, "BTC-200730-9000-C", received()).is_none(),
                "{body:?}"
            );
        }
    }

    #[test]
    fn venue_transaction_time_stamps_as_of_but_never_the_wire_shape() {
        let snap =
            depth_snapshot_from_eapi_json(OFFICIAL, "BTC-200730-9000-C", received()).expect("snap");
        assert_eq!(snap.as_of.timestamp_millis(), 1_762_780_909_676);
        // No `T` when the venue omits it — Station's own stamp stands in.
        let no_t = r#"{"lastUpdateId":361,"bids":[["1000.000","0.1000"]],"asks":[]}"#;
        let snap = depth_snapshot_from_eapi_json(no_t, "BTC-200730-9000-C", received())
            .expect("snap without T");
        assert_eq!(snap.as_of, received());
    }
}

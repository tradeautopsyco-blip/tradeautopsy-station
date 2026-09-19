//! USDM order book from `GET /fapi/v1/depth` — a REST **bounded snapshot**.
//!
//! Not spot's managed `@depth` loop. Not Coin-M. Not eapi. Public, no HMAC.
//! Lock: `locks/binance-com-usdm.md`.

use super::binance_options_public::is_dated_option_contract;
use super::binance_usdm_ticker::normalize_usdm_instrument;
use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_USDM_BOOK_ID};
use super::kotak_depth::{DepthLevel, DepthSnapshot};
use super::tick::Transport;
use crate::egress::{EgressCall, Lane};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const USDM_DEPTH_HOST: &str = "fapi.binance.com";
pub const USDM_DEPTH_PATH: &str = "/fapi/v1/depth";
/// SDK `OrderBookParams` valid limits include 50. Weight-1 band, not spot's 5000.
pub const USDM_DEPTH_LIMIT: u32 = 50;
const USDM_DEPTH_MAX_AGE_MS: i64 = 1_000;

fn is_usdm_pair(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('|')
        && !is_dated_option_contract(id)
        && id.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn usdm_depth_query(symbol: &str) -> String {
    format!(
        "symbol={}&limit={USDM_DEPTH_LIMIT}",
        normalize_usdm_instrument(symbol)
    )
}

pub fn binance_usdm_depth_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_usdm_instrument(instrument_id);
    if !is_usdm_pair(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_USDM_BOOK_ID,
            USDM_DEPTH_HOST,
            USDM_DEPTH_PATH,
            Lane::MarketData,
        )
        .with_query(usdm_depth_query(&id))
        .with_max_age_ms(USDM_DEPTH_MAX_AGE_MS)
        .with_timeout(Duration::from_secs(15)),
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

fn level_from_pair(value: &Value) -> Option<DepthLevel> {
    let pair = value.as_array()?;
    if pair.len() < 2 {
        return None;
    }
    let price = string_scalar(&pair[0])?;
    let quantity = string_scalar(&pair[1])?;
    let price_n: f64 = price.parse().ok()?;
    let qty_n: f64 = quantity.parse().ok()?;
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

fn last_update_id(value: &Value) -> Option<u64> {
    let n = value.get("lastUpdateId")?;
    if let Some(u) = n.as_u64() {
        return Some(u);
    }
    n.as_i64().and_then(|i| u64::try_from(i).ok())
}

/// `None` — never an empty success — when the body will not parse, when
/// `lastUpdateId` is missing, or when neither side holds a usable level.
pub fn depth_snapshot_from_fapi_json(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<DepthSnapshot> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let instrument_id = normalize_usdm_instrument(symbol);
    if !is_usdm_pair(&instrument_id) {
        return None;
    }
    let sequence = last_update_id(&value)?;
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
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        book_id: BINANCE_COM_USDM_BOOK_ID.to_string(),
        bids,
        asks,
        completeness: true,
        bound_levels,
        as_of,
        transport: Transport::Rest,
        sequence: Some(sequence),
    })
}

pub async fn ensure_usdm_depth(book: Arc<Mutex<super::depthbook::DepthBook>>, instrument: &str) {
    let id = normalize_usdm_instrument(instrument);
    let Some(call) = binance_usdm_depth_call(&id) else {
        return;
    };
    {
        let depth = book.lock().expect("depthbook mutex poisoned");
        if let Some(row) = depth.get(BINANCE_COM_USDM_BOOK_ID, &id) {
            if row.completeness {
                return;
            }
        }
    }
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let Some(snapshot) = depth_snapshot_from_fapi_json(&resp.body, &id, chrono::Utc::now()) else {
        return;
    };
    book.lock()
        .expect("depthbook mutex poisoned")
        .upsert(snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::kotak_depth::depth_snapshot_from_kotak_json;
    use chrono::TimeZone;

    /// SDK websocket example result shape (REST `OrderBookResponse` aliases).
    const OFFICIAL: &str = r#"{"lastUpdateId":1027024,"E":1589436922972,"T":1589436922959,"bids":[["4.00000000","431.00000000"]],"asks":[["4.00000200","12.00000000"]]}"#;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 20, 2, 0, 0).unwrap()
    }

    #[test]
    fn official_body_is_named_usdm_book() {
        let snap = depth_snapshot_from_fapi_json(OFFICIAL, "btcusdt", received()).expect("snap");
        assert_eq!(snap.instrument_id, "BTCUSDT");
        assert_ne!(snap.instrument_id, "btcusdt");
        assert_eq!(snap.book_id, BINANCE_COM_USDM_BOOK_ID);
        assert_ne!(snap.book_id, "binance-com-spot");
        assert_ne!(snap.book_id, "binance-com-coinm");
        assert_eq!(snap.transport, Transport::Rest);
        assert_eq!(snap.sequence, Some(1_027_024));
        assert_eq!(snap.bids[0].price, "4.00000000");
        assert!(snap.bids.iter().all(|l| l.orders.is_none()));
    }

    #[test]
    fn empty_both_sides_is_none() {
        let body = r#"{"lastUpdateId":1,"bids":[],"asks":[]}"#;
        assert!(depth_snapshot_from_fapi_json(body, "BTCUSDT", received()).is_none());
    }

    #[test]
    fn missing_last_update_id_is_unusable() {
        let body = r#"{"bids":[["4","1"]],"asks":[["5","1"]]}"#;
        assert!(depth_snapshot_from_fapi_json(body, "BTCUSDT", received()).is_none());
    }

    #[test]
    fn kotak_object_levels_are_not_a_fapi_ladder() {
        let kotak = r#"{"lastUpdateId":9,"bids":[{"price":"1","quantity":"1","orders":"2"}],"asks":[]}"#;
        assert!(depth_snapshot_from_fapi_json(kotak, "BTCUSDT", received()).is_none());
        assert!(depth_snapshot_from_kotak_json(OFFICIAL, received()).is_none());
    }

    #[test]
    fn dated_contract_and_underscore_ids_never_dial() {
        assert!(binance_usdm_depth_call("BTC-260925-90000-C").is_none());
        assert!(binance_usdm_depth_call("BTCUSD_PERP").is_none());
        assert!(binance_usdm_depth_call("nse_fo|56526").is_none());
        assert!(binance_usdm_depth_call("BTCUSDT").is_some());
        assert_eq!(
            usdm_depth_query("btcusdt"),
            "symbol=BTCUSDT&limit=50"
        );
        assert_eq!(USDM_DEPTH_PATH, "/fapi/v1/depth");
        assert_eq!(USDM_DEPTH_HOST, "fapi.binance.com");
        let call = binance_usdm_depth_call("BTCUSDT").expect("call");
        assert_eq!(call.method, "GET");
        assert!(call.headers.is_empty(), "public depth must not attach HMAC headers");
        assert_ne!(call.path, "/api/v3/depth");
        assert_ne!(call.path, "/eapi/v1/depth");
        assert_ne!(call.path, "/dapi/v1/depth");
        assert_ne!(call.host, "api.binance.com");
    }

    #[test]
    fn spot_depth_stream_body_is_not_a_fapi_snapshot() {
        let stream = r#"{"e":"depthUpdate","E":1,"s":"BTCUSDT","U":1,"u":2,"b":[["4","1"]],"a":[["5","1"]]}"#;
        assert!(depth_snapshot_from_fapi_json(stream, "BTCUSDT", received()).is_none());
    }
}

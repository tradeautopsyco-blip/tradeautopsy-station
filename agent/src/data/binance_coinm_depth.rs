//! Coin-M order book from `GET /dapi/v1/depth` — a REST **bounded snapshot**.
//!
//! Not USDM. Not spot `@depth`. Not eapi. Public, no HMAC.
//! Lock: `locks/binance-com-coinm.md`. Underscore symbols keep venue case.

use super::binance_coinm_ticker::normalize_coinm_instrument;
use super::binance_options_public::is_dated_option_contract;
use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_COINM_BOOK_ID};
use super::kotak_depth::{DepthLevel, DepthSnapshot};
use super::tick::Transport;
use crate::egress::{EgressCall, Lane};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const COINM_DEPTH_HOST: &str = "dapi.binance.com";
pub const COINM_DEPTH_PATH: &str = "/dapi/v1/depth";
/// Same valid-limit band as the USDM SDK table, on this host/path only.
pub const COINM_DEPTH_LIMIT: u32 = 50;
const COINM_DEPTH_MAX_AGE_MS: i64 = 1_000;

fn is_coinm_pair(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('|')
        && !is_dated_option_contract(id)
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn coinm_depth_query(symbol: &str) -> String {
    format!(
        "symbol={}&limit={COINM_DEPTH_LIMIT}",
        normalize_coinm_instrument(symbol)
    )
}

pub fn binance_coinm_depth_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_coinm_instrument(instrument_id);
    if !is_coinm_pair(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_COINM_BOOK_ID,
            COINM_DEPTH_HOST,
            COINM_DEPTH_PATH,
            Lane::MarketData,
        )
        .with_query(coinm_depth_query(&id))
        .with_max_age_ms(COINM_DEPTH_MAX_AGE_MS)
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

pub fn depth_snapshot_from_dapi_json(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<DepthSnapshot> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let instrument_id = normalize_coinm_instrument(symbol);
    if !is_coinm_pair(&instrument_id) {
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
        book_id: BINANCE_COM_COINM_BOOK_ID.to_string(),
        bids,
        asks,
        completeness: true,
        bound_levels,
        as_of,
        transport: Transport::Rest,
        sequence: Some(sequence),
    })
}

pub async fn ensure_coinm_depth(book: Arc<Mutex<super::depthbook::DepthBook>>, instrument: &str) {
    let id = normalize_coinm_instrument(instrument);
    let Some(call) = binance_coinm_depth_call(&id) else {
        return;
    };
    {
        let depth = book.lock().expect("depthbook mutex poisoned");
        if let Some(row) = depth.get(BINANCE_COM_COINM_BOOK_ID, &id) {
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
    let Some(snapshot) = depth_snapshot_from_dapi_json(&resp.body, &id, chrono::Utc::now()) else {
        return;
    };
    book.lock()
        .expect("depthbook mutex poisoned")
        .upsert(snapshot);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_usdm_depth::depth_snapshot_from_fapi_json;
    use chrono::TimeZone;

    const OFFICIAL: &str = r#"{"lastUpdateId":1027024,"symbol":"BTCUSD_PERP","pair":"BTCUSD","E":1589436922972,"T":1589436922959,"bids":[["4.00000000","431.00000000"]],"asks":[["4.00000200","12.00000000"]]}"#;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 20, 2, 0, 0).unwrap()
    }

    #[test]
    fn official_body_is_named_coinm_book() {
        let snap =
            depth_snapshot_from_dapi_json(OFFICIAL, "btcusd_perp", received()).expect("snap");
        assert_eq!(snap.instrument_id, "BTCUSD_PERP");
        assert_eq!(snap.book_id, BINANCE_COM_COINM_BOOK_ID);
        assert_ne!(snap.book_id, "binance-com-usdm");
        assert_ne!(snap.book_id, "binance-com-spot");
        assert_eq!(snap.sequence, Some(1_027_024));
        assert!(snap.asks.iter().all(|l| l.orders.is_none()));
    }

    #[test]
    fn empty_is_unavailable() {
        assert!(depth_snapshot_from_dapi_json(
            r#"{"lastUpdateId":1,"bids":[],"asks":[]}"#,
            "BTCUSD_PERP",
            received()
        )
        .is_none());
    }

    #[test]
    fn usdm_letters_without_underscore_still_stamp_this_book_if_forced() {
        // Parser identity is the named book, not the letters. DualNoBlend is
        // book_id on the snapshot; USDM `BTCUSDT` is a different book.
        let snap = depth_snapshot_from_dapi_json(OFFICIAL, "BTCUSDT", received()).expect("snap");
        assert_eq!(snap.book_id, BINANCE_COM_COINM_BOOK_ID);
        let usdm = depth_snapshot_from_fapi_json(
            r#"{"lastUpdateId":1,"bids":[["1","1"]],"asks":[]}"#,
            "BTCUSDT",
            received(),
        )
        .expect("usdm");
        assert_eq!(usdm.book_id, crate::data::BINANCE_COM_USDM_BOOK_ID);
        assert_ne!(snap.book_id, usdm.book_id);
    }

    #[test]
    fn dated_and_cash_ids_never_dial() {
        assert!(binance_coinm_depth_call("BTC-260925-90000-C").is_none());
        assert!(binance_coinm_depth_call("nse_fo|56526").is_none());
        assert!(binance_coinm_depth_call("BTCUSD_PERP").is_some());
        assert_eq!(
            coinm_depth_query("btcusd_perp"),
            "symbol=BTCUSD_PERP&limit=50"
        );
        assert_eq!(COINM_DEPTH_PATH, "/dapi/v1/depth");
        assert_eq!(COINM_DEPTH_HOST, "dapi.binance.com");
        let call = binance_coinm_depth_call("BTCUSD_PERP").expect("call");
        assert_eq!(call.method, "GET");
        assert!(call.headers.is_empty(), "public depth must not attach HMAC headers");
        assert_ne!(call.path, "/fapi/v1/depth");
        assert_ne!(call.path, "/api/v3/depth");
        assert_ne!(call.host, "fapi.binance.com");
    }
}

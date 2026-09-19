//! Coin-M `GET /dapi/v1/klines` → `market/ohlcv` HistoricalSeries.
//!
//! Lock: `locks/binance-com-coinm.md`. SDK `kline_candlestick_data` path
//! `/dapi/v1/klines` on `dapi.binance.com`. Public. HMAC must not attach.
//! Never fapi / eapi / spot klines.

use super::binance_coinm_ticker::normalize_coinm_instrument;
use super::binance_klines::{HistoryCandle, HistorySeries, KlineRequestRefuse};
use super::binance_options_public::is_dated_option_contract;
use super::candle_builder::{apply_history_series_and_seed, CandleBuilders};
use super::descriptor::BINANCE_COM_COINM_BOOK_ID;
use super::historybook::HistoryBook;
use super::tick::Transport;
use crate::egress::{EgressCall, Lane};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const COINM_KLINES_HOST: &str = "dapi.binance.com";
pub const COINM_KLINES_PATH: &str = "/dapi/v1/klines";

/// SDK `KlineCandlestickDataIntervalEnum`. **No `1s`.**
pub const COINM_KLINE_INTERVALS: &[&str] = &[
    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w", "1M",
];

pub const DEFAULT_COINM_HISTORY_INTERVAL: &str = "1m";
const COINM_KLINES_MAX_AGE_MS: i64 = 2_000;

pub fn coinm_interval_allowed(interval: &str) -> bool {
    COINM_KLINE_INTERVALS
        .iter()
        .any(|allowed| *allowed == interval)
}

pub fn validate_coinm_kline_request(
    interval: &str,
    limit: Option<u32>,
) -> Result<(), KlineRequestRefuse> {
    if !coinm_interval_allowed(interval) {
        return Err(KlineRequestRefuse::UnsupportedInterval);
    }
    if matches!(limit, Some(0)) {
        return Err(KlineRequestRefuse::UnsupportedRange);
    }
    Ok(())
}

pub fn coinm_klines_query(symbol: &str, interval: &str) -> String {
    format!(
        "symbol={}&interval={interval}",
        normalize_coinm_instrument(symbol)
    )
}

fn is_coinm_pair(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('|')
        && !is_dated_option_contract(id)
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn binance_coinm_klines_call(instrument_id: &str, interval: &str) -> Option<EgressCall> {
    let id = normalize_coinm_instrument(instrument_id);
    if !is_coinm_pair(&id) || validate_coinm_kline_request(interval, None).is_err() {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_COINM_BOOK_ID,
            COINM_KLINES_HOST,
            COINM_KLINES_PATH,
            Lane::MarketData,
        )
        .with_query(coinm_klines_query(&id, interval))
        .with_max_age_ms(COINM_KLINES_MAX_AGE_MS)
        .with_timeout(Duration::from_secs(15)),
    )
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .or_else(|| value.as_f64().map(|n| n as i64))
}

fn json_dec(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn candle_from_live_array(row: &Value) -> Option<HistoryCandle> {
    let arr = row.as_array()?;
    if arr.len() < 7 {
        return None;
    }
    Some(HistoryCandle {
        open_time_ms: json_i64(&arr[0])?,
        open: json_dec(&arr[1])?,
        high: json_dec(&arr[2])?,
        low: json_dec(&arr[3])?,
        close: json_dec(&arr[4])?,
        volume: json_dec(&arr[5])?,
        close_time_ms: json_i64(&arr[6])?,
    })
}

pub fn series_from_dapi_klines_json(
    raw: &str,
    instrument_id: &str,
    interval: &str,
    transport: Transport,
) -> Option<HistorySeries> {
    if !coinm_interval_allowed(interval) {
        return None;
    }
    let id = normalize_coinm_instrument(instrument_id);
    if !is_coinm_pair(&id) {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;
    let rows = value.as_array()?;
    let candles: Vec<HistoryCandle> = rows.iter().filter_map(candle_from_live_array).collect();
    if candles.is_empty() {
        return None;
    }
    Some(HistorySeries {
        instrument_id: id,
        adapter_id: BINANCE_COM_COINM_BOOK_ID.to_string(),
        interval: interval.to_string(),
        candles,
        transport,
    })
}

pub async fn ensure_coinm_klines(
    book: Arc<Mutex<HistoryBook>>,
    builders: Arc<Mutex<CandleBuilders>>,
    inflight: Arc<Mutex<HashSet<String>>>,
    instrument: &str,
) {
    let id = normalize_coinm_instrument(instrument);
    let interval = DEFAULT_COINM_HISTORY_INTERVAL;
    if binance_coinm_klines_call(&id, interval).is_none() {
        return;
    }
    {
        let history = book.lock().expect("historybook mutex poisoned");
        if history
            .get(BINANCE_COM_COINM_BOOK_ID, &id, interval)
            .is_some()
        {
            return;
        }
    }
    let inflight_key = format!("{BINANCE_COM_COINM_BOOK_ID}\0{id}\0{interval}");
    {
        let mut guard = inflight.lock().expect("klines inflight poisoned");
        if !guard.insert(inflight_key) {
            return;
        }
    }
    let Some(call) = binance_coinm_klines_call(&id, interval) else {
        return;
    };
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let Some(series) = series_from_dapi_klines_json(&resp.body, &id, interval, Transport::Rest)
    else {
        return;
    };
    apply_history_series_and_seed(
        &mut book.lock().expect("historybook mutex poisoned"),
        &mut builders.lock().expect("candle builders mutex poisoned"),
        series,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIVE_ARRAY: &str = r#"[[1499040000000,"950","1100","900","1000","100",1499040059999,"2",10,"100","10000","0"]]"#;

    #[test]
    fn live_array_keeps_venue_underscore_case() {
        let series =
            series_from_dapi_klines_json(LIVE_ARRAY, "btcusd_perp", "1m", Transport::Fixture)
                .expect("dapi array");
        assert_eq!(series.instrument_id, "BTCUSD_PERP");
        assert_eq!(series.adapter_id, BINANCE_COM_COINM_BOOK_ID);
        assert_eq!(series.candles[0].close, "1000");
    }

    #[test]
    fn empty_array_is_unavailable_not_a_zero_candle() {
        assert!(series_from_dapi_klines_json("[]", "BTCUSD_PERP", "1m", Transport::Rest).is_none());
    }

    #[test]
    fn one_s_is_unsupported() {
        assert!(!coinm_interval_allowed("1s"));
        assert_eq!(
            validate_coinm_kline_request("1s", None).unwrap_err(),
            KlineRequestRefuse::UnsupportedInterval
        );
    }

    #[test]
    fn query_is_dapi_only_never_fapi() {
        let call = binance_coinm_klines_call("BTCUSD_PERP", "1m").expect("coinm pair");
        assert_eq!(call.host, COINM_KLINES_HOST);
        assert_eq!(call.path, COINM_KLINES_PATH);
        assert_ne!(call.path, "/fapi/v1/klines");
        assert_ne!(call.path, "/api/v3/klines");
        assert_ne!(call.path, "/eapi/v1/klines");
        assert!(!coinm_klines_query("BTCUSD_PERP", "1m").contains("limit="));
        assert!(binance_coinm_klines_call("BTCUSDT", "1m").is_some());
        assert!(binance_coinm_klines_call("BTC-200730-9000-C", "1m").is_none());
    }
}

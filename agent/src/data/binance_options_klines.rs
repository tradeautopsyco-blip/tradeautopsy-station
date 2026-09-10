//! Options `GET /eapi/v1/klines` → `market/ohlcv` HistoricalSeries.
//!
//! Lock: `locks/binance-com-options.md` (#70). Official HTML Response Example
//! names **object keys**. Live public GET returns a 12-slot array. This parser
//! reads official names (objects by key; arrays by the lock's live slot table).
//! It does **not** call the COM `/api/v3/klines` parser and does **not** copy
//! spot REST.md index comments.

use super::binance_klines::{HistoryCandle, HistorySeries, KlineRequestRefuse};
use super::binance_options_public::normalize_options_instrument;
use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::tick::Transport;
use serde_json::Value;

pub const OPTIONS_KLINES_HOST: &str = "eapi.binance.com";
pub const OPTIONS_KLINES_PATH: &str = "/eapi/v1/klines";

/// Official Options ENUM. **No `1s`.** Unknown interval = unsupported, not resample.
pub const OPTIONS_KLINE_INTERVALS: &[&str] = &[
    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w", "1M",
];

/// Official HTML Default:500 Max:1500.
pub const OPTIONS_KLINE_LIMIT_DEFAULT: u32 = 500;
pub const OPTIONS_KLINE_LIMIT_MAX: u32 = 1500;
pub const DEFAULT_OPTIONS_HISTORY_INTERVAL: &str = "1m";

pub fn options_klines_query(symbol: &str, interval: &str, limit: u32) -> String {
    format!(
        "symbol={}&interval={interval}&limit={limit}",
        normalize_options_instrument(symbol)
    )
}

pub fn options_interval_allowed(interval: &str) -> bool {
    OPTIONS_KLINE_INTERVALS
        .iter()
        .any(|allowed| *allowed == interval)
}

pub fn validate_options_kline_request(
    interval: &str,
    limit: Option<u32>,
) -> Result<u32, KlineRequestRefuse> {
    if !options_interval_allowed(interval) {
        return Err(KlineRequestRefuse::UnsupportedInterval);
    }
    let n = limit.unwrap_or(OPTIONS_KLINE_LIMIT_DEFAULT);
    if n == 0 || n > OPTIONS_KLINE_LIMIT_MAX {
        return Err(KlineRequestRefuse::UnsupportedRange);
    }
    Ok(n)
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

fn candle_from_named_object(row: &Value, want_interval: &str) -> Option<HistoryCandle> {
    let obj = row.as_object()?;
    if let Some(iv) = obj.get("interval").and_then(Value::as_str) {
        if iv != want_interval {
            return None;
        }
    }
    Some(HistoryCandle {
        open_time_ms: json_i64(obj.get("openTime")?)?,
        open: json_dec(obj.get("open")?)?,
        high: json_dec(obj.get("high")?)?,
        low: json_dec(obj.get("low")?)?,
        close: json_dec(obj.get("close")?)?,
        volume: json_dec(obj.get("volume")?)?,
        close_time_ms: json_i64(obj.get("closeTime")?)?,
    })
}

/// Live 12-slot row → official names (lock table). Slot 11 is unnamed.
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

fn candle_from_row(row: &Value, want_interval: &str) -> Option<HistoryCandle> {
    if row.is_object() {
        candle_from_named_object(row, want_interval)
    } else {
        candle_from_live_array(row)
    }
}

/// Empty / unparseable → `None` (unavailable). Never a fake zero candle.
pub fn series_from_eapi_klines_json(
    raw: &str,
    instrument_id: &str,
    interval: &str,
    transport: Transport,
) -> Option<HistorySeries> {
    if !options_interval_allowed(interval) {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;
    let rows = value.as_array()?;
    let candles: Vec<HistoryCandle> = rows
        .iter()
        .filter_map(|row| candle_from_row(row, interval))
        .collect();
    if candles.is_empty() {
        return None;
    }
    Some(HistorySeries {
        instrument_id: normalize_options_instrument(instrument_id),
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        interval: interval.to_string(),
        candles,
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMED: &str = r#"[{
        "open": "950",
        "high": "1100",
        "low": "900",
        "close": "1000",
        "volume": "100",
        "amount": "2",
        "interval": "1m",
        "tradeCount": 10,
        "takerVolume": "100",
        "takerAmount": "10000",
        "openTime": 1499040000000,
        "closeTime": 1499040059999
    }]"#;

    const LIVE_ARRAY: &str = r#"[[1499040000000,"950","1100","900","1000","100",1499040059999,"2",10,"100","10000","0"]]"#;

    #[test]
    fn named_object_uses_official_keys() {
        let series =
            series_from_eapi_klines_json(NAMED, "BTC-200730-9000-C", "1m", Transport::Fixture)
                .expect("named object");
        assert_eq!(series.instrument_id, "BTC-200730-9000-C");
        assert_eq!(series.candles.len(), 1);
        let c = &series.candles[0];
        assert_eq!(c.open_time_ms, 1_499_040_000_000);
        assert_eq!(c.open, "950");
        assert_eq!(c.high, "1100");
        assert_eq!(c.low, "900");
        assert_eq!(c.close, "1000");
        assert_eq!(c.volume, "100");
        assert_eq!(c.close_time_ms, 1_499_040_059_999);
    }

    #[test]
    fn live_array_aligns_to_the_same_official_names() {
        let named =
            series_from_eapi_klines_json(NAMED, "XRP-260911-1.36-C", "1m", Transport::Fixture)
                .unwrap();
        let live =
            series_from_eapi_klines_json(LIVE_ARRAY, "XRP-260911-1.36-C", "1m", Transport::Fixture)
                .unwrap();
        assert_eq!(live.instrument_id, "XRP-260911-1.36-C");
        assert_eq!(live.candles, named.candles);
    }

    #[test]
    fn mixed_case_is_not_lowercased() {
        let series =
            series_from_eapi_klines_json(NAMED, "  BTC-200730-9000-C  ", "1m", Transport::Fixture)
                .unwrap();
        assert_eq!(series.instrument_id, "BTC-200730-9000-C");
        assert_ne!(series.instrument_id, "btc-200730-9000-c");
    }

    #[test]
    fn empty_array_is_unavailable_not_a_zero_candle() {
        assert!(
            series_from_eapi_klines_json("[]", "BTC-200730-9000-C", "1m", Transport::Rest)
                .is_none()
        );
    }

    #[test]
    fn one_s_is_unsupported() {
        assert!(!options_interval_allowed("1s"));
        assert_eq!(
            validate_options_kline_request("1s", None).unwrap_err(),
            KlineRequestRefuse::UnsupportedInterval
        );
        assert!(
            series_from_eapi_klines_json(NAMED, "BTC-200730-9000-C", "1s", Transport::Fixture)
                .is_none()
        );
    }

    #[test]
    fn unknown_interval_is_unsupported_not_resample() {
        assert_eq!(
            validate_options_kline_request("2m", None).unwrap_err(),
            KlineRequestRefuse::UnsupportedInterval
        );
        assert!(OPTIONS_KLINE_INTERVALS.contains(&"1m"));
        assert!(OPTIONS_KLINE_INTERVALS.contains(&"8h"));
        assert!(OPTIONS_KLINE_INTERVALS.contains(&"1M"));
    }

    #[test]
    fn limit_max_is_1500_default_500() {
        assert_eq!(validate_options_kline_request("1m", None).unwrap(), 500);
        assert_eq!(
            validate_options_kline_request("1m", Some(1500)).unwrap(),
            1500
        );
        assert_eq!(
            validate_options_kline_request("1m", Some(1501)).unwrap_err(),
            KlineRequestRefuse::UnsupportedRange
        );
    }

    #[test]
    fn object_interval_mismatch_is_dropped() {
        let json = r#"[{"open":"1","high":"1","low":"1","close":"1","volume":"1","openTime":1,"closeTime":2,"interval":"5m"}]"#;
        assert!(
            series_from_eapi_klines_json(json, "BTC-200730-9000-C", "1m", Transport::Fixture)
                .is_none()
        );
    }

    #[test]
    fn query_preserves_mixed_case_and_never_spot_path() {
        let q = options_klines_query("BTC-200730-9000-C", "1m", 500);
        assert!(q.contains("symbol=BTC-200730-9000-C"));
        assert!(!q.contains("btcusdt"));
        assert_eq!(OPTIONS_KLINES_HOST, "eapi.binance.com");
        assert_eq!(OPTIONS_KLINES_PATH, "/eapi/v1/klines");
        assert_ne!(OPTIONS_KLINES_PATH, "/api/v3/klines");
    }
}

//! Public COM `GET /api/v3/klines` → `market/ohlcv/historical_series`.
//!
//! REST.md (re-fetched 2026-08-26): Security NONE, weight 2, host `api.binance.com`.
//! Do not attach HMAC. Do not emit `insufficient_retention` (retention NOT SPECIFIED).

use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::tick::Transport;
use serde::Serialize;
use serde_json::Value;

/// Documented interval enum (case-sensitive). Unsupported → `unsupported_interval`.
pub const KLINE_INTERVALS: &[&str] = &[
    "1s", "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w",
    "1M",
];

pub const KLINE_LIMIT_DEFAULT: u32 = 500;
pub const KLINE_LIMIT_MAX: u32 = 1000;
/// COM klines host only. Not `api.binance.us`, not `data-api.binance.vision`, not `api1`–`api4`.
#[allow(dead_code)]
pub const KLINE_COM_HOST: &str = "api.binance.com";
#[allow(dead_code)]
pub const KLINE_PATH: &str = "/api/v3/klines";
pub const DEFAULT_HISTORY_INTERVAL: &str = "1m";

/// Closed ineligible set. Emit only when true. Never `insufficient_retention` from klines.
#[allow(dead_code)]
pub const CLOSED_INELIGIBLE: &[&str] = &[
    "unsupported_interval",
    "unsupported_range",
    "insufficient_retention",
    "unhealthy",
    "exhausted",
    "rights_forbid_canonical",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KlineRequestRefuse {
    UnsupportedInterval,
    UnsupportedRange,
}

impl KlineRequestRefuse {
    pub fn as_ineligible(self) -> &'static str {
        match self {
            KlineRequestRefuse::UnsupportedInterval => "unsupported_interval",
            KlineRequestRefuse::UnsupportedRange => "unsupported_range",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryCandle {
    pub open_time_ms: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time_ms: i64,
}

/// One instrument+interval series. Not a TickBook last. Not session OHLC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySeries {
    pub instrument_id: String,
    pub adapter_id: String,
    pub interval: String,
    pub candles: Vec<HistoryCandle>,
    pub transport: Transport,
}

pub fn interval_allowed(interval: &str) -> bool {
    KLINE_INTERVALS.iter().any(|allowed| *allowed == interval)
}

/// `limit` omitted → default 500. Asking above the documented max 1000 → `unsupported_range`.
pub fn validate_kline_request(
    interval: &str,
    limit: Option<u32>,
) -> Result<u32, KlineRequestRefuse> {
    if !interval_allowed(interval) {
        return Err(KlineRequestRefuse::UnsupportedInterval);
    }
    let n = limit.unwrap_or(KLINE_LIMIT_DEFAULT);
    if n == 0 || n > KLINE_LIMIT_MAX {
        return Err(KlineRequestRefuse::UnsupportedRange);
    }
    Ok(n)
}

/// Optional `startTime` / `endTime` query pairs (UTC ms). Omitted when `None`.
/// Not an hours-between gate; `-1127` is not attached to klines.
#[allow(dead_code)]
pub fn klines_time_query(start_time_ms: Option<i64>, end_time_ms: Option<i64>) -> String {
    let mut parts = Vec::new();
    if let Some(start) = start_time_ms {
        parts.push(format!("startTime={start}"));
    }
    if let Some(end) = end_time_ms {
        parts.push(format!("endTime={end}"));
    }
    parts.join("&")
}

/// One COM `GET /api/v3/klines` URL. Does not raise `limit` and does not fetch.
#[allow(dead_code)]
pub fn klines_url(
    symbol: &str,
    interval: &str,
    limit: u32,
    start_time_ms: Option<i64>,
    end_time_ms: Option<i64>,
) -> String {
    let mut url = format!(
        "https://{KLINE_COM_HOST}{KLINE_PATH}?symbol={}&interval={}",
        symbol.trim().to_ascii_uppercase(),
        interval
    );
    let times = klines_time_query(start_time_ms, end_time_ms);
    if !times.is_empty() {
        url.push('&');
        url.push_str(&times);
    }
    url.push_str("&limit=");
    url.push_str(&limit.to_string());
    url
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

fn candle_from_array(row: &Value) -> Option<HistoryCandle> {
    let arr = row.as_array()?;
    if arr.len() < 7 {
        return None;
    }
    let open_time_ms = json_i64(&arr[0])?;
    let open = json_dec(&arr[1])?;
    let high = json_dec(&arr[2])?;
    let low = json_dec(&arr[3])?;
    let close = json_dec(&arr[4])?;
    let volume = json_dec(&arr[5])?;
    let close_time_ms = json_i64(&arr[6])?;
    let _ignore_unused = arr.get(11);
    Some(HistoryCandle {
        open_time_ms,
        open,
        high,
        low,
        close,
        volume,
        close_time_ms,
    })
}

/// Array-of-arrays klines body. Empty / unparseable → `None` (not empty success).
pub fn candles_from_klines_json(raw: &str) -> Option<Vec<HistoryCandle>> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let rows = value.as_array()?;
    let candles: Vec<HistoryCandle> = rows.iter().filter_map(candle_from_array).collect();
    if candles.is_empty() {
        None
    } else {
        Some(candles)
    }
}

/// Array-of-arrays klines body. Empty / unparseable → `None` (not empty success).
pub fn series_from_klines_json(
    raw: &str,
    instrument_id: &str,
    interval: &str,
    transport: Transport,
) -> Option<HistorySeries> {
    if !interval_allowed(interval) {
        return None;
    }
    let candles = candles_from_klines_json(raw)?;
    Some(HistorySeries {
        instrument_id: super::binance_public::normalize_quote_instrument(instrument_id),
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        interval: interval.to_string(),
        candles,
        transport,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../fixtures/binance/klines.json");

    #[test]
    fn fixture_klines_use_documented_indexes_and_last_close() {
        let series =
            series_from_klines_json(FIXTURE, "BTCUSDT", "1m", Transport::Fixture).expect("fixture");
        assert_eq!(series.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(series.instrument_id, "btcusdt");
        assert_eq!(series.interval, "1m");
        assert_eq!(series.candles.len(), 2);
        assert_eq!(series.candles[0].open_time_ms, 1_499_040_000_000);
        assert_eq!(series.candles[0].close, "0.01577100");
        assert_eq!(series.candles[0].close_time_ms, 1_499_644_799_999);
        assert_eq!(series.candles[1].close, "0.01590000");
        assert_eq!(
            series.candles.last().map(|c| c.close.as_str()),
            Some("0.01590000")
        );
        assert_eq!(series.transport, Transport::Fixture);
        assert_eq!(
            validate_kline_request("1m", None).unwrap(),
            KLINE_LIMIT_DEFAULT
        );
    }

    #[test]
    fn unsupported_interval_is_ineligible_not_retention() {
        assert_eq!(
            validate_kline_request("2m", None).unwrap_err(),
            KlineRequestRefuse::UnsupportedInterval
        );
        assert_eq!(validate_kline_request("1M", Some(10)).unwrap(), 10);
        assert!(!interval_allowed("1mo"));
        assert!(series_from_klines_json(FIXTURE, "btcusdt", "2m", Transport::Fixture).is_none());
        assert_ne!(
            KlineRequestRefuse::UnsupportedInterval.as_ineligible(),
            "insufficient_retention"
        );
    }

    #[test]
    fn limit_above_1000_is_unsupported_range() {
        assert_eq!(
            validate_kline_request("1m", Some(1001)).unwrap_err(),
            KlineRequestRefuse::UnsupportedRange
        );
        assert_eq!(
            validate_kline_request("1h", Some(1000)).unwrap(),
            KLINE_LIMIT_MAX
        );
        assert_eq!(
            KlineRequestRefuse::UnsupportedRange.as_ineligible(),
            "unsupported_range"
        );
        assert!(!CLOSED_INELIGIBLE.is_empty());
        assert!(CLOSED_INELIGIBLE.contains(&"insufficient_retention"));
    }

    #[test]
    fn empty_or_unparseable_body_is_not_empty_success_series() {
        assert!(candles_from_klines_json("[]").is_none());
        assert!(candles_from_klines_json("not-json").is_none());
        assert!(series_from_klines_json("[]", "BTCUSDT", "1m", Transport::Fixture).is_none());
    }

    #[test]
    fn optional_start_and_end_time_are_query_pairs_on_com_klines_only() {
        assert_eq!(klines_time_query(None, None), "");
        assert_eq!(
            klines_time_query(Some(1_499_040_000_000), None),
            "startTime=1499040000000"
        );
        assert_eq!(
            klines_time_query(None, Some(1_499_644_800_000)),
            "endTime=1499644800000"
        );
        assert_eq!(
            klines_time_query(Some(1_499_040_000_000), Some(1_499_644_800_000)),
            "startTime=1499040000000&endTime=1499644800000"
        );

        let with_both = klines_url(
            "btcusdt",
            "1m",
            500,
            Some(1_499_040_000_000),
            Some(1_499_644_800_000),
        );
        assert!(with_both.starts_with("https://api.binance.com/api/v3/klines?"));
        assert!(with_both.contains("symbol=BTCUSDT"));
        assert!(with_both.contains("interval=1m"));
        assert!(with_both.contains("startTime=1499040000000"));
        assert!(with_both.contains("endTime=1499644800000"));
        assert!(with_both.contains("limit=500"));
        assert!(!with_both.contains("uiKlines"));
        assert!(!with_both.contains("data-api.binance.vision"));
        assert!(!with_both.contains("api1.binance.com"));
        assert!(!with_both.contains("api.binance.us"));

        let recent = klines_url("BTCUSDT", "1m", 1000, None, None);
        assert!(!recent.contains("startTime"));
        assert!(!recent.contains("endTime"));
        assert!(recent.contains("limit=1000"));
    }
}

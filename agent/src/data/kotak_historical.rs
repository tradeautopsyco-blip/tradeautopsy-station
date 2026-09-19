//! Kotak Neo v3.0.6 `historical_data` — official REST, not the Python SDK runtime.
//!
//! Cite: `kotak-neo-python-3.0.6/neo_api_client/settings.py` PROD_URL
//! `historical_data` = `market-data/1.0/historical/details`
//! `services/historical_data.py` GET query `neosymbol` / `interval` / `fromdate` / `todate`
//! (wire keys lowercase). `Authorization` = consumer_key only. No TOTP required.
//! SDK `get_url_details`: `{host}/market-data/1.0/historical/details` — never the
//! session `/trading` prefix. Fallback host without session:
//! `https://mis.kotaksecurities.com/market-data/1.0/historical/details`.
//!
//! Refuse `mcx_fo` / `nse_com` (SDK historical_data.md). Cash v1: `nse_cm` / `bse_cm` only.
//! Empty `candles[]` is unavailable, never a zero candle, never a vendor fill.

use super::binance_klines::{HistoryCandle, HistorySeries};
use super::candle_builder::interval_ms;
use super::descriptor::{KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID};
use super::kotak_quotes::{encode_neo_symbol, is_cash_segment};
use super::source_manifest::{ObtainEnvelope, ObtainStatus};
use super::tick::Transport;
use chrono::{DateTime, NaiveDate};
use serde_json::{json, Value};

pub const HISTORICAL_DETAILS_PATH: &str = "/market-data/1.0/historical/details";
pub const DEFAULT_INTERVAL: &str = "15min";
pub const ALLOWED_INTERVALS: &[&str] = &[
    "1min", "3min", "5min", "10min", "15min", "30min", "60min", "D", "W",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeHistoryPlan {
    /// No cash id on this request — caller may use the licensed_history gap.
    Skip,
    /// `mcx_fo` / `nse_com` / non-cash on the cash book. Do not vendor-blend.
    Unsupported,
    Fetch {
        neosymbol: String,
        interval: String,
        fromdate: String,
        todate: String,
    },
}

pub fn is_kotak_historical_path(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    let lower = lower.split('?').next().unwrap_or(&lower);
    let lower = lower.trim_end_matches('/');
    lower.ends_with("/market-data/1.0/historical/details")
        || lower == "/market-data/1.0/historical/details"
}

#[cfg(test)]
pub fn historical_neosymbol_segment(path: &str) -> Option<String> {
    if !is_kotak_historical_path(path) {
        return None;
    }
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if !key.eq_ignore_ascii_case("neosymbol") {
            continue;
        }
        let decoded = value.replace("%7C", "|").replace("%7c", "|");
        let segment = decoded.split('|').next()?.trim().to_ascii_lowercase();
        if segment.is_empty() {
            return None;
        }
        return Some(segment);
    }
    None
}

pub fn normalize_interval(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    ALLOWED_INTERVALS
        .iter()
        .copied()
        .find(|allowed| *allowed == trimmed)
}

/// Cash `nse_cm|token` / `bse_cm|token` only. FO / MCX / COM → None.
pub fn cash_neosymbol(raw: &str) -> Option<String> {
    let decoded = raw.trim().replace("%7C", "|").replace("%7c", "|");
    let (segment, token) = decoded.split_once('|')?;
    let segment = segment.trim().to_ascii_lowercase();
    let token = token.trim();
    if token.is_empty() || !is_cash_segment(&segment) {
        return None;
    }
    if token.chars().any(|c| !c.is_ascii_digit()) {
        return None;
    }
    Some(format!("{segment}|{token}"))
}

pub fn refused_history_segment(raw: &str) -> bool {
    let decoded = raw.trim().replace("%7C", "|").replace("%7c", "|");
    let segment = decoded
        .split('|')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    matches!(segment.as_str(), "mcx_fo" | "nse_com")
}

fn ymd(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.len() != 10 {
        return None;
    }
    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    if !trimmed.bytes().enumerate().all(|(i, c)| {
        if i == 4 || i == 7 {
            true
        } else {
            c.is_ascii_digit()
        }
    }) {
        return None;
    }
    Some(trimmed)
}

/// Backend-enforced max days from SDK `historical_data.md` (not SDK-validated).
pub fn default_window(interval: &str, today: NaiveDate) -> (String, String) {
    let days = match interval {
        "1min" | "3min" | "5min" => 30,
        "10min" | "15min" => 60,
        "30min" | "60min" => 90,
        "D" | "W" => 180,
        _ => 60,
    };
    let from = today - chrono::Duration::days(days);
    (
        from.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
}

pub fn plan_cash_history(
    book_id: &str,
    instrument: Option<&str>,
    interval: Option<&str>,
    fromdate: Option<&str>,
    todate: Option<&str>,
    today: NaiveDate,
) -> NativeHistoryPlan {
    if book_id != KOTAK_NSE_BSE_CASH_BOOK_ID {
        return NativeHistoryPlan::Skip;
    }
    let raw = instrument.map(str::trim).filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return NativeHistoryPlan::Skip;
    };
    if refused_history_segment(raw) {
        return NativeHistoryPlan::Unsupported;
    }
    let Some(neosymbol) = cash_neosymbol(raw) else {
        return NativeHistoryPlan::Unsupported;
    };
    let interval = interval
        .and_then(normalize_interval)
        .unwrap_or(DEFAULT_INTERVAL)
        .to_string();
    let (default_from, default_to) = default_window(&interval, today);
    let fromdate = fromdate.and_then(ymd).unwrap_or(&default_from).to_string();
    let todate = todate.and_then(ymd).unwrap_or(&default_to).to_string();
    NativeHistoryPlan::Fetch {
        neosymbol,
        interval,
        fromdate,
        todate,
    }
}

pub fn query_path(neosymbol: &str, interval: &str, fromdate: &str, todate: &str) -> String {
    let encoded = encode_neo_symbol(neosymbol);
    format!(
        "{HISTORICAL_DETAILS_PATH}?neosymbol={encoded}&interval={interval}&fromdate={fromdate}&todate={todate}"
    )
}

/// SDK ISO `2026-08-20T09:15:00+0530` → UTC ms. IST is a label, never rewritten.
pub fn open_time_ms_from_iso(ts: &str) -> Option<i64> {
    let trimmed = ts.trim();
    DateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%z")
        .or_else(|_| DateTime::parse_from_rfc3339(trimmed))
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Positional row `[timestamp, open, high, low, close, volume, oi]`.
/// SDK phase-1 note: `oi` may be missing — do not require it.
pub fn parse_candle_row(row: &Value) -> Option<Value> {
    let arr = row.as_array()?;
    if arr.len() < 6 {
        return None;
    }
    let ts = arr[0].as_str()?.trim();
    if ts.is_empty() {
        return None;
    }
    let open_time_ms = open_time_ms_from_iso(ts)?;
    Some(json!({
        "open_time": ts,
        "open_time_ms": open_time_ms,
        "open": number_or_string(&arr[1])?,
        "high": number_or_string(&arr[2])?,
        "low": number_or_string(&arr[3])?,
        "close": number_or_string(&arr[4])?,
        "volume": number_or_string(&arr[5])?,
    }))
}

pub fn history_series_from_obtain_candles(
    instrument_id: &str,
    interval: &str,
    candles: &[Value],
) -> Option<HistorySeries> {
    if candles.is_empty() {
        return None;
    }
    let step = interval_ms(interval)?;
    let mut rows = Vec::with_capacity(candles.len());
    for c in candles {
        let open_time_ms = c.get("open_time_ms").and_then(|v| v.as_i64())?;
        let open = c.get("open").and_then(Value::as_str)?.to_string();
        let high = c.get("high").and_then(Value::as_str)?.to_string();
        let low = c.get("low").and_then(Value::as_str)?.to_string();
        let close = c.get("close").and_then(Value::as_str)?.to_string();
        let volume = c.get("volume").and_then(Value::as_str)?.to_string();
        rows.push(HistoryCandle {
            open_time_ms,
            open,
            high,
            low,
            close,
            volume,
            close_time_ms: open_time_ms.saturating_add(step.saturating_sub(1)),
        });
    }
    Some(HistorySeries {
        instrument_id: instrument_id.to_string(),
        adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
        interval: interval.to_string(),
        candles: rows,
        transport: Transport::Rest,
    })
}

fn number_or_string(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        let t = s.trim();
        return (!t.is_empty()).then(|| t.to_string());
    }
    if let Some(n) = v.as_f64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_i64() {
        return Some(n.to_string());
    }
    None
}

pub fn candles_from_body(body: &Value) -> Option<Vec<Value>> {
    let status = body.get("status").and_then(|s| s.as_str()).unwrap_or("");
    if !status.eq_ignore_ascii_case("success") {
        return None;
    }
    let rows = body
        .get("data")
        .and_then(|d| d.get("candles"))
        .and_then(|c| c.as_array())?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(parse_candle_row(row)?);
    }
    Some(out)
}

pub fn obtain_from_body(instrument_id: &str, interval: &str, body: &Value) -> ObtainEnvelope {
    match candles_from_body(body) {
        Some(candles) if candles.is_empty() => unavailable(instrument_id),
        Some(candles) => ObtainEnvelope {
            adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
            book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
            operation: "history".into(),
            status: ObtainStatus::Success,
            data: Some(json!({
                "instrument_id": instrument_id,
                "interval": interval,
                "candles": candles,
                "source": "kotak_neo_historical",
                "transport": "rest",
            })),
            provenance_adapter_id: Some(KOTAK_NEO_ADAPTER_ID.into()),
            provenance_path: Some(HISTORICAL_DETAILS_PATH.into()),
            product_use: None,
        },
        None => unavailable(instrument_id),
    }
}

pub fn unavailable(_instrument_id: &str) -> ObtainEnvelope {
    ObtainEnvelope {
        adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
        book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
        operation: "history".into(),
        status: ObtainStatus::Unavailable,
        data: None,
        provenance_adapter_id: Some(KOTAK_NEO_ADAPTER_ID.into()),
        provenance_path: Some(HISTORICAL_DETAILS_PATH.into()),
        product_use: None,
    }
}

pub fn unsupported_segment() -> ObtainEnvelope {
    ObtainEnvelope {
        adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
        book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
        operation: "history".into(),
        status: ObtainStatus::Unsupported,
        data: None,
        provenance_adapter_id: Some(KOTAK_NEO_ADAPTER_ID.into()),
        provenance_path: Some(HISTORICAL_DETAILS_PATH.into()),
        product_use: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_matches_sdk_prod_url() {
        assert!(is_kotak_historical_path(HISTORICAL_DETAILS_PATH));
        assert!(is_kotak_historical_path(
            "/market-data/1.0/historical/details?neosymbol=nse_cm%7C2885"
        ));
        assert!(!is_kotak_historical_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm|2885/ltp"
        ));
        assert_eq!(
            historical_neosymbol_segment(
                "/market-data/1.0/historical/details?neosymbol=nse_cm%7C1333&interval=15min"
            )
            .as_deref(),
            Some("nse_cm")
        );
    }

    #[test]
    fn cash_id_only() {
        assert_eq!(
            cash_neosymbol("nse_cm|2885").as_deref(),
            Some("nse_cm|2885")
        );
        assert_eq!(cash_neosymbol("nse_fo|61466"), None);
        assert!(refused_history_segment("mcx_fo|1"));
        assert!(refused_history_segment("nse_com|1"));
    }

    #[test]
    fn sdk_fixture_candle_parses() {
        let body = json!({
            "status": "success",
            "interval": "10min",
            "data": {"candles": [[
                "2026-08-20T09:15:00+0530", 100.0, 105.0, 99.0, 103.0, 5000, 12000
            ]]}
        });
        let env = obtain_from_body("nse_cm|1333", "10min", &body);
        assert_eq!(env.status, ObtainStatus::Success);
        assert_eq!(env.book_id, KOTAK_NSE_BSE_CASH_BOOK_ID);
        assert_eq!(env.adapter_id, KOTAK_NEO_ADAPTER_ID);
        let data = env.data.unwrap();
        let candles = data["candles"].as_array().unwrap();
        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0]["close"], "103");
        assert_eq!(candles[0]["open_time"], "2026-08-20T09:15:00+0530");
        let ms = candles[0]["open_time_ms"].as_i64().expect("utc ms");
        assert_eq!(
            ms,
            open_time_ms_from_iso("2026-08-20T09:15:00+0530").unwrap()
        );
        // 09:15 IST is 03:45 UTC — not the wall-clock IST hour as unix.
        assert_eq!(ms, 1_787_197_500_000);
    }

    #[test]
    fn empty_success_is_unavailable() {
        let body = json!({"status":"success","interval":"D","data":{"candles":[]}});
        let env = obtain_from_body("nse_cm|1333", "D", &body);
        assert_eq!(env.status, ObtainStatus::Unavailable);
        assert!(env.data.is_none());
    }

    #[test]
    fn query_uses_fromdate_not_from_date() {
        let q = query_path("nse_cm|1333", "10min", "2026-08-20", "2026-09-01");
        assert!(q.contains("fromdate=2026-08-20"));
        assert!(q.contains("todate=2026-09-01"));
        assert!(!q.contains("from_date"));
        assert!(q.contains("neosymbol=nse_cm%7C1333"));
    }

    #[test]
    fn plan_skips_without_cash_id_and_refuses_mcx() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        assert_eq!(
            plan_cash_history(KOTAK_NSE_BSE_CASH_BOOK_ID, None, None, None, None, today),
            NativeHistoryPlan::Skip
        );
        assert_eq!(
            plan_cash_history(
                KOTAK_NSE_BSE_CASH_BOOK_ID,
                Some("mcx_fo|1"),
                None,
                None,
                None,
                today
            ),
            NativeHistoryPlan::Unsupported
        );
        assert_eq!(
            plan_cash_history(
                "kotak-nse-nfo",
                Some("nse_cm|1333"),
                None,
                None,
                None,
                today
            ),
            NativeHistoryPlan::Skip
        );
        match plan_cash_history(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            Some("nse_cm|1333"),
            None,
            None,
            None,
            today,
        ) {
            NativeHistoryPlan::Fetch {
                neosymbol,
                interval,
                fromdate,
                todate,
            } => {
                assert_eq!(neosymbol, "nse_cm|1333");
                assert_eq!(interval, DEFAULT_INTERVAL);
                assert_eq!(fromdate, "2026-07-20");
                assert_eq!(todate, "2026-09-18");
            }
            other => panic!("expected fetch, got {other:?}"),
        }
    }
}

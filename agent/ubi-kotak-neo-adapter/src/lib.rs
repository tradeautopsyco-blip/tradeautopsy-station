//! `kotak_neo` adapter component (ADR 0001 · B6 `kotak_neo` · R5).
//!
//! Reads the session trade book through the host's `broker_http_call`; the session
//! token, `Sid`, and base URL live in the Enforcer's credential blob and never enter
//! component memory. v1 is equities **cash only** — F&O segments and NRML/CO/BO
//! products are refused here, not silently normalised (B6 §12).

#![allow(clippy::all)]

wit_bindgen::generate!({
    world: "broker-adapter",
    path: "../wit",
});

use crate::exports::tradeautopsy::ubi::adapter::Guest;
use crate::tradeautopsy::ubi::broker_http;
use crate::tradeautopsy::ubi::types::{
    BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent,
};

/// Placeholder host: the Enforcer rewrites this to the base URL held with the session
/// credential (R6 §3.4), so the component never learns the tenant's gateway.
const HOST: &str = "cis.kotaksecurities.com";
/// B6 §2: session trade book, no date-range params.
const TRADES_PATH: &str = "/quick/user/trades";
/// B6 §1: equities cash only in v1.
const ALLOWED_SEGMENTS: &[&str] = &["nse_cm", "bse_cm"];
/// B6 §12: CNC + MIS ingested; NRML/CO/BO refused.
const ALLOWED_PRODUCTS: &[&str] = &["CNC", "MIS"];
/// IST is UTC+05:30; the trade book prints local exchange time with no offset.
const IST_OFFSET_SECONDS: i64 = 5 * 3600 + 30 * 60;

struct KotakNeoAdapter;

export!(KotakNeoAdapter);

impl Guest for KotakNeoAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let response = broker_http::broker_http_call(&BrokerHttpRequest {
            method: "GET".to_string(),
            host: HOST.to_string(),
            path: TRADES_PATH.to_string(),
            // Trade book takes no range params — the day book is the whole response.
            query: vec![],
            headers: vec![],
            body: None,
        })?;

        let body = require_ok(&response)?;
        let mut fills = map_trade_book(&body)?;

        // Day-book only (B6 §5): the cursor can filter, never widen, the window.
        if let Some(since) = cursor.since_unix_ms {
            fills.retain(|f| f.filled_at_unix_ms >= since);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

fn require_ok(response: &BrokerHttpResponse) -> Result<String, String> {
    if let Some(class) = response.error_class.as_deref() {
        return Err(format!("kotak_neo {class} (http {})", response.status));
    }
    if response.status != 200 {
        return Err(format!("kotak_neo http {}", response.status));
    }
    Ok(response.body.clone())
}

fn map_trade_book(body: &str) -> Result<Vec<FillEvent>, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("trade book json: {e}"))?;

    // `stCode` 1003 is a dead session; the host also classifies it, this is belt-and-braces.
    if root.get("stCode").and_then(|v| v.as_i64()) == Some(1003) {
        return Err("kotak_neo session_expired".to_string());
    }
    let rows = root
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("trade book missing data[]")?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let exchange_segment = string_field(row, "exSeg");
        let product = string_field(row, "prod");
        if !is_supported_cash_row(exchange_segment.as_deref(), product.as_deref()) {
            continue;
        }

        let raw_symbol = string_field(row, "trdSym")
            .or_else(|| string_field(row, "sym"))
            .ok_or("trade book row missing trdSym")?;
        let symbol = strip_equity_suffix(&raw_symbol);
        let side = match string_field(row, "trnsTp").as_deref() {
            Some("S") | Some("s") => "SELL",
            Some("B") | Some("b") => "BUY",
            other => {
                return Err(format!(
                    "trade book row has unknown trnsTp {:?}",
                    other.unwrap_or("")
                ))
            }
        };
        let qty = parse_f64(row.get("fldQty").or_else(|| row.get("qty")))?;
        let price = parse_f64(row.get("avgPrc").or_else(|| row.get("trdPrc")))?;
        let filled_at_unix_ms = parse_trade_time(row)?;
        let trade_id = string_field(row, "exOrdId").or_else(|| string_field(row, "nOrdNo"));
        let fill_id = string_field(row, "flId")
            .or_else(|| trade_id.clone())
            .unwrap_or_else(|| format!("{symbol}-{side}-{filled_at_unix_ms}"));
        // Statutory charges are not itemised on the trade book (B6 §7) — leave unset
        // rather than fabricating a fee.
        let fee_amount = parse_f64(row.get("chrgs")).ok();

        out.push(FillEvent {
            fill_id,
            broker_slug: "kotak_neo".to_string(),
            connection_id: String::new(),
            asset_class: "equities".to_string(),
            symbol,
            side: side.to_string(),
            qty,
            price,
            currency: "INR".to_string(),
            filled_at_unix_ms,
            fee_currency: fee_amount.map(|_| "INR".to_string()),
            fee_amount,
            exchange_segment,
            product,
            trade_id,
        });
    }
    Ok(out)
}

fn is_supported_cash_row(segment: Option<&str>, product: Option<&str>) -> bool {
    let segment_ok = segment
        .map(|s| ALLOWED_SEGMENTS.contains(&s.to_ascii_lowercase().as_str()))
        .unwrap_or(false);
    let product_ok = product
        .map(|p| ALLOWED_PRODUCTS.contains(&p.to_ascii_uppercase().as_str()))
        .unwrap_or(false);
    segment_ok && product_ok
}

fn strip_equity_suffix(symbol: &str) -> String {
    let trimmed = symbol.trim();
    for suffix in ["-EQ", "-eq", "-BE", "-be"] {
        if let Some(base) = trimmed.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    trimmed.to_string()
}

/// `exTm` / `flDt`(+`flTm`) are IST with no zone marker.
/// SDK samples use month names: `22-Jan-2025 14:28:01` / `flDt=22-Jan-2025` + `flTm=14:28:16`.
/// Also accept numeric `dd-MM-yyyy HH:mm:ss` if a gateway emits it.
fn parse_trade_time(row: &serde_json::Value) -> Result<i64, String> {
    if let Some(raw) = string_field(row, "exTm") {
        return parse_ist_timestamp(&raw);
    }
    if let Some(date) = string_field(row, "flDt") {
        let time = string_field(row, "flTm").unwrap_or_else(|| "00:00:00".into());
        return parse_ist_timestamp(&format!("{date} {time}"));
    }
    Err("trade book row missing exTm/flDt".to_string())
}

fn parse_ist_timestamp(raw: &str) -> Result<i64, String> {
    let mut parts = raw.trim().split_whitespace();
    let date = parts.next().ok_or("empty timestamp")?;
    let time = parts.next().unwrap_or("00:00:00");

    let (day, month, year) = parse_date_parts(date)?;

    let mut time_parts = time.split(':');
    let hour: i64 = next_num(&mut time_parts, "hour")?;
    let minute: i64 = next_num(&mut time_parts, "minute")?;
    let second: i64 = time_parts
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format!("bad trade date '{raw}'"));
    }

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - IST_OFFSET_SECONDS;
    Ok(seconds * 1000)
}

fn parse_date_parts(date: &str) -> Result<(i64, i64, i64), String> {
    let mut date_parts = date.split('-');
    let day: i64 = next_num(&mut date_parts, "day")?;
    let month_raw = date_parts
        .next()
        .ok_or_else(|| "timestamp missing month".to_string())?
        .trim();
    let year: i64 = next_num(&mut date_parts, "year")?;
    let month = parse_month(month_raw)?;
    Ok((day, month, year))
}

fn parse_month(raw: &str) -> Result<i64, String> {
    if let Ok(n) = raw.parse::<i64>() {
        return Ok(n);
    }
    let month = match raw.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        other => return Err(format!("bad month '{other}'")),
    };
    Ok(month)
}

fn next_num<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<i64, String> {
    parts
        .next()
        .ok_or_else(|| format!("timestamp missing {label}"))?
        .trim()
        .parse()
        .map_err(|e| format!("timestamp {label}: {e}"))
}

/// Howard Hinnant's days-from-civil (proleptic Gregorian, days since 1970-01-01).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn string_field(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_f64(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or_else(|| "number not f64".to_string()),
        Some(serde_json::Value::String(s)) => {
            s.trim().parse().map_err(|e| format!("parse f64 '{s}': {e}"))
        }
        _ => Err("missing number".to_string()),
    }
}

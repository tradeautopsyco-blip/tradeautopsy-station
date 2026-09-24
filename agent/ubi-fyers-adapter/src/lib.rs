//! `fyers` adapter component (ADR 0001 · B6 `fyers` · R5).
//!
//! Day-book fills via host `broker_http_call` → `GET /api/v3/tradebook`.
//! `Authorization: <app_id>:<access_token>` is attached outside Wasm (R6). Cash book:
//! NSE/BSE CM (`segment` 10/12) + CNC/INTRADAY only. NFO book: `segment` 11 +
//! `MARGIN`/`INTRADAY` → `exchange_segment` `nse_fo` (host split parity with Kite).

#![allow(clippy::all)]

wit_bindgen::generate!({
    world: "broker-adapter-data",
    path: "../../docs/contracts",
});

use crate::exports::tradeautopsy::ubi_data::adapter::Guest as AdapterGuest;
use crate::exports::tradeautopsy::ubi_data::data_adapter::Guest as DataAdapterGuest;
use crate::tradeautopsy::ubi_data::broker_http;
use crate::tradeautopsy::ubi_data::types::{
    AssetClass, BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, InstrumentClass,
};

const HOST: &str = "api-t1.fyers.in";
const TRADEBOOK_PATH: &str = "/api/v3/tradebook";
/// Fyers appendix: NSE CM = 10, BSE CM = 12.
const ALLOWED_CASH_SEGMENTS: &[i64] = &[10, 12];
/// Fyers appendix: NSE FO = 11 (B6 row 1).
const NFO_SEGMENT_CODE: i64 = 11;
/// Host split + NFO PnL owner key off `nse_fo`, not the `NSE:` symbol prefix.
const NFO_SEGMENT: &str = "nse_fo";
/// Wire strings plus numeric appendix codes (1 = intraday, 2 = CNC).
const ALLOWED_CASH_PRODUCT_STRINGS: &[&str] = &["INTRADAY", "CNC"];
const ALLOWED_CASH_PRODUCT_CODES: &[i64] = &[1, 2];
const ALLOWED_NFO_PRODUCT_STRINGS: &[&str] = &["MARGIN", "INTRADAY"];
const ALLOWED_NFO_PRODUCT_CODES: &[i64] = &[3, 1];
/// Trade timestamps are IST with no zone marker (`DD-MM-YYYY hh:mm:ss`).
const IST_OFFSET_SECONDS: i64 = 5 * 3600 + 30 * 60;

struct FyersAdapter;

export!(FyersAdapter);

impl AdapterGuest for FyersAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let response = broker_http::broker_http_call(&BrokerHttpRequest {
            method: "GET".to_string(),
            host: HOST.to_string(),
            path: TRADEBOOK_PATH.to_string(),
            query: vec![],
            headers: vec![],
            body: None,
        })?;

        let body = require_ok(&response)?;
        let mut fills = map_tradebook(&body)?;

        if let Some(since) = cursor.since_unix_ms {
            fills.retain(|f| f.filled_at_unix_ms >= since);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for FyersAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn require_ok(response: &BrokerHttpResponse) -> Result<String, String> {
    if let Some(class) = response.error_class.as_deref() {
        if class == "session_expired" || class == "unauthorized" {
            return Err("fyers session_expired".to_string());
        }
        return Err(format!("fyers {class} (http {})", response.status));
    }
    if response.status == 401 || response.status == 403 {
        return Err("fyers session_expired".to_string());
    }
    if response.status != 200 {
        return Err(format!("fyers http {}", response.status));
    }
    Ok(response.body.clone())
}

fn map_tradebook(body: &str) -> Result<Vec<FillEvent>, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("tradebook json: {e}"))?;

    if is_auth_error(&root) {
        return Err("fyers session_expired".to_string());
    }

    let status = string_field(&root, "s").unwrap_or_default();
    if status.eq_ignore_ascii_case("error") {
        if is_auth_error(&root) {
            return Err("fyers session_expired".to_string());
        }
        let code = i64_field(&root, "code").unwrap_or(0);
        return Err(format!("fyers tradebook_error (code {code})"));
    }
    if !status.is_empty() && !status.eq_ignore_ascii_case("ok") {
        return Err(format!("fyers tradebook_not_ok ({status})"));
    }

    let rows = tradebook_rows(&root)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let nfo = is_nfo_row(row);
        if !nfo && !is_supported_cash_row(row) {
            continue;
        }
        if nfo && !is_supported_nfo_row(row) {
            continue;
        }

        let symbol = resolve_symbol(row)?;
        let side = parse_side(row)?;
        let qty = parse_f64(row.get("tradedQty"))
            .or_else(|_| parse_f64(row.get("qty_traded")))
            .or_else(|_| parse_f64(row.get("qty")))?;
        let price = parse_f64(row.get("tradePrice"))
            .or_else(|_| parse_f64(row.get("price_traded")))?;
        let filled_at_unix_ms = parse_trade_time(row)?;
        let fill_id = string_field(row, "tradeNumber")
            .or_else(|| string_field(row, "id_fill"))
            .unwrap_or_else(|| format!("{symbol}-{side}-{filled_at_unix_ms}"));
        let trade_id = string_field(row, "orderNumber").or_else(|| string_field(row, "id"));
        let exchange_segment = if nfo {
            Some(NFO_SEGMENT.to_string())
        } else {
            exchange_label(row)
        };
        let product = if nfo {
            normalize_nfo_product(row)
        } else {
            normalize_product(row)
        };

        out.push(FillEvent {
            fill_id,
            broker_slug: "fyers".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Equity,
            instrument_class: if nfo {
                nfo_instrument_class(&symbol)
            } else {
                InstrumentClass::Spot
            },
            is_inverse: false,
            symbol,
            side: side.to_string(),
            qty,
            price,
            currency: "INR".to_string(),
            filled_at_unix_ms,
            fee_currency: None,
            fee_amount: None,
            exchange_segment,
            product,
            trade_id,
        });
    }
    Ok(out)
}

fn nfo_instrument_class(symbol: &str) -> InstrumentClass {
    let upper = symbol.trim().to_ascii_uppercase();
    if upper.ends_with("FUT") {
        InstrumentClass::Future
    } else {
        InstrumentClass::Option
    }
}

fn is_nfo_row(row: &serde_json::Value) -> bool {
    i64_field(row, "segment") == Some(NFO_SEGMENT_CODE)
}

fn is_supported_nfo_row(row: &serde_json::Value) -> bool {
    nfo_product_code(row)
        .map(|code| ALLOWED_NFO_PRODUCT_CODES.contains(&code))
        .unwrap_or_else(|| {
            string_field(row, "productType")
                .or_else(|| string_field(row, "product_type"))
                .map(|p| {
                    ALLOWED_NFO_PRODUCT_STRINGS
                        .iter()
                        .any(|allowed| p.eq_ignore_ascii_case(allowed))
                })
                .unwrap_or(false)
        })
}

fn nfo_product_code(row: &serde_json::Value) -> Option<i64> {
    if let Some(code) = i64_field(row, "productType").or_else(|| i64_field(row, "product_type")) {
        return Some(code);
    }
    let s = string_field(row, "productType").or_else(|| string_field(row, "product_type"))?;
    match s.to_ascii_uppercase().as_str() {
        "INTRADAY" | "MIS" => Some(1),
        "MARGIN" | "NRML" => Some(3),
        _ => None,
    }
}

/// Map Fyers FO wire products to host `nse_fo` NRML/MIS (Kite/Kotak parity).
fn normalize_nfo_product(row: &serde_json::Value) -> Option<String> {
    match nfo_product_code(row) {
        Some(1) => Some("MIS".to_string()),
        Some(3) => Some("NRML".to_string()),
        None => {
            let s = string_field(row, "productType").or_else(|| string_field(row, "product_type"))?;
            match s.to_ascii_uppercase().as_str() {
                "INTRADAY" => Some("MIS".to_string()),
                "MARGIN" => Some("NRML".to_string()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn tradebook_rows(root: &serde_json::Value) -> Result<&[serde_json::Value], String> {
    if let Some(rows) = root.get("tradeBook").and_then(|v| v.as_array()) {
        return Ok(rows.as_slice());
    }
    if let Some(rows) = root.get("orderBook").and_then(|v| v.as_array()) {
        return Ok(rows.as_slice());
    }
    if let Some(data) = root.get("data") {
        if let Some(rows) = data.get("tradeBook").and_then(|v| v.as_array()) {
            return Ok(rows.as_slice());
        }
        if let Some(rows) = data.get("trades").and_then(|v| v.as_array()) {
            return Ok(rows.as_slice());
        }
        if let Some(rows) = data.as_array() {
            return Ok(rows.as_slice());
        }
    }
    if root.get("tradeBook").is_some() || root.get("orderBook").is_some() {
        return Err("tradebook array missing or wrong type".to_string());
    }
    Ok(&[])
}

fn resolve_symbol(row: &serde_json::Value) -> Result<String, String> {
    let raw = string_field(row, "symbol").ok_or("trade row missing symbol")?;
    if let Some((_exch, sym)) = raw.split_once(':') {
        if !sym.is_empty() {
            return Ok(sym.to_string());
        }
    }
    Ok(raw)
}

fn exchange_label(row: &serde_json::Value) -> Option<String> {
    if let Some(sym) = string_field(row, "symbol") {
        if let Some((prefix, _)) = sym.split_once(':') {
            return Some(prefix.to_string());
        }
    }
    match i64_field(row, "exchange") {
        Some(10) => Some("NSE".to_string()),
        Some(12) => Some("BSE".to_string()),
        other => other.map(|n| n.to_string()),
    }
}

fn normalize_product(row: &serde_json::Value) -> Option<String> {
    if let Some(code) = product_code(row) {
        return Some(code.to_string());
    }
    string_field(row, "productType").or_else(|| string_field(row, "product_type"))
}

fn product_code(row: &serde_json::Value) -> Option<i64> {
    if let Some(code) = i64_field(row, "productType").or_else(|| i64_field(row, "product_type")) {
        return Some(code);
    }
    let s = string_field(row, "productType").or_else(|| string_field(row, "product_type"))?;
    match s.to_ascii_uppercase().as_str() {
        "INTRADAY" | "MIS" => Some(1),
        "CNC" => Some(2),
        _ => None,
    }
}

fn is_supported_cash_row(row: &serde_json::Value) -> bool {
    let segment_ok = i64_field(row, "segment")
        .map(|seg| ALLOWED_CASH_SEGMENTS.contains(&seg))
        .unwrap_or(false);
    let product_ok = product_code(row)
        .map(|code| ALLOWED_CASH_PRODUCT_CODES.contains(&code))
        .unwrap_or_else(|| {
            string_field(row, "productType")
                .or_else(|| string_field(row, "product_type"))
                .map(|p| {
                    ALLOWED_CASH_PRODUCT_STRINGS
                        .iter()
                        .any(|allowed| p.eq_ignore_ascii_case(allowed))
                })
                .unwrap_or(false)
        });
    segment_ok && product_ok
}

fn parse_side(row: &serde_json::Value) -> Result<&'static str, String> {
    if let Some(side) = i64_field(row, "side") {
        return match side {
            1 => Ok("BUY"),
            -1 => Ok("SELL"),
            other => Err(format!("trade row has unknown side code {other}")),
        };
    }
    match string_field(row, "side").as_deref() {
        Some("BUY") | Some("buy") | Some("1") => Ok("BUY"),
        Some("SELL") | Some("sell") | Some("-1") => Ok("SELL"),
        other => Err(format!(
            "trade row has unknown side {:?}",
            other.unwrap_or("")
        )),
    }
}

fn is_auth_error(root: &serde_json::Value) -> bool {
    if string_field(root, "s").is_some_and(|s| s.eq_ignore_ascii_case("error")) {
        if let Some(code) = i64_field(root, "code") {
            if matches!(code, -8 | -15 | -16 | -17 | 401 | 403) {
                return true;
            }
        }
        if let Some(msg) = string_field(root, "message") {
            let upper = msg.to_ascii_uppercase();
            if upper.contains("TOKEN") || upper.contains("AUTH") || upper.contains("INVALID") {
                return true;
            }
        }
    }
    false
}

fn parse_trade_time(row: &serde_json::Value) -> Result<i64, String> {
    if let Some(raw) = string_field(row, "orderDateTime") {
        return parse_ist_timestamp(&raw);
    }
    if let Some(raw) = string_field(row, "fill_time") {
        return parse_ist_timestamp(&raw);
    }
    if let Some(epoch) = i64_field(row, "executionTime") {
        return Ok(epoch * 1000);
    }
    Err("trade row missing orderDateTime/fill_time/executionTime".to_string())
}

fn parse_ist_timestamp(raw: &str) -> Result<i64, String> {
    let trimmed = raw.trim();
    if let Ok(ms) = parse_dd_mm_yyyy(trimmed) {
        return Ok(ms);
    }
    parse_iso_date_time(trimmed)
}

fn parse_iso_date_time(raw: &str) -> Result<i64, String> {
    let mut parts = raw.split_whitespace();
    let date = parts.next().ok_or("empty timestamp")?;
    let time = parts.next().unwrap_or("00:00:00");

    let mut date_parts = date.split('-');
    let year: i64 = next_num(&mut date_parts, "year")?;
    let month: i64 = next_num(&mut date_parts, "month")?;
    let day: i64 = next_num(&mut date_parts, "day")?;

    let mut time_parts = time.split(':');
    let hour: i64 = next_num(&mut time_parts, "hour")?;
    let minute: i64 = next_num(&mut time_parts, "minute")?;
    let second: i64 = time_parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format!("bad trade date '{raw}'"));
    }

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - IST_OFFSET_SECONDS;
    Ok(seconds * 1000)
}

fn parse_dd_mm_yyyy(raw: &str) -> Result<i64, String> {
    let mut parts = raw.split_whitespace();
    let date = parts.next().ok_or("empty timestamp")?;
    let time = parts.next().unwrap_or("00:00:00");
    let mut date_parts = date.split('-');
    let day: i64 = next_num(&mut date_parts, "day")?;
    let month: i64 = next_num(&mut date_parts, "month")?;
    let year: i64 = next_num(&mut date_parts, "year")?;

    let mut time_parts = time.split(':');
    let hour: i64 = next_num(&mut time_parts, "hour")?;
    let minute: i64 = next_num(&mut time_parts, "minute")?;
    let second: i64 = time_parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - IST_OFFSET_SECONDS;
    Ok(seconds * 1000)
}

fn next_num<'a>(parts: &mut impl Iterator<Item = &'a str>, label: &str) -> Result<i64, String> {
    parts
        .next()
        .ok_or_else(|| format!("timestamp missing {label}"))?
        .trim()
        .parse()
        .map_err(|e| format!("timestamp {label}: {e}"))
}

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

fn i64_field(row: &serde_json::Value, key: &str) -> Option<i64> {
    row.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        serde_json::Value::String(s) => s.trim().parse().ok(),
        _ => None,
    })
}

fn parse_f64(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(serde_json::Value::Number(n)) => {
            n.as_f64().ok_or_else(|| "number not f64".to_string())
        }
        Some(serde_json::Value::String(s)) => s
            .trim()
            .parse()
            .map_err(|e| format!("parse f64 '{s}': {e}")),
        _ => Err("missing number".to_string()),
    }
}

fn describe_json() -> String {
    serde_json::json!({
        "manifest_id": "tradeautopsy:fyers-cash@0.1.0",
        "adapter_id": "fyers",
        "implemented": ["tradebook"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "fyers",
                "family": "account",
                "capability_id": "fills",
                "physics": "bounded_snapshot",
            },
        ],
    })
    .to_string()
}

fn obtain_json(request: &str) -> Result<String, String> {
    let req: serde_json::Value =
        serde_json::from_str(request).map_err(|e| format!("obtain json: {e}"))?;
    let family = json_str(&req, &["family"]).unwrap_or_default();
    let capability = json_str(&req, &["capability-id", "capability_id"]).unwrap_or_default();
    let physics = json_str(&req, &["physics"]).unwrap_or_default();
    let operation = json_str(&req, &["operation"]).unwrap_or_default();
    let is_history = operation == "history"
        || (family == "market" && capability == "ohlcv" && physics == "historical_series");
    if is_history {
        return Ok(serde_json::json!({
            "status": "unsupported",
            "operation": "history",
            "reason": "historical candles are not user trade history",
        })
        .to_string());
    }
    Err(format!(
        "fyers obtain unsupported: {family}/{capability}/{physics}"
    ))
}

fn json_str(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

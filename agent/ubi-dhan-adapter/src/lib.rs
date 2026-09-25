//! `dhan` adapter component (ADR 0001 · B6 `dhan` · R5).
//!
//! Day-book fills via host `broker_http_call` → `GET /v2/trades` on `api.dhan.co`.
//! `access-token` is attached outside Wasm (R6); `client-id` is a marketfeed-only
//! header and never appears here (B6 row 10, D2). First book: NSE_EQ/BSE_EQ cash,
//! products CNC + INTRADAY-as-MIS only; MARGIN/CO/BO/MTF, non-cash segments,
//! crossCurrency rows, and unmapped segments are skipped in the mapper (B6 rows
//! 1/7/8/12/21). Timestamps are IST with no zone marker (`YYYY-MM-DD HH:MM:SS`);
//! `exchangeTime` is the fill clock — `createTime`/`updateTime` may be `"NA"` and
//! are never read (B6 row 8).
//!
//! Ranged history (`GET /v2/trades/{from}/{to}/{page}`, B6 row 5) shares this row
//! mapper — ranged rows only add per-trade charge fields — but date-range cursor /
//! backfill over it belongs to the Z8 cursor-law lane; `fetch_fills` polls the day
//! book only (sibling parity), and the mapper already aggregates ranged charge
//! fields so no mapper change is needed when that lane lands.

#![allow(clippy::all)]
#![cfg_attr(test, allow(dead_code, unused_imports))]

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

const HOST: &str = "api.dhan.co";
const TRADES_PATH: &str = "/v2/trades";
/// Cash segments, matched case-insensitively but stamped verbatim (B6 row 8).
const ALLOWED_CASH_SEGMENTS: &[&str] = &["NSE_EQ", "BSE_EQ"];
/// NFO wire segment (B6 annexure); host split uses `nse_fo` (lock `dhan-nse-nfo`).
const ALLOWED_NFO_SEGMENTS: &[&str] = &["NSE_FNO"];
const NFO_HOST_SEGMENT: &str = "nse_fo";
/// Per-trade charge fields present on ranged rows only (B6 row 8). Day rows carry
/// no charges — Station never invents a fee there. Currency unit is NOT SPECIFIED
/// IN SOURCE (B6 row 7); the cash book assumes INR at insert (Z9 lock).
const CHARGE_FIELDS: &[&str] = &[
    "sebiTax",
    "stt",
    "brokerageCharges",
    "serviceTax",
    "exchangeTransactionCharges",
    "stampDuty",
];
/// Trade timestamps are IST with no zone marker (`YYYY-MM-DD HH:MM:SS`).
const IST_OFFSET_SECONDS: i64 = 5 * 3600 + 30 * 60;

struct DhanAdapter;

// `export!` + the `AdapterGuest` impl reference the `broker-http` Wasm import,
// which has no host-side symbol when unit tests link natively. Gate them out under
// `cfg(test)` so the pure mapper stays unit-testable; the Wasm contract (fetch,
// describe, obtain) is proven by `agent/tests/ubi_dhan_component.rs`.
#[cfg(not(test))]
export!(DhanAdapter);

#[cfg(not(test))]
impl AdapterGuest for DhanAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let response = broker_http::broker_http_call(&BrokerHttpRequest {
            method: "GET".to_string(),
            host: HOST.to_string(),
            path: TRADES_PATH.to_string(),
            query: vec![],
            headers: vec![],
            body: None,
        })?;

        let body = require_ok(&response)?;
        let mut fills = map_trades_day_book(&body)?;

        if let Some(since) = cursor.since_unix_ms {
            fills.retain(|f| f.filled_at_unix_ms >= since);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for DhanAdapter {
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
            return Err("dhan session_expired".to_string());
        }
        return Err(format!("dhan {class} (http {})", response.status));
    }
    if response.status == 401 || response.status == 403 {
        return Err("dhan session_expired".to_string());
    }
    if response.status != 200 {
        return Err(format!("dhan http {}", response.status));
    }
    Ok(response.body.clone())
}

fn map_trades_day_book(body: &str) -> Result<Vec<FillEvent>, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("trades json: {e}"))?;

    if is_session_error(&root) {
        return Err("dhan session_expired".to_string());
    }
    if let Some(venue_err) = venue_error(&root) {
        return Err(venue_err);
    }

    let rows = trade_rows(&root)?;
    let mut out = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if let Some(fill) = map_trade_row(row, index)? {
            out.push(fill);
        }
    }
    Ok(out)
}

/// One Dhan trade row (day or ranged shape) → one cash `FillEvent`, or `None`
/// when the row is refused by the cash-book gates. Refusals are silent skips of
/// that row (sibling parity); malformed *accepted* rows fail loudly instead of
/// inventing values.
fn map_trade_row(row: &serde_json::Value, index: usize) -> Result<Option<FillEvent>, String> {
    if is_nfo_wire_segment(row) {
        return map_nfo_trade_row(row, index);
    }

    // Gate 1 — cash segments only, stamped verbatim. Unmapped segments are
    // refused visibly (row dropped, never null-symbol), never defaulted.
    let exchange_segment = match string_field(row, "exchangeSegment") {
        Some(seg)
            if ALLOWED_CASH_SEGMENTS
                .iter()
                .any(|allowed| seg.eq_ignore_ascii_case(allowed)) =>
        {
            seg
        }
        _ => return Ok(None),
    };

    // Gate 2 — cash products only, named explicitly. B6 row 8 warns the OpenAlgo
    // oracle defaults unknown → INTRADAY; Station refuses instead. INTRADAY maps
    // to Station's intraday label `MIS` (same coalition as Kite MIS); CNC stays.
    let product = match string_field(row, "productType")
        .map(|p| p.to_ascii_uppercase())
        .as_deref()
    {
        Some("CNC") => "CNC".to_string(),
        Some("INTRADAY") => "MIS".to_string(),
        _ => return Ok(None), // MARGIN/CO/BO/MTF/unknown — refused on cash (B6 rows 12/21).
    };

    // Gate 3 — non-INR legs never touch the INR cash strip (B6 row 7).
    if row.get("crossCurrency").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(None);
    }

    let symbol = resolve_symbol(row)?;
    let side = match string_field(row, "transactionType").as_deref() {
        Some("BUY") | Some("buy") => "BUY",
        Some("SELL") | Some("sell") => "SELL",
        other => {
            return Err(format!(
                "trade row has unknown transactionType {:?}",
                other.unwrap_or("")
            ))
        }
    };
    let qty = parse_f64(row.get("tradedQuantity"))
        .map_err(|_| "trade row missing tradedQuantity".to_string())?;
    let price = parse_f64(row.get("tradedPrice"))
        .map_err(|_| "trade row missing tradedPrice".to_string())?;
    let filled_at_unix_ms = parse_trade_time(row)?;

    // fill_id: venue trade id preferred; fallback is deterministic and unique per
    // response (`{orderId}#{row index}`), last resort symbol-side-time. Documented
    // choice: `exchangeTradeId` is the only per-trade venue key (B6 row 8); the
    // orderId+index fallback keeps fills distinct when it is absent without
    // inventing a venue id.
    let order_id = string_field(row, "orderId");
    let fill_id = string_field(row, "exchangeTradeId")
        .or_else(|| string_number_field(row, "exchangeTradeId"))
        .or_else(|| order_id.clone().map(|id| format!("{id}#{index}")))
        .unwrap_or_else(|| format!("{symbol}-{side}-{filled_at_unix_ms}"));
    let trade_id = order_id.or_else(|| string_field(row, "exchangeOrderId"));

    let (fee_amount, fee_currency) = parse_charges(row)?;

    out_fill(
        fill_id,
        symbol,
        side,
        qty,
        price,
        filled_at_unix_ms,
        fee_amount,
        fee_currency,
        exchange_segment,
        product,
        trade_id,
        InstrumentClass::Spot,
    )
    .map(Some)
}

fn is_nfo_wire_segment(row: &serde_json::Value) -> bool {
    string_field(row, "exchangeSegment")
        .map(|seg| {
            ALLOWED_NFO_SEGMENTS
                .iter()
                .any(|allowed| seg.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false)
}

fn map_nfo_trade_row(row: &serde_json::Value, index: usize) -> Result<Option<FillEvent>, String> {
    if row.get("crossCurrency").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(None);
    }

    let product = match string_field(row, "productType")
        .map(|p| p.to_ascii_uppercase())
        .as_deref()
    {
        Some("MARGIN") => "NRML".to_string(),
        Some("INTRADAY") => "MIS".to_string(),
        _ => return Ok(None),
    };

    let symbol = resolve_symbol(row)?;
    let side = match string_field(row, "transactionType").as_deref() {
        Some("BUY") | Some("buy") => "BUY",
        Some("SELL") | Some("sell") => "SELL",
        other => {
            return Err(format!(
                "nfo trade row has unknown transactionType {:?}",
                other.unwrap_or("")
            ))
        }
    };
    let qty = parse_f64(row.get("tradedQuantity"))
        .map_err(|_| "nfo trade row missing tradedQuantity".to_string())?;
    let price = parse_f64(row.get("tradedPrice"))
        .map_err(|_| "nfo trade row missing tradedPrice".to_string())?;
    let filled_at_unix_ms = parse_trade_time(row)?;

    let order_id = string_field(row, "orderId");
    let fill_id = string_field(row, "exchangeTradeId")
        .or_else(|| string_number_field(row, "exchangeTradeId"))
        .or_else(|| order_id.clone().map(|id| format!("{id}#{index}")))
        .unwrap_or_else(|| format!("{symbol}-{side}-{filled_at_unix_ms}"));
    let trade_id = order_id.or_else(|| string_field(row, "exchangeOrderId"));

    let instrument_class = nfo_instrument_class(&symbol);

    out_fill(
        fill_id,
        symbol,
        side,
        qty,
        price,
        filled_at_unix_ms,
        None,
        None,
        NFO_HOST_SEGMENT.to_string(),
        product,
        trade_id,
        instrument_class,
    )
    .map(Some)
}

fn nfo_instrument_class(symbol: &str) -> InstrumentClass {
    let upper = symbol.trim().to_ascii_uppercase();
    if upper.ends_with("FUT") {
        InstrumentClass::Future
    } else {
        InstrumentClass::Option
    }
}

#[allow(clippy::too_many_arguments)]
fn out_fill(
    fill_id: String,
    symbol: String,
    side: &str,
    qty: f64,
    price: f64,
    filled_at_unix_ms: i64,
    fee_amount: Option<f64>,
    fee_currency: Option<String>,
    exchange_segment: String,
    product: String,
    trade_id: Option<String>,
    instrument_class: InstrumentClass,
) -> Result<FillEvent, String> {
    Ok(FillEvent {
        fill_id,
        broker_slug: "dhan".to_string(),
        connection_id: String::new(),
        // Placeholders only: the host stamps asset/instrument axes from the
        // shipping book (ADR 0004). NFO rows set option/future hint from symbol.
        asset_class: AssetClass::Equity,
        instrument_class,
        is_inverse: false,
        symbol,
        side: side.to_string(),
        qty,
        price,
        currency: "INR".to_string(),
        filled_at_unix_ms,
        fee_amount,
        fee_currency,
        exchange_segment: Some(exchange_segment),
        product: Some(product),
        trade_id,
    })
}

fn resolve_symbol(row: &serde_json::Value) -> Result<String, String> {
    // `tradingSymbol` primary; `customSymbol` then `securityId` fallbacks (B6 row 8
    // vocabulary). Never null-symbol: all-missing fails loudly.
    if let Some(sym) = string_field(row, "tradingSymbol") {
        return Ok(sym);
    }
    if let Some(sym) = string_field(row, "customSymbol") {
        return Ok(sym);
    }
    if let Some(id) = string_number_field(row, "securityId") {
        return Ok(id);
    }
    Err("trade row missing tradingSymbol/customSymbol/securityId".to_string())
}

/// Day rows carry no charges → `(None, None)`, never an invented zero. Ranged rows
/// aggregate every present charge field into one INR fee; a present-but-unparseable
/// charge fails loudly (a wrong fee is worse than a visible error).
fn parse_charges(row: &serde_json::Value) -> Result<(Option<f64>, Option<String>), String> {
    let mut total = 0.0;
    let mut seen = 0u32;
    for key in CHARGE_FIELDS {
        if let Some(value) = row.get(*key) {
            if matches!(value, serde_json::Value::Null) {
                continue;
            }
            let amount = parse_f64(Some(value))
                .map_err(|_| format!("trade row has unparseable charge field '{key}'"))?;
            total += amount;
            seen += 1;
        }
    }
    if seen == 0 {
        Ok((None, None))
    } else {
        Ok((Some(total), Some("INR".to_string())))
    }
}

fn trade_rows(root: &serde_json::Value) -> Result<&[serde_json::Value], String> {
    // Official success envelope is `{status: success, data: [...]}` (B6 row 4);
    // tolerate a bare array too. Missing/null `data` on success is a quiet day —
    // zero fills, never an error (sibling parity).
    if let Some(rows) = root.as_array() {
        return Ok(rows.as_slice());
    }
    match root.get("data") {
        None | Some(serde_json::Value::Null) => Ok(&[]),
        Some(serde_json::Value::Array(rows)) => Ok(rows.as_slice()),
        // `{status: failed|error, data: {code: msg}}` is handled by `venue_error`
        // above; reaching here means success-with-object-data, not a trade list.
        Some(serde_json::Value::Object(_)) => Err("trades data is not an array".to_string()),
        _ => Err("trades missing data[]".to_string()),
    }
}

/// Dead/expired token per B6 row 9: trading `DH-901`, data-API `807`/`808`/`809`
/// (+`810` auth failure). Surfaces as `session_expired` → reconnect, never Kill.
fn is_session_error(root: &serde_json::Value) -> bool {
    for code in session_codes(root) {
        let upper = code.to_ascii_uppercase();
        if upper == "DH-901" || matches!(upper.as_str(), "807" | "808" | "809" | "810") {
            return true;
        }
    }
    false
}

fn session_codes(root: &serde_json::Value) -> Vec<String> {
    let mut codes = Vec::new();
    if let Some(code) = string_number_field(root, "errorCode") {
        codes.push(code);
    }
    if let Some(remarks) = root.get("remarks") {
        if let Some(code) =
            string_number_field(remarks, "error_code").or_else(|| string_number_field(remarks, "errorCode"))
        {
            codes.push(code);
        }
    }
    if let Some(data) = root.get("data") {
        if let Some(code) = string_number_field(data, "code") {
            codes.push(code);
        }
    }
    codes
}

/// Non-session venue failures, surfaced visibly (never empty success):
/// `{status: failure, remarks: {...}}`, `{status: failed|error, data: {code: msg}}`,
/// and `{errorType, errorCode, errorMessage}`.
fn venue_error(root: &serde_json::Value) -> Option<String> {
    let status = string_field(root, "status").unwrap_or_default();
    let failed = status.eq_ignore_ascii_case("failure")
        || status.eq_ignore_ascii_case("failed")
        || status.eq_ignore_ascii_case("error");
    // The intro error shape `{errorType, errorCode, errorMessage}` carries no
    // status — without this, a non-session error (e.g. DH-904) would look like a
    // quiet day. Success-with-data is never an error.
    let error_shape = root.get("errorCode").is_some()
        || root.get("errorType").is_some()
        || root.get("remarks").is_some();
    let success = status.eq_ignore_ascii_case("success");
    if !failed && (success || !error_shape) {
        return None;
    }
    let detail = string_number_field(root, "errorCode")
        .map(|c| format!("errorCode {c}"))
        .or_else(|| {
            root.get("remarks").and_then(|r| {
                string_number_field(r, "error_code")
                    .or_else(|| string_number_field(r, "errorCode"))
                    .map(|c| format!("remarks error_code {c}"))
            })
        })
        .or_else(|| {
            string_field(root, "errorMessage")
                .or_else(|| string_field(root, "error_message"))
                .map(|m| format!("error {m}"))
        })
        .unwrap_or_else(|| format!("status {status}"));
    Some(format!("dhan trades_error ({detail})"))
}

fn parse_trade_time(row: &serde_json::Value) -> Result<i64, String> {
    // `exchangeTime` only. `createTime`/`updateTime` may be `"NA"` (B6 row 8) and
    // are never consulted — an `"NA"` exchangeTime fails loudly, never silently.
    match string_field(row, "exchangeTime") {
        Some(raw) if !raw.eq_ignore_ascii_case("NA") => parse_ist_timestamp(&raw),
        _ => Err("trade row missing usable exchangeTime (createTime/updateTime may be NA)".to_string()),
    }
}

fn parse_ist_timestamp(raw: &str) -> Result<i64, String> {
    let mut parts = raw.trim().split_whitespace();
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

fn string_number_field(row: &serde_json::Value, key: &str) -> Option<String> {
    if let Some(s) = string_field(row, key) {
        return Some(s);
    }
    row.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => Some(n.to_string()),
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
        "manifest_id": "tradeautopsy:dhan-cash@0.1.0",
        "adapter_id": "dhan",
        "implemented": ["tradebook"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "dhan",
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
        "dhan obtain unsupported: {family}/{capability}/{physics}"
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

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_BOOK: &str = include_str!("../../fixtures/dhan/trades_day_book.json");
    const RANGED_PAGE: &str = include_str!("../../fixtures/dhan/trades_ranged_page.json");

    /// 2026-07-25 10:15:30 IST → 04:45:30 UTC.
    const ITBEES_FILLED_AT_MS: i64 = 1_784_954_730_000;
    /// 2026-07-25 12:00:00 IST → 06:30:00 UTC.
    const SBIN_FILLED_AT_MS: i64 = 1_784_961_000_000;
    /// 2026-07-25 14:05:00 IST → 08:35:00 UTC.
    const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;
    /// 2026-07-25 15:30:00 IST → 10:00:00 UTC.
    const INFY_FILLED_AT_MS: i64 = 1_784_973_600_000;

    fn sorted(mut fills: Vec<FillEvent>) -> Vec<FillEvent> {
        fills.sort_by_key(|f| f.filled_at_unix_ms);
        fills
    }

    #[test]
    fn day_book_maps_cash_rows_and_refuses_the_rest() {
        let fills = sorted(map_trades_day_book(DAY_BOOK).expect("day book maps"));
        assert_eq!(
            fills.len(),
            5,
            "4 cash + 1 NFO (MARGIN→NRML); EQ MARGIN, crossCurrency, IDX_I still dropped"
        );

        let itbees = &fills[0];
        assert_eq!(itbees.fill_id, "900001");
        assert_eq!(itbees.broker_slug, "dhan");
        assert_eq!(itbees.symbol, "ITBEES");
        assert_eq!(itbees.side, "BUY");
        assert!((itbees.qty - 100.0).abs() < 1e-9);
        assert!((itbees.price - 25.5).abs() < 1e-9);
        assert_eq!(itbees.currency, "INR");
        assert_eq!(itbees.exchange_segment.as_deref(), Some("NSE_EQ"));
        assert_eq!(itbees.product.as_deref(), Some("CNC"));
        assert_eq!(itbees.trade_id.as_deref(), Some("100001"));
        assert_eq!(itbees.filled_at_unix_ms, ITBEES_FILLED_AT_MS);
        // Day rows carry no charges — no invented fee.
        assert!(itbees.fee_amount.is_none());
        assert!(itbees.fee_currency.is_none());

        let nfo = fills
            .iter()
            .find(|f| f.symbol == "NIFTY26JUL24000CE")
            .expect("NFO row maps");
        assert_eq!(nfo.exchange_segment.as_deref(), Some("nse_fo"));
        assert_eq!(nfo.product.as_deref(), Some("MIS"));

        let sbin = fills.iter().find(|f| f.symbol == "SBIN").expect("sbin");
        assert_eq!(sbin.side, "BUY");
        assert_eq!(sbin.exchange_segment.as_deref(), Some("BSE_EQ"));
        assert_eq!(sbin.filled_at_unix_ms, SBIN_FILLED_AT_MS);

        let reliance = fills.iter().find(|f| f.symbol == "RELIANCE").expect("reliance");
        assert_eq!(reliance.side, "SELL");
        assert_eq!(reliance.product.as_deref(), Some("MIS"));
        assert_eq!(reliance.fill_id, "900002");

        let infy = fills.iter().find(|f| f.symbol == "INFY").expect("infy");
        assert_eq!(infy.fill_id, "100008#7");
        assert_eq!(infy.filled_at_unix_ms, INFY_FILLED_AT_MS);

        assert!(!fills.iter().any(|f| f.symbol == "TCS"
            || f.symbol == "USDINR"
            || f.symbol == "NIFTY 50"));
    }

    #[test]
    fn ranged_rows_aggregate_charge_fields_into_one_inr_fee() {
        let fills = sorted(map_trades_day_book(RANGED_PAGE).expect("ranged page maps"));
        assert_eq!(fills.len(), 2);

        let full = &fills[0];
        assert_eq!(full.symbol, "ITBEES");
        assert_eq!(full.product.as_deref(), Some("CNC"));
        // 0.05 + 12.50 + 17.70 + 3.19 + 8.85 + 2.50 = 44.79.
        assert!((full.fee_amount.unwrap_or(-1.0) - 44.79).abs() < 1e-9);
        assert_eq!(full.fee_currency.as_deref(), Some("INR"));

        let partial = &fills[1];
        assert_eq!(partial.symbol, "RELIANCE");
        assert_eq!(partial.product.as_deref(), Some("MIS"));
        // Only stt + brokerageCharges present: 31.25 + 17.70 = 48.95.
        assert!((partial.fee_amount.unwrap_or(-1.0) - 48.95).abs() < 1e-9);
        assert_eq!(partial.fee_currency.as_deref(), Some("INR"));
    }

    #[test]
    fn symbol_falls_back_through_custom_symbol_to_security_id() {
        let body = r#"{"status":"success","data":[
            {"exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"BUY",
             "tradedQuantity":1,"tradedPrice":10.0,"exchangeTime":"2026-07-25 12:00:00",
             "customSymbol":"CUSTOM","securityId":999},
            {"exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"BUY",
             "tradedQuantity":1,"tradedPrice":10.0,"exchangeTime":"2026-07-25 12:00:00",
             "securityId":12345}
        ]}"#;
        let fills = map_trades_day_book(body).expect("fallbacks map");
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].symbol, "CUSTOM");
        assert_eq!(fills[1].symbol, "12345");
    }

    #[test]
    fn missing_symbol_fails_loudly_never_null() {
        let body = r#"{"status":"success","data":[
            {"exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"BUY",
             "tradedQuantity":1,"tradedPrice":10.0,"exchangeTime":"2026-07-25 12:00:00"}
        ]}"#;
        let err = map_trades_day_book(body).expect_err("null symbol must fail");
        assert!(err.contains("tradingSymbol"), "unexpected: {err}");
    }

    #[test]
    fn na_exchange_time_fails_loudly() {
        let body = r#"{"status":"success","data":[
            {"tradingSymbol":"SBIN","exchangeSegment":"BSE_EQ","productType":"CNC",
             "transactionType":"BUY","tradedQuantity":1,"tradedPrice":10.0,
             "exchangeTime":"NA","createTime":"NA","updateTime":"NA"}
        ]}"#;
        let err = map_trades_day_book(body).expect_err("NA clock must fail");
        assert!(err.contains("exchangeTime"), "unexpected: {err}");
    }

    #[test]
    fn unknown_side_and_missing_numbers_fail_loudly() {
        for (label, row) in [
            ("side", r#"{"tradingSymbol":"X","exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"HOLD","tradedQuantity":1,"tradedPrice":1.0,"exchangeTime":"2026-07-25 12:00:00"}"#),
            ("qty", r#"{"tradingSymbol":"X","exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"BUY","tradedPrice":1.0,"exchangeTime":"2026-07-25 12:00:00"}"#),
            ("price", r#"{"tradingSymbol":"X","exchangeSegment":"NSE_EQ","productType":"CNC","transactionType":"BUY","tradedQuantity":1,"exchangeTime":"2026-07-25 12:00:00"}"#),
        ] {
            let body = format!(r#"{{"status":"success","data":[{row}]}}"#);
            map_trades_day_book(&body).expect_err(&format!("{label} must fail loudly"));
        }
    }

    #[test]
    fn unknown_product_is_refused_not_defaulted_to_intraday() {
        // B6 row 8: the oracle defaults unknown → INTRADAY; Station refuses.
        let body = r#"{"status":"success","data":[
            {"tradingSymbol":"X","exchangeSegment":"NSE_EQ","productType":"LEVERAGE",
             "transactionType":"BUY","tradedQuantity":1,"tradedPrice":1.0,
             "exchangeTime":"2026-07-25 12:00:00"},
            {"tradingSymbol":"Y","exchangeSegment":"NSE_EQ",
             "transactionType":"BUY","tradedQuantity":1,"tradedPrice":1.0,
             "exchangeTime":"2026-07-25 12:00:00"}
        ]}"#;
        let fills = map_trades_day_book(body).expect("unknown products refused");
        assert!(fills.is_empty());
    }

    #[test]
    fn empty_day_book_is_zero_fills() {
        for body in [
            r#"{"status":"success","data":[]}"#,
            r#"{"status":"success","data":null}"#,
            r#"{"status":"success"}"#,
            r#"[]"#,
        ] {
            let fills =
                map_trades_day_book(body).unwrap_or_else(|e| panic!("empty is success ({body}): {e}"));
            assert!(fills.is_empty(), "body={body}");
        }
    }

    #[test]
    fn session_codes_surface_as_session_expired() {
        for body in [
            r#"{"errorType":"InvalidToken","errorCode":"DH-901","errorMessage":"Invalid token"}"#,
            r#"{"status":"failure","remarks":{"error_code":"DH-901","error_type":"Auth","error_message":"expired"}}"#,
            r#"{"status":"error","errorType":"Auth","errorCode":807,"errorMessage":"expired"}"#,
        ] {
            let err = map_trades_day_book(body).expect_err("session error");
            assert!(
                err.contains("session_expired"),
                "body={body} unexpected: {err}"
            );
        }
    }

    #[test]
    fn non_session_venue_errors_are_visible_not_empty_success() {
        let body = r#"{"status":"failure","remarks":{"error_code":"DH-904","error_type":"RateLimit","error_message":"too many"}}"#;
        let err = map_trades_day_book(body).expect_err("venue error");
        assert!(err.contains("trades_error"), "unexpected: {err}");
        assert!(err.contains("DH-904"), "unexpected: {err}");
    }

    #[test]
    fn host_refusals_fail_closed_with_names() {
        let blocked = BrokerHttpResponse {
            status: 0,
            headers: vec![],
            body: String::new(),
            error_class: Some("host_blocked".to_string()),
        };
        let err = require_ok(&blocked).expect_err("host_blocked fails closed");
        assert!(err.contains("host_blocked"), "unexpected: {err}");

        let expired = BrokerHttpResponse {
            status: 0,
            headers: vec![],
            body: String::new(),
            error_class: Some("session_expired".to_string()),
        };
        let err = require_ok(&expired).expect_err("session fence");
        assert!(err.contains("session_expired"), "unexpected: {err}");

        let unauthorized = BrokerHttpResponse {
            status: 401,
            headers: vec![],
            body: "{}".to_string(),
            error_class: None,
        };
        let err = require_ok(&unauthorized).expect_err("401");
        assert!(err.contains("session_expired"), "unexpected: {err}");
    }

    #[test]
    fn describe_claims_tradebook_only() {
        let json: serde_json::Value =
            serde_json::from_str(&describe_json()).expect("describe json");
        assert_eq!(json["adapter_id"], "dhan");
        let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
        assert_eq!(implemented.len(), 1);
        assert!(!implemented.iter().any(|v| v.as_str() == Some("history")));
    }

    #[test]
    fn obtain_history_is_unsupported() {
        let request = serde_json::json!({
            "family": "market",
            "capability-id": "ohlcv",
            "physics": "historical_series",
            "operation": "history",
        })
        .to_string();
        let json: serde_json::Value =
            serde_json::from_str(&obtain_json(&request).expect("obtain")).expect("json");
        assert_eq!(json["status"], "unsupported");
    }
}

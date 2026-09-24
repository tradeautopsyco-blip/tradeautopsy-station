//! `zerodha_kite` adapter component (ADR 0001 · B6 `zerodha_kite` · R5).
//!
//! Day-book fills via host `broker_http_call` → `GET /trades`. Bearer `Authorization`
//! and `X-Kite-Version` are attached outside Wasm (R6). Cash book: NSE/BSE + CNC/MIS.
//! NFO book: `exchange` NFO + NRML/MIS → `exchange_segment` `nse_fo` for host split.

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

const HOST: &str = "api.kite.trade";
const TRADES_PATH: &str = "/trades";
const ALLOWED_CASH_EXCHANGES: &[&str] = &["NSE", "BSE"];
const ALLOWED_CASH_PRODUCTS: &[&str] = &["CNC", "MIS"];
const NFO_EXCHANGE: &str = "NFO";
/// Host split + NFO PnL owner key off `nse_fo`, not the Kite `NFO` exchange code.
const NFO_SEGMENT: &str = "nse_fo";
const ALLOWED_NFO_PRODUCTS: &[&str] = &["NRML", "MIS"];
/// Trades timestamps are IST with no zone marker (`YYYY-MM-DD HH:MM:SS`).
const IST_OFFSET_SECONDS: i64 = 5 * 3600 + 30 * 60;

struct ZerodhaKiteAdapter;

export!(ZerodhaKiteAdapter);

impl AdapterGuest for ZerodhaKiteAdapter {
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

impl DataAdapterGuest for ZerodhaKiteAdapter {
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
            return Err("zerodha_kite session_expired".to_string());
        }
        return Err(format!("zerodha_kite {class} (http {})", response.status));
    }
    if response.status == 403 {
        return Err("zerodha_kite session_expired".to_string());
    }
    if response.status != 200 {
        return Err(format!("zerodha_kite http {}", response.status));
    }
    Ok(response.body.clone())
}

fn map_trades_day_book(body: &str) -> Result<Vec<FillEvent>, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("trades json: {e}"))?;

    if is_token_error(&root) {
        return Err("zerodha_kite session_expired".to_string());
    }

    let status = string_field(&root, "status").unwrap_or_default();
    if status.eq_ignore_ascii_case("error") {
        let err_type = string_field(&root, "error_type").unwrap_or_default();
        let message = string_field(&root, "message").unwrap_or_default();
        if err_type.eq_ignore_ascii_case("TokenException") {
            return Err("zerodha_kite session_expired".to_string());
        }
        return Err(format!("zerodha_kite trades_error ({err_type}: {message})"));
    }
    if !status.is_empty() && !status.eq_ignore_ascii_case("success") {
        return Err(format!("zerodha_kite trades_not_success ({status})"));
    }

    let rows = match root.get("data") {
        None | Some(serde_json::Value::Null) => &[][..],
        Some(serde_json::Value::Array(rows)) => rows.as_slice(),
        _ => return Err("trades missing data[]".to_string()),
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let exchange = string_field(row, "exchange");
        let product = string_field(row, "product");
        let nfo = is_nfo_row(exchange.as_deref());
        if !nfo && !is_supported_cash_row(exchange.as_deref(), product.as_deref()) {
            continue;
        }
        if nfo && !is_supported_nfo_row(product.as_deref()) {
            continue;
        }

        let symbol = string_field(row, "tradingsymbol").ok_or("trade row missing tradingsymbol")?;
        let side = match string_field(row, "transaction_type").as_deref() {
            Some("BUY") | Some("buy") => "BUY",
            Some("SELL") | Some("sell") => "SELL",
            other => {
                return Err(format!(
                    "trade row has unknown transaction_type {:?}",
                    other.unwrap_or("")
                ))
            }
        };
        let qty = parse_qty(row)?;
        let price = parse_f64(row.get("average_price"))?;
        let filled_at_unix_ms = parse_trade_time(row)?;
        let fill_id = string_field(row, "trade_id")
            .unwrap_or_else(|| format!("{symbol}-{side}-{filled_at_unix_ms}"));
        let trade_id = string_field(row, "order_id").or_else(|| string_field(row, "exchange_order_id"));
        let exchange_segment = if nfo {
            Some(NFO_SEGMENT.to_string())
        } else {
            exchange
        };

        out.push(FillEvent {
            fill_id,
            broker_slug: "zerodha_kite".to_string(),
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

fn is_nfo_row(exchange: Option<&str>) -> bool {
    exchange
        .map(|e| e.eq_ignore_ascii_case(NFO_EXCHANGE))
        .unwrap_or(false)
}

fn is_supported_nfo_row(product: Option<&str>) -> bool {
    product
        .map(|p| {
            ALLOWED_NFO_PRODUCTS
                .iter()
                .any(|allowed| p.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false)
}

fn is_supported_cash_row(exchange: Option<&str>, product: Option<&str>) -> bool {
    let exchange_ok = exchange
        .map(|e| {
            ALLOWED_CASH_EXCHANGES
                .iter()
                .any(|allowed| e.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false);
    let product_ok = product
        .map(|p| {
            ALLOWED_CASH_PRODUCTS
                .iter()
                .any(|allowed| p.eq_ignore_ascii_case(allowed))
        })
        .unwrap_or(false);
    exchange_ok && product_ok
}

fn is_token_error(root: &serde_json::Value) -> bool {
    string_field(root, "error_type").is_some_and(|t| t.eq_ignore_ascii_case("TokenException"))
}

fn parse_qty(row: &serde_json::Value) -> Result<f64, String> {
    parse_f64(row.get("filled"))
        .or_else(|_| parse_f64(row.get("quantity")))
        .map_err(|_| "trade row missing filled/quantity".to_string())
}

fn parse_trade_time(row: &serde_json::Value) -> Result<i64, String> {
    if let Some(raw) = string_field(row, "fill_timestamp") {
        return parse_ist_timestamp(&raw);
    }
    if let Some(raw) = string_field(row, "exchange_timestamp") {
        return parse_ist_timestamp(&raw);
    }
    Err("trade row missing fill_timestamp/exchange_timestamp".to_string())
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
        "manifest_id": "zerodha_kite.s1.v1",
        "adapter_id": "zerodha_kite",
        "implemented": ["tradebook"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "zerodha_kite",
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
        "zerodha_kite obtain unsupported: {family}/{capability}/{physics}"
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

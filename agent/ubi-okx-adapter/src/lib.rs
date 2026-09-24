//! `okx_com` adapter component (ADR 0016 · B6 `okx_com`).
//!
//! Global OKX REST only (`www.okx.com`). US (`us.okx.com`) and EEA (`eea.okx.com`) are
//! refused venues. Sandbox/demo (`x-simulated-trading`) is never sent — host refuses
//! that header. Auth (`OK-ACCESS-*`) is attached outside Wasm (R6).

#![allow(clippy::all)]

wit_bindgen::generate!({
    world: "broker-adapter-data",
    path: "../../docs/contracts",
});

use crate::exports::tradeautopsy::ubi_data::adapter::Guest as AdapterGuest;
use crate::exports::tradeautopsy::ubi_data::data_adapter::Guest as DataAdapterGuest;
use crate::tradeautopsy::ubi_data::broker_http;
use crate::tradeautopsy::ubi_data::types::{
    AssetClass, BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpQueryParam,
    InstrumentClass,
};

/// B6 row 0 — canonical prod host (www avoids redirect stripping auth headers).
const HOST: &str = "www.okx.com";
const FILLS_PATH: &str = "/api/v5/trade/fills";
const CANDLES_PATH: &str = "/api/v5/market/candles";
const INST_SPOT: &str = "SPOT";
/// Official default/max page size for transaction details (last 3 days).
const FILLS_LIMIT: &str = "100";
const FILLS_LIMIT_N: usize = 100;
const MAX_FILL_PAGES: usize = 40;
const CANDLE_LIMIT_DEFAULT: u32 = 100;
const CANDLE_LIMIT_MAX: u32 = 300;
const DEFAULT_HISTORY_INTERVAL: &str = "1m";
const CANDLE_INTERVALS: &[&str] = &[
    "1m", "3m", "5m", "15m", "30m", "1H", "2H", "4H", "6H", "12H", "1D", "1W", "1M", "3M",
];
/// Quote currencies recognised for `currency` on SPOT `instId` (`BASE-QUOTE`).
const QUOTE_CCY: &[&str] = &["USDT", "USDC", "USD", "EUR", "BTC", "ETH"];

struct OkxComAdapter;

export!(OkxComAdapter);

impl AdapterGuest for OkxComAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let mut fills = Vec::new();
        if let Some(inst) = cursor.symbol.clone().filter(|s| !s.trim().is_empty()) {
            fetch_fills_pages(&inst, &cursor, &mut fills)?;
        } else {
            fetch_fills_pages("", &cursor, &mut fills)?;
        }
        if let Some(since) = cursor.since_unix_ms {
            fills.retain(|f| f.filled_at_unix_ms >= since);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for OkxComAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn fetch_fills_pages(
    inst_id: &str,
    cursor: &FillCursor,
    fills: &mut Vec<FillEvent>,
) -> Result<(), String> {
    let mut after = cursor.from_id.clone().filter(|id| !id.trim().is_empty());
    for _ in 0..MAX_FILL_PAGES {
        let mut query = vec![
            param("instType", INST_SPOT),
            param("limit", FILLS_LIMIT),
        ];
        if !inst_id.trim().is_empty() {
            query.push(param("instId", inst_id.trim()));
        }
        if let Some(a) = &after {
            query.push(param("after", a));
        }
        let response = call(FILLS_PATH, query)?;
        let body = require_ok_envelope("fills", &response)?;
        let (page, next_after) = map_fills(&body)?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();
        fills.extend(page);
        if page_len < FILLS_LIMIT_N {
            break;
        }
        let Some(last_bill) = next_after else {
            break;
        };
        after = Some(last_bill);
    }
    Ok(())
}

fn call(path: &str, query: Vec<HttpQueryParam>) -> Result<BrokerHttpResponse, String> {
    broker_http::broker_http_call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: HOST.to_string(),
        path: path.to_string(),
        query,
        headers: vec![],
        body: None,
    })
}

fn require_ok_envelope(label: &str, response: &BrokerHttpResponse) -> Result<String, String> {
    if response.status != 200 {
        let class = response.error_class.clone().unwrap_or_else(|| {
            if response.status == 0 {
                "network".to_string()
            } else {
                "http_error".to_string()
            }
        });
        return Err(format!("okx_com {label} http {}: {class}", response.status));
    }
    let root: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|e| format!("{label} json: {e}"))?;
    let code = root.get("code").and_then(|v| v.as_str()).unwrap_or("");
    if code != "0" {
        let msg = root.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("okx_com {label} api {code}: {msg}"));
    }
    Ok(response.body.clone())
}

fn map_fills(body: &str) -> Result<(Vec<FillEvent>, Option<String>), String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("fills json: {e}"))?;
    let rows = root
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("fills missing data[]")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let inst_type = row
            .get("instType")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !inst_type.eq_ignore_ascii_case(INST_SPOT) {
            continue;
        }
        let inst_id = row
            .get("instId")
            .and_then(|v| v.as_str())
            .ok_or("fill row missing instId")?;
        let trade_id = row
            .get("tradeId")
            .and_then(|v| v.as_str())
            .ok_or("fill row missing tradeId")?;
        let side_raw = row
            .get("side")
            .and_then(|v| v.as_str())
            .ok_or("fill row missing side")?;
        let side = if side_raw.eq_ignore_ascii_case("buy") {
            "BUY".to_string()
        } else if side_raw.eq_ignore_ascii_case("sell") {
            "SELL".to_string()
        } else {
            side_raw.to_ascii_uppercase()
        };
        let qty = parse_f64(row.get("fillSz"))?;
        let price = parse_f64(row.get("fillPx"))?;
        let filled_at_unix_ms = row
            .get("ts")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .or_else(|| row.get("ts").and_then(|v| v.as_i64()))
            .ok_or("fill row missing ts")?;
        let fee_amount = parse_f64_opt(row.get("fee")).map(|f| f.abs());
        let fee_currency = row
            .get("feeCcy")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let order_id = row
            .get("ordId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        out.push(FillEvent {
            fill_id: trade_id.to_string(),
            broker_slug: "okx_com".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            currency: quote_from_inst_id(inst_id),
            symbol: inst_id.to_string(),
            side,
            qty,
            price,
            filled_at_unix_ms,
            fee_amount,
            fee_currency,
            exchange_segment: None,
            product: None,
            trade_id: order_id,
        });
    }
    let next_after = rows
        .last()
        .and_then(|row| row.get("billId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Ok((out, next_after))
}

fn quote_from_inst_id(inst_id: &str) -> String {
    let upper = inst_id.to_uppercase();
    let mut best = "";
    for quote in QUOTE_CCY {
        let suffix = format!("-{quote}");
        if upper.ends_with(&suffix) && quote.len() > best.len() {
            best = quote;
        }
    }
    if best.is_empty() {
        "UNKNOWN".to_string()
    } else {
        best.to_string()
    }
}

fn param(name: &str, value: &str) -> HttpQueryParam {
    HttpQueryParam {
        name: name.to_string(),
        value: value.to_string(),
    }
}

fn parse_f64(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(serde_json::Value::Number(n)) => {
            n.as_f64().ok_or_else(|| "number not f64".to_string())
        }
        Some(serde_json::Value::String(s)) => {
            s.parse().map_err(|e| format!("parse f64 '{s}': {e}"))
        }
        _ => Err("missing number".to_string()),
    }
}

fn parse_f64_opt(value: Option<&serde_json::Value>) -> Option<f64> {
    parse_f64(value).ok()
}

fn describe_json() -> String {
    serde_json::json!({
        "manifest_id": "okx_com.s1.v1",
        "adapter_id": "okx_com",
        "implemented": ["tradebook"],
        "bindings": [{
            "operation": "tradebook",
            "adapter_id": "okx_com",
            "family": "account",
            "capability_id": "fills",
            "physics": "bounded_snapshot",
        }],
    })
    .to_string()
}

fn obtain_json(request: &str) -> Result<String, String> {
    let req: serde_json::Value =
        serde_json::from_str(request).map_err(|e| format!("obtain json: {e}"))?;
    let op = req.get("operation").and_then(|v| v.as_str()).unwrap_or("");
    if op != "history" {
        return Ok(serde_json::json!({"status": "unsupported"}).to_string());
    }
    let instrument = req
        .get("instrument-id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or("history requires instrument-id")?;
    let interval = req
        .get("interval")
        .and_then(|v| v.as_str())
        .unwrap_or(DEFAULT_HISTORY_INTERVAL);
    if !CANDLE_INTERVALS.contains(&interval) {
        return Err(format!("unsupported interval {interval}"));
    }
    let limit = req
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n.min(CANDLE_LIMIT_MAX as u64) as u32)
        .unwrap_or(CANDLE_LIMIT_DEFAULT);
    let query = vec![
        param("instId", instrument),
        param("bar", interval),
        param("limit", &limit.to_string()),
    ];
    let response = call(CANDLES_PATH, query)?;
    let body = require_ok_envelope("candles", &response)?;
    Ok(body)
}

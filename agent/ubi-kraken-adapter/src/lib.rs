//! `kraken` adapter component (ADR 0017 · B6 `kraken` · R5).
//!
//! Sandboxed Wasm: egress is host `broker_http_call` only. Kraken spot HMAC-SHA512 +
//! nonce is attached on the host. Never `futures.kraken.com` / `demo-futures.kraken.com`.

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

const HOST: &str = "api.kraken.com";
const TRADES_HISTORY_PATH: &str = "/0/private/TradesHistory";
const MAX_TRADES_PAGES: usize = 40;
const PAGE_SIZE: usize = 50;
/// B6 quote-filter: desk v1 books USDT/USDC/USD/EUR-quoted spot pairs only.
const DESK_QUOTE_SUFFIXES: &[&str] = &["USDT", "USDC", "USD", "EUR"];

struct KrakenAdapter;

export!(KrakenAdapter);

impl AdapterGuest for KrakenAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let mut fills = fetch_trades_history(&cursor)?;
        if let Some(symbol) = cursor.symbol.as_ref().filter(|s| !s.trim().is_empty()) {
            let want = symbol.trim().to_uppercase();
            fills.retain(|f| f.symbol.eq_ignore_ascii_case(&want));
        }
        if let Some(since) = cursor.since_unix_ms {
            fills.retain(|f| f.filled_at_unix_ms >= since);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for KrakenAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn fetch_trades_history(cursor: &FillCursor) -> Result<Vec<FillEvent>, String> {
    let mut fills = Vec::new();
    let mut ofs = 0u32;
    for _ in 0..MAX_TRADES_PAGES {
        let mut extra = String::from("type=trade&trades=true");
        if let Some(since) = cursor.since_unix_ms {
            let start_secs = since / 1000;
            extra.push_str(&format!("&start={start_secs}"));
        }
        extra.push_str(&format!("&ofs={ofs}"));
        let response = call_private(TRADES_HISTORY_PATH, &extra)?;
        let body = require_ok_kraken("TradesHistory", &response)?;
        let page = map_trades(&body)?;
        let count = body
            .pointer("/result/count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();
        fills.extend(page);
        ofs = ofs.saturating_add(PAGE_SIZE as u32);
        if ofs >= count as u32 || page_len < PAGE_SIZE {
            break;
        }
    }
    Ok(fills)
}

fn call_private(path: &str, extra_form: &str) -> Result<BrokerHttpResponse, String> {
    broker_http::broker_http_call(&BrokerHttpRequest {
        method: "POST".to_string(),
        host: HOST.to_string(),
        path: path.to_string(),
        query: vec![],
        headers: vec![],
        body: Some(extra_form.to_string()),
    })
}

fn require_ok_kraken(label: &str, response: &BrokerHttpResponse) -> Result<serde_json::Value, String> {
    if response.status != 200 {
        let class = response
            .error_class
            .clone()
            .unwrap_or_else(|| "http_error".to_string());
        return Err(format!("kraken {label} http {}: {class}", response.status));
    }
    let json: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|e| format!("{label} json: {e}"))?;
    if let Some(errors) = json.get("error").and_then(|v| v.as_array()) {
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            if msg.contains("EAPI:Invalid nonce") || msg.contains("EGeneral:Permission denied") {
                return Err(format!("kraken session_expired: {msg}"));
            }
            return Err(format!("kraken {label} error: {msg}"));
        }
    }
    Ok(json)
}

fn map_trades(body: &serde_json::Value) -> Result<Vec<FillEvent>, String> {
    let trades = body
        .pointer("/result/trades")
        .and_then(|v| v.as_object())
        .ok_or("TradesHistory missing result.trades")?;
    let mut out = Vec::with_capacity(trades.len());
    for (trade_key, row) in trades {
        let pair = row
            .get("pair")
            .and_then(|v| v.as_str())
            .ok_or("trade row missing pair")?
            .to_string();
        if !symbol_allowed_on_desk(&pair) {
            continue;
        }
        let side = row
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("trade row missing type")?;
        let qty = parse_f64(row.get("vol"))?;
        let price = parse_f64(row.get("price"))?;
        let time_secs = row
            .get("time")
            .and_then(|v| v.as_f64())
            .ok_or("trade row missing time")?;
        let filled_at_unix_ms = (time_secs * 1000.0).round() as i64;
        let fee_amount = parse_f64_opt(row.get("fee"));
        let trade_id = row
            .get("trade_id")
            .map(|v| {
                v.as_i64()
                    .map(|n| n.to_string())
                    .or_else(|| v.as_str().map(|s| s.to_string()))
            })
            .flatten()
            .or_else(|| {
                row.get("ordertxid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });
        out.push(FillEvent {
            fill_id: trade_key.clone(),
            broker_slug: "kraken".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            currency: desk_quote_for_symbol(&pair),
            symbol: pair,
            side: if side.eq_ignore_ascii_case("buy") {
                "BUY".to_string()
            } else {
                "SELL".to_string()
            },
            qty,
            price,
            filled_at_unix_ms,
            fee_amount,
            fee_currency: None,
            exchange_segment: None,
            product: None,
            trade_id,
        });
    }
    Ok(out)
}

fn symbol_allowed_on_desk(symbol: &str) -> bool {
    desk_quote_for_symbol(symbol) != "UNKNOWN"
}

fn desk_quote_for_symbol(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let mut best = "";
    for quote in DESK_QUOTE_SUFFIXES {
        if upper.len() > quote.len() && upper.ends_with(quote) && quote.len() > best.len() {
            best = quote;
        }
    }
    if best.is_empty() {
        "UNKNOWN".to_string()
    } else {
        best.to_string()
    }
}

fn parse_f64(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or_else(|| "number not f64".to_string()),
        Some(serde_json::Value::String(s)) => s.parse().map_err(|e| format!("parse f64 '{s}': {e}")),
        _ => Err("missing number".to_string()),
    }
}

fn parse_f64_opt(value: Option<&serde_json::Value>) -> Option<f64> {
    parse_f64(value).ok()
}

fn describe_json() -> String {
    serde_json::json!({
        "manifest_id": "kraken.spot.v1",
        "adapter_id": "kraken",
        "implemented": ["tradebook"],
        "bindings": [{
            "operation": "tradebook",
            "adapter_id": "kraken",
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
    let family = json_str(&req, &["family"]).unwrap_or_default();
    let capability = json_str(&req, &["capability-id", "capability_id"]).unwrap_or_default();
    let physics = json_str(&req, &["physics"]).unwrap_or_default();
    Err(format!(
        "kraken obtain unsupported: {family}/{capability}/{physics}"
    ))
}

fn json_str(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| value.get(*k).and_then(|v| v.as_str()).map(|s| s.to_string()))
}

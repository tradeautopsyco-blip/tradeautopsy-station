//! `bybit` adapter component (ADR 0015 · B6 `bybit` · R5).
//!
//! Sandboxed Wasm: egress is host `broker_http_call` only. Bybit v5 HMAC is attached
//! on the host (`X-BAPI-*` headers). Never `api-testnet.bybit.com` / `api-demo.bybit.com`.

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

const HOST: &str = "api.bybit.com";
const WALLET_BALANCE_PATH: &str = "/v5/account/wallet-balance";
const EXECUTION_LIST_PATH: &str = "/v5/execution/list";
const KLINES_PATH: &str = "/v5/market/kline";
const CATEGORY_SPOT: &str = "spot";
const ACCOUNT_TYPE_UNIFIED: &str = "UNIFIED";
const EXEC_LIMIT: &str = "100";
const EXEC_LIMIT_N: usize = 100;
const MS_7D: i64 = 7 * 86_400_000;
const MAX_EXEC_PAGES: usize = 40;
const KLINE_LIMIT_DEFAULT: u32 = 200;
const KLINE_LIMIT_MAX: u32 = 1000;
const DEFAULT_HISTORY_INTERVAL: &str = "1";
const KLINE_INTERVALS: &[&str] = &[
    "1", "3", "5", "15", "30", "60", "120", "240", "360", "720", "D", "W", "M",
];
/// B6 quote-filter: desk v1 books USDT/USDC/USD-quoted spot pairs only.
const DESK_QUOTE_SUFFIXES: &[&str] = &["USDT", "USDC", "USD"];
const STABLE_COINS: &[&str] = &["USDT", "USDC", "USD", "BUSD", "DAI"];

struct BybitAdapter;

export!(BybitAdapter);

impl AdapterGuest for BybitAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let symbols = match cursor.symbol.clone() {
            Some(symbol) if !symbol.trim().is_empty() => {
                let s = symbol.trim().to_uppercase();
                if !symbol_allowed_on_desk(&s) {
                    return Err(format!("bybit quote-filter refused symbol {s}"));
                }
                vec![s]
            }
            _ => bootstrap_symbols()?,
        };

        let mut fills = Vec::new();
        for symbol in symbols {
            fills.extend(fetch_symbol_executions(&symbol, &cursor)?);
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for BybitAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn bootstrap_symbols() -> Result<Vec<String>, String> {
    let response = call(
        WALLET_BALANCE_PATH,
        vec![param("accountType", ACCOUNT_TYPE_UNIFIED)],
    )?;
    let body = require_ok_envelope("wallet-balance", &response)?;
    let coins = body
        .pointer("/result/list/0/coin")
        .and_then(|v| v.as_array())
        .ok_or("wallet-balance missing result.list[0].coin[]")?;

    let mut symbols = Vec::new();
    for row in coins {
        let coin = row
            .get("coin")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_uppercase();
        if coin.is_empty() || STABLE_COINS.contains(&coin.as_str()) {
            continue;
        }
        let wallet = parse_f64_opt(row.get("walletBalance")).unwrap_or(0.0);
        let locked = parse_f64_opt(row.get("locked")).unwrap_or(0.0);
        if wallet + locked <= 0.0 {
            continue;
        }
        let candidate = format!("{coin}USDT");
        if symbol_allowed_on_desk(&candidate) {
            symbols.push(candidate);
        }
    }
    Ok(symbols)
}

fn fetch_symbol_executions(symbol: &str, cursor: &FillCursor) -> Result<Vec<FillEvent>, String> {
    let mut fills = Vec::new();
    let mut page_cursor = cursor.from_id.clone();
    for _ in 0..MAX_EXEC_PAGES {
        let mut query = vec![
            param("category", CATEGORY_SPOT),
            param("symbol", symbol),
            param("limit", EXEC_LIMIT),
        ];
        if let Some(c) = page_cursor.as_ref().filter(|s| !s.is_empty()) {
            query.push(param("cursor", c));
        } else if let Some(since) = cursor.since_unix_ms {
            query.push(param("startTime", &since.to_string()));
            let end = since.saturating_add(MS_7D.saturating_sub(1));
            query.push(param("endTime", &end.to_string()));
        }
        let response = call(EXECUTION_LIST_PATH, query)?;
        let body = require_ok_envelope("execution/list", &response)?;
        let page = map_executions(&body, symbol)?;
        let page_len = page.len();
        fills.extend(page);
        let next = body
            .pointer("/result/nextPageCursor")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
        if page_len < EXEC_LIMIT_N {
            break;
        }
        match next {
            Some(c) => page_cursor = Some(c),
            None => break,
        }
    }
    Ok(fills)
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

fn require_ok_envelope(label: &str, response: &BrokerHttpResponse) -> Result<serde_json::Value, String> {
    if response.status != 200 {
        let class = response.error_class.clone().unwrap_or_else(|| "http_error".to_string());
        return Err(format!("bybit {label} http {}: {class}", response.status));
    }
    let json: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|e| format!("{label} json: {e}"))?;
    let code = json.get("retCode").and_then(|v| v.as_i64()).unwrap_or(-1);
    if code != 0 {
        let msg = json
            .get("retMsg")
            .and_then(|v| v.as_str())
            .unwrap_or("retCode error");
        return Err(format!("bybit {label} retCode {code}: {msg}"));
    }
    Ok(json)
}

fn map_executions(body: &serde_json::Value, symbol: &str) -> Result<Vec<FillEvent>, String> {
    let rows = body
        .pointer("/result/list")
        .and_then(|v| v.as_array())
        .ok_or("execution/list missing result.list")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let exec_id = row
            .get("execId")
            .and_then(|v| v.as_str())
            .ok_or("execution row missing execId")?;
        let sym = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or(symbol)
            .to_string();
        let side = row
            .get("side")
            .and_then(|v| v.as_str())
            .ok_or("execution row missing side")?;
        let qty = parse_f64(row.get("execQty"))?;
        let price = parse_f64(row.get("execPrice"))?;
        let filled_at_unix_ms = row
            .get("execTime")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .or_else(|| row.get("execTime").and_then(|v| v.as_i64()))
            .ok_or("execution row missing execTime")?;
        let fee_amount = parse_f64_opt(row.get("execFee"));
        let fee_currency = row
            .get("feeCurrency")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let trade_id = row
            .get("orderId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        out.push(FillEvent {
            fill_id: exec_id.to_string(),
            broker_slug: "bybit".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            currency: desk_quote_for_symbol(&sym),
            symbol: sym,
            side: if side.eq_ignore_ascii_case("buy") {
                "BUY".to_string()
            } else {
                "SELL".to_string()
            },
            qty,
            price,
            filled_at_unix_ms,
            fee_amount,
            fee_currency,
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
        "manifest_id": "bybit.spot.v1",
        "adapter_id": "bybit",
        "implemented": ["tradebook", "history"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "bybit",
                "family": "account",
                "capability_id": "fills",
                "physics": "bounded_snapshot",
            },
            {
                "operation": "history",
                "adapter_id": "bybit",
                "family": "market",
                "capability_id": "ohlcv",
                "physics": "historical_series",
                "coverage": { "intervals": KLINE_INTERVALS },
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
    let instrument = json_str(&req, &["instrument-id", "instrument_id"]).unwrap_or_default();
    if family != "market" || capability != "ohlcv" || physics != "historical_series" {
        return Err(format!(
            "bybit obtain unsupported: {family}/{capability}/{physics}"
        ));
    }
    obtain_history(&req, &instrument)
}

fn obtain_history(req: &serde_json::Value, instrument: &str) -> Result<String, String> {
    let symbol = instrument.trim().to_ascii_uppercase();
    if symbol.is_empty() || !symbol_allowed_on_desk(&symbol) {
        return Err("bybit history missing or quote-filter refused instrument-id".into());
    }
    let interval = json_str(req, &["interval"]).unwrap_or_else(|| DEFAULT_HISTORY_INTERVAL.into());
    if !KLINE_INTERVALS.iter().any(|allowed| *allowed == interval) {
        return Err("unsupported_interval".into());
    }
    let limit = json_u32(req, "limit").unwrap_or(KLINE_LIMIT_DEFAULT);
    if limit == 0 || limit > KLINE_LIMIT_MAX {
        return Err("unsupported_range".into());
    }
    let response = call(
        KLINES_PATH,
        vec![
            param("category", CATEGORY_SPOT),
            param("symbol", &symbol),
            param("interval", &interval),
            param("limit", &limit.to_string()),
        ],
    )?;
    let body = require_ok_envelope("kline", &response)?;
    map_klines_obtain(&body, &symbol, &interval)
}

fn map_klines_obtain(body: &serde_json::Value, symbol: &str, interval: &str) -> Result<String, String> {
    let rows = body
        .pointer("/result/list")
        .and_then(|v| v.as_array())
        .ok_or("kline body missing result.list")?;
    let mut candles = Vec::new();
    for row in rows {
        candles.push(candle_from_array(row)?);
    }
    if candles.is_empty() {
        return Err("kline empty".into());
    }
    let last_close = candles
        .last()
        .and_then(|c| c.get("close").cloned())
        .ok_or("kline missing close")?;
    Ok(serde_json::json!({
        "identity": {
            "family": "market",
            "capability_id": "ohlcv",
            "physics": "historical_series",
        },
        "instrument_id": symbol,
        "interval": interval,
        "candles": candles,
        "last_close": last_close,
        "source": "bybit_kline",
    })
    .to_string())
}

fn candle_from_array(row: &serde_json::Value) -> Result<serde_json::Value, String> {
    let arr = row.as_array().ok_or("kline row is not an array")?;
    if arr.len() < 7 {
        return Err("kline row missing documented indexes 0-6".into());
    }
    Ok(serde_json::json!({
        "open_time_ms": json_i64(&arr[0]).ok_or("kline open time")?,
        "open": json_dec(&arr[1]).ok_or("kline open")?,
        "high": json_dec(&arr[2]).ok_or("kline high")?,
        "low": json_dec(&arr[3]).ok_or("kline low")?,
        "close": json_dec(&arr[4]).ok_or("kline close")?,
        "volume": json_dec(&arr[5]).ok_or("kline volume")?,
        "close_time_ms": json_i64(&arr[6]).ok_or("kline close time")?,
    }))
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

fn json_u32(value: &serde_json::Value, key: &str) -> Option<u32> {
    let v = value.get(key)?;
    v.as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn json_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
        .or_else(|| value.as_f64().map(|n| n as i64))
}

fn json_dec(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

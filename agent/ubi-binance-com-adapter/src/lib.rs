//! `binance_com` adapter component (ADR 0001 · B6 `binance_com` · R5).
//!
//! Sandboxed Wasm: the only egress is the host's `broker_http_call`, which attaches the
//! HMAC signature and `X-MBX-APIKEY` outside this module. No credential ever reaches
//! component memory, and nothing here may talk to `api.binance.us` (B6 refuse list).

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

/// COM only — Binance.US is a different venue with its own sheet (B6 §0).
const HOST: &str = "api.binance.com";
const ACCOUNT_PATH: &str = "/api/v3/account";
const MY_TRADES_PATH: &str = "/api/v3/myTrades";
/// Public klines only (`GET /api/v3/klines`). Not uiKlines. Not api.binance.us.
const KLINES_PATH: &str = "/api/v3/klines";
/// B6 §4: limit default 500, max 1000. Same documented max as klines.
const TRADES_LIMIT: &str = "500";
const TRADES_LIMIT_N: usize = 500;
/// Official `my_trades` (binance-sdk 70.1.0): startTime↔endTime cannot exceed 24h.
const MS_24H: i64 = 86_400_000;
/// Safety cap so a stuck fixture / repeating page cannot loop forever.
const MAX_MY_TRADES_PAGES: usize = 40;
const KLINE_LIMIT_DEFAULT: u32 = 500;
const KLINE_LIMIT_MAX: u32 = 1000;
const DEFAULT_HISTORY_INTERVAL: &str = "1m";
/// Documented interval enum (case-sensitive). REST.md Kline/Candlestick.
const KLINE_INTERVALS: &[&str] = &[
    "1s", "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "6h", "8h", "12h", "1d", "3d", "1w",
    "1M",
];
/// Quote assets recognised for `currency`; longest suffix wins.
const QUOTE_ASSETS: &[&str] = &[
    "USDT", "USDC", "FDUSD", "TUSD", "BUSD", "BTC", "ETH", "BNB", "EUR", "TRY", "BRL", "INR",
];
/// Balances that are the quote side, never a `{ASSET}USDT` bootstrap symbol.
const STABLE_ASSETS: &[&str] = &["USDT", "USDC", "BUSD", "FDUSD", "TUSD", "USD"];

struct BinanceComAdapter;

export!(BinanceComAdapter);

impl AdapterGuest for BinanceComAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        // myTrades requires a symbol (B6 §4); with none pinned we derive the traded set
        // from balances, same bootstrap the native reference client used.
        let symbols = match cursor.symbol.clone() {
            Some(symbol) if !symbol.trim().is_empty() => vec![symbol.trim().to_uppercase()],
            _ => bootstrap_symbols()?,
        };

        let mut fills = Vec::new();
        for symbol in symbols {
            match fetch_symbol_trades(&symbol, &cursor)? {
                Some(mut rows) => fills.append(&mut rows),
                // 400 = symbol not tradable for this account; skip, don't fail the sync.
                None => continue,
            }
        }
        fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
        Ok(fills)
    }
}

impl DataAdapterGuest for BinanceComAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn bootstrap_symbols() -> Result<Vec<String>, String> {
    let response = call(ACCOUNT_PATH, vec![])?;
    let body = require_ok("account", &response)?;
    let account: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("account json: {e}"))?;
    let balances = account
        .get("balances")
        .and_then(|v| v.as_array())
        .ok_or("account missing balances[]")?;

    let mut symbols = Vec::new();
    for balance in balances {
        let asset = balance
            .get("asset")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_uppercase();
        if asset.is_empty() || STABLE_ASSETS.contains(&asset.as_str()) {
            continue;
        }
        let free = parse_f64_opt(balance.get("free")).unwrap_or(0.0);
        let locked = parse_f64_opt(balance.get("locked")).unwrap_or(0.0);
        if free + locked <= 0.0 {
            continue;
        }
        symbols.push(format!("{asset}USDT"));
    }
    Ok(symbols)
}

fn fetch_symbol_trades(
    symbol: &str,
    cursor: &FillCursor,
) -> Result<Option<Vec<FillEvent>>, String> {
    let mut fills = Vec::new();
    // B6 §4 / R1 + official SDK notes: `fromId` cannot combine with startTime/endTime
    // (-1128). Prefer incremental `fromId` when the caller pinned one symbol.
    let mut page_from = cursor
        .symbol
        .as_ref()
        .and_then(|_| cursor.from_id.as_ref())
        .filter(|id| is_digits(id))
        .cloned();
    let mut prev_max_id: Option<i64> = None;

    for _ in 0..MAX_MY_TRADES_PAGES {
        let mut query = vec![
            HttpQueryParam {
                name: "symbol".to_string(),
                value: symbol.to_string(),
            },
            HttpQueryParam {
                name: "limit".to_string(),
                value: TRADES_LIMIT.to_string(),
            },
        ];
        if let Some(from_id) = &page_from {
            query.push(HttpQueryParam {
                name: "fromId".to_string(),
                value: from_id.clone(),
            });
        } else if let Some(since) = cursor.since_unix_ms {
            query.push(HttpQueryParam {
                name: "startTime".to_string(),
                value: since.to_string(),
            });
            // Official combo `symbol + startTime + endTime`; window ≤ 24h.
            let end = since.saturating_add(MS_24H.saturating_sub(1));
            query.push(HttpQueryParam {
                name: "endTime".to_string(),
                value: end.to_string(),
            });
        }

        let response = call(MY_TRADES_PATH, query)?;
        if response.status == 400 {
            return Ok(if fills.is_empty() { None } else { Some(fills) });
        }
        let body = require_ok("myTrades", &response)?;
        let page = map_my_trades(&body)?;
        if page.is_empty() {
            break;
        }
        let page_len = page.len();
        let page_max = page
            .iter()
            .filter_map(|f| f.fill_id.parse::<i64>().ok())
            .max();
        if let (Some(prev), Some(max)) = (prev_max_id, page_max) {
            if max <= prev {
                break;
            }
        }
        fills.extend(page);
        if page_len < TRADES_LIMIT_N {
            break;
        }
        let Some(max_id) = page_max else {
            break;
        };
        // Official: fromId returns trades >= fromId. Step past this page.
        page_from = Some(max_id.saturating_add(1).to_string());
        prev_max_id = Some(max_id);
    }
    Ok(Some(fills))
}

fn call(path: &str, query: Vec<HttpQueryParam>) -> Result<BrokerHttpResponse, String> {
    broker_http::broker_http_call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: HOST.to_string(),
        path: path.to_string(),
        query,
        // Auth headers are the host's job; sending any here is rejected (R6 §3.3).
        headers: vec![],
        body: None,
    })
}

fn require_ok(label: &str, response: &BrokerHttpResponse) -> Result<String, String> {
    if response.status == 200 {
        return Ok(response.body.clone());
    }
    let class = response.error_class.clone().unwrap_or_else(|| {
        if response.status == 0 {
            "network".to_string()
        } else {
            "http_error".to_string()
        }
    });
    Err(format!(
        "binance_com {label} http {}: {class}",
        response.status
    ))
}

fn map_my_trades(body: &str) -> Result<Vec<FillEvent>, String> {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(body).map_err(|e| format!("myTrades json: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row
            .get("id")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
            .ok_or("myTrades row missing id")?;
        let symbol = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or("myTrades row missing symbol")?
            .to_string();
        let is_buyer = row
            .get("isBuyer")
            .and_then(|v| v.as_bool())
            .ok_or("myTrades row missing isBuyer")?;
        let qty = parse_f64(row.get("qty"))?;
        let price = parse_f64(row.get("price"))?;
        let filled_at_unix_ms = row
            .get("time")
            .and_then(|v| v.as_i64())
            .ok_or("myTrades row missing time")?;
        let fee_amount = parse_f64_opt(row.get("commission"));
        let fee_currency = row
            .get("commissionAsset")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let trade_id = row
            .get("orderId")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
            .map(|v| v.to_string());

        out.push(FillEvent {
            fill_id: id.to_string(),
            // Host overwrites identity + taxonomy axes from the connection book
            // (R5 §3.5; ADR 0004 §1). Placeholder only — adapter never classifies.
            broker_slug: "binance_com".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            currency: quote_asset(&symbol),
            symbol,
            side: if is_buyer { "BUY" } else { "SELL" }.to_string(),
            qty,
            price,
            filled_at_unix_ms,
            fee_amount,
            fee_currency,
            // Spot has no segment/product dimension (R5 §3.2).
            exchange_segment: None,
            product: None,
            trade_id,
        });
    }
    Ok(out)
}

/// Quote side of the pair, i.e. the currency the price is denominated in (B6 §7).
fn quote_asset(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let mut best = "";
    for quote in QUOTE_ASSETS {
        if upper.len() > quote.len() && upper.ends_with(quote) && quote.len() > best.len() {
            best = quote;
        }
    }
    // No guessing: an unrecognised quote is reported as unknown so the desk can flag it
    // rather than silently booking the fill as USD-ish (B6 §7 honesty rule).
    if best.is_empty() {
        "UNKNOWN".to_string()
    } else {
        best.to_string()
    }
}

fn is_digits(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit())
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
        "manifest_id": "binance_com.s1.v1",
        "adapter_id": "binance_com",
        "implemented": ["tradebook", "history"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "binance_com",
                "family": "account",
                "capability_id": "fills",
                "physics": "bounded_snapshot",
            },
            {
                "operation": "history",
                "adapter_id": "binance_com",
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
    if is_yahoo_request(&req) {
        return Err("yahoo is not a binance_com capability".into());
    }
    let family = json_str(&req, &["family"]).unwrap_or_default();
    let capability = json_str(&req, &["capability-id", "capability_id"]).unwrap_or_default();
    let physics = json_str(&req, &["physics"]).unwrap_or_default();
    let instrument = json_str(&req, &["instrument-id", "instrument_id"]).unwrap_or_default();
    if family != "market" || capability != "ohlcv" || physics != "historical_series" {
        return Err(format!(
            "binance_com obtain unsupported: {family}/{capability}/{physics}"
        ));
    }
    obtain_history(&req, &instrument)
}

fn obtain_history(req: &serde_json::Value, instrument: &str) -> Result<String, String> {
    let symbol = instrument.trim().to_ascii_uppercase();
    if symbol.is_empty() {
        return Err("binance_com history missing instrument-id".into());
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
            HttpQueryParam {
                name: "symbol".into(),
                value: symbol.clone(),
            },
            HttpQueryParam {
                name: "interval".into(),
                value: interval.clone(),
            },
            HttpQueryParam {
                name: "limit".into(),
                value: limit.to_string(),
            },
        ],
    )?;
    let body = require_ok("klines", &response)?;
    map_klines_obtain(&body, &symbol, &interval)
}

fn map_klines_obtain(body: &str, symbol: &str, interval: &str) -> Result<String, String> {
    let rows: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("klines json: {e}"))?;
    let rows = rows.as_array().ok_or("klines body is not an array")?;
    let mut candles = Vec::new();
    for row in rows {
        candles.push(candle_from_array(row)?);
    }
    if candles.is_empty() {
        return Err("klines empty".into());
    }
    let last_close = candles
        .last()
        .and_then(|c| c.get("close").cloned())
        .ok_or("klines missing close")?;
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
        "source": "binance_klines",
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

fn is_yahoo_request(req: &serde_json::Value) -> bool {
    for key in [
        "product-use",
        "product_use",
        "source",
        "source_id",
        "adapter_id",
    ] {
        if let Some(s) = req.get(key).and_then(|v| v.as_str()) {
            let n = s.trim().to_ascii_lowercase();
            if n == "yahoo" || n == "yahoo_chart" {
                return true;
            }
        }
    }
    false
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

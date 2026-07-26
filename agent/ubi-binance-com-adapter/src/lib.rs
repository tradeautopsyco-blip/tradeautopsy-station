//! `binance_com` adapter component (ADR 0001 · B6 `binance_com` · R5).
//!
//! Sandboxed Wasm: the only egress is the host's `broker_http_call`, which attaches the
//! HMAC signature and `X-MBX-APIKEY` outside this module. No credential ever reaches
//! component memory, and nothing here may talk to `api.binance.us` (B6 refuse list).

#![allow(clippy::all)]

wit_bindgen::generate!({
    world: "broker-adapter",
    path: "../wit",
});

use crate::exports::tradeautopsy::ubi::adapter::Guest;
use crate::tradeautopsy::ubi::broker_http;
use crate::tradeautopsy::ubi::types::{
    BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpQueryParam,
};

/// COM only — Binance.US is a different venue with its own sheet (B6 §0).
const HOST: &str = "api.binance.com";
const ACCOUNT_PATH: &str = "/api/v3/account";
const MY_TRADES_PATH: &str = "/api/v3/myTrades";
/// B6 §4: limit default 500, max 1000.
const TRADES_LIMIT: &str = "500";
/// Quote assets recognised for `currency`; longest suffix wins.
const QUOTE_ASSETS: &[&str] = &[
    "USDT", "USDC", "FDUSD", "TUSD", "BUSD", "BTC", "ETH", "BNB", "EUR", "TRY", "BRL", "INR",
];
/// Balances that are the quote side, never a `{ASSET}USDT` bootstrap symbol.
const STABLE_ASSETS: &[&str] = &["USDT", "USDC", "BUSD", "FDUSD", "TUSD", "USD"];

struct BinanceComAdapter;

export!(BinanceComAdapter);

impl Guest for BinanceComAdapter {
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
    // B6 §4 / R1: `fromId` + `startTime`/`endTime` together is refused (-1128).
    // Prefer incremental `fromId` when the caller pinned one symbol (B6 §5/§6).
    let from_id = cursor
        .symbol
        .as_ref()
        .and_then(|_| cursor.from_id.as_ref())
        .filter(|id| is_digits(id));
    if let Some(from_id) = from_id {
        query.push(HttpQueryParam {
            name: "fromId".to_string(),
            value: from_id.clone(),
        });
    } else if let Some(since) = cursor.since_unix_ms {
        query.push(HttpQueryParam {
            name: "startTime".to_string(),
            value: since.to_string(),
        });
    }

    let response = call(MY_TRADES_PATH, query)?;
    if response.status == 400 {
        return Ok(None);
    }
    let body = require_ok("myTrades", &response)?;
    map_my_trades(&body).map(Some)
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
            // Host overwrites identity fields from the loaded connection (R5 §3.5).
            broker_slug: "binance_com".to_string(),
            connection_id: String::new(),
            asset_class: "crypto_spot".to_string(),
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
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or_else(|| "number not f64".to_string()),
        Some(serde_json::Value::String(s)) => {
            s.parse().map_err(|e| format!("parse f64 '{s}': {e}"))
        }
        _ => Err("missing number".to_string()),
    }
}

fn parse_f64_opt(value: Option<&serde_json::Value>) -> Option<f64> {
    parse_f64(value).ok()
}

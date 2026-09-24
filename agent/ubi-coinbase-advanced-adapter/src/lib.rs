//! `coinbase_advanced` adapter component (ADR 0018 · B6 `coinbase_advanced` · R5).
//!
//! Sandboxed Wasm: host attaches per-request ES256 JWT (`Bearer`) outside this module.
//! Production host `api.coinbase.com` only — sandbox is refused (B6 row 0/23).

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
    AssetClass, BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpQueryParam,
    InstrumentClass,
};

const HOST: &str = "api.coinbase.com";
const FILLS_PATH: &str = "/api/v3/brokerage/orders/historical/fills";
const FILLS_LIMIT: &str = "100";
const MAX_FILL_PAGES: usize = 50;

/// Quote suffixes for `currency` (longest match wins). B6 named quote rows — not
/// a silent `crypto_spot_usd` default for every pair.
const QUOTE_SUFFIXES: &[&str] = &["USDC", "USDT", "USD", "EUR"];

struct CoinbaseAdvancedAdapter;

#[cfg(not(test))]
export!(CoinbaseAdvancedAdapter);

#[cfg(not(test))]
impl AdapterGuest for CoinbaseAdvancedAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        fetch_all_fills(&broker_http::broker_http_call, &cursor)
    }
}

impl DataAdapterGuest for CoinbaseAdvancedAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

fn fetch_all_fills(
    call: &dyn Fn(&BrokerHttpRequest) -> Result<BrokerHttpResponse, String>,
    cursor: &FillCursor,
) -> Result<Vec<FillEvent>, String> {
    let mut fills = Vec::new();
    let mut page_cursor: Option<String> = None;

    for _ in 0..MAX_FILL_PAGES {
        let mut query = vec![HttpQueryParam {
            name: "limit".to_string(),
            value: FILLS_LIMIT.to_string(),
        }];
        if let Some(pid) = cursor.symbol.as_ref().filter(|s| !s.trim().is_empty()) {
            query.push(HttpQueryParam {
                name: "product_ids".to_string(),
                value: normalize_product_id(pid),
            });
        }
        if let Some(c) = page_cursor.as_ref().filter(|s| !s.is_empty()) {
            query.push(HttpQueryParam {
                name: "cursor".to_string(),
                value: c.clone(),
            });
        }

        let response = broker_get(call, FILLS_PATH, query)?;
        let body = require_ok("fills", &response)?;
        let page = map_fills_page(&body)?;
        let next = page.1;
        fills.extend(page.0);
        if next.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            break;
        }
        page_cursor = next;
    }

    if let Some(since) = cursor.since_unix_ms {
        fills.retain(|f| f.filled_at_unix_ms >= since);
    }
    fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
    Ok(fills)
}

fn normalize_product_id(symbol: &str) -> String {
    let s = symbol.trim().to_uppercase();
    if s.contains('-') {
        s
    } else if s.ends_with("USDT") && s.len() > 4 {
        format!("{}-USDT", &s[..s.len() - 4])
    } else if s.ends_with("USD") && s.len() > 3 {
        format!("{}-USD", &s[..s.len() - 3])
    } else {
        s
    }
}

fn broker_get(
    call: &dyn Fn(&BrokerHttpRequest) -> Result<BrokerHttpResponse, String>,
    path: &str,
    query: Vec<HttpQueryParam>,
) -> Result<BrokerHttpResponse, String> {
    call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: HOST.to_string(),
        path: path.to_string(),
        query,
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
        "coinbase_advanced {label} http {}: {class}",
        response.status
    ))
}

fn map_fills_page(body: &str) -> Result<(Vec<FillEvent>, Option<String>), String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("fills json: {e}"))?;
    let rows = root
        .get("fills")
        .and_then(|v| v.as_array())
        .ok_or("fills missing fills[]")?;
    let cursor = root
        .get("cursor")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(map_fill_row(row)?);
    }
    Ok((out, cursor))
}

fn map_fill_row(row: &serde_json::Value) -> Result<FillEvent, String> {
    let trade_id = row
        .get("trade_id")
        .and_then(|v| v.as_str())
        .ok_or("fill missing trade_id")?
        .to_string();
    let product_id = row
        .get("product_id")
        .and_then(|v| v.as_str())
        .ok_or("fill missing product_id")?
        .to_string();
    let side = row
        .get("side")
        .and_then(|v| v.as_str())
        .ok_or("fill missing side")?
        .to_uppercase();
    let price = parse_decimal(row.get("price"))?;
    let qty = parse_decimal(row.get("size"))?;
    let filled_at_unix_ms = parse_trade_time_ms(row.get("trade_time"))?;
    let fee_amount = parse_decimal_opt(row.get("commission"));
    let fee_currency = quote_from_product(&product_id);
    let order_id = row
        .get("order_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(FillEvent {
        fill_id: trade_id.clone(),
        broker_slug: "coinbase_advanced".to_string(),
        connection_id: String::new(),
        asset_class: AssetClass::Cryptocurrency,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        currency: quote_from_product(&product_id),
        symbol: product_id,
        side,
        qty,
        price,
        filled_at_unix_ms,
        fee_amount,
        fee_currency: fee_amount.map(|_| fee_currency),
        exchange_segment: None,
        product: None,
        trade_id: order_id,
    })
}

fn quote_from_product(product_id: &str) -> String {
    let upper = product_id.trim().to_uppercase();
    for quote in QUOTE_SUFFIXES {
        if upper.ends_with(&format!("-{quote}")) {
            return (*quote).to_string();
        }
    }
    "USD".to_string()
}

fn parse_decimal(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(v) if v.is_string() => v
            .as_str()
            .unwrap_or_default()
            .parse::<f64>()
            .map_err(|e| format!("decimal parse: {e}")),
        Some(v) if v.is_number() => v.as_f64().ok_or_else(|| "decimal number".into()),
        _ => Err("missing decimal field".into()),
    }
}

fn parse_decimal_opt(value: Option<&serde_json::Value>) -> Option<f64> {
    parse_decimal(value).ok()
}

fn parse_trade_time_ms(value: Option<&serde_json::Value>) -> Result<i64, String> {
    let s = value
        .and_then(|v| v.as_str())
        .ok_or("fill missing trade_time")?;
    let parsed = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e| format!("trade_time rfc3339: {e}"))?;
    Ok(parsed.timestamp_millis())
}

fn describe_json() -> String {
    serde_json::json!({
        "manifest_id": "tradeautopsy:coinbase-advanced-spot@0.1.0",
        "adapter_id": "coinbase_advanced",
        "implemented": ["tradebook"],
        "bindings": [{
            "operation": "tradebook",
            "adapter_id": "coinbase_advanced",
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
    let operation = req
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if operation == "history" {
        return Ok(serde_json::json!({"status": "unsupported"}).to_string());
    }
    Ok(serde_json::json!({"status": "unsupported"}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tradeautopsy::ubi_data::types::FillCursor;

    #[test]
    fn maps_fixture_fill_usd_quote() {
        let body = include_str!("../../fixtures/coinbase_advanced/fills_page.json");
        let (fills, cursor) = map_fills_page(body).expect("map");
        assert!(cursor.as_ref().map(|c| c.is_empty()).unwrap_or(true));
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].currency, "USD");
        assert_eq!(fills[0].symbol, "BTC-USD");
        assert_eq!(fills[0].side, "BUY");
        assert_eq!(fills[1].currency, "USD");
    }

    #[test]
    fn product_id_normalization_adds_dash() {
        assert_eq!(normalize_product_id("BTCUSDT"), "BTC-USDT");
        assert_eq!(normalize_product_id("ETH-USD"), "ETH-USD");
    }

    #[test]
    fn fetch_respects_since_cursor() {
        let fixture = include_str!("../../fixtures/coinbase_advanced/fills_page.json");
        let call = |req: &BrokerHttpRequest| {
            assert_eq!(req.path, FILLS_PATH);
            Ok(BrokerHttpResponse {
                status: 200,
                headers: vec![],
                body: fixture.to_string(),
                error_class: None,
            })
        };
        let all = fetch_all_fills(
            &call,
            &FillCursor {
                since_unix_ms: None,
                from_id: None,
                symbol: None,
            },
        )
        .expect("fetch all");
        let btc_ms = all
            .iter()
            .find(|f| f.symbol == "BTC-USD")
            .expect("btc")
            .filled_at_unix_ms;
        let filtered = fetch_all_fills(
            &call,
            &FillCursor {
                since_unix_ms: Some(btc_ms),
                from_id: None,
                symbol: None,
            },
        )
        .expect("fetch filtered");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].symbol, "BTC-USD");
    }
}

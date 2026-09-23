//! Phase-1 fixture Wasm component for UBI contract tests.
//! Maps COM-shaped and Kotak-shaped JSON (via host `broker_http_call`) → FillEvent.
//! Never receives or logs credentials (ADR 0001 / R6).

#![allow(clippy::all)]

wit_bindgen::generate!({
    world: "broker-adapter-data",
    path: "../../docs/contracts",
});

use crate::exports::tradeautopsy::ubi_data::adapter::Guest as AdapterGuest;
use crate::exports::tradeautopsy::ubi_data::data_adapter::Guest as DataAdapterGuest;
use crate::tradeautopsy::ubi_data::broker_http;
use crate::tradeautopsy::ubi_data::types::{
    AssetClass, BrokerHttpRequest, FillCursor, FillEvent, HttpQueryParam, InstrumentClass,
};

struct FixtureAdapter;

export!(FixtureAdapter);

impl AdapterGuest for FixtureAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        let fixture_kind = cursor.from_id.as_deref().unwrap_or("binance_com");

        match fixture_kind {
            "kotak_neo" => fetch_kotak(cursor),
            _ => fetch_binance_com(cursor),
        }
    }
}

impl DataAdapterGuest for FixtureAdapter {
    fn describe() -> Result<String, String> {
        Ok(
            r#"{"manifest_id":"fixture.v1","adapter_id":"fixture","implemented":["tradebook"]}"#
                .into(),
        )
    }

    fn obtain(_request: String) -> Result<String, String> {
        Err("fixture obtain unsupported".into())
    }
}

fn fetch_binance_com(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
    let mut query = vec![];
    if let Some(symbol) = cursor.symbol.clone() {
        query.push(HttpQueryParam {
            name: "symbol".to_string(),
            value: symbol,
        });
    }
    if let Some(since) = cursor.since_unix_ms {
        query.push(HttpQueryParam {
            name: "startTime".to_string(),
            value: since.to_string(),
        });
    }

    let response = broker_http::broker_http_call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: "api.binance.com".to_string(),
        path: "/api/v3/myTrades".to_string(),
        query,
        headers: vec![],
        body: None,
    })?;

    if response.status != 200 {
        return Err(format!(
            "binance_com http {}: {}",
            response.status,
            response.error_class.unwrap_or_default()
        ));
    }

    map_binance_com_body(&response.body)
}

fn fetch_kotak(_cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
    let response = broker_http::broker_http_call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: "cis.kotaksecurities.com".to_string(),
        path: "/quick/user/trades".to_string(),
        query: vec![],
        headers: vec![],
        body: None,
    })?;

    if response.status != 200 {
        return Err(format!(
            "kotak_neo http {}: {}",
            response.status,
            response.error_class.unwrap_or_default()
        ));
    }

    map_kotak_body(&response.body)
}

fn map_binance_com_body(body: &str) -> Result<Vec<FillEvent>, String> {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(body).map_err(|e| format!("com json: {e}"))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row
            .get("id")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
            .ok_or("com missing id")?;
        let is_buyer = row
            .get("isBuyer")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let qty = parse_f64(row.get("qty"))?;
        let price = parse_f64(row.get("price"))?;
        let time_ms = row
            .get("time")
            .and_then(|v| v.as_i64())
            .ok_or("com missing time")?;
        let fee_amount = parse_f64_opt(row.get("commission"));
        let fee_currency = row
            .get("commissionAsset")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let symbol = row
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let order_id = row.get("orderId").map(|v| v.to_string());

        out.push(FillEvent {
            fill_id: id.to_string(),
            // Host overwrites identity + taxonomy axes from the connection book
            // (R5 §3.5; ADR 0004 §1). Placeholder only — adapter never classifies.
            broker_slug: "binance_com".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            symbol,
            side: if is_buyer {
                "BUY".to_string()
            } else {
                "SELL".to_string()
            },
            qty,
            price,
            currency: "USDT".to_string(),
            filled_at_unix_ms: time_ms,
            fee_amount,
            fee_currency,
            exchange_segment: None,
            product: None,
            trade_id: order_id,
        });
    }
    Ok(out)
}

fn map_kotak_body(body: &str) -> Result<Vec<FillEvent>, String> {
    let root: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("kotak json: {e}"))?;
    let rows = root
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or("kotak missing data[]")?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let raw_sym = row
            .get("trdSym")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let symbol = raw_sym
            .strip_suffix("-EQ")
            .or_else(|| raw_sym.strip_suffix("-eq"))
            .unwrap_or(&raw_sym)
            .to_string();
        let side = match row.get("trnsTp").and_then(|v| v.as_str()).unwrap_or("B") {
            "S" | "s" => "SELL".to_string(),
            _ => "BUY".to_string(),
        };
        let qty = parse_f64(row.get("fldQty").or_else(|| row.get("qty")))?;
        let price = parse_f64(row.get("avgPrc"))?;
        let exchange_segment = row
            .get("exSeg")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let product = row
            .get("prod")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let trade_id = row
            .get("exOrdId")
            .or_else(|| row.get("nOrdNo"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let fill_id = trade_id
            .clone()
            .unwrap_or_else(|| format!("{symbol}-{side}-{qty}-{price}"));
        // Phase 1 fixture: stable ms from flDt/exTm when unparsed — use a fixed epoch for contract.
        let filled_at_unix_ms = 1_753_423_530_000_i64; // 2026-07-25 10:15:30 UTC approx for fixture

        out.push(FillEvent {
            fill_id,
            // Same uniform fixture placeholder as the binance path above: the host
            // overwrites identity + taxonomy axes from the connection book
            // (R5 §3.5; ADR 0004 §1), so this shape mapper never classifies.
            broker_slug: "kotak_neo".to_string(),
            connection_id: String::new(),
            asset_class: AssetClass::Cryptocurrency,
            instrument_class: InstrumentClass::Spot,
            is_inverse: false,
            symbol,
            side,
            qty,
            price,
            currency: "INR".to_string(),
            filled_at_unix_ms,
            fee_amount: None,
            fee_currency: None,
            exchange_segment,
            product,
            trade_id,
        });
    }
    Ok(out)
}

fn parse_f64(v: Option<&serde_json::Value>) -> Result<f64, String> {
    match v {
        Some(serde_json::Value::Number(n)) => {
            n.as_f64().ok_or_else(|| "number not f64".to_string())
        }
        Some(serde_json::Value::String(s)) => {
            s.parse().map_err(|e| format!("parse f64 '{s}': {e}"))
        }
        _ => Err("missing number".into()),
    }
}

fn parse_f64_opt(v: Option<&serde_json::Value>) -> Option<f64> {
    parse_f64(v).ok()
}

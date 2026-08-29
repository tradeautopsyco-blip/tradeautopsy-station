//! Open interest from `GET /eapi/v1/openInterest`.
//!
//! Official SDK `OpenInterestResponseInner`: `symbol`, `sumOpenInterest`,
//! `sumOpenInterestUsd`, `timestamp` (strings). Public. Not depth.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OptionsOiRow {
    pub symbol: String,
    #[serde(rename = "sumOpenInterest")]
    pub sum_open_interest: String,
    #[serde(rename = "sumOpenInterestUsd")]
    pub sum_open_interest_usd: String,
    pub timestamp: String,
}

fn row_from_object(value: &Value) -> Option<OptionsOiRow> {
    let symbol = value.get("symbol").and_then(Value::as_str)?.trim();
    if symbol.is_empty() {
        return None;
    }
    let sum_open_interest = value.get("sumOpenInterest").and_then(Value::as_str)?;
    let sum_open_interest_usd = value
        .get("sumOpenInterestUsd")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let timestamp = value
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some(OptionsOiRow {
        symbol: symbol.to_string(),
        sum_open_interest: sum_open_interest.to_string(),
        sum_open_interest_usd,
        timestamp,
    })
}

pub fn oi_rows_from_json(raw: &str) -> Vec<OptionsOiRow> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter().filter_map(row_from_object).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_names_sum_open_interest_string() {
        let json = include_str!("../../fixtures/binance/options_open_interest.json");
        let rows = oi_rows_from_json(json);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "BTC-200730-9000-C");
        assert_eq!(rows[0].sum_open_interest, "12.5");
        assert_ne!(rows[0].symbol, rows[0].symbol.to_ascii_lowercase());
    }

    #[test]
    fn ticker_payload_is_not_oi() {
        let json = include_str!("../../fixtures/binance/options_ticker.json");
        assert!(oi_rows_from_json(json).is_empty());
    }
}

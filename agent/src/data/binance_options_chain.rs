//! Options chain master from `GET /eapi/v1/exchangeInfo` `optionSymbols[]`.
//!
//! Not `/eapi/v1/optionChain` (NOT SPECIFIED). Not ticker-row stuffing.
//! BoundedSnapshot = rows sharing the declared contract's (`underlying`, `expiryDate`).
//! Do not persist `unit` as lot.

use super::glance::ChainRow;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsSymbolRow {
    pub symbol: String,
    pub underlying: String,
    pub strike_price: String,
    pub expiry_date: i64,
    pub side: String,
}

impl OptionsSymbolRow {
    pub fn to_chain_row(&self) -> ChainRow {
        ChainRow {
            instrument_id: self.symbol.clone(),
            lot: 0,
            trading_symbol: self.symbol.clone(),
            segment: String::new(),
            instrument_type: String::new(),
            option_type: self.side.clone(),
            strike_raw: self.strike_price.clone(),
            expiry_raw: self.expiry_date.to_string(),
            last: None,
        }
    }
}

fn row_from_object(value: &Value) -> Option<OptionsSymbolRow> {
    let symbol = value.get("symbol").and_then(Value::as_str)?.trim();
    if symbol.is_empty() {
        return None;
    }
    let underlying = value.get("underlying").and_then(Value::as_str)?.trim();
    if underlying.is_empty() {
        return None;
    }
    let strike_price = value
        .get("strikePrice")
        .and_then(Value::as_str)?
        .to_string();
    let expiry_date = value.get("expiryDate").and_then(Value::as_i64)?;
    let side = value.get("side").and_then(Value::as_str)?.trim();
    if side.is_empty() {
        return None;
    }
    Some(OptionsSymbolRow {
        symbol: symbol.to_string(),
        underlying: underlying.to_string(),
        strike_price,
        expiry_date,
        side: side.to_string(),
    })
}

/// Parse `optionSymbols` from exchangeInfo JSON. Ignores spot `symbols[]`.
pub fn option_symbols_from_exchange_info_json(raw: &str) -> Vec<OptionsSymbolRow> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(arr) = value.get("optionSymbols").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter().filter_map(row_from_object).collect()
}

/// Rows for the declared mixed-case contract: same `underlying` + `expiryDate`.
pub fn chain_rows_for_contract(instrument: &str, rows: &[OptionsSymbolRow]) -> Vec<ChainRow> {
    let wanted = instrument.trim();
    let Some(anchor) = rows.iter().find(|row| row.symbol == wanted) else {
        return Vec::new();
    };
    rows.iter()
        .filter(|row| row.underlying == anchor.underlying && row.expiry_date == anchor.expiry_date)
        .map(OptionsSymbolRow::to_chain_row)
        .collect()
}

/// Official symbol shape `BTC-200730-9000-C` → OI `underlyingAsset` = first segment.
pub fn underlying_asset_from_dated_contract(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let first = trimmed.split('-').next()?;
    if first.is_empty() || first.contains('|') {
        return None;
    }
    Some(first)
}

/// Official symbol shape `BTC-200730-9000-C` → OI `expiration` = YYMMDD second segment.
pub fn expiration_from_dated_contract(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let mut parts = trimmed.split('-');
    let _underlying = parts.next()?;
    let expiration = parts.next()?;
    if expiration.len() == 6 && expiration.bytes().all(|b| b.is_ascii_digit()) {
        return Some(expiration);
    }
    None
}

pub fn binance_options_exchange_info_url() -> String {
    "https://eapi.binance.com/eapi/v1/exchangeInfo".to_string()
}

pub fn binance_options_open_interest_url(underlying_asset: &str, expiration: &str) -> String {
    format!(
        "https://eapi.binance.com/eapi/v1/openInterest?underlyingAsset={underlying_asset}&expiration={expiration}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_chain_is_same_underlying_and_expiry_not_eth() {
        let json = include_str!("../../fixtures/binance/options_exchange_info.json");
        let rows = option_symbols_from_exchange_info_json(json);
        let chain = chain_rows_for_contract("BTC-200730-9000-C", &rows);
        assert_eq!(chain.len(), 3);
        assert!(chain
            .iter()
            .all(|row| row.instrument_id.starts_with("BTC-200730-")));
        assert!(!chain
            .iter()
            .any(|row| row.instrument_id.starts_with("ETH-")));
        assert_eq!(chain[0].option_type, "CALL");
        assert_eq!(chain[0].lot, 0);
        assert_ne!(
            chain[0].instrument_id,
            chain[0].instrument_id.to_ascii_lowercase()
        );
    }

    #[test]
    fn unknown_contract_is_empty() {
        let json = include_str!("../../fixtures/binance/options_exchange_info.json");
        let rows = option_symbols_from_exchange_info_json(json);
        assert!(chain_rows_for_contract("BTCUSDT", &rows).is_empty());
        assert!(chain_rows_for_contract("BTC-200730-9000-c", &rows).is_empty());
    }

    #[test]
    fn spot_exchange_info_is_not_option_symbols() {
        let json = r#"{"symbols":[{"symbol":"BTCUSDT"}]}"#;
        assert!(option_symbols_from_exchange_info_json(json).is_empty());
    }

    #[test]
    fn oi_query_parts_from_official_cli_shape() {
        assert_eq!(
            underlying_asset_from_dated_contract("BTC-200730-9000-C"),
            Some("BTC")
        );
        assert_eq!(
            expiration_from_dated_contract("BTC-200730-9000-C"),
            Some("200730")
        );
        assert!(expiration_from_dated_contract("BTCUSDT").is_none());
        assert!(underlying_asset_from_dated_contract("nse_fo|1").is_none());
    }

    #[test]
    fn urls_are_eapi_not_spot() {
        assert_eq!(
            binance_options_exchange_info_url(),
            "https://eapi.binance.com/eapi/v1/exchangeInfo"
        );
        assert_eq!(
            binance_options_open_interest_url("BTC", "200730"),
            "https://eapi.binance.com/eapi/v1/openInterest?underlyingAsset=BTC&expiration=200730"
        );
    }
}

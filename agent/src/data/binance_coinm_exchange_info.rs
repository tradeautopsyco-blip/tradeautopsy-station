//! Persist Coin-M `GET /dapi/v1/exchangeInfo` filters. Own cache — never USDM.
//!
//! Lock: `issues/compliance/locks/binance-com-coinm.md`. Tick = `PRICE_FILTER.tickSize`.
//! Never `pricePrecision`. HMAC off. TRADE is not this module.

use super::{authorize_book_call, BINANCE_COM_COINM_BOOK_ID};
use crate::api::AppState;
use crate::egress::{EgressCall, Lane};
use crate::usdm_realized_pnl::tick_from_exchange_info_filters;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub const COINM_EXCHANGE_INFO_HOST: &str = "dapi.binance.com";
pub const COINM_EXCHANGE_INFO_PATH: &str = "/dapi/v1/exchangeInfo";

const EXCHANGE_INFO_MAX_AGE_MS: i64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoinmSymbolFilters {
    pub tick_size: Option<String>,
    pub step_size: Option<String>,
    pub min_qty: Option<String>,
    pub max_qty: Option<String>,
}

impl CoinmSymbolFilters {
    fn is_empty(&self) -> bool {
        self.tick_size.is_none()
            && self.step_size.is_none()
            && self.min_qty.is_none()
            && self.max_qty.is_none()
    }
}

/// Coin-M-only filter cache. DualNoBlend vs USDM and spot.
#[derive(Debug, Clone, Default)]
pub struct CoinmExchangeInfoCache {
    by_symbol: HashMap<String, CoinmSymbolFilters>,
}

impl CoinmExchangeInfoCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }

    pub fn contains_symbol(&self, symbol: &str) -> bool {
        self.by_symbol.contains_key(&normalize_coinm_symbol(symbol))
    }

    pub fn get(&self, symbol: &str) -> Option<&CoinmSymbolFilters> {
        self.by_symbol.get(&normalize_coinm_symbol(symbol))
    }

    pub fn tick_size_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.tick_size.clone())
    }

    pub fn step_size_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.step_size.clone())
    }

    pub fn search_symbols(&self, q: &str, limit: usize) -> Vec<String> {
        let q = q.trim().to_ascii_uppercase();
        if q.len() < 2 || limit == 0 {
            return Vec::new();
        }
        let mut hits: Vec<String> = self
            .by_symbol
            .keys()
            .filter(|symbol| symbol.contains(&q))
            .cloned()
            .collect();
        hits.sort();
        hits.truncate(limit);
        hits
    }

    pub fn merge(&mut self, other: Self) {
        self.by_symbol.extend(other.by_symbol);
    }

    pub fn from_exchange_info_json(json: &str) -> Self {
        let Ok(value) = serde_json::from_str::<Value>(json) else {
            return Self::empty();
        };
        let Some(symbols) = value.get("symbols").and_then(Value::as_array) else {
            return Self::empty();
        };
        let mut cache = Self::empty();
        for entry in symbols {
            let Some(symbol) = entry
                .get("symbol")
                .and_then(Value::as_str)
                .map(normalize_coinm_symbol)
            else {
                continue;
            };
            if symbol.is_empty() {
                continue;
            }
            let filters = match entry.get("filters").and_then(Value::as_array) {
                Some(filters) => filters_from_array(filters),
                None => continue,
            };
            if filters.is_empty() {
                continue;
            }
            cache.by_symbol.insert(symbol, filters);
        }
        cache
    }
}

fn normalize_coinm_symbol(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn venue_decimal(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn filters_from_array(filters: &[Value]) -> CoinmSymbolFilters {
    let mut out = CoinmSymbolFilters {
        tick_size: tick_from_exchange_info_filters(filters),
        step_size: None,
        min_qty: None,
        max_qty: None,
    };
    for filter in filters {
        if filter.get("filterType").and_then(Value::as_str) != Some("LOT_SIZE") {
            continue;
        }
        out.step_size = filter.get("stepSize").and_then(venue_decimal);
        out.min_qty = filter.get("minQty").and_then(venue_decimal);
        out.max_qty = filter.get("maxQty").and_then(venue_decimal);
    }
    out
}

pub fn coinm_tick_size_for(state: &AppState, symbol: &str) -> Option<String> {
    state
        .coinm_exchange_info
        .lock()
        .expect("coinm exchange info mutex poisoned")
        .tick_size_for(symbol)
}

pub fn coinm_step_size_for(state: &AppState, symbol: &str) -> Option<String> {
    state
        .coinm_exchange_info
        .lock()
        .expect("coinm exchange info mutex poisoned")
        .step_size_for(symbol)
}

pub async fn ensure_coinm_exchange_info(state: &AppState, symbol: &str) {
    let key = normalize_coinm_symbol(symbol);
    {
        let cache = state
            .coinm_exchange_info
            .lock()
            .expect("coinm exchange info mutex poisoned");
        if key.is_empty() {
            if !cache.is_empty() {
                return;
            }
        } else if cache.tick_size_for(&key).is_some() && cache.step_size_for(&key).is_some() {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_COINM_BOOK_ID,
        COINM_EXCHANGE_INFO_HOST,
        "GET",
        COINM_EXCHANGE_INFO_PATH,
        false,
    )
    .is_err()
    {
        return;
    }
    let call = EgressCall::get(
        BINANCE_COM_COINM_BOOK_ID,
        COINM_EXCHANGE_INFO_HOST,
        COINM_EXCHANGE_INFO_PATH,
        Lane::MarketData,
    )
    .with_max_age_ms(EXCHANGE_INFO_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let parsed = CoinmExchangeInfoCache::from_exchange_info_json(&resp.body);
    if parsed.is_empty() {
        return;
    }
    state
        .coinm_exchange_info
        .lock()
        .expect("coinm exchange info mutex poisoned")
        .merge(parsed);
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../fixtures/binance/coinm_exchange_info.json");

    #[test]
    fn fixture_tick_is_tick_size_not_price_precision() {
        let cache = CoinmExchangeInfoCache::from_exchange_info_json(FIXTURE);
        assert_eq!(cache.tick_size_for("BTCUSD_PERP").as_deref(), Some("0.10"));
        assert_ne!(cache.tick_size_for("BTCUSD_PERP").as_deref(), Some("8"));
        assert_eq!(cache.step_size_for("BTCUSD_PERP").as_deref(), Some("1"));
        assert!(cache.contains_symbol("BTCUSD_PERP"));
        assert!(!cache.contains_symbol("BTCUSDT"));
        assert_eq!(
            cache.search_symbols("btcusd", 10),
            vec!["BTCUSD_PERP".to_string()]
        );
        assert!(cache.search_symbols("btcusdt", 10).is_empty());
    }
}

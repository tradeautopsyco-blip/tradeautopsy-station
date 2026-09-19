//! Persist USDM `GET /fapi/v1/exchangeInfo` filters for the sizer.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md`. Tick = `PRICE_FILTER.tickSize`
//! (never `pricePrecision`). Lot = `LOT_SIZE` `minQty` / `maxQty` / `stepSize` when
//! named. Third identity: never write spot `exchange_info.rs` / `instrument_master`.
//! HMAC off. TRADE place is not this module.
//!
//! Kick: call [`ensure_usdm_exchange_info`] from a USDM quote/bind. `desk.rs`
//! `bind_quote_selection` is owned by the quotes agent — not wired here.
//! `kick_usdm_private` has a `quotes` arm that fires once obtain(quotes) is
//! Unavailable rather than Unsupported.

use super::{authorize_book_call, BINANCE_COM_USDM_BOOK_ID};
use crate::api::AppState;
use crate::egress::{EgressCall, Lane};
use crate::usdm_realized_pnl::tick_from_exchange_info_filters;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub const USDM_EXCHANGE_INFO_HOST: &str = "fapi.binance.com";
pub const USDM_EXCHANGE_INFO_PATH: &str = "/fapi/v1/exchangeInfo";

/// Unparameterised exchangeInfo is the listing set, not a per-tick poll.
const EXCHANGE_INFO_MAX_AGE_MS: i64 = 300_000;

/// Venue filter strings as published. Keep decimals; do not integer-cast qty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsdmSymbolFilters {
    pub tick_size: Option<String>,
    pub step_size: Option<String>,
    pub min_qty: Option<String>,
    pub max_qty: Option<String>,
}

impl UsdmSymbolFilters {
    fn is_empty(&self) -> bool {
        self.tick_size.is_none()
            && self.step_size.is_none()
            && self.min_qty.is_none()
            && self.max_qty.is_none()
    }
}

/// USDM-only filter cache. DualNoBlend vs spot `ExchangeInfoSymbolCache`.
#[derive(Debug, Clone, Default)]
pub struct UsdmExchangeInfoCache {
    by_symbol: HashMap<String, UsdmSymbolFilters>,
}

impl UsdmExchangeInfoCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    pub fn contains_symbol(&self, symbol: &str) -> bool {
        self.by_symbol.contains_key(&normalize_usdm_symbol(symbol))
    }

    pub fn get(&self, symbol: &str) -> Option<&UsdmSymbolFilters> {
        self.by_symbol.get(&normalize_usdm_symbol(symbol))
    }

    /// Sizer tick. Venue `tickSize` string. Never `pricePrecision`.
    pub fn tick_size_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.tick_size.clone())
    }

    /// Sizer lot step. Venue `stepSize` string. Do not integer-cast qty.
    pub fn step_size_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.step_size.clone())
    }

    pub fn min_qty_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.min_qty.clone())
    }

    pub fn max_qty_for(&self, symbol: &str) -> Option<String> {
        self.get(symbol).and_then(|f| f.max_qty.clone())
    }

    /// Prefix/contains search on persisted listing symbols. Empty cache → `[]`.
    /// Never the spot `instrument_master`.
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

    /// Parse `symbols[]` → PRICE_FILTER / LOT_SIZE. Ignores `pricePrecision`.
    pub fn from_exchange_info_json(json: &str) -> Self {
        let Ok(value) = serde_json::from_str::<Value>(json) else {
            return Self::empty();
        };
        Self::from_exchange_info_value(&value)
    }

    fn from_exchange_info_value(value: &Value) -> Self {
        let Some(symbols) = value.get("symbols").and_then(Value::as_array) else {
            return Self::empty();
        };
        let mut cache = Self::empty();
        for entry in symbols {
            let Some(symbol) = entry
                .get("symbol")
                .and_then(Value::as_str)
                .map(normalize_usdm_symbol)
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

fn normalize_usdm_symbol(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

/// Keep the venue decimal as a string. Never `as_i64` / integer-cast qty.
fn venue_decimal(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn field_decimal(obj: &Value, key: &str) -> Option<String> {
    obj.get(key).and_then(venue_decimal)
}

fn filters_from_array(filters: &[Value]) -> UsdmSymbolFilters {
    let mut out = UsdmSymbolFilters {
        tick_size: tick_from_exchange_info_filters(filters),
        step_size: None,
        min_qty: None,
        max_qty: None,
    };
    for filter in filters {
        if filter.get("filterType").and_then(Value::as_str) != Some("LOT_SIZE") {
            continue;
        }
        out.step_size = field_decimal(filter, "stepSize");
        out.min_qty = field_decimal(filter, "minQty");
        out.max_qty = field_decimal(filter, "maxQty");
    }
    out
}

/// Sizer API: persisted `tickSize` for this USDM symbol, or none until fetched.
pub fn tick_size_for(state: &AppState, symbol: &str) -> Option<String> {
    state
        .usdm_exchange_info
        .lock()
        .expect("usdm exchange info mutex poisoned")
        .tick_size_for(symbol)
}

/// Sizer API: persisted `stepSize` for this USDM symbol, or none until fetched.
pub fn step_size_for(state: &AppState, symbol: &str) -> Option<String> {
    state
        .usdm_exchange_info
        .lock()
        .expect("usdm exchange info mutex poisoned")
        .step_size_for(symbol)
}

/// Fetch unsigned `GET /fapi/v1/exchangeInfo` and persist filters for `symbol`.
///
/// SDK `exchange_information` sends no `?symbol=` — fetch the listing and pick.
/// HMAC off. If host_policy has not allow-listed the path yet, return without dial.
pub async fn ensure_usdm_exchange_info(state: &AppState, symbol: &str) {
    let key = normalize_usdm_symbol(symbol);
    {
        let cache = state
            .usdm_exchange_info
            .lock()
            .expect("usdm exchange info mutex poisoned");
        if key.is_empty() {
            if !cache.is_empty() {
                return;
            }
        } else if cache.tick_size_for(&key).is_some() && cache.step_size_for(&key).is_some() {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_USDM_BOOK_ID,
        USDM_EXCHANGE_INFO_HOST,
        "GET",
        USDM_EXCHANGE_INFO_PATH,
        false,
    )
    .is_err()
    {
        return;
    }
    let call = EgressCall::get(
        BINANCE_COM_USDM_BOOK_ID,
        USDM_EXCHANGE_INFO_HOST,
        USDM_EXCHANGE_INFO_PATH,
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
    let parsed = UsdmExchangeInfoCache::from_exchange_info_json(&resp.body);
    if parsed.is_empty() {
        return;
    }
    state
        .usdm_exchange_info
        .lock()
        .expect("usdm exchange info mutex poisoned")
        .merge(parsed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange_info::ExchangeInfoSymbolCache;

    const FIXTURE: &str = include_str!("../../fixtures/binance/usdm_exchange_info.json");

    #[test]
    fn fixture_tick_is_tick_size_not_price_precision() {
        let cache = UsdmExchangeInfoCache::from_exchange_info_json(FIXTURE);
        assert_eq!(cache.tick_size_for("BTCUSDT").as_deref(), Some("0.10"));
        assert_eq!(cache.tick_size_for("btcusdt").as_deref(), Some("0.10"));
        assert_ne!(cache.tick_size_for("BTCUSDT").as_deref(), Some("8"));
        assert_eq!(cache.step_size_for("BTCUSDT").as_deref(), Some("0.001"));
        assert_eq!(cache.min_qty_for("BTCUSDT").as_deref(), Some("0.001"));
        assert_eq!(cache.max_qty_for("BTCUSDT").as_deref(), Some("1000"));
        assert!(cache.contains_symbol("BTCUSDT"));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.search_symbols("btc", 10), vec!["BTCUSDT".to_string()]);
        assert!(cache.search_symbols("nifty", 10).is_empty());
        assert!(cache.search_symbols("b", 10).is_empty());
        let filters: &UsdmSymbolFilters = cache.get("BTCUSDT").expect("btc filters");
        assert_eq!(filters.tick_size.as_deref(), Some("0.10"));
        let _: fn(&AppState, &str) -> Option<String> = tick_size_for;
        let _: fn(&AppState, &str) -> Option<String> = step_size_for;
    }

    #[test]
    fn qty_two_does_not_collapse_to_one() {
        let json = r#"{
            "symbols": [{
                "symbol": "BTCUSDT",
                "pricePrecision": 8,
                "filters": [
                    {"filterType": "PRICE_FILTER", "tickSize": "0.10"},
                    {"filterType": "LOT_SIZE", "minQty": "2", "maxQty": "2", "stepSize": "2"}
                ]
            }]
        }"#;
        let cache = UsdmExchangeInfoCache::from_exchange_info_json(json);
        assert_eq!(cache.min_qty_for("BTCUSDT").as_deref(), Some("2"));
        assert_ne!(cache.min_qty_for("BTCUSDT").as_deref(), Some("1"));
        assert_eq!(cache.max_qty_for("BTCUSDT").as_deref(), Some("2"));
        assert_eq!(cache.step_size_for("BTCUSDT").as_deref(), Some("2"));
    }

    #[test]
    fn price_precision_only_is_not_a_tick() {
        let json = r#"{
            "symbols": [{
                "symbol": "BTCUSDT",
                "pricePrecision": 8,
                "filters": []
            }]
        }"#;
        let cache = UsdmExchangeInfoCache::from_exchange_info_json(json);
        assert!(cache.tick_size_for("BTCUSDT").is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn dual_no_blend_does_not_write_spot_cache() {
        let usdm = UsdmExchangeInfoCache::from_exchange_info_json(FIXTURE);
        let spot = ExchangeInfoSymbolCache::empty();
        assert_eq!(usdm.tick_size_for("BTCUSDT").as_deref(), Some("0.10"));
        assert!(
            spot.get("BTCUSDT").is_none(),
            "USDM parser must not populate spot ExchangeInfoSymbolCache"
        );
        assert!(!spot.contains_symbol("BTCUSDT"));
    }
}

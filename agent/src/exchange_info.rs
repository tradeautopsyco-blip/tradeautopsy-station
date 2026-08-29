//! Binance.US / Binance.com spot symbol metadata from `GET /api/v3/exchangeInfo`.
//!
//! Authoritative source for `baseAsset` / `quoteAsset` per symbol — used by fee normalization
//! instead of symbol-string heuristics when populated.
//!
//! I-S4: LOT_SIZE / MIN_NOTIONAL|NOTIONAL / PRICE_FILTER.tickSize are stored and consulted.
//! `pricePrecision` is never treated as tick. An empty cache is not “filters ready”.

use serde::Deserialize;
use std::collections::HashMap;

/// Per-symbol assets from exchangeInfo `symbols[]`.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolAssets {
    pub base_asset: String,
    pub quote_asset: String,
    pub filters: Option<SymbolFilters>,
}

/// Search hit from the local COM instrument master. Not a Zerodha row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentSearchHit {
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
}

/// Venue filters stored from exchangeInfo (I-S4). Not an order-placement validator.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolFilters {
    pub min_qty: Option<f64>,
    pub max_qty: Option<f64>,
    pub step_size: Option<f64>,
    pub min_notional: Option<f64>,
    pub tick_size: Option<f64>,
}

impl SymbolFilters {
    pub fn lot_and_tick_present(&self) -> bool {
        self.min_qty.is_some() && self.step_size.is_some() && self.tick_size.is_some()
    }
}

/// In-memory cache of exchangeInfo symbol metadata.
#[derive(Debug, Clone, Default)]
pub struct ExchangeInfoSymbolCache {
    by_symbol: HashMap<String, SymbolAssets>,
}

impl ExchangeInfoSymbolCache {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, symbol: &str, base_asset: &str, quote_asset: &str) {
        self.insert_with_filters(symbol, base_asset, quote_asset, None);
    }

    pub fn insert_with_filters(
        &mut self,
        symbol: &str,
        base_asset: &str,
        quote_asset: &str,
        filters: Option<SymbolFilters>,
    ) {
        self.by_symbol.insert(
            symbol.to_ascii_uppercase(),
            SymbolAssets {
                base_asset: base_asset.to_ascii_uppercase(),
                quote_asset: quote_asset.to_ascii_uppercase(),
                filters,
            },
        );
    }

    pub fn get(&self, symbol: &str) -> Option<&SymbolAssets> {
        self.by_symbol.get(&symbol.to_ascii_uppercase())
    }

    /// Consult stored LOT_SIZE / notional / tick (I-S4).
    pub fn filters_for(&self, symbol: &str) -> Option<&SymbolFilters> {
        self.get(symbol).and_then(|s| s.filters.as_ref())
    }

    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }

    pub fn contains_symbol(&self, symbol: &str) -> bool {
        self.by_symbol.contains_key(&symbol.to_ascii_uppercase())
    }

    pub fn iter_symbols(&self) -> impl Iterator<Item = &str> {
        self.by_symbol.keys().map(|s| s.as_str())
    }

    /// Prefix search on TRADING symbols and base assets. Shortest symbol first.
    pub fn search_trading(&self, q: &str, limit: usize) -> Vec<InstrumentSearchHit> {
        let q = q.trim().to_ascii_uppercase();
        if q.len() < 2 {
            return Vec::new();
        }
        let mut hits: Vec<InstrumentSearchHit> = self
            .by_symbol
            .iter()
            .filter(|(symbol, assets)| symbol.starts_with(&q) || assets.base_asset.starts_with(&q))
            .map(|(symbol, assets)| InstrumentSearchHit {
                symbol: symbol.clone(),
                base_asset: assets.base_asset.clone(),
                quote_asset: assets.quote_asset.clone(),
            })
            .collect();
        hits.sort_by(|a, b| {
            a.symbol
                .len()
                .cmp(&b.symbol.len())
                .then_with(|| a.symbol.cmp(&b.symbol))
        });
        hits.truncate(limit);
        hits
    }

    /// I-S4: empty cache (or symbols without LOT_SIZE + tickSize) is never “filters ready”.
    /// Live COM must not treat this as success / silently heuristic-as-ok.
    pub fn filters_ready(&self) -> bool {
        self.by_symbol
            .values()
            .any(|s| s.filters.as_ref().is_some_and(|f| f.lot_and_tick_present()))
    }

    /// Parse `GET /api/v3/exchangeInfo` JSON — keeps `TRADING` symbols only.
    pub fn from_exchange_info_json(json: &str) -> Result<Self, serde_json::Error> {
        let parsed: ExchangeInfoResponse = serde_json::from_str(json)?;
        let mut cache = Self::empty();
        for entry in parsed.symbols {
            if entry.status.as_deref() != Some("TRADING") {
                continue;
            }
            let filters = parse_symbol_filters(entry.filters.as_deref());
            cache.insert_with_filters(
                &entry.symbol,
                &entry.base_asset,
                &entry.quote_asset,
                filters,
            );
        }
        Ok(cache)
    }
}

/// I-S4: live COM path — empty / filter-less cache is not ready.
pub fn live_com_filters_ready(broker_slug: Option<&str>, cache: &ExchangeInfoSymbolCache) -> bool {
    match broker_slug.map(|s| s.to_ascii_lowercase()) {
        Some(slug) if slug == "binance_com" => cache.filters_ready(),
        _ => true,
    }
}

/// USD-pegged stablecoins treated as 1:1 USD for fee normalization and P&L denomination.
pub fn is_usd_pegged_stablecoin(asset: &str) -> bool {
    matches!(asset, "USDT" | "USD" | "USDC" | "BUSD")
}

/// Returns `(base, quote, used_heuristic)`.
pub fn resolve_symbol_assets(
    cache: &ExchangeInfoSymbolCache,
    symbol: &str,
    allow_heuristic_fallback: bool,
) -> Option<(String, String, bool)> {
    if let Some(SymbolAssets {
        base_asset,
        quote_asset,
        ..
    }) = cache.get(symbol)
    {
        return Some((base_asset.clone(), quote_asset.clone(), false));
    }
    if !allow_heuristic_fallback {
        return None;
    }
    heuristic_symbol_assets(symbol).map(|(b, q)| (b, q, true))
}

/// Whether a symbol's quote asset is USD-pegged (MECHANICS.md §3).
///
/// Returns `false` when quote identity cannot be resolved — safe-by-construction default.
pub fn is_usd_quoted_symbol(
    cache: &ExchangeInfoSymbolCache,
    symbol: &str,
    allow_heuristic_fallback: bool,
) -> bool {
    resolve_symbol_assets(cache, symbol, allow_heuristic_fallback)
        .map(|(_, quote, _)| is_usd_pegged_stablecoin(&quote))
        .unwrap_or(false)
}

/// Symbol-string heuristic — used only when exchangeInfo cache miss and fallback enabled.
fn heuristic_symbol_assets(symbol: &str) -> Option<(String, String)> {
    let upper = symbol.to_ascii_uppercase();
    for quote in ["USDT", "USDC", "BUSD", "USD"] {
        if let Some(base) = upper.strip_suffix(quote) {
            if !base.is_empty() {
                return Some((base.to_string(), quote.to_string()));
            }
        }
    }
    None
}

fn parse_f64_str(raw: Option<&str>) -> Option<f64> {
    raw.and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn parse_symbol_filters(filters: Option<&[ExchangeInfoFilter]>) -> Option<SymbolFilters> {
    let filters = filters?;
    let mut out = SymbolFilters {
        min_qty: None,
        max_qty: None,
        step_size: None,
        min_notional: None,
        tick_size: None,
    };
    for filter in filters {
        match filter.filter_type.as_str() {
            "LOT_SIZE" => {
                out.min_qty = parse_f64_str(filter.min_qty.as_deref());
                out.max_qty = parse_f64_str(filter.max_qty.as_deref());
                out.step_size = parse_f64_str(filter.step_size.as_deref());
            }
            "MIN_NOTIONAL" | "NOTIONAL" => {
                out.min_notional = parse_f64_str(filter.min_notional.as_deref());
            }
            "PRICE_FILTER" => {
                // I-S4: tick comes from tickSize only — never pricePrecision.
                out.tick_size = parse_f64_str(filter.tick_size.as_deref());
            }
            _ => {}
        }
    }
    if out.min_qty.is_none()
        && out.max_qty.is_none()
        && out.step_size.is_none()
        && out.min_notional.is_none()
        && out.tick_size.is_none()
    {
        return None;
    }
    Some(out)
}

#[derive(Debug, Deserialize)]
struct ExchangeInfoResponse {
    symbols: Vec<ExchangeInfoSymbolEntry>,
}

#[derive(Debug, Deserialize)]
struct ExchangeInfoSymbolEntry {
    symbol: String,
    #[serde(rename = "baseAsset")]
    base_asset: String,
    #[serde(rename = "quoteAsset")]
    quote_asset: String,
    status: Option<String>,
    /// Present on the venue payload; I-S4 forbids using this as tick.
    #[serde(rename = "pricePrecision", default)]
    _price_precision: Option<i64>,
    #[serde(default)]
    filters: Option<Vec<ExchangeInfoFilter>>,
}

#[derive(Debug, Deserialize)]
struct ExchangeInfoFilter {
    #[serde(rename = "filterType")]
    filter_type: String,
    #[serde(rename = "minQty")]
    min_qty: Option<String>,
    #[serde(rename = "maxQty")]
    max_qty: Option<String>,
    #[serde(rename = "stepSize")]
    step_size: Option<String>,
    #[serde(rename = "minNotional")]
    min_notional: Option<String>,
    #[serde(rename = "tickSize")]
    tick_size: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILTERS_FIXTURE: &str = r#"{
        "symbols": [
            {
                "symbol": "BTCUSDT",
                "baseAsset": "BTC",
                "quoteAsset": "USDT",
                "status": "TRADING",
                "pricePrecision": 8,
                "filters": [
                    {
                        "filterType": "PRICE_FILTER",
                        "minPrice": "0.01000000",
                        "maxPrice": "1000000.00000000",
                        "tickSize": "0.01000000"
                    },
                    {
                        "filterType": "LOT_SIZE",
                        "minQty": "0.00001000",
                        "maxQty": "9000.00000000",
                        "stepSize": "0.00001000"
                    },
                    {
                        "filterType": "MIN_NOTIONAL",
                        "minNotional": "10.00000000"
                    }
                ]
            },
            {
                "symbol": "ETHUSDT",
                "baseAsset": "ETH",
                "quoteAsset": "USDT",
                "status": "TRADING",
                "pricePrecision": 2,
                "filters": [
                    {
                        "filterType": "PRICE_FILTER",
                        "tickSize": "0.01000000"
                    },
                    {
                        "filterType": "LOT_SIZE",
                        "minQty": "0.00010000",
                        "maxQty": "9000.00000000",
                        "stepSize": "0.00010000"
                    },
                    {
                        "filterType": "NOTIONAL",
                        "minNotional": "5.00000000"
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn parses_exchange_info_json() {
        let json = r#"{
            "symbols": [
                { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" },
                { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" },
                { "symbol": "OLDCOIN", "baseAsset": "OLD", "quoteAsset": "USDT", "status": "BREAK" }
            ]
        }"#;
        let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
        assert_eq!(cache.len(), 2);
        let btc = cache.get("BTCUSDT").expect("btcusdt");
        assert_eq!(btc.base_asset, "BTC");
        assert_eq!(btc.quote_asset, "USDT");
        let eth = cache.get("ethbtc").expect("ethbtc case-insensitive");
        assert_eq!(eth.base_asset, "ETH");
        assert_eq!(eth.quote_asset, "BTC");
    }

    #[test]
    fn search_trading_matches_symbol_and_base() {
        let json = r#"{
            "symbols": [
                { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" },
                { "symbol": "ETHUSDT", "baseAsset": "ETH", "quoteAsset": "USDT", "status": "TRADING" }
            ]
        }"#;
        let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
        let hits = cache.search_trading("btc", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].symbol, "BTCUSDT");
        assert_eq!(hits[0].base_asset, "BTC");
        assert!(cache.contains_symbol("btcusdt"));
    }

    #[test]
    fn is_usd_quoted_symbol_from_cache_and_heuristic() {
        let json = r#"{
            "symbols": [
                { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" },
                { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" }
            ]
        }"#;
        let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
        assert!(is_usd_quoted_symbol(&cache, "BTCUSDT", false));
        assert!(!is_usd_quoted_symbol(&cache, "ETHBTC", false));
        assert!(is_usd_quoted_symbol(
            &ExchangeInfoSymbolCache::empty(),
            "SOLUSDT",
            true
        ));
        assert!(!is_usd_quoted_symbol(
            &ExchangeInfoSymbolCache::empty(),
            "ETHBTC",
            true
        ));
    }

    #[test]
    fn filters_are_persisted_from_fixture_json() {
        // I-S4
        let cache =
            ExchangeInfoSymbolCache::from_exchange_info_json(FILTERS_FIXTURE).expect("parse");
        let btc = cache.filters_for("BTCUSDT").expect("btc filters");
        assert!((btc.min_qty.unwrap() - 0.00001).abs() < 1e-12);
        assert!((btc.max_qty.unwrap() - 9000.0).abs() < 1e-9);
        assert!((btc.step_size.unwrap() - 0.00001).abs() < 1e-12);
        assert!((btc.min_notional.unwrap() - 10.0).abs() < 1e-9);
        assert!((btc.tick_size.unwrap() - 0.01).abs() < 1e-12);
        // pricePrecision=8 must not be used as tick.
        assert!((btc.tick_size.unwrap() - 8.0).abs() > 1.0);

        let eth = cache.filters_for("ETHUSDT").expect("eth filters");
        assert!((eth.min_notional.unwrap() - 5.0).abs() < 1e-9);
        assert!(cache.filters_ready());
    }

    #[test]
    fn empty_cache_is_not_filters_ready() {
        // I-S4: empty cache at boot is not success for live COM.
        let empty = ExchangeInfoSymbolCache::empty();
        assert!(!empty.filters_ready());
        assert!(!live_com_filters_ready(Some("binance_com"), &empty));
        assert!(
            live_com_filters_ready(Some("kotak_neo"), &empty),
            "Kotak does not use COM exchangeInfo filters"
        );
        let symbols_only = ExchangeInfoSymbolCache::from_exchange_info_json(
            r#"{ "symbols": [
                { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING", "pricePrecision": 8 }
            ] }"#,
        )
        .expect("parse");
        assert!(
            !symbols_only.filters_ready(),
            "pricePrecision-only payload is not filters ready"
        );
        assert!(!live_com_filters_ready(Some("binance_com"), &symbols_only));
    }
}

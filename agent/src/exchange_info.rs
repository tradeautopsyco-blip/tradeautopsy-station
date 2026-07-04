//! Binance.US spot symbol metadata from `GET /api/v3/exchangeInfo` (REST.md).
//!
//! Authoritative source for `baseAsset` / `quoteAsset` per symbol — used by fee normalization
//! instead of symbol-string heuristics when populated.

use serde::Deserialize;
use std::collections::HashMap;

/// Per-symbol assets from exchangeInfo `symbols[]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolAssets {
    pub base_asset: String,
    pub quote_asset: String,
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
        self.by_symbol.insert(
            symbol.to_ascii_uppercase(),
            SymbolAssets {
                base_asset: base_asset.to_ascii_uppercase(),
                quote_asset: quote_asset.to_ascii_uppercase(),
            },
        );
    }

    pub fn get(&self, symbol: &str) -> Option<&SymbolAssets> {
        self.by_symbol.get(&symbol.to_ascii_uppercase())
    }

    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }

    /// Parse `GET /api/v3/exchangeInfo` JSON — keeps `TRADING` symbols only.
    pub fn from_exchange_info_json(json: &str) -> Result<Self, serde_json::Error> {
        let parsed: ExchangeInfoResponse = serde_json::from_str(json)?;
        let mut cache = Self::empty();
        for entry in parsed.symbols {
            if entry.status.as_deref() != Some("TRADING") {
                continue;
            }
            cache.insert(&entry.symbol, &entry.base_asset, &entry.quote_asset);
        }
        Ok(cache)
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(is_usd_quoted_symbol(&ExchangeInfoSymbolCache::empty(), "SOLUSDT", true));
        assert!(!is_usd_quoted_symbol(&ExchangeInfoSymbolCache::empty(), "ETHBTC", true));
    }
}

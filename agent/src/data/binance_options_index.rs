//! Options `GET /eapi/v1/index` → LatestState `indexPrice` (lock-named S).
//!
//! Lock: `locks/binance-com-options.md` (fetched 2026-09-10 IST). Official
//! `IndexPriceResponse`: alias `indexPrice` (string, “index Price”) + `time`
//! (integer). Query is `underlying=` — the catalog `optionSymbols.underlying`
//! (e.g. `BTCUSDT` / `XRPUSDT`), **not** OI’s `underlyingAsset=BTC`.
//!
//! Empty / fail is unavailable, never last=0, never spot TickBook last, never
//! option `lastPrice` / `markPrice` as S. Public, no HMAC.

use super::binance_options_chain::OptionsSymbolRow;
use serde_json::Value;

pub const OPTIONS_INDEX_HOST: &str = "eapi.binance.com";
pub const OPTIONS_INDEX_PATH: &str = "/eapi/v1/index";

/// One cached `/eapi/v1/index` row, keyed by catalog `underlying` (`BTCUSDT`).
///
/// Index is per underlying, not per dated contract. A request whose catalog
/// underlying differs is a miss — never a repaint of the previous S.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedIndex {
    pub underlying: String,
    pub index_price: String,
    pub time: Option<i64>,
    pub as_of: String,
}

/// `?underlying=` from the catalog row. Never `underlyingAsset=` (that is OI;
/// live `BTC` on this path is `-1128 Invalid underlying`).
pub fn options_index_query(underlying: &str) -> String {
    format!("underlying={}", underlying.trim())
}

/// Catalog `optionSymbols.underlying` for this mixed-case contract, or none.
/// Typing `XRP` / first-segment `BTC` is not a substitute.
pub fn index_underlying_for_contract<'a>(
    instrument: &str,
    rows: &'a [OptionsSymbolRow],
) -> Option<&'a str> {
    let wanted = instrument.trim();
    if wanted.is_empty() {
        return None;
    }
    rows.iter()
        .find(|row| row.symbol == wanted)
        .map(|row| row.underlying.as_str())
        .filter(|u| !u.is_empty())
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

/// Lock-named `indexPrice` string, or none. Empty / missing / a ticker or mark
/// body is a miss — never `"0"`.
pub fn index_price_from_json(raw: &str) -> Option<CachedIndex> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return None;
    };
    let obj = if let Some(arr) = value.as_array() {
        arr.first()?.as_object()?
    } else {
        value.as_object()?
    };
    let index_price = obj.get("indexPrice").and_then(Value::as_str)?.trim();
    if index_price.is_empty() {
        return None;
    }
    let time = obj.get("time").and_then(json_i64);
    Some(CachedIndex {
        underlying: String::new(),
        index_price: index_price.to_string(),
        time,
        as_of: String::new(),
    })
}

pub fn index_price_from_json_for_underlying(raw: &str, underlying: &str) -> Option<CachedIndex> {
    let want = underlying.trim();
    if want.is_empty() {
        return None;
    }
    let mut row = index_price_from_json(raw)?;
    row.underlying = want.to_string();
    Some(row)
}

/// Cached row only when it is for this catalog underlying.
pub fn cached_index_hit<'a>(
    cached: &'a Option<CachedIndex>,
    underlying: &str,
) -> Option<&'a CachedIndex> {
    let want = underlying.trim();
    cached
        .as_ref()
        .filter(|hit| hit.underlying == want && !hit.index_price.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::option_symbols_from_exchange_info_json;

    #[test]
    fn planted_fixture_names_index_price_not_last_or_mark() {
        let json = include_str!("../../fixtures/binance/options_index.json");
        let row = index_price_from_json(json).expect("fixture");
        assert_eq!(row.index_price, "27670.21666667");
        assert_eq!(row.time, Some(1_655_421_615_000));
        assert!(!json.contains("lastPrice"));
        assert!(!json.contains("markPrice"));
        assert_eq!(OPTIONS_INDEX_PATH, "/eapi/v1/index");
        assert_eq!(OPTIONS_INDEX_HOST, "eapi.binance.com");
    }

    #[test]
    fn empty_or_foreign_body_is_not_an_index() {
        assert!(index_price_from_json("").is_none());
        assert!(index_price_from_json("{}").is_none());
        assert!(index_price_from_json(r#"{"indexPrice":""}"#).is_none());
        assert!(index_price_from_json(r#"{"indexPrice":"   "}"#).is_none());
        assert!(index_price_from_json(r#"{"lastPrice":"1.23"}"#).is_none());
        assert!(index_price_from_json(r#"{"markPrice":"1343.2883"}"#).is_none());
        assert!(index_price_from_json(r#"{"c":"100.00"}"#).is_none());
        assert!(index_price_from_json("[]").is_none());
    }

    #[test]
    fn query_is_underlying_equals_catalog_not_oi_asset() {
        assert_eq!(options_index_query("BTCUSDT"), "underlying=BTCUSDT");
        assert_eq!(options_index_query("  XRPUSDT  "), "underlying=XRPUSDT");
        assert_ne!(options_index_query("BTCUSDT"), "underlyingAsset=BTC");
        assert_ne!(options_index_query("BTCUSDT"), "underlying=BTC");
        assert_ne!(options_index_query("BTCUSDT"), "symbol=BTCUSDT");
    }

    #[test]
    fn catalog_underlying_comes_from_option_symbols_not_typed_xrp() {
        let rows = option_symbols_from_exchange_info_json(include_str!(
            "../../fixtures/binance/options_exchange_info.json"
        ));
        assert_eq!(
            index_underlying_for_contract("BTC-200730-9000-C", &rows),
            Some("BTCUSDT")
        );
        assert_eq!(
            index_underlying_for_contract("  BTC-200730-9000-C  ", &rows),
            Some("BTCUSDT")
        );
        // Mixed-case mismatch is a miss — do not smash lowercase onto the desk key.
        assert!(index_underlying_for_contract("btc-200730-9000-c", &rows).is_none());
        // Typing the coin / OI asset / a leftover pair is not the catalog row.
        assert!(index_underlying_for_contract("XRP", &rows).is_none());
        assert!(index_underlying_for_contract("BTC", &rows).is_none());
        assert!(index_underlying_for_contract("BTCUSDT", &rows).is_none());
        assert!(index_underlying_for_contract("", &rows).is_none());
    }

    #[test]
    fn cache_hit_is_per_underlying_never_a_foreign_s() {
        let row = CachedIndex {
            underlying: "BTCUSDT".into(),
            index_price: "27670.21666667".into(),
            time: Some(1),
            as_of: "2026-09-10T00:00:00Z".into(),
        };
        let store = Some(row);
        assert_eq!(
            cached_index_hit(&store, "BTCUSDT").map(|h| h.index_price.as_str()),
            Some("27670.21666667")
        );
        assert!(cached_index_hit(&store, "ETHUSDT").is_none());
        assert!(cached_index_hit(&store, "BTC").is_none());
        assert!(cached_index_hit(&store, "XRPUSDT").is_none());
        assert!(cached_index_hit(&None, "BTCUSDT").is_none());
    }
}

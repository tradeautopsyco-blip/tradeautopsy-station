//! Typed desk query → subscribed / master S1 instrument. Does not open a new stream.

use super::binance_public::normalize_quote_instrument;

/// Map what the trader typed to one subscribed or env desk pair.
///
/// Requires 3+ characters so `bt` does not steal NSE search. `btc` matches `btcusdt`.
pub fn resolve_desk_instrument(query: &str, desk_symbol: Option<&str>) -> Option<String> {
    resolve_among(query, desk_symbol)
}

/// Prefix-match `query` against broker-normalized instrument ids. Shortest match wins.
pub fn resolve_among<'a>(
    query: &str,
    symbols: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let q = normalize_quote_instrument(query);
    if q.len() < 3 {
        return None;
    }
    let mut matches: Vec<String> = symbols
        .into_iter()
        .map(normalize_quote_instrument)
        .filter(|symbol| !symbol.is_empty() && (*symbol == q || symbol.starts_with(&q)))
        .collect();
    matches.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
    matches.dedup();
    matches.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_maps_to_subscribed_btcusdt() {
        assert_eq!(
            resolve_desk_instrument("btc", Some("BTCUSDT")).as_deref(),
            Some("btcusdt")
        );
        assert_eq!(
            resolve_desk_instrument("BTCUSDT", Some("btcusdt")).as_deref(),
            Some("btcusdt")
        );
    }

    #[test]
    fn short_or_unrelated_does_not_map() {
        assert_eq!(resolve_desk_instrument("bt", Some("btcusdt")), None);
        assert_eq!(resolve_desk_instrument("eth", Some("btcusdt")), None);
        assert_eq!(resolve_desk_instrument("btc", None), None);
        assert_eq!(resolve_desk_instrument("RELIANCE", Some("btcusdt")), None);
    }

    #[test]
    fn resolve_among_uses_master_symbols_not_env_only() {
        assert_eq!(
            resolve_among("btc", ["ethusdt", "btcusdt", "solusdt"]).as_deref(),
            Some("btcusdt")
        );
        assert_eq!(resolve_among("btc", ["ethusdt"]), None);
    }
}

//! Venue-published option greeks from `GET /eapi/v1/mark`.
//!
//! Station **copies** the strings Binance published. It does not price this book,
//! does not rescale, and does not relabel. Units, day-count and the venue's model
//! are NOT SPECIFIED IN SOURCE, which is exactly why this is `VenuePublished` and
//! never `ModelComputed`.
//!
//! Lock: `locks/binance-com-options.md` Slice 3 ·
//! `docs/reference/crypto/binance-global/options/REST.md` Slice 3.

use serde_json::Value;

/// Official host + path (public, weight 5 IP).
pub const OPTIONS_MARK_PATH: &str = "/eapi/v1/mark";

/// One `/eapi/v1/mark` row, as published. Every field is the venue's own string —
/// parsing to a number here would invent a precision the lock does not name.
///
/// There is **no `rho`** on this payload. Four greeks, not five.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsMarkRow {
    pub symbol: String,
    pub mark_price: String,
    pub delta: String,
    pub gamma: String,
    pub theta: String,
    pub vega: String,
    /// `None` when the venue published a non-positive sentinel (see below).
    pub mark_iv: Option<String>,
    pub bid_iv: Option<String>,
    pub ask_iv: Option<String>,
    /// Binance's own USDT rate. Never carry this onto the NFO book.
    pub risk_free_interest: Option<String>,
}

/// One cached `/eapi/v1/mark` row plus the stamp of the fetch that produced it.
///
/// Keyed by its **own** mixed-case symbol, not by whatever was asked for: mark is
/// per contract, so a request for a different contract is a miss, never a repaint
/// of the previous contract's delta.
///
/// `as_of` is Station's RFC3339 fetch stamp. The official mark table publishes no
/// timestamp field, so there is no venue time to copy — inventing one would be a
/// lie about freshness, and `greeks_may_render_number` reads this via `input_at`.
#[derive(Debug, Clone)]
pub struct CachedMark {
    pub symbol: String,
    pub row: OptionsMarkRow,
    pub as_of: String,
}

/// `?symbol=` is required by the lock even though the docs mark it optional: one
/// unfiltered call returned 1736 rows at weight 5.
pub fn options_mark_query(symbol: &str) -> String {
    format!("{OPTIONS_MARK_PATH}?symbol={}", symbol.trim())
}

/// An implied volatility the venue could not quote.
///
/// Observed live 2026-08-31 on `BTC-260925-145000-C`: `"bidIV":"-1.0"` alongside a
/// real `askIV`. A negative implied volatility is not a volatility — it is a
/// no-bid sentinel, and the official field table does not mention it. Refuse it
/// rather than render a number that looks real.
fn usable_iv(raw: Option<&str>) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Some(raw.to_string()),
        _ => None,
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    let raw = value.get(key)?.as_str()?.trim();
    (!raw.is_empty()).then(|| raw.to_string())
}

fn row_from_object(value: &Value) -> Option<OptionsMarkRow> {
    // A greek row is complete or it is refused: a chip showing three of four
    // greeks is a bounded snapshot that failed its own physics.
    Some(OptionsMarkRow {
        symbol: string_field(value, "symbol")?,
        mark_price: string_field(value, "markPrice")?,
        delta: string_field(value, "delta")?,
        gamma: string_field(value, "gamma")?,
        theta: string_field(value, "theta")?,
        vega: string_field(value, "vega")?,
        mark_iv: usable_iv(string_field(value, "markIV").as_deref()),
        bid_iv: usable_iv(string_field(value, "bidIV").as_deref()),
        ask_iv: usable_iv(string_field(value, "askIV").as_deref()),
        risk_free_interest: string_field(value, "riskFreeInterest"),
    })
}

/// Rows from a `/eapi/v1/mark` body. The documented shape is a JSON array; a bare
/// object is accepted so a single-symbol response cannot be silently dropped.
pub fn mark_rows_from_json(raw: &str) -> Vec<OptionsMarkRow> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    match &value {
        Value::Array(rows) => rows.iter().filter_map(row_from_object).collect(),
        Value::Object(_) => row_from_object(&value).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The one row for `symbol`, matched exactly. Binance option symbols are
/// mixed-case dated contracts (`BTC-260925-100000-C`) — never lowercase them.
pub fn mark_row_for_symbol(raw: &str, symbol: &str) -> Option<OptionsMarkRow> {
    let symbol = symbol.trim();
    mark_rows_from_json(raw)
        .into_iter()
        .find(|row| row.symbol == symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Official response example, verbatim from the Option Mark Price docs page.
    const OFFICIAL_EXAMPLE: &str = r#"[ { "symbol": "BTC-200730-9000-C", "markPrice": "1343.2883", "bidIV": "1.40000077", "askIV": "1.50000153", "markIV": "1.45000000", "delta": "0.55937056", "theta": "3739.82509871", "gamma": "0.00010969", "vega": "978.58874732", "highPriceLimit": "1618.241", "lowPriceLimit": "1068.3356", "riskFreeInterest": "0.1" } ]"#;

    /// Observed live 2026-08-31 — note `bidIV` is the -1.0 no-bid sentinel.
    const LIVE_NO_BID: &str = r#"[{"symbol":"BTC-260925-145000-C","markPrice":"2.439","bidIV":"-1.0","askIV":"0.74930251","markIV":"0.709","delta":"0.00065535","theta":"-0.84070424","gamma":"0.00000012","vega":"0.46744754","highPriceLimit":"750","lowPriceLimit":"5","riskFreeInterest":"0.0454"}]"#;

    #[test]
    fn official_example_copies_the_published_strings() {
        let row = mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row");
        // Verbatim, not reparsed into floats and back.
        assert_eq!(row.delta, "0.55937056");
        assert_eq!(row.gamma, "0.00010969");
        assert_eq!(row.theta, "3739.82509871");
        assert_eq!(row.vega, "978.58874732");
        assert_eq!(row.mark_price, "1343.2883");
        assert_eq!(row.mark_iv.as_deref(), Some("1.45000000"));
        assert_eq!(row.bid_iv.as_deref(), Some("1.40000077"));
        assert_eq!(row.ask_iv.as_deref(), Some("1.50000153"));
        assert_eq!(row.risk_free_interest.as_deref(), Some("0.1"));
    }

    #[test]
    fn negative_iv_is_a_no_bid_sentinel_not_a_volatility() {
        let row = mark_row_for_symbol(LIVE_NO_BID, "BTC-260925-145000-C").expect("row");
        assert_eq!(row.bid_iv, None, "-1.0 must never render as an IV");
        // The rest of the row is still good — one dead IV does not void the greeks.
        assert_eq!(row.ask_iv.as_deref(), Some("0.74930251"));
        assert_eq!(row.mark_iv.as_deref(), Some("0.709"));
        assert_eq!(row.delta, "0.00065535");
    }

    #[test]
    fn zero_and_unparseable_ivs_are_refused_too() {
        for bad in ["0", "0.0", "-0.5", "", "   ", "null", "NaN", "abc"] {
            let body = format!(
                r#"[{{"symbol":"S","markPrice":"1","delta":"1","gamma":"1","theta":"1","vega":"1","markIV":"{bad}"}}]"#
            );
            let row = mark_row_for_symbol(&body, "S").expect("row");
            assert_eq!(row.mark_iv, None, "markIV {bad:?} must not render");
        }
    }

    #[test]
    fn a_partial_greek_row_is_refused_whole() {
        // vega missing: a three-of-four greek chip is not a bounded snapshot.
        let body = r#"[{"symbol":"S","markPrice":"1","delta":"1","gamma":"1","theta":"1"}]"#;
        assert!(mark_rows_from_json(body).is_empty());
    }

    #[test]
    fn there_is_no_rho_on_this_payload() {
        // Four greeks, not five. Nothing here may invent one.
        assert!(!OFFICIAL_EXAMPLE.contains("rho"));
        assert!(!LIVE_NO_BID.contains("rho"));
    }

    #[test]
    fn dated_symbol_case_is_preserved() {
        let row = mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row");
        assert_eq!(row.symbol, "BTC-200730-9000-C");
        // Not a spot pair — lowercasing would make it unmatchable.
        assert!(mark_row_for_symbol(OFFICIAL_EXAMPLE, "btc-200730-9000-c").is_none());
    }

    #[test]
    fn query_always_names_one_symbol() {
        assert_eq!(
            options_mark_query("BTC-260925-100000-C"),
            "/eapi/v1/mark?symbol=BTC-260925-100000-C"
        );
    }

    #[test]
    fn a_cached_mark_is_keyed_by_its_own_symbol() {
        // Mark is per contract. The store keys on the row's own mixed-case symbol
        // so a different contract reads as a miss, never as the previous delta.
        let row = mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row");
        let cached = CachedMark {
            symbol: row.symbol.clone(),
            row,
            as_of: "2026-08-31T09:00:00Z".to_string(),
        };
        assert_eq!(cached.symbol, "BTC-200730-9000-C");
        assert_eq!(cached.row.delta, "0.55937056");
        assert_ne!(cached.symbol, "btc-200730-9000-c");
    }

    #[test]
    fn junk_bodies_are_empty_not_a_panic() {
        for body in ["", "null", "{}", "[]", "not json", "[1,2,3]"] {
            assert!(mark_rows_from_json(body).is_empty(), "{body:?}");
        }
    }
}

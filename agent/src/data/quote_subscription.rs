//! Which quote path an id may open. Shape first — dated never becomes spot.

use super::is_dated_option_contract;

/// What Last / paste / LTP may open for `id`. Dated option contracts take the
/// options-quote record only — never spot `@trade` / depth / klines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteSubscription {
    OptionsQuote,
    Desk,
    None,
}

/// Way 3 last-only: shape, not `book=`, decides the stream. `desk_would_subscribe`
/// is `AppState::should_subscribe_quote`.
pub fn quote_subscription_for(id: &str, desk_would_subscribe: bool) -> QuoteSubscription {
    if is_dated_option_contract(id) {
        return QuoteSubscription::OptionsQuote;
    }
    if desk_would_subscribe {
        return QuoteSubscription::Desk;
    }
    QuoteSubscription::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dated_contract_is_options_quote_even_when_desk_would_subscribe() {
        assert_eq!(
            quote_subscription_for("BTC-200730-9000-C", true),
            QuoteSubscription::OptionsQuote
        );
        assert_eq!(
            quote_subscription_for("BTC-200730-9000-C", false),
            QuoteSubscription::OptionsQuote
        );
        assert_eq!(
            quote_subscription_for("btcusdt", true),
            QuoteSubscription::Desk
        );
        assert_eq!(
            quote_subscription_for("btcusdt", false),
            QuoteSubscription::None
        );
        assert_eq!(
            quote_subscription_for("nse_fo|12345", true),
            QuoteSubscription::Desk
        );
    }
}

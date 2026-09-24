//! Which book may carry which symbol. Shape only — does not dial.
//!
//! New-broker card (checklist, not a feature). Before a book ships:
//! 1. docs/reference lock (hosts, 429/418, headers, or NOT SPECIFIED)
//! 2. route_book + slot + meter in policy.rs
//! 3. Hosts on hosts_for_broker and therefore slot_for_host
//! 4. book_accepts_symbol cases for that book
//! 5. Card tests: wrong shape refuse, mutation refuse, freeze isolation vs the other slot
//! 6. Ring 2 green (no private Client::)
//! 7. Notch knows the venue key
//! No lock → no route_book row → admit is WrongBook → nothing sends.

use super::is_dated_option_contract;

/// First `symbol=` pair in a raw query string (no leading `?`).
/// Empty query / no `symbol=` → `None`. Does not refuse symbol-less calls.
pub fn query_symbol(query: &str) -> Option<&str> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("symbol="))
}

/// Spot rejects dated `…-C`/`…-P` and `segment|token`. Options accepts dated
/// only. Kotak and every other book return true — the host/path/segment fence
/// stays the Kotak gate.
pub fn book_accepts_symbol(book_id: &str, symbol: &str) -> bool {
    match book_id {
        "binance-com-spot" | crate::ubi::bybit_session::BYBIT_BOOK_ID => {
            !is_dated_option_contract(symbol) && !symbol.contains('|')
        }
        "binance-com-options" => is_dated_option_contract(symbol),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_symbol_reads_the_first_symbol_pair() {
        assert_eq!(
            query_symbol("symbol=BTC-260925-145000-C"),
            Some("BTC-260925-145000-C")
        );
        assert_eq!(query_symbol("symbol=BTCUSDT&limit=5000"), Some("BTCUSDT"));
        assert_eq!(query_symbol("symbol=nse_fo|12345"), Some("nse_fo|12345"));
        assert_eq!(query_symbol(""), None);
        assert_eq!(query_symbol("limit=5000"), None);
    }

    #[test]
    fn dated_contract_is_options_not_spot() {
        assert!(!book_accepts_symbol(
            "binance-com-spot",
            "BTC-260925-145000-C"
        ));
        assert!(book_accepts_symbol(
            "binance-com-options",
            "BTC-260925-145000-C"
        ));
        assert!(book_accepts_symbol(
            "kotak-nse-bse-cash",
            "BTC-260925-145000-C"
        ));
        assert!(book_accepts_symbol("kotak-nse-nfo", "BTC-260925-145000-C"));
    }

    #[test]
    fn btcusdt_is_spot_not_options() {
        assert!(book_accepts_symbol("binance-com-spot", "BTCUSDT"));
        assert!(!book_accepts_symbol("binance-com-options", "BTCUSDT"));
        assert!(book_accepts_symbol("kotak-nse-bse-cash", "BTCUSDT"));
        assert!(book_accepts_symbol("kotak-nse-nfo", "BTCUSDT"));
    }

    #[test]
    fn nse_fo_token_is_not_spot() {
        assert!(!book_accepts_symbol("binance-com-spot", "nse_fo|12345"));
        assert!(!book_accepts_symbol("binance-com-options", "nse_fo|12345"));
        assert!(book_accepts_symbol("kotak-nse-bse-cash", "nse_fo|12345"));
        assert!(book_accepts_symbol("kotak-nse-nfo", "nse_fo|12345"));
    }
}

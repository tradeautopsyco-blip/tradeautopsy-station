//! Sync-state account capability honesty.
//!
//! `GET /api/daemon/broker/sync-state` `capabilities.{funds,fills,holdings,positions,orders}`
//! must follow AccountBook + the shipping SourceManifest — never poller
//! `data_classes` freshness. A CountingPollAdapter success is not a funds slot.

use super::account_book::AccountBook;
use super::source_manifest::SourceManifest;

/// OpenAlgo account nouns whose slots live on the shipping book.
pub const ACCOUNT_CAPABILITY_OPS: [&str; 5] = [
    "funds",
    "tradebook",
    "holdings",
    "positionbook",
    "orderbook",
];

/// Wire keys on `capabilities` (fills = tradebook slot; orders = orderbook slot).
pub fn account_capability_wire_key(operation: &str) -> Option<&'static str> {
    match operation {
        "funds" => Some("funds"),
        "tradebook" => Some("fills"),
        "holdings" => Some("holdings"),
        "positionbook" => Some("positions"),
        "orderbook" => Some("orders"),
        _ => None,
    }
}

/// Honesty for one declared account operation on the connected shipping book.
///
/// - no Start slug → `unavailable`
/// - slug with no shipping manifest, or op not implemented → `unsupported`
/// - implemented + AccountBook slot (empty snapshot counts) → `fresh`
/// - implemented + no slot → `unavailable`
pub fn account_capability_wire(
    slug: Option<&str>,
    manifest: Option<&SourceManifest>,
    book: &AccountBook,
    operation: &str,
) -> &'static str {
    let Some(slug) = slug.map(str::trim).filter(|s| !s.is_empty()) else {
        return "unavailable";
    };
    let Some(manifest) = manifest else {
        return "unsupported";
    };
    if manifest.adapter_id != slug {
        return "unsupported";
    }
    if !manifest.implemented.iter().any(|op| op == operation) {
        return "unsupported";
    }
    let book_id = manifest.book_id.as_str();
    let has_slot = match operation {
        "funds" => book.funds_slot(book_id).is_some(),
        "tradebook" => book.fills_slot(book_id).is_some(),
        "holdings" => book.holdings_slot(book_id).is_some(),
        "positionbook" => book.positions_slot(book_id).is_some(),
        "orderbook" => book.orders_slot(book_id).is_some(),
        _ => false,
    };
    if has_slot {
        "fresh"
    } else {
        "unavailable"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding};
    use crate::data::source_manifest::{binance_com_s1_manifest, kotak_neo_s1k_manifest};
    use crate::data::BINANCE_COM_SPOT_BOOK_ID;

    #[test]
    fn before_start_is_unavailable_even_with_a_planted_slot() {
        let mut book = AccountBook::new();
        book.replace_funds(
            BINANCE_COM_SPOT_BOOK_ID,
            BrokerBalancesSnapshot {
                holdings: vec![BrokerHolding {
                    asset: "BTC".into(),
                    free: 0.01,
                    locked: 0.0,
                }],
                unrealized_pnl: None,
            },
            "/api/v3/account",
            1,
        );
        let manifest = binance_com_s1_manifest();
        assert_eq!(
            account_capability_wire(None, Some(&manifest), &book, "funds"),
            "unavailable"
        );
    }

    #[test]
    fn poller_slug_without_manifest_is_unsupported() {
        let book = AccountBook::new();
        assert_eq!(
            account_capability_wire(Some("binance_us"), None, &book, "funds"),
            "unsupported"
        );
    }

    #[test]
    fn implemented_without_slot_is_unavailable_not_fresh() {
        let book = AccountBook::new();
        let manifest = binance_com_s1_manifest();
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "funds"),
            "unavailable"
        );
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "tradebook"),
            "unavailable"
        );
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "orderbook"),
            "unavailable"
        );
    }

    #[test]
    fn undeclared_holdings_on_spot_is_unsupported() {
        let book = AccountBook::new();
        let manifest = binance_com_s1_manifest();
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "holdings"),
            "unsupported"
        );
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "positionbook"),
            "unsupported"
        );
    }

    #[test]
    fn funds_slot_on_shipping_book_is_fresh() {
        let mut book = AccountBook::new();
        book.replace_funds(
            BINANCE_COM_SPOT_BOOK_ID,
            BrokerBalancesSnapshot::default(),
            "/api/v3/account",
            2,
        );
        let manifest = binance_com_s1_manifest();
        assert_eq!(
            account_capability_wire(Some("binance_com"), Some(&manifest), &book, "funds"),
            "fresh"
        );
    }

    #[test]
    fn kotak_holdings_dark_until_cash_slot() {
        let book = AccountBook::new();
        let manifest = kotak_neo_s1k_manifest();
        assert_eq!(
            account_capability_wire(Some("kotak_neo"), Some(&manifest), &book, "holdings"),
            "unavailable"
        );
        assert_eq!(
            account_capability_wire(Some("kotak_neo"), Some(&manifest), &book, "funds"),
            "unavailable"
        );
    }

    #[test]
    fn wire_keys_map_obtain_nouns() {
        assert_eq!(account_capability_wire_key("funds"), Some("funds"));
        assert_eq!(account_capability_wire_key("tradebook"), Some("fills"));
        assert_eq!(account_capability_wire_key("orderbook"), Some("orders"));
        assert_eq!(account_capability_wire_key("quotes"), None);
    }
}

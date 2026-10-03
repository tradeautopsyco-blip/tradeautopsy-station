//! India multi-book desk stack — E2E parity checks vs Binance four-book shape.
//!
//! Law: [issues/brokers/playbooks/INDIA-MULTI-BOOK-STACK.md](https://github.com/FExEVIL/tradeautopsy-issues/blob/main/brokers/playbooks/INDIA-MULTI-BOOK-STACK.md)

use super::source_manifest::SourceManifest;
use crate::money_matrix::shipping_money_matrix;

/// Desk operations every India SHIPPING book must expose at Tier I (Connect beta).
pub const INDIA_DESK_CORE_OPS: &[&str] = &["quotes", "tradebook", "funds"];

/// Kotak private reads routed through `kick_kotak_private` (no sync enricher).
pub const KOTAK_PRIVATE_KICK_OPS: &[&str] = &["orderbook", "positionbook"];

/// Reference four-book stacks (slug-level product shape, not Tier II marketing).
pub const BINANCE_COM_FOUR_BOOKS: &[&str] = &[
    "binance-com-spot",
    "binance-com-options",
    "binance-com-usdm",
    "binance-com-coinm",
];

pub const KOTAK_NEO_FOUR_BOOKS: &[&str] = &[
    "kotak-nse-bse-cash",
    "kotak-nse-nfo",
    "kotak-nse-cds",
    "kotak-mcx-future",
];

pub fn manifest_declares_op(manifest: &SourceManifest, op: &str) -> bool {
    manifest.implemented.iter().any(|implemented| implemented == op)
}

/// Returns ops listed in `implemented` that lack a desk path (enricher or Kotak kick).
pub fn missing_desk_paths(
    book_id: &str,
    manifest: &SourceManifest,
    has_enricher: impl Fn(&str, &str) -> bool,
) -> Vec<String> {
    let mut missing = Vec::new();
    for op in INDIA_DESK_CORE_OPS {
        if !manifest_declares_op(manifest, op) {
            missing.push(format!("{op} (not in manifest.implemented)"));
            continue;
        }
        if has_enricher(book_id, op) {
            continue;
        }
        if book_id.starts_with("kotak-") && KOTAK_PRIVATE_KICK_OPS.contains(op) {
            continue;
        }
        missing.push(op.to_string());
    }
    missing
}

pub fn shipping_book_has_money_owner(book_id: &str) -> bool {
    shipping_money_matrix()
        .iter()
        .any(|row| row.book_id == book_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kotak_four_book_shape_matches_binance_count() {
        assert_eq!(BINANCE_COM_FOUR_BOOKS.len(), 4);
        assert_eq!(KOTAK_NEO_FOUR_BOOKS.len(), 4);
    }

    #[test]
    fn shipping_kotak_books_have_money_owner_and_manifest() {
        for book_id in KOTAK_NEO_FOUR_BOOKS {
            assert!(
                shipping_book_has_money_owner(book_id),
                "{book_id} missing money_matrix row"
            );
            assert!(
                manifest_for_book_id(book_id).is_some(),
                "{book_id} missing source manifest"
            );
        }
    }

    #[test]
    fn kotak_books_declare_desk_core_ops() {
        for book_id in KOTAK_NEO_FOUR_BOOKS {
            let manifest = manifest_for_book_id(book_id).expect("manifest");
            for op in INDIA_DESK_CORE_OPS {
                assert!(
                    manifest_declares_op(&manifest, op),
                    "{book_id} must declare {op}"
                );
            }
        }
    }
}

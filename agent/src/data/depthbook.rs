//! In-memory REST/stream depth snapshots. Not TickBook, not an ordered replica.
//!
//! Keys are `{book_id}\0{instrument}` (same idea as TickBook). Two books can
//! share adapter `binance_com` and BTCUSDT without overwriting — a future USD-M
//! BTCUSDT subscribe must not close spot REST for that ticker. Locks:
//! binance-com-spot + kotak-nse-bse-cash (fetch 2026-08-22 IST).

use super::binance_public::normalize_quote_instrument;
use super::descriptor::{
    BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
    KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
};
use super::kotak_depth::DepthSnapshot;
use super::tick::Transport;
use chrono::Utc;
use std::collections::HashMap;

pub(crate) fn depth_key(book_id: &str, instrument_id: &str) -> String {
    format!(
        "{}\0{}",
        book_id.trim(),
        normalize_depth_instrument(book_id, instrument_id)
    )
}

fn normalize_depth_instrument(book_id: &str, instrument_id: &str) -> String {
    if book_id.trim() == BINANCE_COM_SPOT_BOOK_ID {
        normalize_quote_instrument(instrument_id)
    } else {
        instrument_id.trim().to_string()
    }
}

/// Provenance slug for a placeholder row keyed by `book_id`. Spot/options/usdm all
/// share the login slug `binance_com`; cash maps to `kotak_neo`; fixtures keep the
/// passed string.
///
/// The options arm matters even though eapi depth has no gap machine: without it a
/// future `invalidate(_, "binance-com-options")` would stamp
/// `adapter_id = "binance-com-options"` — a book id sitting in the login-slug
/// field, which is not a slug anyone can log in with.
fn placeholder_adapter_id(book_id: &str) -> String {
    match book_id.trim() {
        BINANCE_COM_SPOT_BOOK_ID | BINANCE_COM_OPTIONS_BOOK_ID | "binance-com-usdm" => {
            BINANCE_COM_ADAPTER_ID.to_string()
        }
        KOTAK_NSE_BSE_CASH_BOOK_ID => KOTAK_NEO_ADAPTER_ID.to_string(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct DepthBook {
    rows: HashMap<String, DepthSnapshot>,
}

impl DepthBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, book_id: &str, instrument_id: &str) -> Option<&DepthSnapshot> {
        self.rows.get(&depth_key(book_id, instrument_id))
    }

    /// Live `@depth` apply must mutate in place. Cloning a 5000-level COM book
    /// on every event pegs a core the moment BTCUSDT binds.
    pub fn get_mut(&mut self, book_id: &str, instrument_id: &str) -> Option<&mut DepthSnapshot> {
        self.rows.get_mut(&depth_key(book_id, instrument_id))
    }

    /// Always replace the stored row. An incomplete/gapped snapshot must not leave
    /// a stale complete Success in place — `extract_depth` returns Unusable when
    /// `!row.completeness`.
    pub fn upsert(&mut self, snapshot: DepthSnapshot) {
        let key = depth_key(&snapshot.book_id, &snapshot.instrument_id);
        self.rows.insert(key, snapshot);
    }

    /// Mark a stored book Unusable (sequence gap / restart).
    ///
    /// If a row exists, only `completeness` is flipped. If absent, insert a
    /// placeholder so `extract_depth` returns Unusable rather than Unavailable.
    /// Caller supplies `book_id` — a Binance gap must not write a Kotak-shaped row.
    pub fn invalidate(&mut self, instrument_id: &str, book_id: &str) {
        let key = depth_key(book_id, instrument_id);
        if let Some(row) = self.rows.get_mut(&key) {
            row.completeness = false;
            return;
        }
        self.rows.insert(
            key,
            DepthSnapshot {
                instrument_id: instrument_id.to_string(),
                adapter_id: placeholder_adapter_id(book_id),
                book_id: book_id.trim().to_string(),
                bids: Vec::new(),
                asks: Vec::new(),
                completeness: false,
                bound_levels: 0,
                as_of: Utc::now(),
                transport: Transport::Stream,
                sequence: None,
            },
        );
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &DepthSnapshot)> {
        self.rows.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::kotak_depth::{extract_depth, DepthLevel, DepthStatus};
    use crate::data::tick::Transport;
    use chrono::{TimeZone, Utc};

    fn received() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
    }

    /// Map a test slot arg to (adapter_id provenance, book_id slot).
    fn snap_ids(arg: &str) -> (String, String) {
        if arg == BINANCE_COM_ADAPTER_ID {
            (
                BINANCE_COM_ADAPTER_ID.to_string(),
                BINANCE_COM_SPOT_BOOK_ID.to_string(),
            )
        } else if arg == BINANCE_COM_SPOT_BOOK_ID || arg == "binance-com-usdm" {
            (BINANCE_COM_ADAPTER_ID.to_string(), arg.to_string())
        } else if arg == KOTAK_NEO_ADAPTER_ID || arg == KOTAK_NSE_BSE_CASH_BOOK_ID {
            (
                KOTAK_NEO_ADAPTER_ID.to_string(),
                KOTAK_NSE_BSE_CASH_BOOK_ID.to_string(),
            )
        } else {
            (arg.to_string(), arg.to_string())
        }
    }

    fn snap(
        id: &str,
        adapter: &str,
        complete: bool,
        sequence: Option<u64>,
        transport: Transport,
    ) -> DepthSnapshot {
        let (adapter_id, book_id) = snap_ids(adapter);
        DepthSnapshot {
            instrument_id: id.to_string(),
            adapter_id,
            book_id,
            bids: vec![DepthLevel {
                price: "1.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            asks: vec![DepthLevel {
                price: "2.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            completeness: complete,
            bound_levels: 1,
            as_of: received(),
            transport,
            sequence,
        }
    }

    #[test]
    fn complete_then_incomplete_upsert_extracts_unusable() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "nse_cm|2885",
            KOTAK_NEO_ADAPTER_ID,
            true,
            None,
            Transport::Rest,
        ));
        let ok = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(ok.status, DepthStatus::Success);

        book.upsert(snap(
            "nse_cm|2885",
            KOTAK_NEO_ADAPTER_ID,
            false,
            None,
            Transport::Rest,
        ));
        let bad = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(bad.status, DepthStatus::Unusable);
        assert!(bad.data.is_none());
    }

    #[test]
    fn invalidate_marks_complete_book_unusable() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "btcusdt",
            BINANCE_COM_ADAPTER_ID,
            true,
            Some(100),
            Transport::Rest,
        ));
        book.invalidate("btcusdt", BINANCE_COM_SPOT_BOOK_ID);
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Unusable);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn invalidate_absent_row_extracts_unusable_not_unavailable() {
        let mut book = DepthBook::new();
        book.invalidate("btcusdt", BINANCE_COM_SPOT_BOOK_ID);
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Unusable);
        assert_ne!(envelope.status, DepthStatus::Unavailable);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn kotak_rest_snapshot_sequence_none_extracts_success() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "nse_cm|2885",
            KOTAK_NEO_ADAPTER_ID,
            true,
            None,
            Transport::Rest,
        ));
        let envelope = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert!(envelope.data.is_some());
    }

    #[test]
    fn stream_snapshot_stamps_provenance_transport() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "btcusdt",
            BINANCE_COM_ADAPTER_ID,
            true,
            Some(105),
            Transport::Stream,
        ));
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert_eq!(envelope.provenance.transport, Some(Transport::Stream));
    }

    #[test]
    fn two_adapters_do_not_share_a_depth_slot() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "btcusdt",
            BINANCE_COM_ADAPTER_ID,
            true,
            Some(1),
            Transport::Rest,
        ));
        book.upsert(snap("btcusdt", "other", true, Some(2), Transport::Rest));
        let spot = book
            .get(BINANCE_COM_SPOT_BOOK_ID, "btcusdt")
            .expect("spot row");
        assert_eq!(spot.sequence, Some(1));
        assert_eq!(spot.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(spot.book_id, BINANCE_COM_SPOT_BOOK_ID);
        let other = book.get("other", "btcusdt").expect("other row");
        assert_eq!(other.sequence, Some(2));
        assert_eq!(other.adapter_id, "other");
        assert_eq!(other.book_id, "other");
    }

    /// Options keys are trim-only. `normalize_depth_instrument` lowercases for the
    /// spot book alone, so a dated contract keeps its case and can never collide
    /// with a spot pair's slot.
    #[test]
    fn an_options_contract_key_is_case_preserving_and_not_a_spot_key() {
        assert_ne!(
            depth_key(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C"),
            depth_key(BINANCE_COM_OPTIONS_BOOK_ID, "btc-200730-9000-c")
        );
        assert_ne!(
            depth_key(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C"),
            depth_key(BINANCE_COM_SPOT_BOOK_ID, "btcusdt")
        );
        // Spot still lowercases its own pairs — that behaviour is unchanged.
        assert_eq!(
            depth_key(BINANCE_COM_SPOT_BOOK_ID, "BTCUSDT"),
            depth_key(BINANCE_COM_SPOT_BOOK_ID, "btcusdt")
        );
        // The options book does not: same instrument text, two different books.
        assert_ne!(
            depth_key(BINANCE_COM_OPTIONS_BOOK_ID, "BTCUSDT"),
            depth_key(BINANCE_COM_SPOT_BOOK_ID, "BTCUSDT")
        );
    }

    /// A placeholder on the options slot carries the login slug, not the book id.
    #[test]
    fn an_options_placeholder_carries_the_login_slug() {
        let mut book = DepthBook::new();
        book.invalidate("BTC-200730-9000-C", BINANCE_COM_OPTIONS_BOOK_ID);
        let row = book
            .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
            .expect("placeholder row");
        assert_eq!(row.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_ne!(row.adapter_id, BINANCE_COM_OPTIONS_BOOK_ID);
        assert_eq!(row.book_id, BINANCE_COM_OPTIONS_BOOK_ID);
        assert!(!row.completeness);
    }

    #[test]
    fn depth_key_spot_and_usdm_prefixes_are_unequal() {
        // `binance-com-usdm` is a slot prefix only — not a live USD-M book.
        assert_ne!(
            depth_key("binance-com-spot", "btcusdt"),
            depth_key("binance-com-usdm", "btcusdt")
        );
    }

    #[test]
    fn two_book_id_prefixes_do_not_share_a_depth_slot() {
        let mut book = DepthBook::new();
        book.upsert(snap(
            "btcusdt",
            "binance-com-spot",
            true,
            Some(1),
            Transport::Rest,
        ));
        book.upsert(snap(
            "btcusdt",
            "binance-com-usdm",
            true,
            Some(2),
            Transport::Rest,
        ));
        let spot = book.get("binance-com-spot", "btcusdt").expect("spot row");
        let usdm = book.get("binance-com-usdm", "btcusdt").expect("usdm row");
        assert_eq!(spot.sequence, Some(1));
        assert_eq!(usdm.sequence, Some(2));
        assert_eq!(spot.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(usdm.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(spot.book_id, "binance-com-spot");
        assert_eq!(usdm.book_id, "binance-com-usdm");
    }
}

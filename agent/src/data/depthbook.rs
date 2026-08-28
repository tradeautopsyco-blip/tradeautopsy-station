//! In-memory REST/stream depth snapshots. Not TickBook, not an ordered replica.

use super::kotak_depth::DepthSnapshot;
use super::tick::Transport;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DepthBook {
    rows: HashMap<String, DepthSnapshot>,
}

impl DepthBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, instrument_id: &str) -> Option<&DepthSnapshot> {
        self.rows.get(instrument_id)
    }

    /// Always replace the stored row. An incomplete/gapped snapshot must not leave
    /// a stale complete Success in place — `extract_depth` returns Unusable when
    /// `!row.completeness`.
    pub fn upsert(&mut self, snapshot: DepthSnapshot) {
        self.rows.insert(snapshot.instrument_id.clone(), snapshot);
    }

    /// Mark a stored book Unusable (sequence gap / restart).
    ///
    /// If a row exists, only `completeness` is flipped. If absent, insert a
    /// placeholder so `extract_depth` returns Unusable rather than Unavailable.
    /// Caller supplies `adapter_id` — a Binance gap must not write a Kotak-shaped row.
    pub fn invalidate(&mut self, instrument_id: &str, adapter_id: &str) {
        if let Some(row) = self.rows.get_mut(instrument_id) {
            row.completeness = false;
            return;
        }
        self.rows.insert(
            instrument_id.to_string(),
            DepthSnapshot {
                instrument_id: instrument_id.to_string(),
                adapter_id: adapter_id.to_string(),
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
    use crate::data::descriptor::{BINANCE_COM_ADAPTER_ID, KOTAK_NEO_ADAPTER_ID};
    use crate::data::kotak_depth::{extract_depth, DepthLevel, DepthStatus};
    use crate::data::tick::Transport;
    use chrono::{TimeZone, Utc};

    fn received() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
    }

    fn snap(
        id: &str,
        adapter: &str,
        complete: bool,
        sequence: Option<u64>,
        transport: Transport,
    ) -> DepthSnapshot {
        DepthSnapshot {
            instrument_id: id.to_string(),
            adapter_id: adapter.to_string(),
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
        book.invalidate("btcusdt", BINANCE_COM_ADAPTER_ID);
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Unusable);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn invalidate_absent_row_extracts_unusable_not_unavailable() {
        let mut book = DepthBook::new();
        book.invalidate("btcusdt", BINANCE_COM_ADAPTER_ID);
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
}

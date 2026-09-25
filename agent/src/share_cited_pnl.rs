//! M1 share-up: serialize Station-cited INR cash trips for Console consume.
//!
//! Lock: `issues/compliance/locks/kotak-nse-bse-cash.md` (2026-08-22).
//! Owner of the number: `inr_cash_wac.rs`. This module only packages citations.
//! DualNoBlend: payload currency is always INR; never includes FX blend fields.

use crate::inr_cash_wac::{InrCashRoundTrip, BOOK_ID, OWNER_PATH};
use serde_json::Value;

pub const SIGNAL_TYPE: &str = "station_cited_pnl";

/// Build the daemon-events value for one batch of cash round trips (M1 v2 envelope).
pub fn cited_cash_pnl_payload(trips: &[InrCashRoundTrip]) -> Value {
    cited_cash_pnl_payload_for_book(BOOK_ID, trips)
}

/// Per-broker cash book (`dhan-nse-bse-cash`, `groww-nse-bse-cash`, …).
pub fn cited_cash_pnl_payload_for_book(book_id: &str, trips: &[InrCashRoundTrip]) -> Value {
    crate::m1_envelope::cited_inr_cash_payload(book_id, OWNER_PATH, trips)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample_trip(pnl: f64) -> InrCashRoundTrip {
        InrCashRoundTrip {
            symbol: "RELIANCE".into(),
            opened_at: Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap(),
            closed_at: Utc.with_ymd_and_hms(2026, 9, 21, 5, 0, 0).unwrap(),
            avg_entry_price: 105.0,
            avg_exit_price: 120.0,
            qty: 2.0,
            realized_pnl_inr: Some(pnl),
            fees_inr: Some(1.5),
            unknown_basis: false,
            product: "CNC".into(),
        }
    }

    #[test]
    fn payload_is_inr_only_dual_no_blend() {
        let payload = cited_cash_pnl_payload(&[sample_trip(30.0)]);
        assert_eq!(payload["v"], crate::m1_envelope::ENVELOPE_V2);
        assert_eq!(payload["cite_kind"], "inr_cash_wac");
        assert_eq!(payload["currency"], "INR");
        assert_eq!(payload["book_id"], BOOK_ID);
        assert_eq!(payload["source"], "station");
        let text = payload.to_string();
        assert!(!text.contains("exchange_rate"));
        assert!(!text.contains("realized_pnl_usd"));
        assert!(text.contains("realized_pnl_inr"));
        assert_eq!(payload["trips"][0]["realized_pnl_inr"], 30.0);
    }

    #[test]
    fn skips_unknown_basis_and_null_pnl() {
        let mut unknown = sample_trip(10.0);
        unknown.unknown_basis = true;
        let mut none = sample_trip(10.0);
        none.realized_pnl_inr = None;
        let payload = cited_cash_pnl_payload(&[unknown, none, sample_trip(5.0)]);
        assert_eq!(payload["trips"].as_array().unwrap().len(), 1);
    }
}

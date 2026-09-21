//! M1 share-up: serialize Station-cited INR cash trips for Console consume.
//!
//! Lock: `issues/compliance/locks/kotak-nse-bse-cash.md` (2026-08-22).
//! Owner of the number: `inr_cash_wac.rs`. This module only packages citations.
//! DualNoBlend: payload currency is always INR; never includes FX blend fields.

use crate::inr_cash_wac::{InrCashRoundTrip, BOOK_ID, OWNER_PATH};
use serde_json::{json, Value};

pub const SIGNAL_TYPE: &str = "station_cited_pnl";

/// Build the daemon-events value for one batch of cash round trips.
pub fn cited_cash_pnl_payload(trips: &[InrCashRoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_inr.is_some() && !t.unknown_basis)
        .map(|t| {
            json!({
                "trip_id": trip_id(t),
                "symbol": t.symbol,
                "qty": t.qty,
                "avg_entry_price": t.avg_entry_price,
                "avg_exit_price": t.avg_exit_price,
                "opened_at": t.opened_at.to_rfc3339(),
                "closed_at": t.closed_at.to_rfc3339(),
                "realized_pnl_inr": t.realized_pnl_inr,
                "fees_inr": t.fees_inr,
                "product": t.product,
                "book_id": BOOK_ID,
                "currency": "INR",
                "owner": OWNER_PATH,
                "source": "station",
            })
        })
        .collect();
    json!({
        "v": 1,
        "book_id": BOOK_ID,
        "currency": "INR",
        "owner": OWNER_PATH,
        "source": "station",
        "trips": cited,
    })
}

fn trip_id(t: &InrCashRoundTrip) -> String {
    format!(
        "inr-{}-{}-{}",
        t.symbol,
        t.closed_at.timestamp_millis(),
        (t.qty * 1e6).round() as i64
    )
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

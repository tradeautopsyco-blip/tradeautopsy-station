//! Derived OHLCV resample hole. Venue interval wins when the connected broker
//! serves it (`market/ohlcv`). Station roll-up of a finer licensed series stays
//! unimplemented until a primary names the aggregation.
//!
//! Source: `docs/reference/crypto/binance-global/spot/OHLCV-RESAMPLE.md` BLOCKER.
//! PRD #358: one licensed finer series; rights inherit; no Yahoo stitch.

use super::descriptor::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use super::honesty::{HonestyStatus, InputHonesty};
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::inherit::inherit;
use super::provenance::ProvenanceLine;
use serde::Serialize;

pub const RESAMPLE_AGGREGATION_UNSPECIFIED: &str = "resample_aggregation_unspecified";
pub const KOTAK_HISTORY_UNSUPPORTED: &str = "kotak_history_unsupported";
pub const OPTIONS_HISTORY_UNSPECIFIED: &str = "options_history_unspecified";
pub const BOOK_REQUIRED: &str = "resample_book_required";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResampleEnvelope {
    pub identity: Identity,
    pub status: HonestyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

fn resample_identity() -> Identity {
    Identity::new(
        Family::Derived,
        CapabilityId::new("ohlcv").expect("canonical ohlcv id"),
        Physics::HistoricalSeries,
    )
}

fn dark_fence(ineligible: &str) -> ResampleEnvelope {
    let identity = resample_identity();
    ResampleEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

/// Book-routed derived resample. Never stitches Yahoo. Never invents OHLC roll-up.
pub fn extract_resample(book_id: Option<&str>, finer: InputHonesty) -> ResampleEnvelope {
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_fence(BOOK_REQUIRED);
    };

    match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID || id == KOTAK_NSE_NFO_BOOK_ID => {
            dark_fence(KOTAK_HISTORY_UNSUPPORTED)
        }
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => dark_fence(OPTIONS_HISTORY_UNSPECIFIED),
        id if id == BINANCE_COM_SPOT_BOOK_ID => extract_spot_resample(finer),
        _ => dark_fence(BOOK_REQUIRED),
    }
}

fn extract_spot_resample(finer: InputHonesty) -> ResampleEnvelope {
    let identity = resample_identity();
    let provenance = ProvenanceLine::raw_hole(identity.clone());
    if let Some(status) = inherit(&[finer]) {
        return ResampleEnvelope {
            identity,
            status,
            data: None,
            provenance,
            ineligible: Vec::new(),
            canonical: false,
            persist_canonical: false,
        };
    }
    dark_fence(RESAMPLE_AGGREGATION_UNSPECIFIED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Family;

    fn assert_no_series(envelope: &ResampleEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("open").is_none());
        assert!(json.get("high").is_none());
        assert!(json.get("low").is_none());
        assert!(json.get("close").is_none());
        assert_eq!(envelope.identity.family, Family::Derived);
        assert_eq!(envelope.identity.capability_id.as_str(), "ohlcv");
        assert_eq!(envelope.identity.physics, Physics::HistoricalSeries);
        assert!(!envelope.canonical);
        assert!(!envelope.persist_canonical);
    }

    #[test]
    fn kotak_cash_and_nfo_are_history_unsupported() {
        for book in [KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID] {
            let envelope = extract_resample(Some(book), InputHonesty::Lit);
            assert_eq!(envelope.status, HonestyStatus::Unavailable);
            assert!(envelope
                .ineligible
                .iter()
                .any(|s| s == KOTAK_HISTORY_UNSUPPORTED));
            assert_no_series(&envelope);
        }
    }

    #[test]
    fn binance_options_has_no_klines_path() {
        let envelope = extract_resample(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == OPTIONS_HISTORY_UNSPECIFIED));
        assert_no_series(&envelope);
    }

    #[test]
    fn dark_finer_on_spot_is_inherited_dark() {
        let envelope = extract_resample(
            Some(BINANCE_COM_SPOT_BOOK_ID),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_eq!(envelope.status, HonestyStatus::InheritedDark);
        assert_no_series(&envelope);
    }

    #[test]
    fn lit_finer_on_spot_is_still_aggregation_unspecified() {
        let envelope = extract_resample(Some(BINANCE_COM_SPOT_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == RESAMPLE_AGGREGATION_UNSPECIFIED));
        assert_no_series(&envelope);
        assert_eq!(envelope.provenance.model, "raw");
    }

    #[test]
    fn missing_book_is_fence() {
        let envelope = extract_resample(None, InputHonesty::Lit);
        assert!(envelope.ineligible.iter().any(|s| s == BOOK_REQUIRED));
        assert_no_series(&envelope);
    }

    #[test]
    fn never_stitches_yahoo_or_nfo_into_spot_resample() {
        let envelope = extract_resample(Some(BINANCE_COM_SPOT_BOOK_ID), InputHonesty::Lit);
        let dumped = serde_json::to_value(&envelope).unwrap().to_string();
        assert!(!dumped.contains("yahoo"));
        assert!(!dumped.contains("NIFTY"));
        assert_no_series(&envelope);
    }
}

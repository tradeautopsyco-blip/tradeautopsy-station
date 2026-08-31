//! Derived greeks dispatcher. Two book modules (or zero), never one
//! `calculate()` with `if CRYPTO`. Both stay dark this slice.
//!
//! NFO: [`greeks_nfo`] — `pricing_model_unspecified` until OPTIONS-PRICING.md
//! is a real lock. Binance options: [`greeks_binance_options`] —
//! `mark_not_this_slice`; do not allowlist `GET /eapi/v1/mark`.
//! Source: `docs/reference/india/nfo/OPTIONS-PRICING.md` BLOCKER.

use super::descriptor::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use super::greeks_binance_options::extract_binance_options_greeks;
use super::greeks_nfo::extract_nfo_greeks;
use super::honesty::{HonestyStatus, InputHonesty};
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use serde::Serialize;

pub const PRICING_MODEL_UNSPECIFIED: &str = "pricing_model_unspecified";
pub const MARK_NOT_THIS_SLICE: &str = "mark_not_this_slice";
pub const CASH_IS_NOT_GREEKS: &str = "cash_is_not_greeks";
pub const SPOT_IS_NOT_GREEKS: &str = "spot_is_not_greeks";
pub const BOOK_REQUIRED: &str = "greeks_book_required";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GreeksEnvelope {
    pub identity: Identity,
    pub status: HonestyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

pub(crate) fn greeks_identity() -> Identity {
    Identity::new(
        Family::Derived,
        CapabilityId::new("greeks").expect("canonical greeks id"),
        Physics::BoundedSnapshot,
    )
}

pub(crate) fn dark_fence(ineligible: &str) -> GreeksEnvelope {
    let identity = greeks_identity();
    GreeksEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

/// Book-routed greeks extract. `book_id` is required — NFO is not the default.
pub fn extract_greeks(book_id: Option<&str>, chain: InputHonesty) -> GreeksEnvelope {
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_fence(BOOK_REQUIRED);
    };

    match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => dark_fence(CASH_IS_NOT_GREEKS),
        id if id == BINANCE_COM_SPOT_BOOK_ID => dark_fence(SPOT_IS_NOT_GREEKS),
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => extract_binance_options_greeks(),
        id if id == KOTAK_NSE_NFO_BOOK_ID => extract_nfo_greeks(chain),
        _ => dark_fence(BOOK_REQUIRED),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::provenance::greeks_may_render_number;
    use crate::data::{
        obtain, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
        KOTAK_NSE_NFO_BOOK_ID,
    };

    fn assert_no_fixture_greeks(envelope: &GreeksEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("delta").is_none());
        assert!(json.get("gamma").is_none());
        assert!(json.get("theta").is_none());
        assert!(json.get("exchange_rate").is_none());
        assert!(!greeks_may_render_number(
            envelope.status,
            &envelope.provenance
        ));
    }

    fn assert_dual_no_blend(envelope: &GreeksEnvelope) {
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json.get("exchange_rate").is_none());
        assert!(json.get("net_pnl").is_none());
        let dumped = json.to_string();
        assert!(
            !dumped.contains("exchange_rate"),
            "DualNoBlend: greeks envelope must not blend USD+INR"
        );
    }

    #[test]
    fn greeks_identity_is_derived_bounded_snapshot() {
        let envelope = extract_greeks(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_eq!(envelope.identity.family, Family::Derived);
        assert_eq!(envelope.identity.capability_id.as_str(), "greeks");
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }

    #[test]
    fn missing_book_is_unavailable_fence() {
        let envelope = extract_greeks(None, InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == BOOK_REQUIRED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn cash_book_is_not_greeks() {
        let envelope = extract_greeks(Some(KOTAK_NSE_BSE_CASH_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == CASH_IS_NOT_GREEKS));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    #[test]
    fn spot_book_is_not_greeks() {
        let envelope = extract_greeks(Some(BINANCE_COM_SPOT_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == SPOT_IS_NOT_GREEKS));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    #[test]
    fn binance_options_never_fills_nfo_delta() {
        let envelope = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.ineligible.iter().any(|s| s == MARK_NOT_THIS_SLICE));
        assert!(!envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
        assert_dual_no_blend(&envelope);
    }

    #[test]
    fn optiongreeks_not_implemented_on_shipping_manifests() {
        use crate::data::source_manifest::binance_com_options_manifest;
        use crate::data::{kotak_neo_nfo_manifest, ObtainStatus};
        assert_eq!(
            obtain(&kotak_neo_nfo_manifest(), "optiongreeks").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&binance_com_options_manifest(), "optiongreeks").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&binance_com_options_manifest(), "multioptiongreeks").status,
            ObtainStatus::Unsupported
        );
    }

    /// Slice 1 research (2026-08-31) closed exercise/settlement, not trader greeks.
    /// Fail-closed: neither book shows a number.
    #[test]
    fn slice1_fail_closed_neither_book_shows_a_number() {
        let nfo = extract_greeks(Some(KOTAK_NSE_NFO_BOOK_ID), InputHonesty::Lit);
        assert!(nfo.data.is_none());
        assert!(!greeks_may_render_number(nfo.status, &nfo.provenance));
        let eapi = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        assert_eq!(eapi.status, HonestyStatus::Unavailable);
        assert!(eapi.ineligible.iter().any(|s| s == MARK_NOT_THIS_SLICE));
        assert!(eapi.data.is_none());
        assert_dual_no_blend(&eapi);
        assert_dual_no_blend(&nfo);
    }
}

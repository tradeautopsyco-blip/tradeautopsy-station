//! Binance.com European options greeks. Dark this slice.
//!
//! Official `GET /eapi/v1/mark` names delta/theta/gamma/vega/IVs. Do not
//! allowlist that path. Do not pass through. Do not copy NFO formulas onto
//! USDT options. Lock: `locks/binance-com-options.md`. S5 Slice 1 (2026-08-31):
//! NFO OPTIONS-PRICING.md still BLOCKER — mark stays dark until NFO can light too.

use super::greeks::{dark_fence, GreeksEnvelope, MARK_NOT_THIS_SLICE};

/// Own hole: mark is not this slice. Not inherited-dark from a chain — Binance
/// greeks are venue-mark pass-through, not chain+contracts compute.
pub fn extract_binance_options_greeks() -> GreeksEnvelope {
    dark_fence(MARK_NOT_THIS_SLICE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::greeks::extract_greeks;
    use crate::data::honesty::{HonestyStatus, InputHonesty};
    use crate::data::provenance::greeks_may_render_number;
    use crate::data::BINANCE_COM_OPTIONS_BOOK_ID;

    fn assert_no_fixture_greeks(envelope: &GreeksEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("delta").is_none());
        assert!(json.get("gamma").is_none());
        assert!(json.get("theta").is_none());
        assert!(json.get("vega").is_none());
        assert!(!greeks_may_render_number(
            envelope.status,
            &envelope.provenance
        ));
    }

    #[test]
    fn lit_or_dark_chain_is_still_mark_not_this_slice() {
        for chain in [
            InputHonesty::Lit,
            InputHonesty::Dark(HonestyStatus::Unavailable),
        ] {
            let envelope = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), chain);
            assert_eq!(envelope.status, HonestyStatus::Unavailable);
            assert!(envelope.ineligible.iter().any(|s| s == MARK_NOT_THIS_SLICE));
            assert_ne!(envelope.status, HonestyStatus::InheritedDark);
            assert_no_fixture_greeks(&envelope);
        }
    }

    #[test]
    fn never_calls_nfo_pricing_model_unspecified() {
        use crate::data::greeks::PRICING_MODEL_UNSPECIFIED;
        let envelope = extract_binance_options_greeks();
        assert!(!envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn mixed_case_symbol_is_not_a_nfo_formula_target() {
        // Identity golden: BTC-200730-9000-C stays mixed-case elsewhere.
        // This module must not emit NFO Δ for that contract.
        let envelope = extract_greeks(Some(BINANCE_COM_OPTIONS_BOOK_ID), InputHonesty::Lit);
        let json = serde_json::to_value(&envelope).unwrap();
        assert!(json.get("delta").is_none());
        assert!(!json.to_string().contains("NIFTY"));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn dual_no_blend_no_inr_usdt_field() {
        let envelope = extract_binance_options_greeks();
        let dumped = serde_json::to_value(&envelope).unwrap().to_string();
        assert!(!dumped.contains("exchange_rate"));
        assert!(!dumped.contains("INR"));
        assert_no_fixture_greeks(&envelope);
    }
}

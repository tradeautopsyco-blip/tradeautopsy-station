//! NFO Station-computed greeks hole. Dark until OPTIONS-PRICING.md is a real lock.
//! S5 Slice 1 (2026-08-31): NCL names European + cash; FAOP PDFs 503; no trader
//! ΔΓΘ. Fail-closed — do not invent Black-76. Do not fill fixture delta/gamma/theta.
//!
//! Named inputs: `market/option_chain` and `reference/derivative_contracts`.
//! A missing pricing model is **this extract's own Unavailable hole**, not a named
//! inherited-dark input. Source: `docs/reference/india/nfo/OPTIONS-PRICING.md` BLOCKER.

use super::contracts::extract_contracts;
use super::greeks::{dark_fence, greeks_identity, GreeksEnvelope, PRICING_MODEL_UNSPECIFIED};
use super::honesty::{HonestyStatus, InputHonesty};
use super::inherit::inherit;
use super::provenance::ProvenanceLine;
use super::KOTAK_NSE_NFO_BOOK_ID;

fn contracts_input_honesty(envelope: super::contracts::ContractsEnvelope) -> InputHonesty {
    if envelope.data.is_some() {
        InputHonesty::Lit
    } else {
        InputHonesty::Dark(envelope.status)
    }
}

pub fn extract_nfo_greeks(chain: InputHonesty) -> GreeksEnvelope {
    extract_greeks_from(
        chain,
        contracts_input_honesty(extract_contracts(Some(KOTAK_NSE_NFO_BOOK_ID))),
    )
}

pub(crate) fn extract_greeks_from(chain: InputHonesty, contracts: InputHonesty) -> GreeksEnvelope {
    let identity = greeks_identity();
    let provenance = ProvenanceLine::raw_hole(identity.clone());
    if let Some(status) = inherit(&[chain, contracts]) {
        return GreeksEnvelope {
            identity,
            status: status.into(),
            data: None,
            provenance,
            source: None,
            ineligible: Vec::new(),
            canonical: false,
            persist_canonical: false,
        };
    }
    // Named inputs lit. No sourced pricing model → this extract's hole.
    dark_fence(PRICING_MODEL_UNSPECIFIED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::greeks::{extract_greeks, greeks_may_render_number, GreeksStatus};
    use crate::data::identity::{Family, Physics};
    use crate::data::KOTAK_NSE_NFO_BOOK_ID;

    fn assert_no_fixture_greeks(envelope: &GreeksEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("delta").is_none());
        assert!(json.get("gamma").is_none());
        assert!(json.get("theta").is_none());
        assert!(!greeks_may_render_number(envelope));
    }

    #[test]
    fn dark_chain_is_inherited_dark_with_data_none() {
        let envelope = extract_greeks(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_eq!(envelope.status, GreeksStatus::InheritedDark);
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn lit_chain_dark_contracts_is_inherited_dark_not_unavailable() {
        // Production extract_contracts(nfo) with no store/rows is Unavailable.
        let envelope = extract_greeks(Some(KOTAK_NSE_NFO_BOOK_ID), InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::InheritedDark);
        assert_ne!(envelope.status, GreeksStatus::Unavailable);
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn lit_nfo_contracts_and_lit_chain_still_pricing_model_unspecified() {
        use crate::data::{extract_contracts_from_rows, ContractRow};

        let rows = [ContractRow {
            instrument_id: "nse_fo|56526".to_string(),
            lot: 65,
            trading_symbol: "NIFTY2692221000PE".to_string(),
            segment: "nse_fo".to_string(),
            instrument_type: "OPTIDX".to_string(),
            option_type: "PE".to_string(),
            strike_raw: "2.1e+06".to_string(),
            expiry_raw: "1474554600".to_string(),
        }];
        let contracts = extract_contracts_from_rows(Some(KOTAK_NSE_NFO_BOOK_ID), Some(&rows));
        assert!(contracts.data.is_some(), "lit = data.is_some()");

        let envelope = extract_greeks_from(InputHonesty::Lit, contracts_input_honesty(contracts));
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
        assert_eq!(envelope.identity.family, Family::Derived);
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
    }

    #[test]
    fn glance_chain_success_plus_lit_contracts_still_pricing_model_unspecified() {
        use crate::data::{
            chain_input_honesty, extract_chain_from, extract_contracts_from_rows, ChainRow,
            ContractRow,
        };

        let rows = [ChainRow {
            instrument_id: "nse_fo|56526".to_string(),
            lot: 65,
            trading_symbol: "NIFTY2692221000PE".to_string(),
            segment: "nse_fo".to_string(),
            instrument_type: "OPTIDX".to_string(),
            option_type: "PE".to_string(),
            strike_raw: "2.1e+06".to_string(),
            expiry_raw: "1474554600".to_string(),
            last: None,
        }];
        let chain = extract_chain_from(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY", Some(&rows), None);
        let contracts = extract_contracts_from_rows(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            Some(&[ContractRow {
                instrument_id: "nse_fo|56526".to_string(),
                lot: 65,
                trading_symbol: "NIFTY2692221000PE".to_string(),
                segment: "nse_fo".to_string(),
                instrument_type: "OPTIDX".to_string(),
                option_type: "PE".to_string(),
                strike_raw: "2.1e+06".to_string(),
                expiry_raw: "1474554600".to_string(),
            }]),
        );
        let envelope = extract_greeks_from(
            chain_input_honesty(&chain),
            contracts_input_honesty(contracts),
        );
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn named_inputs_lit_missing_model_is_this_extract_unavailable() {
        let envelope = extract_greeks_from(InputHonesty::Lit, InputHonesty::Lit);
        assert_eq!(envelope.status, GreeksStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn stub_never_fills_fixture_greeks() {
        let envelope = extract_greeks(Some(KOTAK_NSE_NFO_BOOK_ID), InputHonesty::Lit);
        assert_no_fixture_greeks(&envelope);
        let lit_inputs = extract_greeks_from(InputHonesty::Lit, InputHonesty::Lit);
        assert_no_fixture_greeks(&lit_inputs);
    }

    #[test]
    fn nfo_lot_fixture_is_sixty_five_not_global_nifty_lot() {
        use crate::data::{extract_contracts_from_rows, ContractRow};
        let rows = [ContractRow {
            instrument_id: "nse_fo|56526".to_string(),
            lot: 65,
            trading_symbol: "NIFTY2692221000PE".to_string(),
            segment: "nse_fo".to_string(),
            instrument_type: "OPTIDX".to_string(),
            option_type: "PE".to_string(),
            strike_raw: "2.1e+06".to_string(),
            expiry_raw: "1474554600".to_string(),
        }];
        let contracts = extract_contracts_from_rows(Some(KOTAK_NSE_NFO_BOOK_ID), Some(&rows));
        let envelope = extract_greeks_from(InputHonesty::Lit, contracts_input_honesty(contracts));
        assert_no_fixture_greeks(&envelope);
        assert_ne!(rows[0].lot, 50, "do not fake lot=50");
        assert_eq!(rows[0].lot, 65);
    }
}

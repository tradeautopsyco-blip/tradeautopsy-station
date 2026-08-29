//! Derived greeks hole. Dark until named inputs are lit and a sourced model exists.
//! Do not invent Black-76 or fixture delta/gamma/theta.
//!
//! Named inputs: `market/option_chain` and `reference/derivative_contracts`.
//! A missing pricing model is **this extract's own Unavailable hole**, not a named
//! inherited-dark input (`honesty.rs`: InheritedDark = a named input is dark;
//! Unavailable = no snapshot / hole / not implemented). Source:
//! `docs/reference/india/nfo/OPTIONS-PRICING.md` BLOCKER — no named NFO model.

use super::contracts::extract_contracts;
use super::honesty::{HonestyStatus, InputHonesty};
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::inherit::inherit;
use super::provenance::ProvenanceLine;
use serde::Serialize;

pub const PRICING_MODEL_UNSPECIFIED: &str = "pricing_model_unspecified";

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

fn greeks_identity() -> Identity {
    Identity::new(
        Family::Derived,
        CapabilityId::new("greeks").expect("canonical greeks id"),
        Physics::BoundedSnapshot,
    )
}

fn contracts_input_honesty(envelope: super::contracts::ContractsEnvelope) -> InputHonesty {
    if envelope.data.is_some() {
        InputHonesty::Lit
    } else {
        InputHonesty::Dark(envelope.status)
    }
}

pub fn extract_greeks(chain: InputHonesty) -> GreeksEnvelope {
    extract_greeks_from(
        chain,
        contracts_input_honesty(extract_contracts(Some(crate::data::KOTAK_NSE_NFO_BOOK_ID))),
    )
}

pub(crate) fn extract_greeks_from(chain: InputHonesty, contracts: InputHonesty) -> GreeksEnvelope {
    let identity = greeks_identity();
    let provenance = ProvenanceLine::raw_hole(identity.clone());
    if let Some(status) = inherit(&[chain, contracts]) {
        return GreeksEnvelope {
            identity,
            status,
            data: None,
            provenance,
            ineligible: Vec::new(),
            canonical: false,
            persist_canonical: false,
        };
    }
    // Named inputs lit. No sourced pricing model → this extract's hole.
    GreeksEnvelope {
        identity,
        status: HonestyStatus::Unavailable,
        data: None,
        provenance,
        ineligible: vec![PRICING_MODEL_UNSPECIFIED.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::provenance::greeks_may_render_number;

    fn assert_no_fixture_greeks(envelope: &GreeksEnvelope) {
        assert!(envelope.data.is_none());
        let json = serde_json::to_value(envelope).unwrap();
        assert!(json["data"].is_null());
        assert!(json.get("delta").is_none());
        assert!(json.get("gamma").is_none());
        assert!(json.get("theta").is_none());
        assert!(!greeks_may_render_number(
            envelope.status,
            &envelope.provenance
        ));
    }

    #[test]
    fn greeks_identity_is_derived_bounded_snapshot() {
        let envelope = extract_greeks(InputHonesty::Dark(HonestyStatus::Unavailable));
        assert_eq!(envelope.identity.family, Family::Derived);
        assert_eq!(envelope.identity.capability_id.as_str(), "greeks");
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }

    #[test]
    fn dark_chain_is_inherited_dark_with_data_none() {
        let envelope = extract_greeks(InputHonesty::Dark(HonestyStatus::Unavailable));
        assert_eq!(envelope.status, HonestyStatus::InheritedDark);
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn lit_chain_dark_contracts_is_inherited_dark_not_unavailable() {
        // Production extract_contracts(nfo) with no store/rows is Unavailable.
        let envelope = extract_greeks(InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::InheritedDark);
        assert_ne!(envelope.status, HonestyStatus::Unavailable);
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn lit_nfo_contracts_and_lit_chain_still_pricing_model_unspecified() {
        use crate::data::{extract_contracts_from_rows, ContractRow, KOTAK_NSE_NFO_BOOK_ID};

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
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn glance_chain_success_plus_lit_contracts_still_pricing_model_unspecified() {
        use crate::data::{
            chain_input_honesty, extract_chain_from, extract_contracts_from_rows, ChainRow,
            ContractRow, KOTAK_NSE_NFO_BOOK_ID,
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
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn named_inputs_lit_missing_model_is_this_extract_unavailable() {
        let envelope = extract_greeks_from(InputHonesty::Lit, InputHonesty::Lit);
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == PRICING_MODEL_UNSPECIFIED));
        assert_no_fixture_greeks(&envelope);
    }

    #[test]
    fn stub_never_fills_fixture_greeks() {
        let envelope = extract_greeks(InputHonesty::Lit);
        assert_no_fixture_greeks(&envelope);
        let lit_inputs = extract_greeks_from(InputHonesty::Lit, InputHonesty::Lit);
        assert_no_fixture_greeks(&lit_inputs);
    }
}

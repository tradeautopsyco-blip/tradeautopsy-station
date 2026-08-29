//! Reference derivative-contracts extract for named book `kotak-nse-nfo` only.
//!
//! Identity is `reference/derivative_contracts/bounded_snapshot` only — never
//! `market/option_chain` and never `market/order_book`.
//!
//! Source: `docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md` (snapshot
//! 2026-08-28). FO column **names** are specified (lock unsigned GET 2026-08-28
//! IST: token `pSymbol`, segment `pExchSeg`=`nse_fo`, lot `lLotSize`+`iLotSize`,
//! strike `dStrikePrice;`, expiry `lExpiryDate `). Strike **scale** and expiry
//! **calendar conversion** remain NOT SPECIFIED — store raw; do not ÷100; do
//! not invent 57500. Cash book still refuses FO CSV. Lit = `data.is_some()`
//! ([`InputHonesty::Lit`]); [`HonestyStatus`] has no Success variant.

use super::descriptor::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NEO_ADAPTER_ID,
    KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use super::honesty::HonestyStatus;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use serde::Serialize;

/// Ineligible reason: F&O scrip master hole / unknown book / empty NFO store.
pub const FO_MASTER_UNSPECIFIED: &str = "nfo_scrip_master_refused";
/// Cash book is not this extract (cash still refuses FO).
pub const CASH_IS_NOT_NFO_CONTRACTS: &str = "cash_is_not_nfo_contracts";
/// Spot book is not this extract.
pub const SPOT_IS_NOT_NFO_CONTRACTS: &str = "spot_is_not_nfo_contracts";
/// Binance options filters unspecified this slice — stay dark.
pub const OPTIONS_FILTERS_UNSPECIFIED: &str = "options_filters_unspecified";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractsEnvelope {
    pub identity: Identity,
    pub status: HonestyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

/// One NFO master row. `lot` serializes as a JSON number. Strike/expiry are raw cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractRow {
    pub instrument_id: String,
    pub lot: i64,
    pub trading_symbol: String,
    pub segment: String,
    pub instrument_type: String,
    pub option_type: String,
    pub strike_raw: String,
    pub expiry_raw: String,
}

fn contracts_identity() -> Identity {
    Identity::new(
        Family::Reference,
        CapabilityId::new("derivative_contracts").expect("canonical derivative_contracts id"),
        Physics::BoundedSnapshot,
    )
}

fn dark_envelope(identity: Identity, ineligible: &str) -> ContractsEnvelope {
    ContractsEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

pub fn extract_contracts(book_id: Option<&str>) -> ContractsEnvelope {
    extract_contracts_from_rows(book_id, None)
}

pub fn extract_contracts_from_rows(
    book_id: Option<&str>,
    rows: Option<&[ContractRow]>,
) -> ContractsEnvelope {
    let identity = contracts_identity();
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_envelope(identity, FO_MASTER_UNSPECIFIED);
    };

    match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => {
            dark_envelope(identity, CASH_IS_NOT_NFO_CONTRACTS)
        }
        id if id == BINANCE_COM_SPOT_BOOK_ID => dark_envelope(identity, SPOT_IS_NOT_NFO_CONTRACTS),
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            dark_envelope(identity, OPTIONS_FILTERS_UNSPECIFIED)
        }
        id if id == KOTAK_NSE_NFO_BOOK_ID => match rows {
            Some(rows) if !rows.is_empty() => lit_nfo_envelope(identity, rows),
            _ => dark_envelope(identity, FO_MASTER_UNSPECIFIED),
        },
        _ => dark_envelope(identity, FO_MASTER_UNSPECIFIED),
    }
}

fn lit_nfo_envelope(identity: Identity, rows: &[ContractRow]) -> ContractsEnvelope {
    // HonestyStatus has no Success. Lit = data.is_some() (InputHonesty::Lit).
    // Empty is the live envelope variant we must pick; tests must not treat it as
    // "zero contracts" or as Success. Empty NFO store uses Unavailable above.
    let data = serde_json::json!({
        "identity": identity,
        "contract_count": rows.len(),
        "rows": rows,
    });
    ContractsEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Empty,
        data: Some(data),
        provenance: ProvenanceLine {
            identity,
            model: "raw".to_string(),
            input_at: None,
            adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
        },
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::matrix::{known_id_physics_ok, known_physics};

    /// Lock sample 2026-08-28: pSymbol=56526, lLotSize=iLotSize=65, raw strike 2.1e+06.
    fn lock_sample_row() -> ContractRow {
        ContractRow {
            instrument_id: "nse_fo|56526".to_string(),
            lot: 65,
            trading_symbol: "NIFTY2692221000PE".to_string(),
            segment: "nse_fo".to_string(),
            instrument_type: "OPTIDX".to_string(),
            option_type: "PE".to_string(),
            strike_raw: "2.1e+06".to_string(),
            expiry_raw: "1474554600".to_string(),
        }
    }

    #[test]
    fn contracts_identity_is_reference_bounded_snapshot() {
        let envelope = extract_contracts(None);
        assert_eq!(envelope.identity.family, Family::Reference);
        assert_eq!(
            envelope.identity.capability_id.as_str(),
            "derivative_contracts"
        );
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }

    #[test]
    fn contracts_known_id_physics_is_bounded_snapshot_not_ordered() {
        assert!(known_id_physics_ok(
            Family::Reference,
            "derivative_contracts",
            Physics::BoundedSnapshot
        ));
        assert!(!known_id_physics_ok(
            Family::Reference,
            "derivative_contracts",
            Physics::OrderedState
        ));
    }

    #[test]
    fn contracts_must_not_bind_as_option_chain_or_order_book() {
        let envelope = extract_contracts(None);
        assert_ne!(envelope.identity.capability_id.as_str(), "option_chain");
        assert_ne!(envelope.identity.capability_id.as_str(), "order_book");
        assert_ne!(envelope.identity.family, Family::Market);
        assert_eq!(envelope.identity.family, Family::Reference);
        // Unknown market id → snake check only (`None`), not a bind. Family is Reference
        // so this cannot collide with market option_chain / order_book.
        assert!(known_physics(Family::Market, "derivative_contracts").is_none());
    }

    #[test]
    fn unknown_or_none_book_is_unavailable_nfo_scrip_master_refused() {
        for book in [None, Some(""), Some("   "), Some("not-a-book")] {
            let envelope = extract_contracts(book);
            assert_eq!(envelope.status, HonestyStatus::Unavailable);
            assert!(envelope.data.is_none());
            assert!(
                envelope
                    .ineligible
                    .iter()
                    .any(|s| s == FO_MASTER_UNSPECIFIED),
                "book {book:?} must name nfo_scrip_master_refused"
            );
        }
    }

    #[test]
    fn cash_book_is_unavailable_with_data_none() {
        let envelope = extract_contracts(Some(KOTAK_NSE_BSE_CASH_BOOK_ID));
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.data.is_none());
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == CASH_IS_NOT_NFO_CONTRACTS));
        let spot = extract_contracts(Some(BINANCE_COM_SPOT_BOOK_ID));
        assert_eq!(spot.status, HonestyStatus::Unavailable);
        assert!(spot.data.is_none());
    }

    #[test]
    fn options_book_is_unavailable() {
        let envelope = extract_contracts(Some(BINANCE_COM_OPTIONS_BOOK_ID));
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn empty_nfo_store_is_unavailable_not_empty_as_success() {
        let none_rows = extract_contracts(Some(KOTAK_NSE_NFO_BOOK_ID));
        assert_eq!(none_rows.status, HonestyStatus::Unavailable);
        assert!(none_rows.data.is_none());
        assert!(none_rows
            .ineligible
            .iter()
            .any(|s| s == FO_MASTER_UNSPECIFIED));

        let empty_slice: [ContractRow; 0] = [];
        let empty = extract_contracts_from_rows(Some(KOTAK_NSE_NFO_BOOK_ID), Some(&empty_slice));
        assert_eq!(empty.status, HonestyStatus::Unavailable);
        assert!(empty.data.is_none());
    }

    #[test]
    fn nfo_fixture_row_lot_is_json_number_65_no_scaled_strike() {
        let rows = [lock_sample_row()];
        let envelope = extract_contracts_from_rows(Some(KOTAK_NSE_NFO_BOOK_ID), Some(&rows));
        let data = envelope
            .data
            .expect("lit = data.is_some(); not HonestyStatus::Success");
        assert_eq!(data["identity"]["family"], "reference");
        assert_eq!(data["identity"]["capability_id"], "derivative_contracts");
        assert_eq!(data["identity"]["physics"], "bounded_snapshot");
        assert_eq!(data["contract_count"], 1);
        assert!(data.get("strike_grid").is_none());
        assert!(data.get("strikes").is_none());

        let lot = &data["rows"][0]["lot"];
        assert!(lot.is_number(), "lot must be a JSON number, not a string");
        assert!(!lot.is_string());
        assert_eq!(lot.as_i64(), Some(65));
        assert_eq!(data["rows"][0]["instrument_id"], "nse_fo|56526");
        assert_eq!(data["rows"][0]["strike_raw"], "2.1e+06");
        assert_eq!(data["rows"][0]["expiry_raw"], "1474554600");

        let dumped = data.to_string();
        assert!(
            !dumped.contains("57500"),
            "must not invent scaled strike 57500: {dumped}"
        );
        assert_eq!(envelope.provenance.model, "raw");
        assert_eq!(envelope.provenance.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert!(!envelope.canonical);
        assert!(!envelope.persist_canonical);
    }
}

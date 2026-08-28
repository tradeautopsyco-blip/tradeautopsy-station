//! Reference derivative-contracts hole. Do not invent F&O rows, lots, strikes, or tokens.
//!
//! Identity is `reference/derivative_contracts/bounded_snapshot` only — never
//! `market/option_chain` and never `market/order_book`.
//!
//! Source: `docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md` — v1 refuses F&O CSV;
//! official NFO schema is NOT SPECIFIED IN SOURCE. Do not allowlist `nse_fo` or
//! download `nse_fo.csv`.

use super::honesty::HonestyStatus;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use serde::Serialize;

/// Ineligible reason: F&O scrip master is refused (schema unspecified).
pub const FO_MASTER_UNSPECIFIED: &str = "nfo_scrip_master_refused";

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

fn contracts_identity() -> Identity {
    Identity::new(
        Family::Reference,
        CapabilityId::new("derivative_contracts").expect("canonical derivative_contracts id"),
        Physics::BoundedSnapshot,
    )
}

pub fn extract_contracts() -> ContractsEnvelope {
    let identity = contracts_identity();
    ContractsEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![FO_MASTER_UNSPECIFIED.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::matrix::{known_id_physics_ok, known_physics};

    #[test]
    fn contracts_identity_is_reference_bounded_snapshot() {
        let envelope = extract_contracts();
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
        let envelope = extract_contracts();
        assert_ne!(envelope.identity.capability_id.as_str(), "option_chain");
        assert_ne!(envelope.identity.capability_id.as_str(), "order_book");
        assert_ne!(envelope.identity.family, Family::Market);
        assert_eq!(envelope.identity.family, Family::Reference);
        // Unknown market id → snake check only (`None`), not a bind. Family is Reference
        // so this cannot collide with market option_chain / order_book.
        assert!(known_physics(Family::Market, "derivative_contracts").is_none());
    }

    #[test]
    fn contracts_is_unavailable_hole_naming_fo_master() {
        let envelope = extract_contracts();
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.data.is_none());
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == FO_MASTER_UNSPECIFIED));
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }
}

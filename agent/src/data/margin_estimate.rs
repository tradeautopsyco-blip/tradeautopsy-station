//! Account margin-estimate hole. Calculator is unspecified — do not invent SPAN.

use super::honesty::HonestyStatus;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use serde::Serialize;

pub const MARGIN_CALCULATOR_UNSPECIFIED: &str = "margin_calculator_unspecified";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarginEstimateEnvelope {
    pub identity: Identity,
    pub status: HonestyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

fn margin_identity() -> Identity {
    Identity::new(
        Family::Account,
        CapabilityId::new("margin_estimate").expect("canonical margin_estimate id"),
        Physics::BoundedSnapshot,
    )
}

pub fn extract_margin_estimate() -> MarginEstimateEnvelope {
    let identity = margin_identity();
    MarginEstimateEnvelope {
        identity: identity.clone(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![MARGIN_CALCULATOR_UNSPECIFIED.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;

    #[test]
    fn margin_estimate_is_unavailable_hole() {
        let envelope = extract_margin_estimate();
        assert_eq!(envelope.status, HonestyStatus::Unavailable);
        assert!(envelope.data.is_none());
        assert!(envelope
            .ineligible
            .iter()
            .any(|s| s == MARGIN_CALCULATOR_UNSPECIFIED));
        assert_eq!(envelope.identity.family, Family::Account);
        assert_eq!(envelope.identity.capability_id.as_str(), "margin_estimate");
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(envelope.provenance.model, "raw");
        assert!(envelope.provenance.input_at.is_none());
    }
}

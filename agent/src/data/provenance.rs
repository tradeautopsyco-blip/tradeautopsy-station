//! Provenance line on glance / greeks / margin envelopes. Not caption chrome.

use super::honesty::HonestyStatus;
use super::identity::Identity;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvenanceLine {
    /// Named identity of this envelope.
    pub identity: Identity,
    /// `"raw"` for holes; never invent Black-76.
    pub model: String,
    /// RFC3339; None when never stamped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_at: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub adapter_id: String,
}

impl ProvenanceLine {
    pub fn raw_hole(identity: Identity) -> Self {
        Self {
            identity,
            model: "raw".to_string(),
            input_at: None,
            adapter_id: String::new(),
        }
    }
}

/// True only if this is not a dark extract, the chain stamp is named, and identity is greeks.
/// `HonestyStatus` has no Lit variant, so current callers never render a number.
pub fn greeks_may_render_number(honesty: HonestyStatus, provenance: &ProvenanceLine) -> bool {
    if provenance.input_at.is_none() {
        return false;
    }
    if provenance.identity.capability_id.as_str() != "greeks" {
        return false;
    }
    match honesty {
        HonestyStatus::Empty
        | HonestyStatus::Unavailable
        | HonestyStatus::Unusable
        | HonestyStatus::InheritedDark => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::{CapabilityId, Family, Physics};

    fn greeks_identity() -> Identity {
        Identity::new(
            Family::Derived,
            CapabilityId::new("greeks").expect("canonical greeks id"),
            Physics::BoundedSnapshot,
        )
    }

    #[test]
    fn missing_input_at_refuses_greeks_number() {
        let provenance = ProvenanceLine {
            identity: greeks_identity(),
            model: "raw".to_string(),
            input_at: None,
            adapter_id: String::new(),
        };
        assert!(!greeks_may_render_number(
            HonestyStatus::Unavailable,
            &provenance
        ));
    }

    #[test]
    fn inherited_dark_refuses_greeks_number() {
        let provenance = ProvenanceLine {
            identity: greeks_identity(),
            model: "raw".to_string(),
            input_at: Some("2026-08-27T18:00:00Z".to_string()),
            adapter_id: String::new(),
        };
        assert!(!greeks_may_render_number(
            HonestyStatus::InheritedDark,
            &provenance
        ));
    }

    #[test]
    fn hole_omits_empty_adapter_and_unstamped_input_at() {
        let json = serde_json::to_value(ProvenanceLine::raw_hole(greeks_identity())).unwrap();
        assert_eq!(json["model"], "raw");
        assert!(json.get("input_at").is_none());
        assert!(json.get("adapter_id").is_none());
        assert_eq!(json["identity"]["capability_id"], "greeks");
    }
}

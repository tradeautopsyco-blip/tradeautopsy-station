//! Provenance line on glance / greeks / margin envelopes. Not caption chrome.

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
    /// Upstream path this envelope's data actually came from. Empty when the
    /// book's official path is NOT SPECIFIED — a Success must never name a
    /// path the venue does not have.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub path: String,
}

impl ProvenanceLine {
    pub fn raw_hole(identity: Identity) -> Self {
        Self {
            identity,
            model: "raw".to_string(),
            input_at: None,
            adapter_id: String::new(),
            path: String::new(),
        }
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
    fn hole_omits_empty_adapter_and_unstamped_input_at() {
        let json = serde_json::to_value(ProvenanceLine::raw_hole(greeks_identity())).unwrap();
        assert_eq!(json["model"], "raw");
        assert!(json.get("input_at").is_none());
        assert!(json.get("adapter_id").is_none());
        assert!(json.get("path").is_none());
        assert_eq!(json["identity"]["capability_id"], "greeks");
    }
}

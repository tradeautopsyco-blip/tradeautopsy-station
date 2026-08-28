//! S3 holes: options chain / OI widgets stay unavailable until those extracts exist.

use super::honesty::HonestyStatus;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlanceEnvelope {
    pub identity: Identity,
    pub instrument_id: String,
    pub status: HonestyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

fn chain_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("option_chain").expect("canonical option_chain id"),
        Physics::BoundedSnapshot,
    )
}

fn oi_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("open_interest").expect("open_interest id"),
        Physics::LatestState,
    )
}

pub fn extract_chain(instrument_id: &str) -> GlanceEnvelope {
    let identity = chain_identity();
    GlanceEnvelope {
        identity: identity.clone(),
        instrument_id: instrument_id.trim().to_ascii_lowercase(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

pub fn extract_open_interest(instrument_id: &str) -> GlanceEnvelope {
    let identity = oi_identity();
    GlanceEnvelope {
        identity: identity.clone(),
        instrument_id: instrument_id.trim().to_ascii_lowercase(),
        status: HonestyStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;

    #[test]
    fn chain_and_oi_are_unavailable_holes() {
        let chain = extract_chain("btcusdt");
        assert_eq!(chain.status, HonestyStatus::Unavailable);
        assert!(chain.data.is_none());
        assert_eq!(chain.identity.capability_id.as_str(), "option_chain");
        assert_eq!(chain.identity.physics, Physics::BoundedSnapshot);
        assert_eq!(chain.provenance.model, "raw");
        assert!(chain.provenance.input_at.is_none());
        assert!(chain.provenance.adapter_id.is_empty());

        let oi = extract_open_interest("BTC");
        assert_eq!(oi.status, HonestyStatus::Unavailable);
        assert_eq!(oi.instrument_id, "btc");
        assert_eq!(oi.identity.capability_id.as_str(), "open_interest");
        assert_eq!(oi.provenance.model, "raw");
    }

    #[test]
    fn chain_wire_keeps_status_unavailable() {
        let json = serde_json::to_value(extract_chain("btcusdt")).unwrap();
        assert_eq!(json["status"], "unavailable");
        assert!(json["data"].is_null());
        assert!(json.get("honesty").is_none());
        assert_eq!(json["provenance"]["model"], "raw");
        assert!(json["provenance"].get("input_at").is_none());
        assert!(json["provenance"].get("adapter_id").is_none());
    }
}

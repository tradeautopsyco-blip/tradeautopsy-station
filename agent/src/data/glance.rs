//! S3 holes: options chain / OI widgets stay unavailable until those extracts exist.

use super::identity::{CapabilityId, Family, Identity, Physics};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlanceStatus {
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlanceEnvelope {
    pub identity: Identity,
    pub instrument_id: String,
    pub status: GlanceStatus,
    pub data: Option<serde_json::Value>,
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
    GlanceEnvelope {
        identity: chain_identity(),
        instrument_id: instrument_id.trim().to_ascii_lowercase(),
        status: GlanceStatus::Unavailable,
        data: None,
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

pub fn extract_open_interest(instrument_id: &str) -> GlanceEnvelope {
    GlanceEnvelope {
        identity: oi_identity(),
        instrument_id: instrument_id.trim().to_ascii_lowercase(),
        status: GlanceStatus::Unavailable,
        data: None,
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
        assert_eq!(chain.status, GlanceStatus::Unavailable);
        assert!(chain.data.is_none());
        assert_eq!(chain.identity.capability_id.as_str(), "option_chain");
        assert_eq!(chain.identity.physics, Physics::BoundedSnapshot);

        let oi = extract_open_interest("BTC");
        assert_eq!(oi.status, GlanceStatus::Unavailable);
        assert_eq!(oi.instrument_id, "btc");
        assert_eq!(oi.identity.capability_id.as_str(), "open_interest");
    }
}

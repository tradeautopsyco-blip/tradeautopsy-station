//! Force-order fixture extract — `market/force_order` lossy_event_observation only.
//!
//! Port of Console `extractForceOrder` / `rejectLossyAsComplete`. No Axum route this PR;
//! callers are unit tests only. No ingestSignal / routeOrder path.

use super::descriptor::DelayClass;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::matrix::{known_id_physics_ok, pair_allowed};
use super::rights::Rights;
use super::tick::Transport;
use serde::Serialize;

pub const LOSSY_CANNOT_CLAIM_COMPLETE: &str = "lossy_cannot_claim_complete";

/// Lossy observation status. Never `synced` / `complete` / `fresh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LossyStatus {
    Observing,
    Interrupted,
    Idle,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ForceOrderProvenance {
    pub adapter_id: String,
    pub transport: Transport,
    pub delay_class: DelayClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ForceOrderEnvelope {
    pub identity: Identity,
    pub status: LossyStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ForceOrderProvenance,
    pub rights: Rights,
    pub ineligible: Vec<String>,
    pub research: bool,
    pub canonical: bool,
    pub persist_canonical: bool,
}

fn force_order_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("force_order").expect("force_order id"),
        Physics::LossyEventObservation,
    )
}

fn fixture_provenance() -> ForceOrderProvenance {
    ForceOrderProvenance {
        adapter_id: "binance_public".to_string(),
        transport: Transport::Fixture,
        delay_class: DelayClass::Realtime,
    }
}

fn base_envelope(
    identity: Identity,
    status: LossyStatus,
    data: Option<serde_json::Value>,
    extra_ineligible: Vec<String>,
) -> ForceOrderEnvelope {
    let mut ineligible = extra_ineligible;
    if !pair_allowed(identity.family, identity.physics) {
        ineligible.push("unknown_family_physics_pair".to_string());
    }
    if !known_id_physics_ok(
        identity.family,
        identity.capability_id.as_str(),
        identity.physics,
    ) && !ineligible
        .iter()
        .any(|reason| reason == LOSSY_CANNOT_CLAIM_COMPLETE)
    {
        ineligible.push("capability_physics_mismatch".to_string());
    }
    ForceOrderEnvelope {
        identity,
        status,
        data,
        provenance: fixture_provenance(),
        rights: Rights::research_fetch_only(),
        ineligible,
        research: true,
        canonical: false,
        persist_canonical: false,
    }
}

/// Lossy force-order observation — observing, never canonical.
pub fn extract_force_order() -> ForceOrderEnvelope {
    base_envelope(
        force_order_identity(),
        LossyStatus::Observing,
        Some(serde_json::json!({"symbol": "BTCUSDT", "side": "SELL"})),
        vec![],
    )
}

/// Wrong physics on force_order — unavailable + `lossy_cannot_claim_complete`.
pub fn reject_lossy_as_complete() -> ForceOrderEnvelope {
    let identity = Identity::new(
        Family::Market,
        CapabilityId::new("force_order").expect("force_order id"),
        Physics::CompleteEventSequence,
    );
    base_envelope(
        identity,
        LossyStatus::Unavailable,
        None,
        vec![LOSSY_CANNOT_CLAIM_COMPLETE.to_string()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lossy_observing_never_synced() {
        let envelope = extract_force_order();
        assert_eq!(envelope.status, LossyStatus::Observing);
        assert_ne!(envelope.status, LossyStatus::Unavailable);
        assert!(!envelope.canonical);
        assert!(!envelope.persist_canonical);
        assert_eq!(envelope.identity.physics, Physics::LossyEventObservation);
        assert!(envelope.data.is_some());
    }

    #[test]
    fn lossy_cannot_register_complete() {
        let envelope = reject_lossy_as_complete();
        assert_eq!(envelope.status, LossyStatus::Unavailable);
        assert!(envelope.data.is_none());
        assert!(
            envelope
                .ineligible
                .iter()
                .any(|reason| reason == LOSSY_CANNOT_CLAIM_COMPLETE),
            "ineligible={:?}",
            envelope.ineligible
        );
        assert_eq!(envelope.identity.physics, Physics::CompleteEventSequence);
    }

    #[test]
    fn envelope_is_not_canonical() {
        let observing = extract_force_order();
        let rejected = reject_lossy_as_complete();
        assert!(!observing.canonical);
        assert!(!observing.persist_canonical);
        assert!(!rejected.canonical);
        assert!(!rejected.persist_canonical);
    }
}

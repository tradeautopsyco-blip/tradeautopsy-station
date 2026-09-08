//! Connected-broker-primary routing (R0). Provenance is the adapter that answered.

#![allow(dead_code)]

use super::identity::{Family, Identity, Physics};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRole {
    ConnectedBroker,
    ExplicitGap,
    SpecializedNonBroker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportPick {
    LocalProjection,
    Stream,
    Rest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCandidate {
    pub adapter_id: String,
    pub identity: Identity,
    pub role: SourceRole,
    pub eligible: bool,
    pub broker_unsupported: bool,
    pub local_fresh: bool,
    pub stream_available: bool,
    pub rest_available: bool,
    pub budget_remaining: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteOutcome {
    Picked,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDecision {
    pub adapter_id: Option<String>,
    pub transport: Option<TransportPick>,
    pub outcome: RouteOutcome,
    pub provenance_adapter_id: Option<String>,
}

fn same_identity(left: &Identity, right: &Identity) -> bool {
    left.family == right.family
        && left.capability_id.as_str() == right.capability_id.as_str()
        && left.physics == right.physics
}

pub fn pick_route(request: &Identity, candidates: &[RouteCandidate]) -> RouteDecision {
    let matching: Vec<&RouteCandidate> = candidates
        .iter()
        .filter(|candidate| same_identity(&candidate.identity, request))
        .collect();

    if matches!(
        request.family,
        Family::Economic | Family::News | Family::Fundamentals
    ) {
        if let Some(specialized) = matching.iter().find(|candidate| {
            candidate.role == SourceRole::SpecializedNonBroker && candidate.eligible
        }) {
            return RouteDecision {
                adapter_id: Some(specialized.adapter_id.clone()),
                transport: Some(TransportPick::Rest),
                outcome: RouteOutcome::Picked,
                provenance_adapter_id: Some(specialized.adapter_id.clone()),
            };
        }
    }

    let connected: Vec<&RouteCandidate> = matching
        .iter()
        .copied()
        .filter(|candidate| candidate.role == SourceRole::ConnectedBroker && candidate.eligible)
        .collect();

    if let Some(local) = connected.iter().find(|candidate| candidate.local_fresh) {
        return RouteDecision {
            adapter_id: Some(local.adapter_id.clone()),
            transport: Some(TransportPick::LocalProjection),
            outcome: RouteOutcome::Picked,
            provenance_adapter_id: Some(local.adapter_id.clone()),
        };
    }
    if let Some(stream) = connected
        .iter()
        .find(|candidate| candidate.stream_available)
    {
        return RouteDecision {
            adapter_id: Some(stream.adapter_id.clone()),
            transport: Some(TransportPick::Stream),
            outcome: RouteOutcome::Picked,
            provenance_adapter_id: Some(stream.adapter_id.clone()),
        };
    }
    if let Some(rest) = connected
        .iter()
        .find(|candidate| candidate.rest_available && candidate.budget_remaining > 0)
    {
        return RouteDecision {
            adapter_id: Some(rest.adapter_id.clone()),
            transport: Some(TransportPick::Rest),
            outcome: RouteOutcome::Picked,
            provenance_adapter_id: Some(rest.adapter_id.clone()),
        };
    }

    if let Some(gap) = matching.iter().find(|candidate| {
        candidate.role == SourceRole::ExplicitGap
            && candidate.eligible
            && candidate.budget_remaining > 0
    }) {
        return RouteDecision {
            adapter_id: Some(gap.adapter_id.clone()),
            transport: Some(TransportPick::Rest),
            outcome: RouteOutcome::Picked,
            provenance_adapter_id: Some(gap.adapter_id.clone()),
        };
    }

    if matching
        .iter()
        .any(|candidate| candidate.role == SourceRole::ExplicitGap && candidate.budget_remaining == 0)
    {
        return RouteDecision {
            adapter_id: None,
            transport: None,
            outcome: RouteOutcome::Unavailable,
            provenance_adapter_id: None,
        };
    }

    if matching.iter().any(|candidate| {
        candidate.role == SourceRole::ConnectedBroker && candidate.broker_unsupported
    }) {
        return RouteDecision {
            adapter_id: None,
            transport: None,
            outcome: RouteOutcome::Unsupported,
            provenance_adapter_id: None,
        };
    }

    RouteDecision {
        adapter_id: None,
        transport: None,
        outcome: RouteOutcome::Unavailable,
        provenance_adapter_id: None,
    }
}

pub fn may_ingest_signal(family: Family) -> bool {
    !matches!(family, Family::Market | Family::Reference | Family::Account)
}

pub fn rest_snapshot_may_claim_synced(physics: Physics, status: &str) -> bool {
    physics == Physics::OrderedState && status == "synced"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::CapabilityId;

    fn quote_identity() -> Identity {
        Identity::new(
            Family::Market,
            CapabilityId::new("quote").unwrap(),
            Physics::LatestState,
        )
    }

    fn history_identity() -> Identity {
        Identity::new(
            Family::Market,
            CapabilityId::new("ohlcv").unwrap(),
            Physics::HistoricalSeries,
        )
    }

    #[test]
    fn connected_broker_wins_eligible_quote_over_vendor() {
        let connected = RouteCandidate {
            adapter_id: "binance_com".into(),
            identity: quote_identity(),
            role: SourceRole::ConnectedBroker,
            eligible: true,
            broker_unsupported: false,
            local_fresh: false,
            stream_available: true,
            rest_available: true,
            budget_remaining: 10,
        };
        let vendor = RouteCandidate {
            adapter_id: "twelve_data".into(),
            identity: quote_identity(),
            role: SourceRole::ExplicitGap,
            eligible: true,
            broker_unsupported: false,
            local_fresh: false,
            stream_available: false,
            rest_available: true,
            budget_remaining: 100,
        };
        let decision = pick_route(&quote_identity(), &[vendor, connected]);
        assert_eq!(decision.adapter_id.as_deref(), Some("binance_com"));
        assert_eq!(decision.transport, Some(TransportPick::Stream));
        assert_eq!(
            decision.provenance_adapter_id.as_deref(),
            Some("binance_com")
        );
    }

    #[test]
    fn unsupported_broker_routes_to_explicit_gap_with_actual_provenance() {
        let kotak = RouteCandidate {
            adapter_id: "kotak_neo".into(),
            identity: history_identity(),
            role: SourceRole::ConnectedBroker,
            eligible: false,
            broker_unsupported: true,
            local_fresh: false,
            stream_available: false,
            rest_available: false,
            budget_remaining: 10,
        };
        let gap = RouteCandidate {
            adapter_id: "licensed_history".into(),
            identity: history_identity(),
            role: SourceRole::ExplicitGap,
            eligible: true,
            broker_unsupported: false,
            local_fresh: false,
            stream_available: false,
            rest_available: true,
            budget_remaining: 5,
        };
        let decision = pick_route(&history_identity(), &[kotak, gap]);
        assert_eq!(decision.adapter_id.as_deref(), Some("licensed_history"));
        assert_eq!(
            decision.provenance_adapter_id.as_deref(),
            Some("licensed_history")
        );
        assert_ne!(decision.provenance_adapter_id.as_deref(), Some("kotak_neo"));
    }

    #[test]
    fn gap_budget_zero_is_unavailable_not_unsupported() {
        let kotak = RouteCandidate {
            adapter_id: "kotak_neo".into(),
            identity: history_identity(),
            role: SourceRole::ConnectedBroker,
            eligible: false,
            broker_unsupported: true,
            local_fresh: false,
            stream_available: false,
            rest_available: false,
            budget_remaining: 10,
        };
        let gap = RouteCandidate {
            adapter_id: "licensed_history".into(),
            identity: history_identity(),
            role: SourceRole::ExplicitGap,
            eligible: false,
            broker_unsupported: false,
            local_fresh: false,
            stream_available: false,
            rest_available: true,
            budget_remaining: 0,
        };
        let decision = pick_route(&history_identity(), &[kotak, gap]);
        assert_eq!(decision.outcome, RouteOutcome::Unavailable);
        assert!(decision.adapter_id.is_none());
    }

    #[test]
    fn market_packets_cannot_enter_ingest_signal() {
        assert!(!may_ingest_signal(Family::Market));
        assert!(!may_ingest_signal(Family::Account));
        assert!(!rest_snapshot_may_claim_synced(
            Physics::BoundedSnapshot,
            "snapshot"
        ));
        assert!(!rest_snapshot_may_claim_synced(
            Physics::BoundedSnapshot,
            "synced"
        ));
        assert!(rest_snapshot_may_claim_synced(
            Physics::OrderedState,
            "synced"
        ));
    }
}

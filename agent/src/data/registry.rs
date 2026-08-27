//! Load-time capability registry. Fail closed: any bad binding refuses the whole load.

use super::descriptor::{forbids_openbb_source, Descriptor};
use super::identity::{validate_capability_id, CapabilityIdError, Identity};
use super::matrix::{known_id_physics_ok, market_account_id_collision, pair_allowed};
use std::fmt;

/// Why a descriptor was refused at load. Structured so tests can match variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    UnknownFamilyPhysicsPair {
        adapter_id: String,
        identity: Identity,
    },
    CapabilityPhysicsMismatch {
        adapter_id: String,
        identity: Identity,
    },
    VendorEncodedInCapabilityId {
        adapter_id: String,
        capability_id: String,
    },
    InvalidCapabilityId {
        adapter_id: String,
        capability_id: String,
    },
    RightsMissing {
        adapter_id: String,
        identity: Identity,
    },
    AccountBindingMissing {
        adapter_id: String,
        identity: Identity,
    },
    OpenBbSourceForbidden {
        adapter_id: String,
    },
    MarketAccountIdCollision {
        adapter_id: String,
        identity: Identity,
    },
}

impl fmt::Display for Reject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reject::UnknownFamilyPhysicsPair {
                adapter_id,
                identity,
            } => write!(
                f,
                "unknown family×physics pair for adapter `{adapter_id}`: {identity}"
            ),
            Reject::CapabilityPhysicsMismatch {
                adapter_id,
                identity,
            } => write!(
                f,
                "capability×physics mismatch for adapter `{adapter_id}`: {identity}"
            ),
            Reject::VendorEncodedInCapabilityId {
                adapter_id,
                capability_id,
            } => write!(
                f,
                "vendor/interval/symbol encoded in capability_id `{capability_id}` (adapter `{adapter_id}`)"
            ),
            Reject::InvalidCapabilityId {
                adapter_id,
                capability_id,
            } => write!(
                f,
                "invalid capability_id `{capability_id}` (adapter `{adapter_id}`)"
            ),
            Reject::RightsMissing {
                adapter_id,
                identity,
            } => write!(
                f,
                "rights object missing for adapter `{adapter_id}` ({identity})"
            ),
            Reject::AccountBindingMissing {
                adapter_id,
                identity,
            } => write!(
                f,
                "account binding missing tenant_id/broker_account_id for adapter `{adapter_id}` ({identity})"
            ),
            Reject::OpenBbSourceForbidden { adapter_id } => {
                write!(f, "OpenBB/ODP is not an installed source (adapter `{adapter_id}`)")
            }
            Reject::MarketAccountIdCollision {
                adapter_id,
                identity,
            } => write!(
                f,
                "market/account capability id collision for adapter `{adapter_id}`: {identity}"
            ),
        }
    }
}

impl std::error::Error for Reject {}

/// Accepted bindings only. Never a mix of good and bad adapters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    descriptors: Vec<Descriptor>,
}

impl Registry {
    /// Validate every descriptor. If any fails, return **all** rejects (no partial load).
    pub fn load(descriptors: &[Descriptor]) -> Result<Self, Vec<Reject>> {
        let mut rejects = Vec::new();
        for descriptor in descriptors {
            rejects.extend(validate_descriptor(descriptor));
        }
        if !rejects.is_empty() {
            return Err(rejects);
        }
        Ok(Self {
            descriptors: descriptors.to_vec(),
        })
    }

    pub fn get(&self, identity: &Identity) -> Option<&Descriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| &descriptor.identity == identity)
    }

    pub fn get_for_adapter(&self, identity: &Identity, adapter_id: &str) -> Option<&Descriptor> {
        self.descriptors.iter().find(|descriptor| {
            &descriptor.identity == identity && descriptor.adapter_id == adapter_id
        })
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Descriptor> {
        self.descriptors.iter()
    }
}

fn validate_descriptor(descriptor: &Descriptor) -> Vec<Reject> {
    let mut rejects = Vec::new();
    let adapter_id = descriptor.adapter_id.clone();
    let identity = descriptor.identity.clone();
    let capability_id = identity.capability_id.as_str();

    if forbids_openbb_source(
        &descriptor.adapter_id,
        descriptor.installed_source.as_deref(),
    ) {
        rejects.push(Reject::OpenBbSourceForbidden {
            adapter_id: adapter_id.clone(),
        });
    }

    if descriptor.rights.is_none() {
        rejects.push(Reject::RightsMissing {
            adapter_id: adapter_id.clone(),
            identity: identity.clone(),
        });
    }

    match validate_capability_id(capability_id) {
        Ok(()) => {}
        Err(CapabilityIdError::EncodedVendorOrInterval(_)) => {
            rejects.push(Reject::VendorEncodedInCapabilityId {
                adapter_id: adapter_id.clone(),
                capability_id: capability_id.to_string(),
            });
        }
        Err(CapabilityIdError::InvalidPattern(_)) => {
            rejects.push(Reject::InvalidCapabilityId {
                adapter_id: adapter_id.clone(),
                capability_id: capability_id.to_string(),
            });
        }
    }

    if !pair_allowed(identity.family, identity.physics) {
        rejects.push(Reject::UnknownFamilyPhysicsPair {
            adapter_id: adapter_id.clone(),
            identity: identity.clone(),
        });
    }

    if market_account_id_collision(identity.family, capability_id) {
        rejects.push(Reject::MarketAccountIdCollision {
            adapter_id: adapter_id.clone(),
            identity: identity.clone(),
        });
    }

    if !known_id_physics_ok(identity.family, capability_id, identity.physics) {
        rejects.push(Reject::CapabilityPhysicsMismatch {
            adapter_id: adapter_id.clone(),
            identity: identity.clone(),
        });
    }

    if identity.family == super::identity::Family::Account {
        let missing = match &descriptor.account {
            None => true,
            Some(binding) => !binding.is_complete(),
        };
        if missing {
            rejects.push(Reject::AccountBindingMissing {
                adapter_id,
                identity,
            });
        }
    }

    rejects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::descriptor::{
        fixture_account_descriptor, fixture_quote_descriptor, DelayClass, Limits,
    };
    use crate::data::identity::{CapabilityId, Family, Physics};

    fn load_ok(descriptors: &[Descriptor]) -> Registry {
        Registry::load(descriptors)
            .unwrap_or_else(|rejects| panic!("expected load Ok, got {rejects:?}"))
    }

    fn load_err(descriptors: &[Descriptor]) -> Vec<Reject> {
        Registry::load(descriptors).expect_err("expected load Err")
    }

    #[test]
    fn accepts_fixture_market_quote_latest_state() {
        let descriptor = fixture_quote_descriptor();
        let registry = load_ok(&[descriptor.clone()]);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(&descriptor.identity), Some(&descriptor));
        assert!(descriptor.rights.is_some());
    }

    #[test]
    fn rejects_market_quote_ordered_state() {
        let mut descriptor = fixture_quote_descriptor();
        descriptor.identity.physics = Physics::OrderedState;
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::CapabilityPhysicsMismatch { .. })),
            "expected CapabilityPhysicsMismatch, got {rejects:?}"
        );
    }

    #[test]
    fn rejects_missing_rights_via_serde_json() {
        let json = serde_json::json!({
            "adapter_id": "fixture_equity_quote",
            "identity": {
                "family": "market",
                "capability_id": "quote",
                "physics": "latest_state"
            },
            "delay_class": "realtime",
            "limits": {}
        });
        assert!(json.get("rights").is_none());
        let descriptor: Descriptor =
            serde_json::from_value(json).expect("parse without rights key");
        assert!(descriptor.rights.is_none());
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::RightsMissing { .. })),
            "expected RightsMissing, got {rejects:?}"
        );
    }

    #[test]
    fn rejects_vendor_encoded_in_capability_id() {
        let mut json = serde_json::to_value(fixture_quote_descriptor()).unwrap();
        json["identity"]["capability_id"] = serde_json::Value::String("twelve_data_aapl_1d".into());
        let descriptor: Descriptor = serde_json::from_value(json).unwrap();
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::VendorEncodedInCapabilityId { .. })),
            "expected VendorEncodedInCapabilityId, got {rejects:?}"
        );
    }

    #[test]
    fn rejects_account_funds_latest_state() {
        let mut descriptor = fixture_account_descriptor();
        descriptor.identity.physics = Physics::LatestState;
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::UnknownFamilyPhysicsPair { .. })),
            "expected UnknownFamilyPhysicsPair, got {rejects:?}"
        );
    }

    #[test]
    fn rejects_account_family_without_tenant_binding() {
        let mut descriptor = fixture_account_descriptor();
        descriptor.account = None;
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::AccountBindingMissing { .. })),
            "expected AccountBindingMissing, got {rejects:?}"
        );
    }

    #[test]
    fn rejects_openbb_adapter_id() {
        let mut descriptor = fixture_quote_descriptor();
        descriptor.adapter_id = "openbb_equity".to_string();
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::OpenBbSourceForbidden { .. })),
            "expected OpenBbSourceForbidden, got {rejects:?}"
        );
    }

    #[test]
    fn fixture_quote_is_loadable_with_no_network() {
        let descriptor = fixture_quote_descriptor();
        assert_eq!(descriptor.adapter_id, "fixture_equity_quote");
        assert_eq!(descriptor.identity.family, Family::Market);
        assert_eq!(descriptor.identity.capability_id.as_str(), "quote");
        assert_eq!(descriptor.identity.physics, Physics::LatestState);
        assert_eq!(descriptor.delay_class, DelayClass::Realtime);
        let registry = load_ok(&[descriptor]);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.iter().count(), 1);
    }

    #[test]
    fn serde_round_trip_valid_descriptor() {
        let descriptor = fixture_quote_descriptor();
        let json = serde_json::to_string(&descriptor).unwrap();
        let back: Descriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(descriptor, back);
        load_ok(&[back]);
    }

    #[test]
    fn fail_closed_does_not_load_partial_registry() {
        let good = fixture_quote_descriptor();
        let mut bad = fixture_quote_descriptor();
        bad.identity.physics = Physics::OrderedState;
        let rejects = load_err(&[good, bad]);
        assert!(!rejects.is_empty());
    }

    #[test]
    fn rejects_market_funds_collision() {
        let mut descriptor = fixture_quote_descriptor();
        descriptor.identity.capability_id = CapabilityId::new("funds").unwrap();
        let rejects = load_err(&[descriptor]);
        assert!(
            rejects
                .iter()
                .any(|reject| matches!(reject, Reject::MarketAccountIdCollision { .. })),
            "expected MarketAccountIdCollision, got {rejects:?}"
        );
    }

    #[test]
    fn empty_limits_object_is_present_and_valid() {
        let descriptor = fixture_quote_descriptor();
        assert_eq!(descriptor.limits, Limits::default());
        load_ok(&[descriptor]);
    }
}

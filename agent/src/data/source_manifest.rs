//! Multi-capability source manifest. Claimed operations must match bindings.
//!
//! `binance_com.s1.v1` binds public quotes + instrument master + private account reads
//! plus S2 public klines `history` (`ohlcv` / `historical_series`).
//! `kotak_neo.s1k.v1` claims private quotes + REST depth snapshot + scrip master +
//! tradebook. Quotes succeed when TickBook has a `kotak_neo` tick (REST latest_state).
//! Depth succeeds when DepthBook has a complete bounded snapshot — never `synced`.
//! Kotak history is not implemented. Missing capabilities stay `unsupported`, not empty success.

use super::descriptor::{DelayClass, Limits};
use super::host_policy::AuthMode;
use super::identity::{Family, Physics};
use super::matrix::{known_id_physics_ok, pair_allowed};
use super::operations::{catalog_row, OperationKind};
use super::rights::Rights;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    LocalProjection,
    Stream,
    Rest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Coverage {
    #[serde(default)]
    pub venues: Vec<String>,
    #[serde(default)]
    pub asset_classes: Vec<String>,
    #[serde(default)]
    pub history_range: Option<String>,
    #[serde(default)]
    pub intervals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestBinding {
    pub operation: String,
    pub adapter_id: String,
    pub family: Family,
    pub capability_id: String,
    pub physics: Physics,
    pub auth_mode: AuthMode,
    pub transports: Vec<TransportKind>,
    pub rights: Rights,
    pub limits: Limits,
    pub coverage: Coverage,
    pub delay_class: DelayClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub manifest_id: String,
    pub adapter_id: String,
    /// Lock book id (locks/binance-com-spot.md, locks/kotak-nse-bse-cash.md; fetch 2026-08-22 IST).
    pub book_id: String,
    pub implemented: Vec<String>,
    pub bindings: Vec<ManifestBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestReject {
    ExecutionNounForbidden,
    UnknownOperation,
    IdentityMismatch,
    ManifestSupportWithoutBinding,
    BindingNotImplemented,
    AdapterMismatch,
    UnknownFamilyPhysicsPair,
    CapabilityPhysicsMismatch,
}

pub fn validate_manifest(manifest: &SourceManifest) -> Vec<ManifestReject> {
    let mut rejects = Vec::new();
    for noun in &manifest.implemented {
        let Some(row) = catalog_row(noun) else {
            rejects.push(ManifestReject::UnknownOperation);
            continue;
        };
        if row.kind == OperationKind::ExecutionForbidden {
            rejects.push(ManifestReject::ExecutionNounForbidden);
            continue;
        }
        if !manifest
            .bindings
            .iter()
            .any(|binding| binding.operation == *noun)
        {
            rejects.push(ManifestReject::ManifestSupportWithoutBinding);
        }
    }
    for binding in &manifest.bindings {
        if binding.adapter_id != manifest.adapter_id {
            rejects.push(ManifestReject::AdapterMismatch);
        }
        if !manifest
            .implemented
            .iter()
            .any(|noun| noun == &binding.operation)
        {
            rejects.push(ManifestReject::BindingNotImplemented);
            continue;
        }
        let Some(row) = catalog_row(&binding.operation) else {
            rejects.push(ManifestReject::UnknownOperation);
            continue;
        };
        if row.family != Some(binding.family)
            || row.capability_id != Some(binding.capability_id.as_str())
            || row.physics != Some(binding.physics)
        {
            rejects.push(ManifestReject::IdentityMismatch);
        }
        if !pair_allowed(binding.family, binding.physics) {
            rejects.push(ManifestReject::UnknownFamilyPhysicsPair);
        }
        if !known_id_physics_ok(binding.family, &binding.capability_id, binding.physics) {
            rejects.push(ManifestReject::CapabilityPhysicsMismatch);
        }
    }
    rejects
}

pub fn describe(manifest: &SourceManifest) -> Result<&SourceManifest, Vec<ManifestReject>> {
    let rejects = validate_manifest(manifest);
    if rejects.is_empty() {
        Ok(manifest)
    } else {
        Err(rejects)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObtainStatus {
    Success,
    Unsupported,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObtainEnvelope {
    pub adapter_id: String,
    /// Lock book id copied from the manifest. Empty when the target is unknown.
    pub book_id: String,
    pub operation: String,
    pub status: ObtainStatus,
    pub data: Option<serde_json::Value>,
    pub provenance_adapter_id: Option<String>,
}

/// Host-owned obtain. Implemented ops without a live snapshot stay `unavailable`.
/// Missing capabilities are `unsupported` with `data: null`.
pub fn obtain(manifest: &SourceManifest, operation: &str) -> ObtainEnvelope {
    if catalog_row(operation).is_some_and(|row| row.kind == OperationKind::ExecutionForbidden) {
        return ObtainEnvelope {
            adapter_id: manifest.adapter_id.clone(),
            book_id: manifest.book_id.clone(),
            operation: operation.to_string(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
        };
    }
    if !manifest.implemented.iter().any(|noun| noun == operation) {
        return ObtainEnvelope {
            adapter_id: manifest.adapter_id.clone(),
            book_id: manifest.book_id.clone(),
            operation: operation.to_string(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
        };
    }
    ObtainEnvelope {
        adapter_id: manifest.adapter_id.clone(),
        book_id: manifest.book_id.clone(),
        operation: operation.to_string(),
        status: ObtainStatus::Unavailable,
        data: None,
        provenance_adapter_id: Some(manifest.adapter_id.clone()),
    }
}

pub fn is_empty_success(envelope: &ObtainEnvelope) -> bool {
    envelope.status == ObtainStatus::Success && envelope.data.is_none()
}

fn account_binding(
    adapter_id: &str,
    operation: &str,
    capability_id: &str,
    coverage: Coverage,
    limits: Limits,
) -> ManifestBinding {
    ManifestBinding {
        operation: operation.to_string(),
        adapter_id: adapter_id.to_string(),
        family: Family::Account,
        capability_id: capability_id.to_string(),
        physics: Physics::BoundedSnapshot,
        auth_mode: AuthMode::PrivateRead,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits,
        coverage,
        delay_class: DelayClass::Unknown,
    }
}

fn quotes_binding(adapter_id: &str, coverage: Coverage, auth_mode: AuthMode) -> ManifestBinding {
    ManifestBinding {
        operation: "quotes".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "quote".into(),
        physics: Physics::LatestState,
        auth_mode,
        transports: vec![TransportKind::Stream, TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Realtime,
    }
}

fn instruments_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
) -> ManifestBinding {
    ManifestBinding {
        operation: "instruments".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Reference,
        capability_id: "instrument_master".into(),
        physics: Physics::VersionedSnapshot,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Unknown,
    }
}

fn history_binding(adapter_id: &str, coverage: Coverage) -> ManifestBinding {
    ManifestBinding {
        operation: "history".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "ohlcv".into(),
        physics: Physics::HistoricalSeries,
        auth_mode: AuthMode::Public,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Unknown,
    }
}

fn depth_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
    transports: Vec<TransportKind>,
) -> ManifestBinding {
    ManifestBinding {
        operation: "depth".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "order_book".into(),
        physics: Physics::BoundedSnapshot,
        auth_mode,
        transports,
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Realtime,
    }
}

/// Binance.com S1 + S2 history: public `@trade` quotes + exchangeInfo master
/// + private fills/funds + public klines `historical_series` + public `@depth`
/// bounded snapshot (not ordered_state).
pub fn binance_com_s1_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: Some("24h_per_symbol".into()),
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: None,
        intervals: super::binance_klines::KLINE_INTERVALS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    let fills_limits = Limits {
        requests_per_window: Some(6000),
        window_secs: Some(60),
    };
    SourceManifest {
        manifest_id: "binance_com.s1.v1".into(),
        adapter_id: "binance_com".into(),
        book_id: "binance-com-spot".into(),
        implemented: vec![
            "quotes".into(),
            "instruments".into(),
            "tradebook".into(),
            "funds".into(),
            "history".into(),
            "depth".into(),
        ],
        bindings: vec![
            quotes_binding("binance_com", coverage.clone(), AuthMode::Public),
            instruments_binding("binance_com", coverage.clone(), AuthMode::Public),
            account_binding(
                "binance_com",
                "tradebook",
                "fills",
                coverage.clone(),
                fills_limits.clone(),
            ),
            account_binding(
                "binance_com",
                "funds",
                "funds",
                coverage.clone(),
                fills_limits,
            ),
            history_binding("binance_com", history_coverage),
            depth_binding(
                "binance_com",
                coverage,
                AuthMode::Public,
                vec![TransportKind::Stream, TransportKind::Rest],
            ),
        ],
    }
}

/// Kotak S1k: quotes (REST latest_state) + REST depth bounded_snapshot + scrip master
/// + session tradebook. No history, no HSM `isDepth`, no optionchain.
pub fn kotak_neo_s1k_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["nse_cm".into(), "bse_cm".into()],
        asset_classes: vec!["equities".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "kotak_neo.s1k.v1".into(),
        adapter_id: "kotak_neo".into(),
        book_id: "kotak-nse-bse-cash".into(),
        implemented: vec![
            "quotes".into(),
            "instruments".into(),
            "tradebook".into(),
            "depth".into(),
        ],
        bindings: vec![
            quotes_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            instruments_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            account_binding(
                "kotak_neo",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            depth_binding(
                "kotak_neo",
                coverage,
                AuthMode::PrivateRead,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

pub fn first_party_s0_manifests() -> Vec<SourceManifest> {
    vec![binance_com_s1_manifest(), kotak_neo_s1k_manifest()]
}

pub fn manifest_for_slug(slug: &str) -> Option<SourceManifest> {
    first_party_s0_manifests()
        .into_iter()
        .find(|manifest| manifest.adapter_id == slug)
}

pub fn manifest_for_book_id(book_id: &str) -> Option<SourceManifest> {
    first_party_s0_manifests()
        .into_iter()
        .find(|manifest| manifest.book_id == book_id)
}

pub fn load_first_party_manifests() -> Result<Vec<SourceManifest>, Vec<ManifestReject>> {
    let manifests = first_party_s0_manifests();
    let mut rejects = Vec::new();
    for manifest in &manifests {
        rejects.extend(validate_manifest(manifest));
    }
    if rejects.is_empty() {
        Ok(manifests)
    } else {
        Err(rejects)
    }
}

pub fn shared_budget(manifest: &SourceManifest) -> u32 {
    manifest
        .bindings
        .iter()
        .filter_map(|binding| binding.limits.requests_per_window)
        .max()
        .unwrap_or(60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quotes_binding() -> ManifestBinding {
        ManifestBinding {
            operation: "quotes".into(),
            adapter_id: "binance_com".into(),
            family: Family::Market,
            capability_id: "quote".into(),
            physics: Physics::LatestState,
            auth_mode: AuthMode::Public,
            transports: vec![TransportKind::Stream],
            rights: Rights::research_fetch_only(),
            limits: Limits::default(),
            coverage: Coverage::default(),
            delay_class: DelayClass::Realtime,
        }
    }

    #[test]
    fn implemented_operations_need_matching_bindings() {
        let manifest = SourceManifest {
            manifest_id: "fixture.quotes".into(),
            adapter_id: "binance_com".into(),
            book_id: String::new(),
            implemented: vec!["quotes".into()],
            bindings: vec![quotes_binding()],
        };
        assert!(validate_manifest(&manifest).is_empty());
    }

    #[test]
    fn optionchain_cannot_bind_as_order_book() {
        let manifest = SourceManifest {
            manifest_id: "fixture.chain".into(),
            adapter_id: "binance_com".into(),
            book_id: String::new(),
            implemented: vec!["optionchain".into()],
            bindings: vec![ManifestBinding {
                operation: "optionchain".into(),
                adapter_id: "binance_com".into(),
                family: Family::Market,
                capability_id: "order_book".into(),
                physics: Physics::OrderedState,
                auth_mode: AuthMode::Public,
                transports: vec![TransportKind::Rest],
                rights: Rights::research_fetch_only(),
                limits: Limits::default(),
                coverage: Coverage::default(),
                delay_class: DelayClass::Unknown,
            }],
        };
        assert!(validate_manifest(&manifest).contains(&ManifestReject::IdentityMismatch));
    }

    #[test]
    fn execution_noun_is_rejected() {
        let manifest = SourceManifest {
            manifest_id: "fixture.exec".into(),
            adapter_id: "binance_com".into(),
            book_id: String::new(),
            implemented: vec!["placeorder".into()],
            bindings: vec![],
        };
        assert!(validate_manifest(&manifest).contains(&ManifestReject::ExecutionNounForbidden));
    }

    #[test]
    fn s1_binance_manifest_implements_quotes_and_account() {
        let manifest = binance_com_s1_manifest();
        assert!(describe(&manifest).is_ok());
        assert_eq!(manifest.manifest_id, "binance_com.s1.v1");
        assert_eq!(manifest.book_id, "binance-com-spot");
        assert_eq!(
            manifest.implemented,
            vec![
                "quotes",
                "instruments",
                "tradebook",
                "funds",
                "history",
                "depth"
            ]
        );
        assert!(!manifest.implemented.iter().any(|op| {
            matches!(
                op.as_str(),
                "optionchain" | "equity" | "fapi" | "premiumIndex" | "mark" | "stocks"
            )
        }));
        let history_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "history")
            .expect("history binding");
        assert_eq!(history_bind.family, Family::Market);
        assert_eq!(history_bind.capability_id, "ohlcv");
        assert_eq!(history_bind.physics, Physics::HistoricalSeries);
        assert_eq!(history_bind.auth_mode, AuthMode::Public);
        assert_eq!(history_bind.transports, vec![TransportKind::Rest]);
        assert!(history_bind.coverage.history_range.is_none());
        assert!(history_bind.coverage.intervals.contains(&"1m".into()));
        assert!(history_bind.coverage.intervals.contains(&"1M".into()));
        let depth_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("depth binding");
        assert_eq!(depth_bind.family, Family::Market);
        assert_eq!(depth_bind.capability_id, "order_book");
        assert_eq!(depth_bind.physics, Physics::BoundedSnapshot);
        assert_ne!(depth_bind.physics, Physics::OrderedState);
        assert_eq!(depth_bind.auth_mode, AuthMode::Public);
        assert_eq!(
            depth_bind.transports,
            vec![TransportKind::Stream, TransportKind::Rest]
        );
        let history = obtain(&manifest, "history");
        assert_eq!(history.status, ObtainStatus::Unavailable);
        assert_eq!(history.book_id, "binance-com-spot");
        assert!(history.data.is_none());
        assert!(!is_empty_success(&history));
        let quotes = obtain(&manifest, "quotes");
        assert_eq!(quotes.status, ObtainStatus::Unavailable);
        assert_eq!(quotes.book_id, "binance-com-spot");
        assert!(quotes.data.is_none());
        assert!(!is_empty_success(&quotes));
        assert_eq!(quotes.provenance_adapter_id.as_deref(), Some("binance_com"));
        let funds = obtain(&manifest, "funds");
        assert_eq!(funds.status, ObtainStatus::Unavailable);
        assert_eq!(funds.book_id, "binance-com-spot");
        assert_eq!(funds.provenance_adapter_id.as_deref(), Some("binance_com"));
        let depth = obtain(&manifest, "depth");
        assert_eq!(depth.status, ObtainStatus::Unavailable);
        assert!(depth.data.is_none());
        assert!(!is_empty_success(&depth));
        let place = obtain(&manifest, "placeorder");
        assert_eq!(place.status, ObtainStatus::Unsupported);
        assert!(place.data.is_none());
        assert_eq!(
            obtain(&manifest, "optionchain").status,
            ObtainStatus::Unsupported
        );
    }

    #[test]
    fn s1k_kotak_does_not_claim_history() {
        let manifest = kotak_neo_s1k_manifest();
        assert!(validate_manifest(&manifest).is_empty());
        assert_eq!(manifest.manifest_id, "kotak_neo.s1k.v1");
        assert_eq!(manifest.book_id, "kotak-nse-bse-cash");
        assert_eq!(
            manifest.implemented,
            vec!["quotes", "instruments", "tradebook", "depth"]
        );
        let quotes_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "quotes")
            .expect("quotes binding");
        assert_eq!(quotes_bind.auth_mode, AuthMode::PrivateRead);
        assert_eq!(quotes_bind.family, Family::Market);
        assert_eq!(quotes_bind.capability_id, "quote");
        assert_eq!(quotes_bind.physics, Physics::LatestState);
        assert_eq!(quotes_bind.coverage.venues, ["nse_cm", "bse_cm"]);
        let depth_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("depth binding");
        assert_eq!(depth_bind.auth_mode, AuthMode::PrivateRead);
        assert_eq!(depth_bind.family, Family::Market);
        assert_eq!(depth_bind.capability_id, "order_book");
        assert_eq!(depth_bind.physics, Physics::BoundedSnapshot);
        assert_eq!(depth_bind.transports, vec![TransportKind::Rest]);
        assert!(!manifest.implemented.iter().any(|op| op == "history"));
        assert!(!manifest.implemented.iter().any(|op| op == "depth_stream"));
        assert!(!manifest.implemented.iter().any(|op| op == "optionchain"));
        let instruments_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "instruments")
            .expect("instruments binding");
        assert_eq!(instruments_bind.auth_mode, AuthMode::PrivateRead);
        let history = obtain(&manifest, "history");
        assert_eq!(history.status, ObtainStatus::Unsupported);
        assert!(history.data.is_none());
        assert_ne!(history.status, ObtainStatus::Success);
        assert_eq!(
            obtain(&manifest, "optionchain").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&manifest, "depth_stream").status,
            ObtainStatus::Unsupported
        );
        let quotes = obtain(&manifest, "quotes");
        assert_eq!(quotes.status, ObtainStatus::Unavailable);
        assert!(quotes.data.is_none());
        assert_ne!(quotes.status, ObtainStatus::Success);
        assert!(!is_empty_success(&quotes));
        let depth = obtain(&manifest, "depth");
        assert_eq!(depth.status, ObtainStatus::Unavailable);
        assert!(depth.data.is_none());
        assert!(!is_empty_success(&depth));
        let instruments = obtain(&manifest, "instruments");
        assert_eq!(instruments.status, ObtainStatus::Unavailable);
        assert!(instruments.data.is_none());
        assert_eq!(
            obtain(&manifest, "tradebook").status,
            ObtainStatus::Unavailable
        );
    }

    #[test]
    fn first_party_manifests_load_fail_closed() {
        let loaded = load_first_party_manifests().expect("S0 first-party manifests must validate");
        assert_eq!(loaded.len(), 2);
        assert!(manifest_for_book_id("binance-com-usdm").is_none());
        assert!(manifest_for_book_id("binance-com-coinm").is_none());
        assert!(manifest_for_book_id("binance-com-options").is_none());
        assert!(manifest_for_book_id("binance-com-stocks").is_none());
        assert_eq!(
            manifest_for_slug("binance_com").unwrap().manifest_id,
            "binance_com.s1.v1"
        );
        assert_eq!(
            manifest_for_slug("binance_com").unwrap().book_id,
            "binance-com-spot"
        );
        assert_eq!(
            manifest_for_book_id("binance-com-spot").unwrap().adapter_id,
            "binance_com"
        );
        assert!(manifest_for_book_id("binance-com-usdm").is_none());
        assert_eq!(
            manifest_for_slug("kotak_neo").unwrap().manifest_id,
            "kotak_neo.s1k.v1"
        );
        assert_eq!(shared_budget(&loaded[0]), 6000);
        assert_eq!(shared_budget(&loaded[1]), 60);
    }

    #[test]
    fn unsupported_is_not_empty_success() {
        let envelope = ObtainEnvelope {
            adapter_id: "binance_com".into(),
            book_id: String::new(),
            operation: "quotes".into(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
        };
        assert!(!is_empty_success(&envelope));
        assert!(is_empty_success(&ObtainEnvelope {
            adapter_id: "binance_com".into(),
            book_id: "binance-com-spot".into(),
            operation: "funds".into(),
            status: ObtainStatus::Success,
            data: None,
            provenance_adapter_id: Some("binance_com".into()),
        }));
    }
}

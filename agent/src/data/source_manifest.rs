//! Multi-capability source manifest. Claimed operations must match bindings.
//!
//! `binance_com.s1.v1` binds public quotes + instrument master + private account reads
//! plus S2 public klines `history` (`ohlcv` / `historical_series`).
//! `kotak_neo.s1k.v1` claims private quotes + REST depth snapshot + scrip master +
//! tradebook + cash REST `history` (`ohlcv` / `historical_series`, v3.0.6
//! `GET /market-data/1.0/historical/details`). Empty candles stay `unavailable`.
//! NFO history stays a declared gap. Missing capabilities stay `unsupported`, not empty success.

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
    /// Upstream path behind a Success. `None` until an enricher names one —
    /// a Success may never claim a path the venue does not have.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_path: Option<String>,
    /// `labs` vs `desk`. Vendor history is labs unless a lock says desk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_use: Option<String>,
}

/// Host-owned obtain. Implemented ops without a live snapshot stay `unavailable`.
/// Missing capabilities are `unsupported` with `data: null`.
pub fn obtain(manifest: &SourceManifest, operation: &str) -> ObtainEnvelope {
    let product_use = (manifest.adapter_id == "amfi").then(|| "labs".to_string());
    if catalog_row(operation).is_some_and(|row| row.kind == OperationKind::ExecutionForbidden) {
        return ObtainEnvelope {
            adapter_id: manifest.adapter_id.clone(),
            book_id: manifest.book_id.clone(),
            operation: operation.to_string(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
            provenance_path: None,
            product_use,
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
            provenance_path: None,
            product_use,
        };
    }
    ObtainEnvelope {
        adapter_id: manifest.adapter_id.clone(),
        book_id: manifest.book_id.clone(),
        operation: operation.to_string(),
        status: ObtainStatus::Unavailable,
        data: None,
        provenance_adapter_id: Some(manifest.adapter_id.clone()),
        provenance_path: None,
        product_use,
    }
}

#[cfg(test)]
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

/// USER_DATA force-order is `market/force_order` lossy_event_observation only.
/// Never CompleteEventSequence. Display stays research-fetch — not the S3 allowlist.
fn forceorder_binding(adapter_id: &str, coverage: Coverage) -> ManifestBinding {
    ManifestBinding {
        operation: "forceorder".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "force_order".into(),
        physics: Physics::LossyEventObservation,
        auth_mode: AuthMode::PrivateRead,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
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

/// REST ticker only. USDM this slice has no fapi trade stream — do not claim Stream.
fn quotes_rest_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
) -> ManifestBinding {
    ManifestBinding {
        operation: "quotes".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "quote".into(),
        physics: Physics::LatestState,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Realtime,
    }
}

fn optionchain_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
) -> ManifestBinding {
    ManifestBinding {
        operation: "optionchain".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "option_chain".into(),
        physics: Physics::BoundedSnapshot,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::desk_display(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Realtime,
    }
}

/// Open interest is `market/open_interest` LatestState. Mirrors
/// [`optionchain_binding`] but stays a separate binding — OI never rides the
/// chain binding.
fn open_interest_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
) -> ManifestBinding {
    ManifestBinding {
        operation: "open_interest".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "open_interest".into(),
        physics: Physics::LatestState,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::desk_display(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Realtime,
    }
}

/// Venue-published option greeks — `derived/greeks` BoundedSnapshot off
/// `GET /eapi/v1/mark`. Clones [`open_interest_binding`]'s shape but is its own
/// binding: greeks never ride the OI binding.
///
/// Display grant: Station copies licensed venue figures. S3 also grants
/// `desk_display` on `depth` (all four shipping books) and on `optionchain` /
/// `open_interest` (NFO + options). Do not copy this onto quotes / history /
/// tradebook / funds.
fn optiongreeks_binding(
    adapter_id: &str,
    coverage: Coverage,
    auth_mode: AuthMode,
) -> ManifestBinding {
    ManifestBinding {
        operation: "optiongreeks".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Derived,
        capability_id: "greeks".into(),
        physics: Physics::BoundedSnapshot,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::desk_display(),
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

fn search_binding(adapter_id: &str, coverage: Coverage, auth_mode: AuthMode) -> ManifestBinding {
    ManifestBinding {
        operation: "search".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Reference,
        capability_id: "instrument_search".into(),
        physics: Physics::BoundedSnapshot,
        auth_mode,
        transports: vec![TransportKind::Rest],
        rights: Rights::research_fetch_only(),
        limits: Limits::default(),
        coverage,
        delay_class: DelayClass::Unknown,
    }
}

fn history_binding(adapter_id: &str, coverage: Coverage, auth_mode: AuthMode) -> ManifestBinding {
    ManifestBinding {
        operation: "history".into(),
        adapter_id: adapter_id.to_string(),
        family: Family::Market,
        capability_id: "ohlcv".into(),
        physics: Physics::HistoricalSeries,
        auth_mode,
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
        rights: Rights::desk_display(),
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
            "search".into(),
            "tradebook".into(),
            "funds".into(),
            "orderbook".into(),
            "history".into(),
            "depth".into(),
        ],
        bindings: vec![
            quotes_binding("binance_com", coverage.clone(), AuthMode::Public),
            instruments_binding("binance_com", coverage.clone(), AuthMode::Public),
            search_binding("binance_com", coverage.clone(), AuthMode::Public),
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
                fills_limits.clone(),
            ),
            account_binding(
                "binance_com",
                "orderbook",
                "orders",
                coverage.clone(),
                fills_limits,
            ),
            history_binding("binance_com", history_coverage, AuthMode::Public),
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
/// + session tradebook + cash REST history (`GET /market-data/1.0/historical/details`).
/// No HSM `isDepth`, no optionchain. NFO history stays a declared gap.
pub fn kotak_neo_s1k_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["nse_cm".into(), "bse_cm".into()],
        asset_classes: vec!["equities".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["nse_cm".into(), "bse_cm".into()],
        asset_classes: vec!["equities".into()],
        history_range: None,
        intervals: super::kotak_historical::ALLOWED_INTERVALS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    SourceManifest {
        manifest_id: "kotak_neo.s1k.v1".into(),
        adapter_id: "kotak_neo".into(),
        book_id: "kotak-nse-bse-cash".into(),
        implemented: vec![
            "quotes".into(),
            "instruments".into(),
            "search".into(),
            "tradebook".into(),
            "depth".into(),
            "orderbook".into(),
            "history".into(),
            "holdings".into(),
            "positionbook".into(),
            "funds".into(),
        ],
        bindings: vec![
            quotes_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            instruments_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            search_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            account_binding(
                "kotak_neo",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "orderbook",
                "orders",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "holdings",
                "holdings",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "positionbook",
                "positions",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "funds",
                "funds",
                coverage.clone(),
                Limits::default(),
            ),
            history_binding("kotak_neo", history_coverage, AuthMode::PrivateRead),
            depth_binding(
                "kotak_neo",
                coverage,
                AuthMode::PrivateRead,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

/// Named NFO book on the same `kotak_neo` adapter. Quotes + FO scrip master
/// (`docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md`, header 2026-08-28).
/// Slice 3: `optionchain` = master rows for (underlying, expiry) + optional last.
/// Catalog / Start slug still ships cash.
pub fn kotak_neo_nfo_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["nse_fo".into()],
        asset_classes: vec!["nfo".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "kotak_neo.nfo.v1".into(),
        adapter_id: "kotak_neo".into(),
        book_id: "kotak-nse-nfo".into(),
        implemented: vec![
            "quotes".into(),
            "instruments".into(),
            "search".into(),
            "tradebook".into(),
            "optionchain".into(),
            "open_interest".into(),
            "depth".into(),
            "orderbook".into(),
            "positionbook".into(),
            "funds".into(),
        ],
        bindings: vec![
            quotes_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            instruments_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            search_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            account_binding(
                "kotak_neo",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "orderbook",
                "orders",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "positionbook",
                "positions",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "kotak_neo",
                "funds",
                "funds",
                coverage.clone(),
                Limits::default(),
            ),
            optionchain_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            // LatestState from `open_int` on the `quote_type=all` body Station
            // already fetches for last. Observed 2026-08-31 — see the lock's OI
            // row. Not the `oi` slice, not master `dOpenInterest `.
            open_interest_binding("kotak_neo", coverage.clone(), AuthMode::PrivateRead),
            // REST bounded snapshot of `quote_type=depth` — observed 2026-09-01 on
            // `nse_fo|56526`. REST only; no HSM `isDepth`.
            depth_binding(
                "kotak_neo",
                coverage,
                AuthMode::PrivateRead,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

/// Named options book on the same `binance_com` adapter. Public last + chain/OI
/// + mark greeks + REST depth + eapi klines `history` (`ohlcv` / `historical_series`).
/// Lock: `locks/binance-com-options.md`. Catalog / Start slug still ships spot.
pub fn binance_com_options_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_options".into()],
        history_range: None,
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_options".into()],
        history_range: None,
        intervals: super::binance_options_klines::OPTIONS_KLINE_INTERVALS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    SourceManifest {
        manifest_id: "binance_com.options.v1".into(),
        adapter_id: "binance_com".into(),
        book_id: "binance-com-options".into(),
        implemented: vec![
            "quotes".into(),
            "optionchain".into(),
            "open_interest".into(),
            "optiongreeks".into(),
            "depth".into(),
            "history".into(),
            "tradebook".into(),
            "search".into(),
            "funds".into(),
            "positionbook".into(),
        ],
        bindings: vec![
            quotes_binding("binance_com", coverage.clone(), AuthMode::Public),
            optionchain_binding("binance_com", coverage.clone(), AuthMode::Public),
            open_interest_binding("binance_com", coverage.clone(), AuthMode::Public),
            optiongreeks_binding("binance_com", coverage.clone(), AuthMode::Public),
            search_binding("binance_com", coverage.clone(), AuthMode::Public),
            account_binding(
                "binance_com",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "binance_com",
                "funds",
                "funds",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "binance_com",
                "positionbook",
                "positions",
                coverage.clone(),
                Limits::default(),
            ),
            history_binding("binance_com", history_coverage, AuthMode::Public),
            // REST bounded snapshot of `GET /eapi/v1/depth` — `Rest` only. Spot's
            // `@depth` reconstruction loop is another book on another host, and
            // this binding must never claim `Stream`.
            depth_binding(
                "binance_com",
                coverage,
                AuthMode::Public,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

/// Named USDM book on the same `binance_com` adapter. USER_DATA funds +
/// positions + lossy force-order + public REST last + public fapi klines +
/// public fapi REST depth this slice — no tradebook, no fapi trade stream,
/// no `@depth`. Lock: `locks/binance-com-usdm.md`.
/// Catalog / Start slug still ships spot.
pub fn binance_com_usdm_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_usdm".into()],
        history_range: None,
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_usdm".into()],
        history_range: None,
        intervals: super::binance_usdm_klines::USDM_KLINE_INTERVALS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    SourceManifest {
        manifest_id: "binance_com.usdm.v1".into(),
        adapter_id: "binance_com".into(),
        book_id: "binance-com-usdm".into(),
        implemented: vec![
            "funds".into(),
            "positionbook".into(),
            "forceorder".into(),
            "quotes".into(),
            "search".into(),
            "history".into(),
            "depth".into(),
        ],
        bindings: vec![
            account_binding(
                "binance_com",
                "funds",
                "funds",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "binance_com",
                "positionbook",
                "positions",
                coverage.clone(),
                Limits::default(),
            ),
            forceorder_binding("binance_com", coverage.clone()),
            quotes_rest_binding("binance_com", coverage.clone(), AuthMode::Public),
            search_binding("binance_com", coverage.clone(), AuthMode::Public),
            history_binding("binance_com", history_coverage, AuthMode::Public),
            // REST bounded snapshot of `GET /fapi/v1/depth` — `Rest` only. Spot
            // `@depth` is another book. Coin-M is another owner file.
            depth_binding(
                "binance_com",
                coverage,
                AuthMode::Public,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

/// Named Coin-M book on the same `binance_com` adapter. Third identity vs
/// USDM: matching is `book_id`. Lock: `locks/binance-com-coinm.md`.
pub fn binance_com_coinm_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_coinm".into()],
        history_range: None,
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["binance.com".into()],
        asset_classes: vec!["crypto_coinm".into()],
        history_range: None,
        intervals: super::binance_coinm_klines::COINM_KLINE_INTERVALS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    SourceManifest {
        manifest_id: "binance_com.coinm.v1".into(),
        adapter_id: "binance_com".into(),
        book_id: "binance-com-coinm".into(),
        implemented: vec![
            "funds".into(),
            "positionbook".into(),
            "forceorder".into(),
            "quotes".into(),
            "search".into(),
            "history".into(),
            "depth".into(),
        ],
        bindings: vec![
            account_binding(
                "binance_com",
                "funds",
                "funds",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "binance_com",
                "positionbook",
                "positions",
                coverage.clone(),
                Limits::default(),
            ),
            forceorder_binding("binance_com", coverage.clone()),
            quotes_rest_binding("binance_com", coverage.clone(), AuthMode::Public),
            search_binding("binance_com", coverage.clone(), AuthMode::Public),
            history_binding("binance_com", history_coverage, AuthMode::Public),
            // REST bounded snapshot of `GET /dapi/v1/depth` — own owner, Rest only.
            depth_binding(
                "binance_com",
                coverage,
                AuthMode::Public,
                vec![TransportKind::Rest],
            ),
        ],
    }
}

/// Zerodha Kite cash CNC+MIS (B6 SIGNED). Day-book fills via Wasm `GET /trades`.
pub fn zerodha_kite_cash_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["NSE".into(), "BSE".into()],
        asset_classes: vec!["equity".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "tradeautopsy:zerodha-kite-cash@0.1.0".into(),
        adapter_id: "zerodha_kite".into(),
        book_id: "zerodha-nse-bse-cash".into(),
        implemented: vec!["tradebook".into(), "orderbook".into()],
        bindings: vec![
            account_binding(
                "zerodha_kite",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "zerodha_kite",
                "orderbook",
                "orders",
                coverage,
                Limits::default(),
            ),
        ],
    }
}

pub fn first_party_s0_manifests() -> Vec<SourceManifest> {
    // Spot then options then USDM then Coin-M so `manifest_for_slug("binance_com")`
    // stays spot (first match). Cash stays before NFO so slug stays cash.
    vec![
        binance_com_s1_manifest(),
        binance_com_options_manifest(),
        binance_com_usdm_manifest(),
        binance_com_coinm_manifest(),
        kotak_neo_s1k_manifest(),
        kotak_neo_nfo_manifest(),
        zerodha_kite_cash_manifest(),
        upstox_cash_manifest(),
        fyers_cash_manifest(),
        bybit_com_spot_manifest(),
        okx_com_spot_manifest(),
        kraken_com_spot_manifest(),
        coinbase_advanced_spot_manifest(),
        super::amfi::amfi_nav_manifest(),
    ]
}

/// Upstox NSE/BSE cash D+I (B6 SIGNED). Day-book fills via Wasm trades-for-day.
/// Fyers NSE/BSE cash D+I (B6 SIGNED). Day-book fills via Wasm tradebook (P3-W0 stub).
pub fn fyers_cash_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["NSE".into(), "BSE".into()],
        asset_classes: vec!["equity".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "tradeautopsy:fyers-cash@0.1.0".into(),
        adapter_id: "fyers".into(),
        book_id: "fyers-nse-bse-cash".into(),
        implemented: vec!["tradebook".into(), "orderbook".into()],
        bindings: vec![
            account_binding(
                "fyers",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "fyers",
                "orderbook",
                "orders",
                coverage,
                Limits::default(),
            ),
        ],
    }
}

/// Bybit v5 spot (B6 SIGNED). Fills + public klines `history`.
pub fn bybit_com_spot_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["bybit.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    let history_coverage = Coverage {
        venues: vec!["bybit.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: None,
        intervals: [
            "1", "3", "5", "15", "30", "60", "120", "240", "360", "720", "D", "W", "M",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect(),
    };
    SourceManifest {
        manifest_id: "bybit.spot.v1".into(),
        adapter_id: "bybit".into(),
        book_id: "bybit-com-spot".into(),
        implemented: vec!["tradebook".into(), "history".into()],
        bindings: vec![
            account_binding(
                "bybit",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            history_binding("bybit", history_coverage, AuthMode::Public),
        ],
    }
}

/// OKX global SPOT (B6 SIGNED). Recent fills poll only v1.
pub fn okx_com_spot_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["okx.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "okx_com.s1.v1".into(),
        adapter_id: "okx_com".into(),
        book_id: "okx-com-spot".into(),
        implemented: vec!["tradebook".into()],
        bindings: vec![account_binding(
            "okx_com",
            "tradebook",
            "fills",
            coverage,
            Limits::default(),
        )],
    }
}

/// Kraken spot REST (B6 SIGNED). TradesHistory fills poll v1.
pub fn kraken_com_spot_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["kraken.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "kraken.spot.v1".into(),
        adapter_id: "kraken".into(),
        book_id: "kraken-com-spot".into(),
        implemented: vec!["tradebook".into()],
        bindings: vec![account_binding(
            "kraken",
            "tradebook",
            "fills",
            coverage,
            Limits::default(),
        )],
    }
}

/// Coinbase Advanced Trade spot (B6 SIGNED). List-fills poll v1.
pub fn coinbase_advanced_spot_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["coinbase.com".into()],
        asset_classes: vec!["crypto_spot".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "tradeautopsy:coinbase-advanced-spot@0.1.0".into(),
        adapter_id: "coinbase_advanced".into(),
        book_id: "coinbase-advanced-spot".into(),
        implemented: vec!["tradebook".into()],
        bindings: vec![account_binding(
            "coinbase_advanced",
            "tradebook",
            "fills",
            coverage,
            Limits::default(),
        )],
    }
}

pub fn upstox_cash_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec!["NSE".into(), "BSE".into()],
        asset_classes: vec!["equity".into()],
        history_range: Some("session".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "tradeautopsy:upstox-cash@0.1.0".into(),
        adapter_id: "upstox".into(),
        book_id: "upstox-nse-bse-cash".into(),
        implemented: vec!["tradebook".into(), "orderbook".into()],
        bindings: vec![
            account_binding(
                "upstox",
                "tradebook",
                "fills",
                coverage.clone(),
                Limits::default(),
            ),
            account_binding(
                "upstox",
                "orderbook",
                "orders",
                coverage,
                Limits::default(),
            ),
        ],
    }
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

/// Current shipping book for this Start/Keychain slug. Not a class picker.
/// Fixtures and unknown slugs → None (callers use their own id as the key prefix).
pub fn shipping_book_id_for_slug(slug: &str) -> Option<String> {
    manifest_for_slug(slug).map(|m| m.book_id)
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
    fn forceorder_complete_binding_refused() {
        let manifest = SourceManifest {
            manifest_id: "fixture.forceorder".into(),
            adapter_id: "binance_com".into(),
            book_id: String::new(),
            implemented: vec!["forceorder".into()],
            bindings: vec![ManifestBinding {
                operation: "forceorder".into(),
                adapter_id: "binance_com".into(),
                family: Family::Market,
                capability_id: "force_order".into(),
                physics: Physics::CompleteEventSequence,
                auth_mode: AuthMode::Public,
                transports: vec![TransportKind::Rest],
                rights: Rights::research_fetch_only(),
                limits: Limits::default(),
                coverage: Coverage::default(),
                delay_class: DelayClass::Unknown,
            }],
        };
        let rejects = validate_manifest(&manifest);
        assert!(
            rejects.contains(&ManifestReject::IdentityMismatch)
                || rejects.contains(&ManifestReject::CapabilityPhysicsMismatch),
            "rejects={rejects:?}"
        );
        assert!(describe(&manifest).is_err());
    }

    #[test]
    fn shipping_obtain_forceorder_unsupported() {
        assert_eq!(
            obtain(&binance_com_s1_manifest(), "forceorder").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&kotak_neo_s1k_manifest(), "forceorder").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&binance_com_options_manifest(), "forceorder").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&kotak_neo_nfo_manifest(), "forceorder").status,
            ObtainStatus::Unsupported
        );
        let usdm_fo = obtain(&binance_com_usdm_manifest(), "forceorder");
        assert_eq!(usdm_fo.status, ObtainStatus::Unavailable);
        assert_ne!(usdm_fo.status, ObtainStatus::Unsupported);
        assert_ne!(usdm_fo.status, ObtainStatus::Success);
        assert!(usdm_fo.data.is_none());
        assert!(!is_empty_success(&usdm_fo));
        let coinm_fo = obtain(&binance_com_coinm_manifest(), "forceorder");
        assert_eq!(coinm_fo.status, ObtainStatus::Unavailable);
        assert_ne!(coinm_fo.status, ObtainStatus::Unsupported);
        assert_ne!(coinm_fo.status, ObtainStatus::Success);
        assert!(coinm_fo.data.is_none());
        assert!(!is_empty_success(&coinm_fo));
    }

    #[test]
    fn spot_and_cash_tradebook_fills_are_bounded_snapshot() {
        for manifest in [
            binance_com_s1_manifest(),
            binance_com_options_manifest(),
            kotak_neo_s1k_manifest(),
            kotak_neo_nfo_manifest(),
        ] {
            let tradebook = manifest
                .bindings
                .iter()
                .find(|binding| binding.operation == "tradebook")
                .expect("tradebook binding");
            assert_eq!(tradebook.capability_id, "fills");
            assert_eq!(tradebook.physics, Physics::BoundedSnapshot);
            assert_ne!(tradebook.physics, Physics::CompleteEventSequence);
        }
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
                "search",
                "tradebook",
                "funds",
                "orderbook",
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
        let orderbook = obtain(&manifest, "orderbook");
        assert_eq!(orderbook.status, ObtainStatus::Unavailable);
        assert_eq!(orderbook.book_id, "binance-com-spot");
        assert_eq!(
            orderbook.provenance_adapter_id.as_deref(),
            Some("binance_com")
        );
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
    fn s1k_kotak_claims_cash_history() {
        let manifest = kotak_neo_s1k_manifest();
        assert!(validate_manifest(&manifest).is_empty());
        assert_eq!(manifest.manifest_id, "kotak_neo.s1k.v1");
        assert_eq!(manifest.book_id, "kotak-nse-bse-cash");
        assert_eq!(
            manifest.implemented,
            vec![
                "quotes",
                "instruments",
                "search",
                "tradebook",
                "depth",
                "orderbook",
                "history",
                "holdings",
                "positionbook",
                "funds",
            ]
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
        assert!(manifest.implemented.iter().any(|op| op == "history"));
        assert!(!manifest.implemented.iter().any(|op| op == "depth_stream"));
        assert!(!manifest.implemented.iter().any(|op| op == "optionchain"));
        let instruments_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "instruments")
            .expect("instruments binding");
        assert_eq!(instruments_bind.auth_mode, AuthMode::PrivateRead);
        let history_bind = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "history")
            .expect("history binding");
        assert_eq!(history_bind.auth_mode, AuthMode::PrivateRead);
        assert_eq!(history_bind.capability_id, "ohlcv");
        assert_eq!(history_bind.physics, Physics::HistoricalSeries);
        assert!(history_bind.coverage.venues.contains(&"nse_cm".into()));
        assert!(history_bind.coverage.intervals.contains(&"15min".into()));
        let history = obtain(&manifest, "history");
        assert_eq!(history.status, ObtainStatus::Unavailable);
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

    /// S3 desk-display allowlist. optiongreeks stays; depth is granted on every
    /// shipping book that binds it; chain/OI only on NFO + options.
    #[test]
    fn shipping_display_grant_is_an_allowlist() {
        let options = binance_com_options_manifest();
        let greeks = options
            .bindings
            .iter()
            .find(|b| b.operation == "optiongreeks")
            .expect("options book binds optiongreeks");
        assert_eq!(greeks.capability_id, "greeks");
        assert_eq!(greeks.family, Family::Derived);
        assert_eq!(greeks.physics, Physics::BoundedSnapshot);
        assert_eq!(greeks.auth_mode, AuthMode::Public);
        assert_eq!(greeks.transports, vec![TransportKind::Rest]);
        assert_eq!(greeks.rights, Rights::desk_display());
        assert!(greeks.rights.display);

        fn may_display(book_id: &str, operation: &str) -> bool {
            matches!(operation, "optiongreeks" | "depth")
                || (matches!(operation, "optionchain" | "open_interest")
                    && matches!(book_id, "binance-com-options" | "kotak-nse-nfo"))
        }

        for manifest in first_party_s0_manifests() {
            for binding in &manifest.bindings {
                let allowed = may_display(&manifest.book_id, &binding.operation);
                assert_eq!(
                    binding.rights.display, allowed,
                    "{} / {} display grant mismatch",
                    manifest.book_id, binding.operation
                );
                if allowed {
                    assert_eq!(binding.rights, Rights::desk_display());
                } else {
                    assert_eq!(binding.rights, Rights::research_fetch_only());
                }
            }
        }
    }

    #[test]
    fn optiongreeks_ships_only_on_the_binance_options_book() {
        // Implemented with no planted snapshot is Unavailable, never Unsupported.
        assert_eq!(
            obtain(&binance_com_options_manifest(), "optiongreeks").status,
            ObtainStatus::Unavailable
        );
        // No other book gained a greeks noun.
        for manifest in [
            kotak_neo_nfo_manifest(),
            binance_com_s1_manifest(),
            kotak_neo_s1k_manifest(),
        ] {
            assert_eq!(
                obtain(&manifest, "optiongreeks").status,
                ObtainStatus::Unsupported,
                "{}",
                manifest.book_id
            );
        }
        // The multi-leg noun is still nobody's.
        assert_eq!(
            obtain(&binance_com_options_manifest(), "multioptiongreeks").status,
            ObtainStatus::Unsupported
        );
    }

    #[test]
    fn first_party_manifests_load_fail_closed() {
        let loaded = load_first_party_manifests().expect("S0 first-party manifests must validate");
        assert!(loaded.len() >= 6);
        assert!(manifest_for_book_id("binance-com-usdm").is_some());
        assert!(manifest_for_book_id("binance-com-coinm").is_some());
        let options = manifest_for_book_id("binance-com-options").expect("options book");
        assert_eq!(options.manifest_id, "binance_com.options.v1");
        assert_eq!(options.adapter_id, "binance_com");
        assert_eq!(options.book_id, "binance-com-options");
        assert_eq!(
            options.implemented,
            vec![
                "quotes",
                "optionchain",
                "open_interest",
                "optiongreeks",
                "depth",
                "history",
                "tradebook",
                "search",
                "funds",
                "positionbook",
            ]
        );
        assert!(options.implemented.iter().any(|op| op == "optionchain"));
        assert!(options.implemented.iter().any(|op| op == "tradebook"));
        assert!(options.implemented.iter().any(|op| op == "open_interest"));
        assert!(options.implemented.iter().any(|op| op == "optiongreeks"));
        // Implemented with no planted snapshot is Unavailable, never Unsupported.
        assert_eq!(
            obtain(&options, "optionchain").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            obtain(&options, "open_interest").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            obtain(&options, "optiongreeks").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(obtain(&options, "funds").status, ObtainStatus::Unavailable);
        assert_ne!(obtain(&options, "funds").status, ObtainStatus::Unsupported);
        assert_eq!(
            obtain(&options, "positionbook").status,
            ObtainStatus::Unavailable
        );
        assert_ne!(
            obtain(&options, "positionbook").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&options, "forceorder").status,
            ObtainStatus::Unsupported
        );
        // Depth with an empty DepthBook is Unavailable — never Unsupported, and
        // never an empty `{bids:[],asks:[]}` Success.
        assert_eq!(obtain(&options, "depth").status, ObtainStatus::Unavailable);
        assert_ne!(obtain(&options, "depth").status, ObtainStatus::Unsupported);
        assert!(obtain(&options, "depth").data.is_none());
        assert_eq!(
            obtain(&options, "history").status,
            ObtainStatus::Unavailable
        );
        assert_ne!(
            obtain(&options, "history").status,
            ObtainStatus::Unsupported
        );
        assert!(obtain(&options, "history").data.is_none());
        // NFO depth with an empty DepthBook is Unavailable — never Unsupported.
        let nfo = manifest_for_book_id("kotak-nse-nfo").expect("nfo book");
        assert_eq!(obtain(&nfo, "depth").status, ObtainStatus::Unavailable);
        assert_ne!(obtain(&nfo, "depth").status, ObtainStatus::Unsupported);
        assert!(obtain(&nfo, "depth").data.is_none());
        let depth_bind = options
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("options depth binding");
        assert_eq!(depth_bind.capability_id, "order_book");
        assert_eq!(depth_bind.family, Family::Market);
        assert_eq!(depth_bind.physics, Physics::BoundedSnapshot);
        assert_ne!(depth_bind.physics, Physics::OrderedState);
        assert_eq!(depth_bind.auth_mode, AuthMode::Public);
        // REST snapshot only. A `Stream` transport here would claim spot's
        // reconstruction loop on a host that never serves it.
        assert_eq!(depth_bind.transports, vec![TransportKind::Rest]);
        assert!(!depth_bind.transports.contains(&TransportKind::Stream));
        assert_eq!(depth_bind.rights, Rights::desk_display());
        assert_ne!(depth_bind.rights, Rights::research_fetch_only());
        assert_eq!(depth_bind.coverage.asset_classes, vec!["crypto_options"]);
        let history_bind = options
            .bindings
            .iter()
            .find(|binding| binding.operation == "history")
            .expect("options history binding");
        assert_eq!(history_bind.capability_id, "ohlcv");
        assert_eq!(history_bind.family, Family::Market);
        assert_eq!(history_bind.physics, Physics::HistoricalSeries);
        assert_eq!(history_bind.auth_mode, AuthMode::Public);
        assert_eq!(history_bind.transports, vec![TransportKind::Rest]);
        assert!(history_bind.coverage.intervals.contains(&"1m".into()));
        assert!(history_bind.coverage.intervals.contains(&"8h".into()));
        assert!(!history_bind.coverage.intervals.contains(&"1s".into()));
        let chain_bind = options
            .bindings
            .iter()
            .find(|binding| binding.operation == "optionchain")
            .expect("options optionchain binding");
        assert_eq!(chain_bind.capability_id, "option_chain");
        assert_eq!(chain_bind.physics, Physics::BoundedSnapshot);
        assert_eq!(chain_bind.auth_mode, AuthMode::Public);
        // OI is its own binding; it never rides the chain binding.
        let oi_bind = options
            .bindings
            .iter()
            .find(|binding| binding.operation == "open_interest")
            .expect("options open_interest binding");
        assert_eq!(oi_bind.capability_id, "open_interest");
        assert_eq!(oi_bind.physics, Physics::LatestState);
        assert_eq!(oi_bind.auth_mode, AuthMode::Public);
        // Spot and cash never grow a chain noun from the options book.
        let spot = manifest_for_book_id("binance-com-spot").expect("spot book");
        assert_eq!(
            obtain(&spot, "optionchain").status,
            ObtainStatus::Unsupported
        );
        assert_eq!(
            obtain(&spot, "open_interest").status,
            ObtainStatus::Unsupported
        );
        let cash = manifest_for_book_id("kotak-nse-bse-cash").expect("cash book");
        assert_eq!(
            obtain(&cash, "optionchain").status,
            ObtainStatus::Unsupported
        );
        let quotes_bind = options
            .bindings
            .iter()
            .find(|binding| binding.operation == "quotes")
            .expect("options quotes binding");
        assert_eq!(quotes_bind.auth_mode, AuthMode::Public);
        assert_eq!(quotes_bind.capability_id, "quote");
        assert_eq!(shared_budget(&options), 60);
        assert!(manifest_for_book_id("binance-com-stocks").is_none());
        assert!(manifest_for_book_id("kotak-nse-nfo").is_some());
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
        assert!(manifest_for_book_id("binance-com-usdm").is_some());
        assert_eq!(
            manifest_for_slug("kotak_neo").unwrap().manifest_id,
            "kotak_neo.s1k.v1"
        );
        assert_eq!(
            manifest_for_slug("kotak_neo").unwrap().book_id,
            "kotak-nse-bse-cash"
        );
        let nfo = manifest_for_book_id("kotak-nse-nfo").unwrap();
        assert_eq!(nfo.manifest_id, "kotak_neo.nfo.v1");
        assert_eq!(nfo.adapter_id, "kotak_neo");
        assert_eq!(
            nfo.implemented,
            vec![
                "quotes",
                "instruments",
                "search",
                "tradebook",
                "optionchain",
                "open_interest",
                "depth",
                "orderbook",
                "positionbook",
                "funds",
            ]
        );
        assert!(nfo.implemented.iter().any(|op| op == "optionchain"));
        assert!(nfo.implemented.iter().any(|op| op == "instruments"));
        // OI on NFO is LatestState from `open_int` (observed 2026-08-31), on the
        // quotes GET Station already makes. Empty slot → Unavailable, never
        // Unsupported and never `{open_interest: 0}` Success.
        assert_eq!(
            obtain(&nfo, "open_interest").status,
            ObtainStatus::Unavailable
        );
        assert_ne!(
            obtain(&nfo, "open_interest").status,
            ObtainStatus::Unsupported
        );
        assert!(obtain(&nfo, "open_interest").data.is_none());
        let nfo_oi = nfo
            .bindings
            .iter()
            .find(|binding| binding.operation == "open_interest")
            .expect("nfo open_interest binding");
        assert_eq!(nfo_oi.capability_id, "open_interest");
        assert_eq!(nfo_oi.physics, Physics::LatestState);
        assert_ne!(nfo_oi.physics, Physics::BoundedSnapshot);
        // Kotak quotes are session-attached, never public.
        assert_eq!(nfo_oi.auth_mode, AuthMode::PrivateRead);
        assert_eq!(nfo_oi.rights, Rights::desk_display());
        assert_ne!(nfo_oi.rights, Rights::research_fetch_only());
        // NFO depth is claimed on the named book — REST bounded snapshot only.
        assert!(nfo.implemented.iter().any(|op| op == "depth"));
        assert_eq!(obtain(&nfo, "depth").status, ObtainStatus::Unavailable);
        assert_ne!(obtain(&nfo, "depth").status, ObtainStatus::Unsupported);
        let nfo_depth = nfo
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("nfo depth binding");
        assert_eq!(nfo_depth.capability_id, "order_book");
        assert_eq!(nfo_depth.physics, Physics::BoundedSnapshot);
        assert_eq!(nfo_depth.auth_mode, AuthMode::PrivateRead);
        assert_eq!(nfo_depth.transports, vec![TransportKind::Rest]);
        assert_eq!(nfo_depth.rights, Rights::desk_display());
        assert_eq!(
            obtain(&nfo, "optionchain").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            obtain(&nfo, "instruments").status,
            ObtainStatus::Unavailable
        );
        // Claimed tradebook with empty store → Unavailable, never Unsupported.
        assert_eq!(obtain(&nfo, "tradebook").status, ObtainStatus::Unavailable);
        assert_ne!(obtain(&nfo, "tradebook").status, ObtainStatus::Unsupported);
        assert!(obtain(&nfo, "tradebook").data.is_none());
        let spot = manifest_for_book_id("binance-com-spot").unwrap();
        let cash = manifest_for_book_id("kotak-nse-bse-cash").unwrap();
        assert_eq!(shared_budget(&spot), 6000);
        assert_eq!(shared_budget(&cash), 60);
        assert_ne!(shared_budget(&spot), shared_budget(&options));
    }

    #[test]
    fn shipping_book_id_for_slug_maps_start_slugs_to_shipping_books_only() {
        assert_eq!(
            shipping_book_id_for_slug("binance_com").as_deref(),
            Some("binance-com-spot")
        );
        assert_eq!(
            shipping_book_id_for_slug("kotak_neo").as_deref(),
            Some("kotak-nse-bse-cash")
        );
        let binance_book = shipping_book_id_for_slug("binance_com").expect("shipping book");
        assert_eq!(binance_book, "binance-com-spot");
        assert_ne!(binance_book, "binance-com-options");
        assert_ne!(binance_book, "binance-com-usdm");
        assert_ne!(binance_book, "binance-com-coinm");
        assert_ne!(binance_book, "binance-com-stocks");
        assert_ne!(
            shipping_book_id_for_slug("kotak_neo").as_deref(),
            Some("kotak-nse-nfo")
        );
        assert!(shipping_book_id_for_slug("fixture_equity_quote").is_none());
        assert!(shipping_book_id_for_slug("binance-com-usdm").is_none());
        assert!(shipping_book_id_for_slug("kotak-nse-nfo").is_none());
        assert_eq!(
            shipping_book_id_for_slug("zerodha_kite").as_deref(),
            Some("zerodha-nse-bse-cash")
        );
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
            provenance_path: None,
            product_use: None,
        };
        assert!(!is_empty_success(&envelope));
        assert!(is_empty_success(&ObtainEnvelope {
            adapter_id: "binance_com".into(),
            book_id: "binance-com-spot".into(),
            operation: "funds".into(),
            status: ObtainStatus::Success,
            data: None,
            provenance_adapter_id: Some("binance_com".into()),
            provenance_path: None,
            product_use: None,
        }));
    }

    #[test]
    fn usdm_manifest_funds_positions_forceorder_lossy() {
        let manifest = binance_com_usdm_manifest();
        assert!(describe(&manifest).is_ok());
        assert_eq!(manifest.manifest_id, "binance_com.usdm.v1");
        assert_eq!(manifest.adapter_id, "binance_com");
        assert_eq!(manifest.book_id, "binance-com-usdm");
        assert_eq!(
            manifest.implemented,
            vec![
                "funds",
                "positionbook",
                "forceorder",
                "quotes",
                "search",
                "history",
                "depth"
            ]
        );
        assert!(manifest.implemented.iter().any(|op| op == "quotes"));
        assert!(manifest.implemented.iter().any(|op| op == "history"));
        assert!(manifest.implemented.iter().any(|op| op == "depth"));
        assert!(!manifest.implemented.iter().any(|op| op == "tradebook"));
        let quotes = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "quotes")
            .expect("usdm quotes binding");
        assert_eq!(quotes.capability_id, "quote");
        assert_eq!(quotes.auth_mode, AuthMode::Public);
        assert_eq!(quotes.transports, vec![TransportKind::Rest]);
        assert_ne!(
            quotes.transports,
            vec![TransportKind::Stream, TransportKind::Rest]
        );
        let history = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "history")
            .expect("usdm history binding");
        assert_eq!(history.capability_id, "ohlcv");
        assert_eq!(history.physics, Physics::HistoricalSeries);
        assert_eq!(history.auth_mode, AuthMode::Public);
        assert!(history.coverage.intervals.contains(&"1m".to_string()));
        assert!(!history.coverage.intervals.iter().any(|i| i == "1s"));
        let depth = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("usdm depth binding");
        assert_eq!(depth.capability_id, "order_book");
        assert_eq!(depth.physics, Physics::BoundedSnapshot);
        assert_eq!(depth.auth_mode, AuthMode::Public);
        assert_eq!(depth.transports, vec![TransportKind::Rest]);
        assert_ne!(
            depth.transports,
            vec![TransportKind::Stream, TransportKind::Rest]
        );
        let hist = obtain(&manifest, "history");
        assert_eq!(hist.status, ObtainStatus::Unavailable);
        assert_ne!(hist.status, ObtainStatus::Unsupported);
        assert!(hist.data.is_none());
        let funds = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "funds")
            .expect("usdm funds binding");
        assert_eq!(funds.family, Family::Account);
        assert_eq!(funds.capability_id, "funds");
        assert_eq!(funds.physics, Physics::BoundedSnapshot);
        assert_eq!(funds.auth_mode, AuthMode::PrivateRead);
        assert_eq!(funds.rights, Rights::research_fetch_only());
        assert!(!funds.rights.display);
        assert_eq!(funds.coverage.venues, ["binance.com"]);
        assert_eq!(funds.coverage.asset_classes, ["crypto_usdm"]);
        let positions = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "positionbook")
            .expect("usdm positionbook binding");
        assert_eq!(positions.family, Family::Account);
        assert_eq!(positions.capability_id, "positions");
        assert_eq!(positions.physics, Physics::BoundedSnapshot);
        assert_eq!(positions.auth_mode, AuthMode::PrivateRead);
        assert_eq!(positions.rights, Rights::research_fetch_only());
        assert!(!positions.rights.display);
        let force = manifest
            .bindings
            .iter()
            .find(|binding| binding.operation == "forceorder")
            .expect("usdm forceorder binding");
        assert_eq!(force.family, Family::Market);
        assert_eq!(force.capability_id, "force_order");
        assert_eq!(force.physics, Physics::LossyEventObservation);
        assert_ne!(force.physics, Physics::CompleteEventSequence);
        assert_eq!(force.auth_mode, AuthMode::PrivateRead);
        assert_eq!(force.rights, Rights::research_fetch_only());
        assert!(!force.rights.display);
        assert_eq!(force.transports, vec![TransportKind::Rest]);
        let fo = obtain(&manifest, "forceorder");
        assert_eq!(fo.status, ObtainStatus::Unavailable);
        assert_ne!(fo.status, ObtainStatus::Unsupported);
        assert!(fo.data.is_none());
        assert!(!is_empty_success(&fo));
        assert_eq!(obtain(&manifest, "funds").status, ObtainStatus::Unavailable);
        assert_eq!(
            obtain(&manifest, "positionbook").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            obtain(&manifest, "tradebook").status,
            ObtainStatus::Unsupported
        );
        let quotes_env = obtain(&manifest, "quotes");
        assert_eq!(quotes_env.status, ObtainStatus::Unavailable);
        assert_ne!(quotes_env.status, ObtainStatus::Unsupported);
        assert!(quotes_env.data.is_none());
        let search_env = obtain(&manifest, "search");
        assert_eq!(search_env.status, ObtainStatus::Unavailable);
        assert_ne!(search_env.status, ObtainStatus::Unsupported);
        let depth_env = obtain(&manifest, "depth");
        assert_eq!(depth_env.status, ObtainStatus::Unavailable);
        assert_ne!(depth_env.status, ObtainStatus::Unsupported);
        assert!(depth_env.data.is_none());
        assert!(!is_empty_success(&depth_env));
    }

    #[test]
    fn coinm_manifest_is_third_identity() {
        let coinm = binance_com_coinm_manifest();
        let usdm = binance_com_usdm_manifest();
        assert!(describe(&coinm).is_ok());
        assert_eq!(coinm.book_id, "binance-com-coinm");
        assert_eq!(coinm.manifest_id, "binance_com.coinm.v1");
        assert_eq!(coinm.adapter_id, "binance_com");
        assert_ne!(coinm.book_id, usdm.book_id);
        assert_ne!(coinm.book_id, "binance-com-spot");
        assert_ne!(coinm.book_id, "binance-com-options");
        let asset = coinm
            .bindings
            .iter()
            .find(|binding| binding.operation == "forceorder")
            .expect("coinm forceorder binding")
            .coverage
            .asset_classes
            .clone();
        assert_eq!(asset, vec!["crypto_coinm".to_string()]);
        assert!(!asset.iter().any(|class| class == "crypto_spot"));
        let force = coinm
            .bindings
            .iter()
            .find(|binding| binding.operation == "forceorder")
            .expect("coinm forceorder binding");
        assert_eq!(force.family, Family::Market);
        assert_eq!(force.capability_id, "force_order");
        assert_eq!(force.physics, Physics::LossyEventObservation);
        assert_ne!(force.physics, Physics::CompleteEventSequence);
        assert_eq!(force.auth_mode, AuthMode::PrivateRead);
        assert_eq!(force.rights, Rights::research_fetch_only());
        assert!(!force.rights.display);
        // DualNoBlend: matching is book_id. Coin-M never shares the USDM slot.
        assert_ne!(coinm.book_id, usdm.book_id);
        assert_ne!(
            force.coverage.asset_classes,
            usdm.bindings
                .iter()
                .find(|binding| binding.operation == "forceorder")
                .expect("usdm forceorder binding")
                .coverage
                .asset_classes
        );
        let fo = obtain(&coinm, "forceorder");
        assert_eq!(fo.status, ObtainStatus::Unavailable);
        assert_ne!(fo.status, ObtainStatus::Unsupported);
        assert!(fo.data.is_none());
        assert!(!is_empty_success(&fo));
        let quotes = obtain(&coinm, "quotes");
        assert_eq!(quotes.status, ObtainStatus::Unavailable);
        assert_ne!(quotes.status, ObtainStatus::Unsupported);
        let search = obtain(&coinm, "search");
        assert_eq!(search.status, ObtainStatus::Unavailable);
        assert_ne!(search.status, ObtainStatus::Unsupported);
        assert!(coinm.implemented.iter().any(|op| op == "quotes"));
        assert!(coinm.implemented.iter().any(|op| op == "search"));
        assert!(coinm.implemented.iter().any(|op| op == "history"));
        assert!(coinm.implemented.iter().any(|op| op == "depth"));
        assert!(!coinm.implemented.iter().any(|op| op == "tradebook"));
        let coinm_depth = coinm
            .bindings
            .iter()
            .find(|binding| binding.operation == "depth")
            .expect("coinm depth binding");
        assert_eq!(coinm_depth.capability_id, "order_book");
        assert_eq!(coinm_depth.physics, Physics::BoundedSnapshot);
        assert_eq!(coinm_depth.transports, vec![TransportKind::Rest]);
        let depth_env = obtain(&coinm, "depth");
        assert_eq!(depth_env.status, ObtainStatus::Unavailable);
        assert_ne!(depth_env.status, ObtainStatus::Unsupported);
        assert_ne!(
            obtain(&usdm, "depth").book_id,
            obtain(&coinm, "depth").book_id
        );
        assert_ne!(
            obtain(&usdm, "quotes").book_id,
            obtain(&coinm, "quotes").book_id
        );
    }

    #[test]
    fn nfo_funds_is_unavailable_not_unsupported() {
        let funds = obtain(&kotak_neo_nfo_manifest(), "funds");
        assert_eq!(funds.status, ObtainStatus::Unavailable);
        assert_ne!(funds.status, ObtainStatus::Unsupported);
        assert!(funds.data.is_none());
        assert!(!is_empty_success(&funds));
    }
}

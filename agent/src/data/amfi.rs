//! AMFI official NAV — SpecializedNonBroker labs vendor.
//!
//! Sheet: `docs/research/sheets/amfi.md` (fetch 2026-09-17 IST).
//! Host fence: `www.amfiindia.com` only. Not Kotak. Not Kill / Today PnL.

use super::descriptor::{DelayClass, Limits};
use super::host_policy::{authorize_book_call, AuthMode};
use super::identity::{Family, Physics};
use super::rights::Rights;
use super::source_manifest::{
    Coverage, ManifestBinding, ObtainEnvelope, ObtainStatus, SourceManifest, TransportKind,
};
use serde_json::json;

pub const AMFI_ADAPTER_ID: &str = "amfi";
pub const AMFI_NAV_BOOK_ID: &str = "amfi-nav";
pub const AMFI_NAV_HOST: &str = "www.amfiindia.com";
pub const AMFI_NAV_PATH: &str = "/spages/NAVAll.txt";
pub const AMFI_PRODUCT_USE: &str = "labs";

fn labs_envelope(
    operation: &str,
    status: ObtainStatus,
    data: Option<serde_json::Value>,
) -> ObtainEnvelope {
    ObtainEnvelope {
        adapter_id: AMFI_ADAPTER_ID.into(),
        book_id: AMFI_NAV_BOOK_ID.into(),
        operation: operation.to_string(),
        status,
        data,
        provenance_adapter_id: Some(AMFI_ADAPTER_ID.into()),
        provenance_path: Some(AMFI_NAV_PATH.into()),
        product_use: Some(AMFI_PRODUCT_USE.into()),
    }
}

fn dark(operation: &str, status: ObtainStatus) -> ObtainEnvelope {
    let mut envelope = labs_envelope(operation, status, None);
    if status != ObtainStatus::Success {
        envelope.provenance_path = None;
    }
    envelope
}

/// Parse the official `NAVAll.txt` shape. Category / AMC header lines are skipped.
pub fn parse_nav_all(body: &str) -> Vec<serde_json::Value> {
    let mut schemes = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("Scheme Code;") {
            continue;
        }
        if !line.as_bytes().first().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        let cols: Vec<&str> = line.split(';').collect();
        // Current AMFI file (fetch 2026-09-17 IST): 8 columns including Plan/Option.
        if cols.len() < 8 {
            continue;
        }
        schemes.push(json!({
            "scheme_code": cols[0],
            "isin_growth": cols[1],
            "isin_div": cols[2],
            "scheme_name": cols[3],
            "plan": cols[4],
            "option": cols[5],
            "nav": cols[6],
            "date": cols[7],
        }));
    }
    schemes
}

fn nav_data(schemes: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "identity": {
            "family": "fundamentals",
            "capability_id": "nav",
            "physics": "historical_series",
        },
        "schemes": schemes,
    })
}

/// Direct labs obtain. Does not consult Kotak `pick_route`.
pub async fn obtain_amfi_nav(
    operation: &str,
    host: &str,
    base_url: Option<&str>,
) -> ObtainEnvelope {
    if operation != "amfi_nav" {
        return dark(operation, ObtainStatus::Unsupported);
    }
    if authorize_book_call(AMFI_NAV_BOOK_ID, host, "GET", AMFI_NAV_PATH, false).is_err() {
        return dark(operation, ObtainStatus::Unavailable);
    }
    let url = match base_url.map(str::trim).filter(|s| !s.is_empty()) {
        Some(base) => format!("{}{AMFI_NAV_PATH}", base.trim_end_matches('/')),
        None => format!("https://{AMFI_NAV_HOST}{AMFI_NAV_PATH}"),
    };
    let client = crate::egress::shared_client();
    let Ok(resp) = client.get(&url).send().await else {
        return dark(operation, ObtainStatus::Unavailable);
    };
    if !resp.status().is_success() {
        return dark(operation, ObtainStatus::Unavailable);
    }
    let Ok(body) = resp.text().await else {
        return dark(operation, ObtainStatus::Unavailable);
    };
    let schemes = parse_nav_all(&body);
    if schemes.is_empty() {
        return dark(operation, ObtainStatus::Unavailable);
    }
    labs_envelope("amfi_nav", ObtainStatus::Success, Some(nav_data(schemes)))
}

pub fn amfi_nav_manifest() -> SourceManifest {
    let coverage = Coverage {
        venues: vec![],
        asset_classes: vec!["mutual_fund_nav".into()],
        history_range: Some("daily_nav".into()),
        intervals: vec![],
    };
    SourceManifest {
        manifest_id: "amfi.nav.v1".into(),
        adapter_id: AMFI_ADAPTER_ID.into(),
        book_id: AMFI_NAV_BOOK_ID.into(),
        implemented: vec!["amfi_nav".into()],
        bindings: vec![ManifestBinding {
            operation: "amfi_nav".into(),
            adapter_id: AMFI_ADAPTER_ID.into(),
            family: Family::Fundamentals,
            capability_id: "nav".into(),
            physics: Physics::HistoricalSeries,
            auth_mode: AuthMode::Public,
            transports: vec![TransportKind::Rest],
            rights: Rights::research_fetch_only(),
            limits: Limits::default(),
            coverage,
            delay_class: DelayClass::Eod,
        }],
    }
}

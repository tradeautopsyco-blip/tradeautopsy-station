//! S7 — obtain selection through [`pick_route`].
//!
//! Kotak history is a declared gap. A fixture vendor (`licensed_history`) may
//! fill it with its own book_id. Quote never consults the gap vendor.
//! Traders paste keys, not URLs.

use super::identity::{CapabilityId, Family, Identity, Physics};
use super::router::{pick_route, RouteCandidate, RouteDecision, RouteOutcome, SourceRole};
use super::source_manifest::{ObtainEnvelope, ObtainStatus};
use super::descriptor::{KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID};

/// CI / declared-gap fixture. Not a B6 product vendor. Not Yahoo.
pub const LICENSED_HISTORY_ADAPTER_ID: &str = "licensed_history";
/// Vendor series book — never a Kotak shipping book (DualNoBlend).
pub const LICENSED_HISTORY_BOOK_ID: &str = "licensed-history";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapVendorConfig {
    pub enabled: bool,
    pub key: Option<String>,
    pub history_budget: u32,
    /// Connected-broker quote meter. Independent of [`Self::history_budget`].
    pub quote_budget: u32,
}

impl Default for GapVendorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key: None,
            history_budget: 0,
            quote_budget: 1_000,
        }
    }
}

/// Host fence: credentials that look like fetch URLs are refused (SSRF).
pub fn secret_looks_like_url(secret: &str) -> bool {
    let trimmed = secret.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

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

fn is_kotak_book(book_id: &str) -> bool {
    book_id == KOTAK_NSE_BSE_CASH_BOOK_ID || book_id == KOTAK_NSE_NFO_BOOK_ID
}

pub fn should_source_route(book_id: &str, operation: &str) -> bool {
    is_kotak_book(book_id) && matches!(operation, "history" | "quotes")
}

fn vendor_key_ok(cfg: &GapVendorConfig) -> bool {
    let Some(key) = cfg.key.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    !secret_looks_like_url(key)
}

fn history_candidates(cfg: &GapVendorConfig) -> Vec<RouteCandidate> {
    let mut out = vec![RouteCandidate {
        adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
        identity: history_identity(),
        role: SourceRole::ConnectedBroker,
        eligible: false,
        broker_unsupported: true,
        local_fresh: false,
        stream_available: false,
        rest_available: false,
        budget_remaining: 0,
    }];
    if cfg.enabled && vendor_key_ok(cfg) {
        out.push(RouteCandidate {
            adapter_id: LICENSED_HISTORY_ADAPTER_ID.into(),
            identity: history_identity(),
            role: SourceRole::ExplicitGap,
            eligible: cfg.history_budget > 0,
            broker_unsupported: false,
            local_fresh: false,
            stream_available: false,
            rest_available: true,
            budget_remaining: cfg.history_budget,
        });
    }
    out
}

fn quote_candidates(cfg: &GapVendorConfig) -> Vec<RouteCandidate> {
    vec![RouteCandidate {
        adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
        identity: quote_identity(),
        role: SourceRole::ConnectedBroker,
        eligible: cfg.quote_budget > 0,
        broker_unsupported: false,
        local_fresh: false,
        stream_available: false,
        rest_available: true,
        budget_remaining: cfg.quote_budget,
    }]
}

pub fn decide_kotak_route(operation: &str, cfg: &GapVendorConfig) -> RouteDecision {
    match operation {
        "history" => pick_route(&history_identity(), &history_candidates(cfg)),
        "quotes" => pick_route(&quote_identity(), &quote_candidates(cfg)),
        _ => RouteDecision {
            adapter_id: None,
            transport: None,
            outcome: RouteOutcome::Unavailable,
            provenance_adapter_id: None,
        },
    }
}

/// Apply pick_route to a Kotak obtain envelope. `gap_history` is the vendor series
/// when the fixture answered (never Kotak last).
pub fn apply_kotak_source_route(
    mut envelope: ObtainEnvelope,
    cfg: &GapVendorConfig,
    gap_history: Option<serde_json::Value>,
) -> ObtainEnvelope {
    if !should_source_route(&envelope.book_id, &envelope.operation) {
        return envelope;
    }
    let decision = decide_kotak_route(&envelope.operation, cfg);
    match (envelope.operation.as_str(), decision.outcome) {
        ("history", RouteOutcome::Picked)
            if decision.adapter_id.as_deref() == Some(LICENSED_HISTORY_ADAPTER_ID) =>
        {
            if let Some(data) = gap_history {
                envelope.status = ObtainStatus::Success;
                envelope.data = Some(data);
                envelope.adapter_id = LICENSED_HISTORY_ADAPTER_ID.into();
                envelope.book_id = LICENSED_HISTORY_BOOK_ID.into();
                envelope.provenance_adapter_id = Some(LICENSED_HISTORY_ADAPTER_ID.into());
            } else {
                envelope.status = ObtainStatus::Unavailable;
                envelope.data = None;
                envelope.provenance_adapter_id = Some(LICENSED_HISTORY_ADAPTER_ID.into());
            }
            envelope
        }
        ("history", RouteOutcome::Unavailable) => {
            envelope.status = ObtainStatus::Unavailable;
            envelope.data = None;
            envelope.provenance_adapter_id = None;
            envelope
        }
        ("history", RouteOutcome::Unsupported) => {
            envelope.status = ObtainStatus::Unsupported;
            envelope.data = None;
            envelope.adapter_id = KOTAK_NEO_ADAPTER_ID.into();
            envelope.provenance_adapter_id = None;
            envelope
        }
        ("quotes", RouteOutcome::Unavailable) => {
            envelope.status = ObtainStatus::Unavailable;
            envelope.data = None;
            envelope
        }
        _ => envelope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(op: &str) -> ObtainEnvelope {
        ObtainEnvelope {
            adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
            book_id: KOTAK_NSE_NFO_BOOK_ID.into(),
            operation: op.into(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
            provenance_path: None,
        }
    }

    #[test]
    fn url_shaped_secret_is_refused() {
        assert!(secret_looks_like_url("https://evil.example/klines"));
        assert!(secret_looks_like_url("  http://127.0.0.1/x "));
        assert!(!secret_looks_like_url("lh-fixture-key"));
    }

    #[test]
    fn kotak_history_without_vendor_is_unsupported() {
        let out = apply_kotak_source_route(env("history"), &GapVendorConfig::default(), None);
        assert_eq!(out.status, ObtainStatus::Unsupported);
        assert!(out.data.is_none());
        assert_ne!(out.book_id, LICENSED_HISTORY_BOOK_ID);
    }

    #[test]
    fn fixture_vendor_history_uses_vendor_book_and_provenance() {
        let cfg = GapVendorConfig {
            enabled: true,
            key: Some("lh-fixture-key".into()),
            history_budget: 5,
            quote_budget: 10,
        };
        let data = serde_json::json!({"candles":[{"close":"1"}]});
        let out = apply_kotak_source_route(env("history"), &cfg, Some(data));
        assert_eq!(out.status, ObtainStatus::Success);
        assert_eq!(out.adapter_id, LICENSED_HISTORY_ADAPTER_ID);
        assert_eq!(out.book_id, LICENSED_HISTORY_BOOK_ID);
        assert_eq!(
            out.provenance_adapter_id.as_deref(),
            Some(LICENSED_HISTORY_ADAPTER_ID)
        );
        assert_ne!(out.book_id, KOTAK_NSE_NFO_BOOK_ID);
        assert_ne!(out.book_id, KOTAK_NSE_BSE_CASH_BOOK_ID);
    }

    #[test]
    fn vendor_budget_zero_history_unavailable_quote_still_routes_to_kotak() {
        let cfg = GapVendorConfig {
            enabled: true,
            key: Some("lh-fixture-key".into()),
            history_budget: 0,
            quote_budget: 10,
        };
        let hist = apply_kotak_source_route(env("history"), &cfg, None);
        assert_eq!(hist.status, ObtainStatus::Unavailable);
        let q = decide_kotak_route("quotes", &cfg);
        assert_eq!(q.outcome, RouteOutcome::Picked);
        assert_eq!(q.adapter_id.as_deref(), Some(KOTAK_NEO_ADAPTER_ID));
    }

    #[test]
    fn url_key_does_not_light_vendor() {
        let cfg = GapVendorConfig {
            enabled: true,
            key: Some("https://api.example/history".into()),
            history_budget: 9,
            quote_budget: 10,
        };
        let out = apply_kotak_source_route(env("history"), &cfg, Some(serde_json::json!({})));
        assert_eq!(out.status, ObtainStatus::Unsupported);
        assert_ne!(out.book_id, LICENSED_HISTORY_BOOK_ID);
    }
}

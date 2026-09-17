//! T6′ Health rows for non-broker vendor bindings. Not Health H4 copy-schema.
//! Broker quote is never a vendors[] row.

use super::source_route::{secret_looks_like_url, GapVendorConfig, LICENSED_HISTORY_ADAPTER_ID};
use serde_json::{json, Value};

const LICENSED_HISTORY_WHAT: &str = "licensed_history India ohlcv (Kotak has none)";

fn vendor_key_ok(cfg: &GapVendorConfig) -> bool {
    let Some(key) = cfg.key.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    !secret_looks_like_url(key)
}

fn licensed_history_status(cfg: &GapVendorConfig) -> &'static str {
    if !cfg.enabled || !vendor_key_ok(cfg) {
        "unsupported"
    } else if cfg.history_budget == 0 {
        "exhausted"
    } else {
        "up"
    }
}

/// One Health row per declared non-broker binding.
pub fn vendor_health_rows(cfg: &GapVendorConfig) -> Vec<Value> {
    let status = licensed_history_status(cfg);
    vec![json!({
        "adapter_id": LICENSED_HISTORY_ADAPTER_ID,
        "parent": "backend_box",
        "kind": "vendor",
        "status": status,
        "what": LICENSED_HISTORY_WHAT,
        "why": status,
        "error": Value::Null,
    })]
}

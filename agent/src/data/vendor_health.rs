//! T6′ Health rows for declared non-broker bindings. Not Health H4 copy-schema.
//! Broker quote is never a vendors[] row. Keys never appear.

use super::source_route::{secret_looks_like_url, GapVendorConfig, LICENSED_HISTORY_ADAPTER_ID};
use super::vendor_registry::{declared_vendor_bindings, AMFI_BINDING, LICENSED_HISTORY_BINDING};
use serde_json::{json, Value};

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

fn amfi_status(enabled: bool) -> &'static str {
    if enabled {
        "up"
    } else {
        "unsupported"
    }
}

/// One Health row per declared non-broker binding.
pub fn vendor_health_rows(cfg: &GapVendorConfig, amfi_enabled: bool) -> Vec<Value> {
    declared_vendor_bindings()
        .iter()
        .map(|spec| {
            let (status, what) = if spec.adapter_id == LICENSED_HISTORY_ADAPTER_ID {
                (
                    licensed_history_status(cfg),
                    LICENSED_HISTORY_BINDING.what,
                )
            } else {
                (amfi_status(amfi_enabled), AMFI_BINDING.what)
            };
            json!({
                "adapter_id": spec.adapter_id,
                "parent": "backend_box",
                "kind": "vendor",
                "status": status,
                "what": what,
                "why": status,
                "error": Value::Null,
                "obtain_noun": spec.obtain_noun,
                "product_use": spec.product_use,
            })
        })
        .collect()
}

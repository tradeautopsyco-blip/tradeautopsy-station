//! S4 freeze: lossy force-order never drives Kill or PnL.
//!
//! Kill apply and the round-trip engine already do not read `force_order`.
//! This test makes that a regression, not a convention.

use std::path::{Path, PathBuf};

/// Owner files at the Kill-apply and PnL seams.
const OWNERS: &[&str] = &[
    "src/round_trip_engine.rs",
    "src/api/kill_switch.rs",
];

fn agent_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn mentions_force_order(source: &str) -> bool {
    source.contains("force_order") || source.contains("forceorder") || source.contains("ForceOrder")
}

#[test]
fn the_detector_flags_an_import() {
    let imported = "use crate::data::force_order::extract_force_order;\n";
    assert!(
        mentions_force_order(imported),
        "detector must see a force_order import"
    );
    assert!(!mentions_force_order("use crate::broker::BrokerFill;\n"));
}

#[test]
fn round_trip_engine_and_kill_apply_do_not_import_force_order() {
    let mut violations: Vec<String> = Vec::new();
    for rel in OWNERS {
        let path = agent_root().join(rel);
        let source = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("failed to read {rel}: {err}");
        });
        if mentions_force_order(&source) {
            violations.push(rel.to_string());
        }
    }
    assert!(
        violations.is_empty(),
        "Kill apply and round-trip PnL must not import force_order:\n  {}",
        violations.join("\n  ")
    );
}

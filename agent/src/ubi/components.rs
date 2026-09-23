//! Adapter component lookup by broker slug (Phase 3).
//!
//! Each enabled broker maps to exactly one `.wasm` component; the Enforcer refuses to
//! start a sync for a slug whose component is missing rather than falling back to
//! native code (ADR 0001 — no native adapter tier).

use std::path::PathBuf;

/// Override the directory the Enforcer loads adapter components from (packaging + tests).
pub const COMPONENT_DIR_ENV: &str = "TRADEAUTOPSY_UBI_COMPONENT_DIR";

/// Slug → (component file name, source crate directory).
const COMPONENTS: &[(&str, &str, &str)] = &[
    (
        "binance_com",
        "ubi_binance_com_adapter.wasm",
        "ubi-binance-com-adapter",
    ),
    (
        "kotak_neo",
        "ubi_kotak_neo_adapter.wasm",
        "ubi-kotak-neo-adapter",
    ),
    (
        "zerodha_kite",
        "ubi_zerodha_kite_adapter.wasm",
        "ubi-zerodha-kite-adapter",
    ),
    (
        "upstox",
        "ubi_upstox_adapter.wasm",
        "ubi-upstox-adapter",
    ),
];

pub fn component_file_name(slug: &str) -> Option<&'static str> {
    COMPONENTS
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map(|(_, file, _)| *file)
}

pub fn component_crate_dir(slug: &str) -> Option<&'static str> {
    COMPONENTS
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map(|(_, _, dir)| *dir)
}

/// Candidate locations, highest priority first: env override, next to the binary,
/// then the dev-tree build output of the component crate.
pub fn component_candidate_paths(slug: &str) -> Vec<PathBuf> {
    let Some(file) = component_file_name(slug) else {
        return Vec::new();
    };
    let mut paths = Vec::new();

    if let Ok(dir) = std::env::var(COMPONENT_DIR_ENV) {
        if !dir.trim().is_empty() {
            paths.push(PathBuf::from(dir).join(file));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(file));
            paths.push(dir.join("ubi").join(file));
        }
    }
    if let Some(crate_dir) = component_crate_dir(slug) {
        paths.push(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(crate_dir)
                .join("target/wasm32-wasip2/release")
                .join(file),
        );
    }
    paths
}

pub fn component_path_for_slug(slug: &str) -> Option<PathBuf> {
    component_candidate_paths(slug)
        .into_iter()
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_first_party_slugs_have_components() {
        assert_eq!(
            component_file_name("binance_com"),
            Some("ubi_binance_com_adapter.wasm")
        );
        assert_eq!(
            component_file_name("kotak_neo"),
            Some("ubi_kotak_neo_adapter.wasm")
        );
        assert!(component_file_name("binance_us").is_none());
    }

    /// B6: no Wasm component for unsheeted / unsigned slugs (named next included).
    ///
    /// W0.8 (F6): the COMPONENTS len-2 guard below is the mechanical half of the
    /// gate — a third component entry cannot land without touching this test,
    /// which forces the author to also prove a SIGNED sheet exists for it.
    #[test]
    fn b6_gate_no_component_for_unsheeted_slugs() {
        assert_eq!(
            component_file_name("zerodha_kite"),
            Some("ubi_zerodha_kite_adapter.wasm")
        );
        assert!(component_file_name("interactive_brokers").is_none());
        assert!(component_file_name("binance_us").is_none());
        // Guard: only SIGNED sheets ship components. Bump this number only in the
        // PR that adds another SIGNED slug's Wasm crate (G0→G3).
        assert_eq!(
            component_file_name("upstox"),
            Some("ubi_upstox_adapter.wasm")
        );
        assert_eq!(COMPONENTS.len(), 4, "signed broker components only");
    }

    #[test]
    fn candidate_paths_include_dev_build_output() {
        let paths = component_candidate_paths("kotak_neo");
        assert!(paths.iter().any(|p| p.ends_with(
            "ubi-kotak-neo-adapter/target/wasm32-wasip2/release/ubi_kotak_neo_adapter.wasm"
        )));
        assert!(component_candidate_paths("unknown_broker").is_empty());
    }
}

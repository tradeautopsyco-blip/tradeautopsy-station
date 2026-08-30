//! Ring 2: the chokepoint is a property of the tree, not a convention.
//!
//! Mirrors the pattern `ubi/allowlist.rs` already uses for Kill DNS ("a test
//! asserts every host here is sinkholed by an L3 block"): the invariant is
//! asserted over the source, so a new call site cannot quietly reintroduce a
//! private connection pool to a venue.

use std::path::{Path, PathBuf};

/// Files permitted to construct their own `reqwest` client, each for a stated
/// reason. This list may shrink. Adding to it needs a reason of the same kind.
const EXEMPT: &[(&str, &str)] = &[
    (
        "src/egress/transport.rs",
        "the chokepoint itself — the one owner of the pool",
    ),
    (
        "src/lib.rs",
        "UpstreamClient talks to TradeAutopsy's own backend, not a venue",
    ),
    (
        "src/instruments/store.rs",
        "Zerodha instruments dump. Zerodha has no docs lock and no egress slot; \
         per docs/reference discipline it cannot be metered until it has one, and \
         it stays behind the zerodha_instruments_enabled flag.",
    ),
    (
        "src/broker_validation.rs",
        "binance.us connect validation. binance_us has no egress slot; \
         `api.binance.us` is on the B6 refuse list for the desk.",
    ),
];

fn agent_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    path.strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")))
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Client construction outside a `#[cfg(test)]` block. Test modules inside `src`
/// may build their own clients against wiremock and loopback.
fn offending_lines(source: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_test_mod = false;
    let mut test_mod_depth: i32 = 0;
    let mut depth: i32 = 0;

    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            in_test_mod = true;
            test_mod_depth = depth;
        }
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;

        if !in_test_mod
            && (trimmed.contains("reqwest::Client::new")
                || trimmed.contains("reqwest::Client::builder")
                || trimmed.contains("reqwest::ClientBuilder"))
        {
            out.push((idx + 1, trimmed.to_string()));
        }

        depth += opens - closes;
        if in_test_mod && depth <= test_mod_depth && closes > 0 {
            in_test_mod = false;
        }
    }
    out
}

#[test]
fn no_module_outside_the_chokepoint_builds_its_own_venue_client() {
    let mut files = Vec::new();
    rust_files(&agent_src(), &mut files);
    files.sort();
    assert!(!files.is_empty(), "found no sources to lint");

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let rel = relative(file);
        if EXEMPT.iter().any(|(p, _)| *p == rel) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };
        for (line_no, text) in offending_lines(&source) {
            violations.push(format!("{rel}:{line_no}: {text}"));
        }
    }

    assert!(
        violations.is_empty(),
        "these modules build their own HTTP client instead of going through \
         VenueEgress. Route them through `crate::egress`, or add a documented \
         exemption to EXEMPT in this file if the target is not a venue:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn every_exemption_still_exists_and_still_needs_its_exemption() {
    for (path, reason) in EXEMPT {
        let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        assert!(full.exists(), "exempt path no longer exists: {path}");
        assert!(
            !reason.trim().is_empty(),
            "exemption without a reason: {path}"
        );

        let source = std::fs::read_to_string(&full).expect("read exempt file");
        assert!(
            !offending_lines(&source).is_empty(),
            "{path} no longer builds its own client — drop it from EXEMPT so the \
             list keeps shrinking"
        );
    }
}

/// The lint would be worthless if it could not see a violation.
#[test]
fn the_lint_detects_a_violation_and_ignores_test_modules() {
    let production = r#"
fn go() {
    let c = reqwest::Client::builder().build();
}
"#;
    assert_eq!(offending_lines(production).len(), 1);

    let test_only = r#"
fn go() {}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let c = reqwest::Client::new();
    }
}
"#;
    assert!(
        offending_lines(test_only).is_empty(),
        "test modules inside src may use wiremock and loopback"
    );
}

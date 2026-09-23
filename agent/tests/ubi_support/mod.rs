//! Shared helpers for UBI component tests (Phase 3).
//! Builds the adapter components on demand and provides sentinel credentials that must
//! never appear inside component memory or on the component data channel (R6).

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use tradeautopsy_agent::{HostCredentialBlob, UbiHostState, FORBIDDEN_COMPONENT_HEADERS};

pub fn agent_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn read_fixture(name: &str) -> String {
    let path = agent_dir().join("fixtures/ubi").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Build (if needed) and return the component `.wasm` for a broker slug.
pub fn component_wasm(slug: &str) -> PathBuf {
    let crate_dir = tradeautopsy_agent::component_crate_dir(slug)
        .unwrap_or_else(|| panic!("no component crate for {slug}"));
    let file = tradeautopsy_agent::component_file_name(slug).expect("component file");
    component_wasm_from_crate(crate_dir, file)
}

/// Build (if needed) a component crate not yet registered in `components.rs` (scaffold / pre-sign).
pub fn component_wasm_from_crate(crate_dir: &str, wasm_file: &str) -> PathBuf {
    let workspace_wasm = agent_dir()
        .join("target/wasm32-wasip2/release")
        .join(wasm_file);
    let crate_wasm = agent_dir()
        .join(crate_dir)
        .join("target/wasm32-wasip2/release")
        .join(wasm_file);
    if workspace_wasm.is_file() {
        return workspace_wasm;
    }
    if crate_wasm.is_file() {
        return crate_wasm;
    }
    let status = Command::new("cargo")
        .current_dir(agent_dir())
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "-p",
            crate_dir,
        ])
        .status()
        .unwrap_or_else(|e| panic!("spawn cargo build for {crate_dir}: {e}"));
    assert!(
        status.success(),
        "failed to build {crate_dir} (is the wasm32-wasip2 target installed?)"
    );
    let wasm = if workspace_wasm.is_file() {
        workspace_wasm
    } else {
        crate_wasm
    };
    assert!(wasm.is_file(), "expected component at {}", wasm.display());
    wasm
}

pub fn sentinel_hmac() -> HostCredentialBlob {
    HostCredentialBlob::hmac(
        "TEST_API_KEY_SHOULD_NEVER_REACH_COMPONENT",
        "TEST_API_SECRET_SHOULD_NEVER_REACH_COMPONENT",
    )
}

pub fn sentinel_kotak_session() -> HostCredentialBlob {
    HostCredentialBlob::KotakSession {
        consumer_key: "TEST_CONSUMER_KEY_NEVER_IN_COMPONENT".into(),
        trade_token: "TEST_TRADE_TOKEN_NEVER_IN_COMPONENT".into(),
        sid: "TEST_SID_NEVER_IN_COMPONENT".into(),
        base_url: "https://cis.kotaksecurities.com/trading".into(),
        hs_server_id: "TEST_HS_SERVER_NEVER_IN_COMPONENT".into(),
    }
}

/// Proof for R6: no secret value crosses into the component, in either direction.
pub fn assert_component_never_saw_secrets(state: &UbiHostState, component_path: &std::path::Path) {
    assert!(!state.credential_leak_attempted);
    let secrets = state.config.credentials.secret_values();
    let bytes = std::fs::read(component_path).expect("read component wasm");
    let as_text = String::from_utf8_lossy(&bytes);

    for secret in &secrets {
        assert!(
            !as_text.contains(secret),
            "credential value found inside component wasm bytes"
        );
    }
    for call in &state.calls {
        for header in &call.headers {
            assert!(
                !FORBIDDEN_COMPONENT_HEADERS
                    .iter()
                    .any(|f| header.name.eq_ignore_ascii_case(f)),
                "component sent forbidden auth header {}",
                header.name
            );
            assert!(secrets.iter().all(|s| !header.value.contains(s)));
        }
        for param in &call.query {
            assert!(secrets.iter().all(|s| !param.value.contains(s)));
            assert_ne!(param.name.to_ascii_lowercase(), "signature");
        }
    }
}

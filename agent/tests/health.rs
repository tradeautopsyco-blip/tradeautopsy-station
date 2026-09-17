//! Tracer bullet (issue #57): agent exposes liveness on loopback with wire v1 (issue #58).
//! Issue #44: health JSON must not leak credential-shaped keys (AGENTS.md invariant 5).

mod common;

use common::{
    apply_wire_v1, client, spawn_test_agent, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID, TEST_SECRET,
};
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerCredentialVault, CredentialBlob, MemoryBrokerCredentialVault, RedactionBoundary,
};

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn get_health_returns_ok_for_headless_agent() {
    const PORT: u16 = 19_337;
    let path = "/api/daemon/health";

    let handle = spawn_test_agent(PORT);

    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );

    let resp = req
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .expect("health request should reach agent");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["daemon"], "agent");
    assert!(
        body["build"].is_string(),
        "health should expose build identity string"
    );
    assert!(
        body["boot_id"].is_string(),
        "health should expose per-boot identity"
    );
    assert!(
        body["uptime_secs"].is_u64(),
        "health should expose uptime for liveness freshness"
    );
    assert!(
        body["sse_signing_pubkey_b64"].is_string(),
        "Phase 9 — health exposes pinned SSE signing pubkey"
    );
    assert!(
        body["observability"].is_object(),
        "Phase 9 — health exposes observability snapshot"
    );
    assert!(
        body["metrics_listen_port"].is_null(),
        "metrics_listen_port null when Prometheus disabled (tests)"
    );
    assert!(
        body["observability"]["agent_uptime_seconds"].is_number(),
        "observability includes agent_uptime_seconds gauge snapshot"
    );

    handle.abort();
}

/// Distinctive vault literals so a leak is obvious in the serialized health body.
const PLANTED_API_KEY: &str = "health-json-probe-api-key-binance-us";
const PLANTED_API_SECRET: &str = "health-json-probe-api-secret-literal";
/// Distinctive gap-vendor literal. Health JSON must never echo it.
const PLANTED_VENDOR_KEY: &str = "lh-health-probe-vendor-key-must-not-leak";

#[tokio::test]
async fn health_json_must_not_leak_credential_shaped_keys() {
    const PORT: u16 = 19_444;
    let path = "/api/daemon/health";

    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            "binance_us",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac(PLANTED_API_KEY, PLANTED_API_SECRET),
        )
        .expect("seed vault");

    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
            gap_vendor_enabled: true,
            gap_vendor_key: Some(PLANTED_VENDOR_KEY.into()),
            gap_vendor_history_budget: 5,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(2))
    .send()
    .await
    .expect("health request should reach agent");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json body");

    assert!(
        !RedactionBoundary::contains_forbidden_material(
            &body,
            &[TEST_SECRET, PLANTED_API_KEY, PLANTED_API_SECRET, PLANTED_VENDOR_KEY]
        ),
        "GET /api/daemon/health must not expose credential-shaped keys or in-process secrets; got {body}"
    );
}

async fn get_health(port: u16) -> serde_json::Value {
    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{port}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(2))
    .send()
    .await
    .expect("health request should reach agent");
    assert_eq!(resp.status(), 200);
    resp.json().await.expect("json body")
}

fn licensed_history_vendor<'a>(body: &'a serde_json::Value) -> &'a serde_json::Value {
    body["vendors"]
        .as_array()
        .expect("vendors array")
        .iter()
        .find(|row| row["adapter_id"] == "licensed_history")
        .expect("licensed_history vendor row")
}

#[tokio::test]
async fn health_vendors_licensed_history_unsupported_by_default() {
    const PORT: u16 = 19_680;

    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            gap_vendor_key: Some(PLANTED_VENDOR_KEY.into()),
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;

    let body = get_health(PORT).await;
    let row = licensed_history_vendor(&body);

    assert_eq!(row["adapter_id"], "licensed_history");
    assert_eq!(row["parent"], "backend_box");
    assert_eq!(row["kind"], "vendor");
    assert_eq!(row["status"], "unsupported");
    assert_eq!(row["what"], "licensed_history India ohlcv (Kotak has none)");
    assert_eq!(row["why"], "unsupported");
    assert!(row["error"].is_null());
    let amfi = body["vendors"]
        .as_array()
        .expect("vendors")
        .iter()
        .find(|row| row["adapter_id"] == "amfi")
        .expect("amfi vendor row");
    assert_eq!(amfi["status"], "up");
    assert_eq!(amfi["what"], "AMFI NAV (labs)");
    assert_eq!(amfi["obtain_noun"], "amfi_nav");
    assert!(
        body["vendors"]
            .as_array()
            .expect("vendors")
            .iter()
            .all(|row| row["adapter_id"] != "kotak_neo"),
        "broker quote is not a vendors[] row; got {body}"
    );
    let serialized = body.to_string();
    assert!(
        !serialized.contains(PLANTED_VENDOR_KEY),
        "health JSON must not contain the planted vendor key; got {serialized}"
    );
}

#[tokio::test]
async fn health_vendors_licensed_history_up_when_enabled_with_budget() {
    const PORT: u16 = 19_681;

    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            gap_vendor_enabled: true,
            gap_vendor_key: Some("lh-fixture-key".into()),
            gap_vendor_history_budget: 5,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;

    let body = get_health(PORT).await;
    let row = licensed_history_vendor(&body);

    assert_eq!(row["adapter_id"], "licensed_history");
    assert_eq!(row["parent"], "backend_box");
    assert_eq!(row["kind"], "vendor");
    assert_eq!(row["status"], "up");
    assert_eq!(row["what"], "licensed_history India ohlcv (Kotak has none)");
    assert_eq!(row["why"], "up");
    assert!(row["error"].is_null());
    assert!(
        !body.to_string().contains("lh-fixture-key"),
        "health JSON must not contain the vendor key; got {body}"
    );
}

#[tokio::test]
async fn health_vendors_licensed_history_exhausted_when_budget_zero() {
    const PORT: u16 = 19_682;

    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            gap_vendor_enabled: true,
            gap_vendor_key: Some("lh-fixture-key".into()),
            gap_vendor_history_budget: 0,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;

    let body = get_health(PORT).await;
    let row = licensed_history_vendor(&body);

    assert_eq!(row["adapter_id"], "licensed_history");
    assert_eq!(row["parent"], "backend_box");
    assert_eq!(row["kind"], "vendor");
    assert_eq!(row["status"], "exhausted");
    assert_eq!(row["what"], "licensed_history India ohlcv (Kotak has none)");
    assert_eq!(row["why"], "exhausted");
    assert!(row["error"].is_null());
}

#[tokio::test]
async fn health_json_must_not_leak_gap_vendor_key() {
    const PORT: u16 = 19_683;

    let _handle = AbortOnDrop(spawn_test_agent_with_options(
        PORT,
        TestAgentOptions {
            gap_vendor_enabled: true,
            gap_vendor_key: Some(PLANTED_VENDOR_KEY.into()),
            gap_vendor_history_budget: 5,
            ..TestAgentOptions::default()
        },
    ));
    wait_ready(PORT).await;

    let body = get_health(PORT).await;
    let row = licensed_history_vendor(&body);
    assert_eq!(row["status"], "up");
    assert!(row["error"].is_null());
    assert!(
        !RedactionBoundary::contains_forbidden_material(&body, &[PLANTED_VENDOR_KEY]),
        "GET /api/daemon/health must not expose the planted vendor key; got {body}"
    );
}

//! S7 vendor machine — Box Enable reaches agent via PUT /api/daemon/vendor-bindings.

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, seeded_hmac_vault, spawn_test_agent_with_options,
    wait_ready, TestAgentOptions, WireHeaderOverrides,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{BrokerAdapter, BrokerCredentialVault, CountingPollAdapter};

const PORT_PUT: u16 = 19_690;
const PORT_URL: u16 = 19_691;
const PORT_DISABLE: u16 = 19_692;
const PORT_AMFI: u16 = 19_693;
const PORT_QUOTE: u16 = 19_694;
const PORT_UNKNOWN: u16 = 19_695;

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn put_binding(port: u16, body: serde_json::Value) -> (u16, serde_json::Value) {
    let path = "/api/daemon/vendor-bindings";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&body).expect("json");
    let resp = apply_wire_v1(
        client()
            .put(&url)
            .header("content-type", "application/json"),
        "PUT",
        path,
        &payload,
        WireHeaderOverrides::default(),
    )
    .body(payload)
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("put");
    let status = resp.status().as_u16();
    let json = resp.json().await.expect("json");
    (status, json)
}

async fn get_health(port: u16) -> serde_json::Value {
    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{port}{path}");
    apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(2))
    .send()
    .await
    .expect("health")
    .json()
    .await
    .expect("json")
}

fn vendor_row<'a>(body: &'a serde_json::Value, adapter: &str) -> &'a serde_json::Value {
    body["vendors"]
        .as_array()
        .expect("vendors")
        .iter()
        .find(|row| row["adapter_id"] == adapter)
        .unwrap_or_else(|| panic!("missing {adapter} in {body}"))
}

fn kotak_opts() -> TestAgentOptions {
    let vault = seeded_hmac_vault("kotak_neo", "TA_S7_VENDOR_PUT");
    let counter = Arc::new(CountingPollAdapter::new());
    TestAgentOptions {
        runtime_poll_adapter: Some(counter as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        plant_kotak_s1k_fixtures: true,
        plant_licensed_history_gap: true,
        ..TestAgentOptions::default()
    }
}

async fn post_kotak_start(port: u16) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body("kotak_neo")).expect("json");
    let resp = apply_wire_v1(
        client()
            .post(&url)
            .header("content-type", "application/json"),
        "POST",
        path,
        &payload,
        WireHeaderOverrides::default(),
    )
    .body(payload)
    .timeout(Duration::from_secs(3))
    .send()
    .await
    .expect("start");
    assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap_or_default());
}

async fn obtain(port: u16, query: &str) -> serde_json::Value {
    client()
        .get(format!(
            "http://127.0.0.1:{port}/api/station/obtain?{query}"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("obtain")
        .json()
        .await
        .expect("json")
}

#[tokio::test]
#[serial]
async fn put_enable_with_key_makes_licensed_history_health_up() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(PORT_PUT, kotak_opts()));
    wait_ready(PORT_PUT).await;

    let before = get_health(PORT_PUT).await;
    assert_eq!(vendor_row(&before, "licensed_history")["status"], "unsupported");
    assert_eq!(vendor_row(&before, "amfi")["status"], "up");

    let (status, body) = put_binding(
        PORT_PUT,
        serde_json::json!({
            "adapter_id": "licensed_history",
            "enabled": true,
            "api_key": "lh-fixture-key",
            "history_budget": 5
        }),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ok"], true);
    assert!(body.get("api_key").is_none());
    assert!(!body.to_string().contains("lh-fixture-key"));

    let after = get_health(PORT_PUT).await;
    assert_eq!(vendor_row(&after, "licensed_history")["status"], "up");
    assert!(!after.to_string().contains("lh-fixture-key"));
}

#[tokio::test]
#[serial]
async fn put_url_shaped_key_is_refused() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_URL,
        TestAgentOptions::default(),
    ));
    wait_ready(PORT_URL).await;

    let (status, body) = put_binding(
        PORT_URL,
        serde_json::json!({
            "adapter_id": "licensed_history",
            "enabled": true,
            "api_key": "https://finnhub.io/api/v1/stock/candle"
        }),
    )
    .await;
    assert_eq!(status, 400);
    assert_eq!(body["error_class"], "url_is_not_a_key");
    assert_eq!(vendor_row(&get_health(PORT_URL).await, "licensed_history")["status"], "unsupported");
}

#[tokio::test]
#[serial]
async fn put_disable_darks_licensed_history() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(PORT_DISABLE, kotak_opts()));
    wait_ready(PORT_DISABLE).await;
    let _ = put_binding(
        PORT_DISABLE,
        serde_json::json!({
            "adapter_id": "licensed_history",
            "enabled": true,
            "api_key": "lh-fixture-key",
            "history_budget": 5
        }),
    )
    .await;
    assert_eq!(
        vendor_row(&get_health(PORT_DISABLE).await, "licensed_history")["status"],
        "up"
    );

    let (status, _) = put_binding(
        PORT_DISABLE,
        serde_json::json!({
            "adapter_id": "licensed_history",
            "enabled": false
        }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        vendor_row(&get_health(PORT_DISABLE).await, "licensed_history")["status"],
        "unsupported"
    );
}

#[tokio::test]
#[serial]
async fn put_amfi_disable_darks_nav_obtain() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_AMFI,
        TestAgentOptions::default(),
    ));
    wait_ready(PORT_AMFI).await;
    assert_eq!(vendor_row(&get_health(PORT_AMFI).await, "amfi")["status"], "up");

    let (status, _) = put_binding(
        PORT_AMFI,
        serde_json::json!({
            "adapter_id": "amfi",
            "enabled": false
        }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(vendor_row(&get_health(PORT_AMFI).await, "amfi")["status"], "unsupported");

    let nav = obtain(PORT_AMFI, "adapter=amfi&operation=amfi_nav").await;
    assert_eq!(nav["status"], "unsupported");
    assert_eq!(nav["product_use"], "labs");
    assert_ne!(nav["book_id"], "kotak-nse-bse-cash");
}

#[tokio::test]
#[serial]
async fn put_vendor_on_does_not_steal_kotak_quotes() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(PORT_QUOTE, kotak_opts()));
    wait_ready(PORT_QUOTE).await;
    post_kotak_start(PORT_QUOTE).await;
    let _ = put_binding(
        PORT_QUOTE,
        serde_json::json!({
            "adapter_id": "licensed_history",
            "enabled": true,
            "api_key": "lh-fixture-key",
            "history_budget": 5
        }),
    )
    .await;

    let quote: serde_json::Value = client()
        .get(format!(
            "http://127.0.0.1:{PORT_QUOTE}/api/station/quote?instrument=nse_cm%7C2885"
        ))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .expect("bind")
        .json()
        .await
        .expect("json");
    assert_ne!(quote["status"], "unavailable");

    let obtained = obtain(PORT_QUOTE, "adapter=kotak_neo&operation=quotes").await;
    assert_eq!(obtained["status"], "success");
    assert_eq!(obtained["provenance_adapter_id"], "kotak_neo");
    assert_eq!(obtained["book_id"], "kotak-nse-bse-cash");
}

#[tokio::test]
#[serial]
async fn put_unknown_and_openbb_are_refused() {
    let _h = AbortOnDrop(spawn_test_agent_with_options(
        PORT_UNKNOWN,
        TestAgentOptions::default(),
    ));
    wait_ready(PORT_UNKNOWN).await;
    for adapter in ["finnhub", "openbb", "yahoo", "fmp"] {
        let (status, body) = put_binding(
            PORT_UNKNOWN,
            serde_json::json!({
                "adapter_id": adapter,
                "enabled": true,
                "api_key": "x"
            }),
        )
        .await;
        assert_eq!(status, 400, "{adapter}: {body}");
        assert_eq!(body["error_class"], "unknown_vendor");
    }
}

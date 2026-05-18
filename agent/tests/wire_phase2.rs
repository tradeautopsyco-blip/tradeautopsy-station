//! Phase 2 — signed wire v1 + structured protocol errors (issue #58).

mod common;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use common::{
    apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides, TEST_SECRET, TEST_USER_ID,
};
use std::time::Duration;
use tradeautopsy_agent::{WireVerifier, WIRE_PROTO_VERSION};

async fn assert_error_json(resp: reqwest::Response, status: reqwest::StatusCode, class: &str) {
    assert_eq!(resp.status(), status);
    let v: serde_json::Value = resp.json().await.expect("json error body");
    assert_eq!(v["error_class"].as_str(), Some(class));
    assert!(v.get("message").is_some());
}

#[tokio::test]
async fn proto_version_mismatch_returns_412_proto_version() {
    const PORT: u16 = 19_401;
    let path = "/api/daemon/health";
    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides {
            proto_version: Some("2"),
            ..Default::default()
        },
    )
    .send()
    .await
    .expect("request");

    assert_error_json(
        resp,
        reqwest::StatusCode::PRECONDITION_FAILED,
        "PROTO_VERSION",
    )
    .await;
    handle.abort();
}

#[tokio::test]
async fn bad_hmac_returns_401_sig_invalid() {
    const PORT: u16 = 19_402;
    let path = "/api/daemon/health";
    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let bad_sig = B64.encode([0u8; 32]);
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides {
            signature_b64: Some(bad_sig.as_str()),
            ..Default::default()
        },
    )
    .send()
    .await
    .expect("request");

    assert_error_json(resp, reqwest::StatusCode::UNAUTHORIZED, "SIG_INVALID").await;
    handle.abort();
}

#[tokio::test]
async fn timestamp_outside_window_returns_401_sig_invalid() {
    const PORT: u16 = 19_403;
    let path = "/api/daemon/health";
    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let stale = (chrono::Utc::now() - chrono::Duration::seconds(90))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides {
            timestamp: Some(stale),
            ..Default::default()
        },
    )
    .send()
    .await
    .expect("request");

    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
    let v: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(v["error_class"].as_str(), Some("SIG_INVALID"));
    assert!(v["retry_after_ms"].is_null());
    handle.abort();
}

#[tokio::test]
async fn invalid_request_id_returns_400_validation() {
    const PORT: u16 = 19_404;
    let path = "/api/daemon/health";
    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let resp = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides {
            request_id: Some("not-a-ulid"),
            ..Default::default()
        },
    )
    .send()
    .await
    .expect("request");

    assert_error_json(resp, reqwest::StatusCode::BAD_REQUEST, "VALIDATION").await;
    handle.abort();
}

#[tokio::test]
async fn nonce_replay_returns_401_sig_invalid() {
    const PORT: u16 = 19_405;
    let path = "/api/daemon/health";
    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let shared_nonce = *uuid::Uuid::new_v4().as_bytes();
    let rid_a = ulid::Ulid::new().to_string();
    let rid_b = ulid::Ulid::new().to_string();
    let ts = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let verifier = WireVerifier::new(TEST_SECRET);
    let nonce_b64 = B64.encode(shared_nonce);

    let sig_a = B64.encode(verifier.compute_signature("GET", path, &ts, &rid_a, b""));
    let sig_b = B64.encode(verifier.compute_signature("GET", path, &ts, &rid_b, b""));

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let client = client();

    let r1 = client
        .get(&url)
        .header("x-proto-version", WIRE_PROTO_VERSION)
        .header("x-daemon-secret", TEST_SECRET)
        .header("x-user-id", TEST_USER_ID)
        .header("x-request-id", &rid_a)
        .header("x-timestamp", &ts)
        .header("x-nonce", &nonce_b64)
        .header("x-signature", &sig_a)
        .send()
        .await
        .expect("r1");
    assert_eq!(r1.status(), reqwest::StatusCode::OK);

    let r2 = client
        .get(&url)
        .header("x-proto-version", WIRE_PROTO_VERSION)
        .header("x-daemon-secret", TEST_SECRET)
        .header("x-user-id", TEST_USER_ID)
        .header("x-request-id", &rid_b)
        .header("x-timestamp", &ts)
        .header("x-nonce", &nonce_b64)
        .header("x-signature", &sig_b)
        .send()
        .await
        .expect("r2");

    assert_error_json(r2, reqwest::StatusCode::UNAUTHORIZED, "SIG_INVALID").await;
    handle.abort();
}

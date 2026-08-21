//! A8 Station device-login loopback API — begin response omits device_code;
//! complete polls WorkOS → mints → proves Console session.

mod common;

use common::{apply_wire_v1, client, spawn_test_agent_with_options, TestAgentOptions, TEST_SECRET};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT: u16 = 39710;

#[tokio::test]
#[serial]
async fn station_auth_begin_returns_user_code_without_device_code() {
    let workos = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user_management/authorize/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "secret-device-code-must-not-leak",
            "user_code": "RRGQ-BJVS",
            "verification_uri": "https://example.authkit.app/device",
            "verification_uri_complete": "https://example.authkit.app/device?user_code=RRGQ-BJVS",
            "expires_in": 300,
            "interval": 5
        })))
        .mount(&workos)
        .await;

    std::env::set_var("WORKOS_API_BASE_URL", workos.uri());
    std::env::set_var("WORKOS_STATION_CLIENT_ID", "client_test_station");
    std::env::set_var(
        "TRADEAUTOPSY_SERVER_BASE_URL",
        "https://localhost:3000",
    );

    let _agent = spawn_test_agent_with_options(PORT, TestAgentOptions::default());
    tokio::time::sleep(Duration::from_millis(400)).await;

    let req = apply_wire_v1(
        client().post(format!(
            "http://127.0.0.1:{PORT}/api/daemon/auth/station/begin"
        )),
        "POST",
        "/api/daemon/auth/station/begin",
        &[],
        Default::default(),
    );
    let resp = req.send().await.expect("begin");
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["user_code"], "RRGQ-BJVS");
    assert!(body.get("device_code").is_none());
    let dumped = body.to_string();
    assert!(!dumped.contains("secret-device-code"));

    std::env::remove_var("WORKOS_API_BASE_URL");
    std::env::remove_var("WORKOS_STATION_CLIENT_ID");
}

#[tokio::test]
#[serial]
async fn station_auth_complete_mints_and_proves_session() {
    let workos = MockServer::start().await;
    let console = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user_management/authorize/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "pending-device",
            "user_code": "AAAA-BBBB",
            "verification_uri": "https://example.authkit.app/device",
            "verification_uri_complete": "https://example.authkit.app/device?user_code=AAAA-BBBB",
            "expires_in": 300,
            "interval": 1
        })))
        .mount(&workos)
        .await;

    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("device_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "authkit_authorization_code": "authkit_code_test"
        })))
        .mount(&workos)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/auth/station/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "station.access.jwt",
            "refresh_token": "station.refresh",
            "expires_in": 900,
            "refresh_expires_in": 1000
        })))
        .mount(&console)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/auth/station/session"))
        .and(header("authorization", "Bearer station.access.jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "profile_id": "11111111-1111-4111-8111-111111111111",
            "workos_user_id": "user_1",
            "effective_user_id": "11111111-1111-4111-8111-111111111111",
            "email": "trader@example.com",
            "aud": "station",
            "scope": "station:api"
        })))
        .mount(&console)
        .await;

    std::env::set_var("WORKOS_API_BASE_URL", workos.uri());
    std::env::set_var("WORKOS_STATION_CLIENT_ID", "client_test_station");
    std::env::set_var("STATION_ACCESS_TOKEN", "bootstrap-ignored-for-mint");

    let store: Arc<dyn tradeautopsy_agent::StationTokenStore> =
        Arc::new(tradeautopsy_agent::MemoryStationTokenStore::default());

    let _agent = spawn_test_agent_with_options(
        PORT + 1,
        TestAgentOptions {
            upstream_base_url_override: Some(console.uri()),
            station_token_store: Some(store),
            ..Default::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(400)).await;

    let begin = apply_wire_v1(
        client().post(format!(
            "http://127.0.0.1:{}/api/daemon/auth/station/begin",
            PORT + 1
        )),
        "POST",
        "/api/daemon/auth/station/begin",
        &[],
        Default::default(),
    );
    let begin_resp = begin.send().await.expect("begin");
    assert_eq!(begin_resp.status(), 200);

    let complete = apply_wire_v1(
        client().post(format!(
            "http://127.0.0.1:{}/api/daemon/auth/station/complete",
            PORT + 1
        )),
        "POST",
        "/api/daemon/auth/station/complete",
        &[],
        Default::default(),
    );
    let complete_resp = complete.send().await.expect("complete");
    // May fail HTTPS check — assert and adjust prove/mint to allow http under test env.
    let status = complete_resp.status();
    let text = complete_resp.text().await.unwrap_or_default();
    assert_eq!(status, 200, "complete body: {text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["email"], "trader@example.com");
    assert_eq!(body["aud"], "station");
    assert!(body.get("access_token").is_none());
    assert!(body.get("refresh_token").is_none());

    std::env::remove_var("WORKOS_API_BASE_URL");
    std::env::remove_var("WORKOS_STATION_CLIENT_ID");
    std::env::remove_var("STATION_ACCESS_TOKEN");
    let _ = TEST_SECRET;
}

#[tokio::test]
#[serial]
async fn station_auth_complete_mints_from_workos_access_token_when_code_absent() {
    let workos = MockServer::start().await;
    let console = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user_management/authorize/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "pending-device",
            "user_code": "CCCC-DDDD",
            "verification_uri": "https://example.authkit.app/device",
            "verification_uri_complete": "https://example.authkit.app/device?user_code=CCCC-DDDD",
            "expires_in": 300,
            "interval": 1
        })))
        .mount(&workos)
        .await;

    Mock::given(method("POST"))
        .and(path("/user_management/authenticate"))
        .and(body_string_contains("device_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "workos.access.jwt"
        })))
        .mount(&workos)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/auth/station/token"))
        .and(body_string_contains("workos_access_token"))
        .and(body_string_contains("workos.access.jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "station.access.jwt",
            "refresh_token": "station.refresh",
            "expires_in": 900,
            "refresh_expires_in": 1000
        })))
        .mount(&console)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/auth/station/session"))
        .and(header("authorization", "Bearer station.access.jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "profile_id": "11111111-1111-4111-8111-111111111111",
            "workos_user_id": "user_1",
            "effective_user_id": "11111111-1111-4111-8111-111111111111",
            "email": "trader@example.com",
            "aud": "station",
            "scope": "station:api"
        })))
        .mount(&console)
        .await;

    std::env::set_var("WORKOS_API_BASE_URL", workos.uri());
    std::env::set_var("WORKOS_STATION_CLIENT_ID", "client_test_station");
    std::env::set_var("STATION_ACCESS_TOKEN", "bootstrap-ignored-for-mint");

    let store: Arc<dyn tradeautopsy_agent::StationTokenStore> =
        Arc::new(tradeautopsy_agent::MemoryStationTokenStore::default());

    let _agent = spawn_test_agent_with_options(
        PORT + 2,
        TestAgentOptions {
            upstream_base_url_override: Some(console.uri()),
            station_token_store: Some(store),
            ..Default::default()
        },
    );
    tokio::time::sleep(Duration::from_millis(400)).await;

    let begin = apply_wire_v1(
        client().post(format!(
            "http://127.0.0.1:{}/api/daemon/auth/station/begin",
            PORT + 2
        )),
        "POST",
        "/api/daemon/auth/station/begin",
        &[],
        Default::default(),
    );
    let begin_resp = begin.send().await.expect("begin");
    assert_eq!(begin_resp.status(), 200);

    let complete = apply_wire_v1(
        client().post(format!(
            "http://127.0.0.1:{}/api/daemon/auth/station/complete",
            PORT + 2
        )),
        "POST",
        "/api/daemon/auth/station/complete",
        &[],
        Default::default(),
    );
    let complete_resp = complete.send().await.expect("complete");
    let status = complete_resp.status();
    let text = complete_resp.text().await.unwrap_or_default();
    assert_eq!(status, 200, "complete body: {text}");
    let body: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["email"], "trader@example.com");
    assert_eq!(body["aud"], "station");
    assert!(body.get("access_token").is_none());
    assert!(body.get("refresh_token").is_none());

    std::env::remove_var("WORKOS_API_BASE_URL");
    std::env::remove_var("WORKOS_STATION_CLIENT_ID");
    std::env::remove_var("STATION_ACCESS_TOKEN");
}

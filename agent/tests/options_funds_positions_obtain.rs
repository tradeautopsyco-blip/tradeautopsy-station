//! T3 — Binance COM options `obtain(funds|positionbook)` via mock USER_DATA egress.
//!
//! Seams:
//! - `GET /api/station/obtain?adapter=binance_com&book=binance-com-options&operation=funds`
//! - `GET /api/station/obtain?adapter=binance_com&book=binance-com-options&operation=positionbook`
//!
//! Kick is host-mediated HMAC `GET /eapi/v1/marginAccount` and `GET /eapi/v1/position`
//! pointed at wiremock — not a planted AccountBook slot, not live eapi, not spot
//! `/api/v3/account`, not `/fapi/`.
//! Lock: `issues/compliance/locks/binance-com-options.md` (fetch 2026-09-15 IST).

mod common;

use common::{
    apply_wire_v1, client, identity_start_body, spawn_test_agent_with_options, wait_ready,
    TestAgentOptions, WireHeaderOverrides, TEST_BROKER_CONNECTION_ID,
};
use serial_test::serial;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    BrokerAdapter, BrokerCredentialVault, CountingPollAdapter, CredentialBlob,
    MemoryBrokerCredentialVault,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PORT: u16 = 19_656;
const EMPTY_PORT: u16 = 19_657;
const API_KEY: &str = "TA_OPTIONS_FUNDS_KEY_NEVER_IN_OBTAIN";
const API_SECRET: &str = "TA_OPTIONS_FUNDS_SECRET_NEVER_IN_OBTAIN";

fn margin_account_body() -> serde_json::Value {
    serde_json::json!({
        "asset": [
            {
                "asset": "USDT",
                "available": "12.5",
                "initialMargin": "3.25",
                "unrealizedPNL": "1.1"
            },
            {
                "asset": "BNB",
                "available": "0",
                "initialMargin": "0",
                "unrealizedPNL": "9"
            }
        ],
        "greek": [
            {
                "underlying": "BTCUSDT",
                "delta": "0.55937056",
                "theta": "-1",
                "gamma": "0.0001",
                "vega": "2"
            }
        ]
    })
}

fn positions_body() -> serde_json::Value {
    serde_json::json!([
        { "symbol": "BTC-260925-145000-C", "quantity": "2" },
        { "symbol": "ETH-260925-3000-P", "quantity": "0" }
    ])
}

async fn post_binance_com_start(port: u16) {
    let path = "/api/daemon/broker/sync/start";
    let url = format!("http://127.0.0.1:{port}{path}");
    let payload = serde_json::to_vec(&identity_start_body("binance_com")).expect("json");
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
    assert_eq!(resp.status(), 200, "identity-only Start must succeed");
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
        .expect("obtain json")
}

fn holdings_by_asset<'a>(
    funds: &'a serde_json::Value,
) -> std::collections::HashMap<&'a str, &'a serde_json::Value> {
    funds["data"]["holdings"]
        .as_array()
        .expect("holdings")
        .iter()
        .filter_map(|h| h["asset"].as_str().map(|a| (a, h)))
        .collect()
}

fn assert_secrets_stay_off_wire(wire: &str) {
    assert!(!wire.contains(API_KEY), "apiKey must not leak on obtain");
    assert!(
        !wire.contains(API_SECRET),
        "apiSecret must not leak on obtain"
    );
    assert!(!wire.contains("signature"), "HMAC signature must not leak");
}

/// CountingPollAdapter is not `binance_com`, so merge_poll cannot plant the
/// options funds/positions slots. The only way obtain lights is the kick → mock GET.
fn start_opts(base_url: String) -> TestAgentOptions {
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            "binance_com",
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac(API_KEY, API_SECRET),
        )
        .expect("seed vault");
    TestAgentOptions {
        runtime_poll_adapter: Some(Arc::new(CountingPollAdapter::new()) as Arc<dyn BrokerAdapter>),
        broker_base_poll_ms: 80,
        credential_vault: Some(vault as Arc<dyn BrokerCredentialVault>),
        binance_eapi_base_url: Some(base_url),
        ..TestAgentOptions::default()
    }
}

#[tokio::test]
#[serial]
async fn obtain_options_funds_and_positions_via_mock_eapi() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/eapi/v1/marginAccount"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(margin_account_body()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/eapi/v1/position"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(positions_body()))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(PORT, start_opts(server.uri()));
    wait_ready(PORT).await;

    let dark_funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-options&operation=funds",
    )
    .await;
    assert_eq!(
        dark_funds["status"], "unavailable",
        "obtain must stay dark until identity-only Start attaches the Keychain handle"
    );
    assert!(dark_funds["data"].is_null());

    let dark_positions = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-options&operation=positionbook",
    )
    .await;
    assert_eq!(dark_positions["status"], "unavailable");
    assert!(dark_positions["data"].is_null());

    post_binance_com_start(PORT).await;

    let funds = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-options&operation=funds",
    )
    .await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["adapter_id"], "binance_com");
    assert_eq!(funds["book_id"], "binance-com-options");
    assert_eq!(funds["data"]["identity"]["family"], "account");
    assert_eq!(funds["data"]["identity"]["capability_id"], "funds");
    assert_eq!(funds["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(funds["provenance_path"], "/eapi/v1/marginAccount");
    assert_eq!(funds["provenance_adapter_id"], "binance_com");
    assert!(
        funds["data"].is_object(),
        "empty-or-populated funds must never be data:null"
    );

    let by_asset = holdings_by_asset(&funds);
    assert_eq!(by_asset["USDT"]["free"], 12.5);
    assert_eq!(by_asset["USDT"]["locked"], 3.25);
    assert!(
        !by_asset.contains_key("BNB"),
        "zero available+initialMargin must not appear on obtain(funds)"
    );
    assert_eq!(funds["data"]["unrealized_pnl"], 1.1);
    let funds_wire = funds.to_string();
    assert!(
        !funds_wire.contains("0.55937056"),
        "marginAccount.greek[] must not paint obtain(funds)"
    );
    assert_secrets_stay_off_wire(&funds_wire);

    let positions = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-options&operation=positionbook",
    )
    .await;
    assert_eq!(positions["status"], "success");
    assert_eq!(positions["book_id"], "binance-com-options");
    assert_eq!(positions["data"]["identity"]["family"], "account");
    assert_eq!(positions["data"]["identity"]["capability_id"], "positions");
    assert_eq!(positions["data"]["identity"]["physics"], "bounded_snapshot");
    assert_eq!(positions["provenance_path"], "/eapi/v1/position");
    assert!(
        positions["data"].is_object(),
        "positions after drop must never be data:null"
    );
    assert_eq!(positions["data"]["position_count"], 1);
    let row = &positions["data"]["rows"][0];
    assert_eq!(row["symbol"], "BTC-260925-145000-C");
    assert_ne!(row["symbol"], "btc-260925-145000-c");
    assert_eq!(row["net_qty"], 2.0);
    assert_ne!(row["net_qty"], 1.0);
    assert_eq!(row["segment"], "options");
    assert_ne!(row["segment"], "usdm");
    assert_ne!(row["segment"], "spot");
    assert_ne!(row["segment"], "nfo");
    assert_secrets_stay_off_wire(&positions.to_string());

    let spot = obtain(PORT, "adapter=binance_com&operation=funds").await;
    assert_ne!(spot["book_id"], "binance-com-options");
    assert_ne!(spot["provenance_path"], "/eapi/v1/marginAccount");
    if spot["status"] == "success" {
        assert_eq!(spot["book_id"], "binance-com-spot");
        let spot_holdings = holdings_by_asset(&spot);
        if let Some(h) = spot_holdings.get("USDT") {
            assert_ne!(
                h["free"], 12.5,
                "options USDT must not paint slug-default spot funds"
            );
        }
    } else {
        assert_eq!(spot["status"], "unavailable");
        assert!(spot["data"].is_null());
    }

    let greeks = obtain(
        PORT,
        "adapter=binance_com&book=binance-com-options&operation=optiongreeks",
    )
    .await;
    assert_eq!(greeks["status"], "unavailable");
    assert!(greeks["data"].is_null());
    assert_ne!(greeks["provenance_path"], "/eapi/v1/marginAccount");
    let greeks_wire = greeks.to_string();
    assert!(
        !greeks_wire.contains("0.55937056"),
        "do not serve marginAccount.greek[] as optiongreeks"
    );

    handle.abort();
}

#[tokio::test]
#[serial]
async fn obtain_options_funds_empty_asset_is_success_not_null() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/eapi/v1/marginAccount"))
        .and(header("X-MBX-APIKEY", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "asset": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let handle = spawn_test_agent_with_options(EMPTY_PORT, start_opts(server.uri()));
    wait_ready(EMPTY_PORT).await;
    post_binance_com_start(EMPTY_PORT).await;

    let funds = obtain(
        EMPTY_PORT,
        "adapter=binance_com&book=binance-com-options&operation=funds",
    )
    .await;
    assert_eq!(funds["status"], "success");
    assert_eq!(funds["book_id"], "binance-com-options");
    assert!(
        !funds["data"].is_null(),
        "empty holdings after drop is success with data, never data:null"
    );
    assert_eq!(funds["data"]["holdings"].as_array().map(Vec::len), Some(0));
    assert_eq!(funds["provenance_path"], "/eapi/v1/marginAccount");
    assert_secrets_stay_off_wire(&funds.to_string());

    handle.abort();
}

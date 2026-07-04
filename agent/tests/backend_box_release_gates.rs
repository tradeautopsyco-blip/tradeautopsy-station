//! Issue #20 — CI release gates for Backend Box v1 (no real Binance.US credentials).

use serde_json::json;
use tradeautopsy_agent::{
    BrokerBehavioralRecorder, BrokerConnectionIdentityFields, BrokerValidationAdapter,
    FakeBinanceUSValidationAdapter, PermissionPosture, RedactionBoundary, ValidationResult,
};

#[tokio::test]
async fn withdraw_permission_hard_block_via_fake_adapter() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_WITHDRAW", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::WithdrawDetected)
    );
}

#[test]
fn redaction_blocks_raw_auth_and_broker_payload_keys() {
    let payload = json!({
        "apiSecret": "abc",
        "raw_response": "{\"foo\":1}",
        "order_id": "42"
    });
    assert!(RedactionBoundary::contains_forbidden_material(&payload, &[]));
    let redacted = RedactionBoundary::redact_for_upload(payload, &["abc"]).expect("redacted");
    assert!(redacted.get("apiSecret").is_none());
    assert!(redacted.get("raw_response").is_none());
}

#[test]
fn behavioral_opt_out_suppresses_upload_path() {
    let recorder = BrokerBehavioralRecorder::new();
    recorder.set_opted_out(true);
    let identity = BrokerConnectionIdentityFields {
        broker_connection_id: "id".into(),
        broker_slug: "binance_us".into(),
        asset_class: "crypto".into(),
        environment: "prod".into(),
    };
    assert!(!recorder.record_normalized(
        &identity,
        "broker_sync_stopped",
        json!({}),
        None,
    ));
    assert!(recorder.recorded_events().is_empty());
}

#[tokio::test]
async fn read_only_fake_adapter_allows_connect_path() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_READ_ONLY", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::ReadOnlyConfirmed)
    );
}

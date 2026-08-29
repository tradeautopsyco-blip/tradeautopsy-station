//! Issue #18 — behavioral opt-out + redaction boundary.

use serde_json::json;
use tradeautopsy_agent::{
    BrokerBehavioralRecorder, BrokerConnectionIdentityFields, RedactionBoundary,
};

fn sample_identity() -> BrokerConnectionIdentityFields {
    BrokerConnectionIdentityFields {
        broker_connection_id: "00000000-0000-4000-8000-000000000001".into(),
        broker_slug: "binance_us".into(),
        asset_class: "crypto".into(),
        environment: "prod".into(),
    }
}

#[test]
fn redaction_strips_raw_broker_payload_keys() {
    let raw = json!({
        "apiKey": "secret-key",
        "raw_response": "{\"balances\":[]}",
        "normalized": { "order_id": "123" }
    });
    assert!(RedactionBoundary::contains_forbidden_material(&raw, &[]));

    let redacted = RedactionBoundary::redact_for_upload(raw, &["secret-key"]).expect("redacted");
    assert!(redacted.get("apiKey").is_none());
    assert!(redacted.get("raw_response").is_none());
    assert_eq!(redacted["normalized"]["order_id"], "123");
}

#[test]
fn redaction_rejects_secret_literals_in_serialized_output() {
    let payload = json!({ "message": "sync ok", "order_id": "42" });
    assert!(RedactionBoundary::redact_for_upload(payload.clone(), &["super-secret"]).is_some());
    assert!(RedactionBoundary::redact_for_upload(
        json!({ "message": "leaked super-secret token" }),
        &["super-secret"]
    )
    .is_none());
}

#[test]
fn behavioral_opt_out_suppresses_uploads_while_recorder_allows_when_enabled() {
    let recorder = BrokerBehavioralRecorder::new();
    recorder.track_secret_literal("hunter2".into());
    let identity = sample_identity();
    let completeness = json!({
        "fills_trade_history": { "current": true },
        "balances_holdings": { "current": false },
        "open_orders": { "current": true }
    });

    recorder.set_opted_out(true);
    assert!(!recorder.record_normalized(
        &identity,
        "broker_partial_sync",
        json!({ "failing_class": "balances_holdings" }),
        Some(completeness.clone()),
    ));
    assert!(recorder.recorded_events().is_empty());

    recorder.set_opted_out(false);
    assert!(recorder.record_normalized(
        &identity,
        "broker_partial_sync",
        json!({ "failing_class": "balances_holdings" }),
        Some(completeness.clone()),
    ));
    let events = recorder.recorded_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event_type"], "broker_partial_sync");
    assert_eq!(events[0]["environment"], "prod");
    assert_eq!(
        events[0]["completeness"]["balances_holdings"]["current"],
        false
    );
}

//! Issue #12 — deterministic fake Binance.US validation adapter for CI.

use tradeautopsy_agent::{
    BrokerValidationAdapter, FakeBinanceUSValidationAdapter, LiveBinanceUSValidationAdapter,
    PermissionPosture, ValidationFailure, ValidationResult,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn live_adapter_maps_api_restrictions_to_read_only_posture() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sapi/v1/account/apiRestrictions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "enableReading": true,
            "enableSpotAndMarginTrading": false,
            "enableWithdrawals": false
        })))
        .mount(&server)
        .await;

    let adapter = LiveBinanceUSValidationAdapter::new(server.uri());
    let result = adapter
        .validate_credentials("live-key", "live-secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::ReadOnlyConfirmed)
    );
}

#[tokio::test]
async fn fake_adapter_read_only_scenario_returns_read_only_confirmed() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_READ_ONLY", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::ReadOnlyConfirmed)
    );
}

#[tokio::test]
async fn fake_adapter_withdraw_scenario_returns_withdraw_detected() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_WITHDRAW", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::WithdrawDetected)
    );
}

#[tokio::test]
async fn fake_adapter_invalid_credentials_returns_permanent_failure() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_INVALID", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials)
    );
}

#[tokio::test]
async fn fake_adapter_rate_limit_returns_transient_failure() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_RATE_LIMIT", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::TransientFailure(ValidationFailure::RateLimited)
    );
}

#[tokio::test]
async fn fake_adapter_network_failure_returns_transient_failure() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_NETWORK", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::TransientFailure(ValidationFailure::NetworkUnavailable)
    );
}

#[tokio::test]
async fn fake_adapter_broker_unavailable_returns_transient_failure() {
    let adapter = FakeBinanceUSValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_UNAVAILABLE", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::TransientFailure(ValidationFailure::BrokerUnavailable)
    );
}

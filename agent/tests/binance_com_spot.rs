//! Binance Global spot client + validation tests.

use tradeautopsy_agent::{
    BinanceComSpotClient, BrokerValidationAdapter, FakeBinanceComValidationAdapter,
    LiveBinanceComValidationAdapter, PermissionPosture, ValidationFailure, ValidationResult,
};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn live_com_adapter_maps_api_restrictions_to_read_only_posture() {
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

    let adapter = LiveBinanceComValidationAdapter::new(server.uri());
    let result = adapter
        .validate_credentials("live-key", "live-secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::ReadOnlyConfirmed)
    );
}

#[tokio::test]
async fn fake_com_adapter_withdraw_scenario_returns_withdraw_detected() {
    let adapter = FakeBinanceComValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_COM_WITHDRAW", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::Success(PermissionPosture::WithdrawDetected)
    );
}

#[tokio::test]
async fn spot_client_fetches_account_info() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/account"))
        .and(header("X-MBX-APIKEY", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "canTrade": true,
            "canWithdraw": false,
            "canDeposit": true,
            "balances": [
                { "asset": "BTC", "free": "0.01000000", "locked": "0.00000000" }
            ]
        })))
        .mount(&server)
        .await;

    let client = BinanceComSpotClient::with_base_url(server.uri(), "test-key", "test-secret");
    let account = client.fetch_account_info().await.expect("account");
    assert!(account.can_trade);
    assert!(!account.can_withdraw);
    assert_eq!(account.balances.len(), 1);
    assert_eq!(account.balances[0].asset, "BTC");
}

#[tokio::test]
async fn spot_client_fetches_my_trades() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/myTrades"))
        .and(header("X-MBX-APIKEY", "test-key"))
        .and(query_param("symbol", "BTCUSDT"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!([{
                "symbol": "BTCUSDT",
                "id": 28457,
                "orderId": 100234,
                "price": "4.00000100",
                "qty": "12.00000000",
                "commission": "0.01200000",
                "commissionAsset": "BNB",
                "time": 1499865549590_i64,
                "isBuyer": true
            }])),
        )
        .mount(&server)
        .await;

    let client = BinanceComSpotClient::with_base_url(server.uri(), "test-key", "test-secret");
    let trades = client
        .fetch_my_trades("BTCUSDT", None, None)
        .await
        .expect("trades");
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].symbol, "BTCUSDT");
    assert!(trades[0].is_buyer);
}

#[tokio::test]
async fn fake_com_invalid_credentials_returns_permanent_failure() {
    let adapter = FakeBinanceComValidationAdapter::read_only();
    let result = adapter
        .validate_credentials("TA_FAKE_COM_INVALID", "secret")
        .await;
    assert_eq!(
        result,
        ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials)
    );
}

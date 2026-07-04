//! Binance.US credential validation adapter boundary (issue #12).

use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionPosture {
    ReadOnlyConfirmed,
    TradeEnabled,
    Unverifiable,
    WithdrawDetected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationFailure {
    InvalidCredentials,
    InsufficientPermissions,
    NetworkUnavailable,
    RateLimited,
    BrokerUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    Success(PermissionPosture),
    TransientFailure(ValidationFailure),
    PermanentFailure(ValidationFailure),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRestrictions {
    enable_reading: Option<bool>,
    enable_spot_and_margin_trading: Option<bool>,
    enable_withdrawals: Option<bool>,
}

#[async_trait]
pub trait BrokerValidationAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    async fn validate_credentials(&self, api_key: &str, api_secret: &str) -> ValidationResult;
}

fn classify_restrictions(restrictions: &ApiRestrictions) -> PermissionPosture {
    if restrictions.enable_withdrawals == Some(true) {
        return PermissionPosture::WithdrawDetected;
    }
    if restrictions.enable_spot_and_margin_trading == Some(true) {
        return PermissionPosture::TradeEnabled;
    }
    if restrictions.enable_reading == Some(true) {
        return PermissionPosture::ReadOnlyConfirmed;
    }
    PermissionPosture::Unverifiable
}

fn classify_http_error(status: u16, binance_code: Option<i64>) -> ValidationResult {
    if status == 429 || binance_code == Some(-1003) {
        return ValidationResult::TransientFailure(ValidationFailure::RateLimited);
    }
    if (500..600).contains(&status) || status == 418 {
        return ValidationResult::TransientFailure(ValidationFailure::BrokerUnavailable);
    }
    if status == 401 || status == 403 || binance_code == Some(-2015) || binance_code == Some(-2014)
    {
        return ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials);
    }
    ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials)
}

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Live Binance.US validation via `/sapi/v1/account/apiRestrictions`.
pub struct LiveBinanceUSValidationAdapter {
    base_url: String,
    client: reqwest::Client,
}

impl LiveBinanceUSValidationAdapter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn map_response(status: u16, body: &str) -> ValidationResult {
        if !(200..300).contains(&status) {
            let code = serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("code").and_then(|c| c.as_i64()));
            return classify_http_error(status, code);
        }

        match serde_json::from_str::<ApiRestrictions>(body) {
            Ok(restrictions) => ValidationResult::Success(classify_restrictions(&restrictions)),
            Err(_) => ValidationResult::Success(PermissionPosture::Unverifiable),
        }
    }
}

#[async_trait]
impl BrokerValidationAdapter for LiveBinanceUSValidationAdapter {
    fn name(&self) -> &'static str {
        "live_binance_us"
    }

    async fn validate_credentials(&self, api_key: &str, api_secret: &str) -> ValidationResult {
        let timestamp = Utc::now().timestamp_millis();
        let unsigned = format!("timestamp={timestamp}");
        let signature = sign_query(api_secret, &unsigned);
        let url = format!(
            "{}/sapi/v1/account/apiRestrictions?{unsigned}&signature={signature}",
            self.base_url
        );

        let response = match self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", api_key)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(_) => {
                return ValidationResult::TransientFailure(ValidationFailure::NetworkUnavailable);
            }
        };

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        Self::map_response(status, &body)
    }
}

/// Deterministic fake Binance.US adapter for CI — keyed off magic API keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeBinanceUSValidationAdapter;

impl FakeBinanceUSValidationAdapter {
    pub fn read_only() -> Self {
        Self
    }
}

#[async_trait]
impl BrokerValidationAdapter for FakeBinanceUSValidationAdapter {
    fn name(&self) -> &'static str {
        "fake_binance_us"
    }

    async fn validate_credentials(&self, api_key: &str, _api_secret: &str) -> ValidationResult {
        match api_key {
            "TA_FAKE_READ_ONLY" => ValidationResult::Success(PermissionPosture::ReadOnlyConfirmed),
            "TA_FAKE_TRADE" => ValidationResult::Success(PermissionPosture::TradeEnabled),
            "TA_FAKE_UNVERIFIABLE" => ValidationResult::Success(PermissionPosture::Unverifiable),
            "TA_FAKE_WITHDRAW" => ValidationResult::Success(PermissionPosture::WithdrawDetected),
            "TA_FAKE_INVALID" => {
                ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials)
            }
            "TA_FAKE_NETWORK" => {
                ValidationResult::TransientFailure(ValidationFailure::NetworkUnavailable)
            }
            "TA_FAKE_RATE_LIMIT" => {
                ValidationResult::TransientFailure(ValidationFailure::RateLimited)
            }
            "TA_FAKE_UNAVAILABLE" => {
                ValidationResult::TransientFailure(ValidationFailure::BrokerUnavailable)
            }
            _ => ValidationResult::PermanentFailure(ValidationFailure::InvalidCredentials),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_read_only_restrictions() {
        let posture = classify_restrictions(&ApiRestrictions {
            enable_reading: Some(true),
            enable_spot_and_margin_trading: Some(false),
            enable_withdrawals: Some(false),
        });
        assert_eq!(posture, PermissionPosture::ReadOnlyConfirmed);
    }

    #[test]
    fn classify_withdraw_takes_precedence() {
        let posture = classify_restrictions(&ApiRestrictions {
            enable_reading: Some(true),
            enable_spot_and_margin_trading: Some(true),
            enable_withdrawals: Some(true),
        });
        assert_eq!(posture, PermissionPosture::WithdrawDetected);
    }

    #[test]
    fn map_response_classifies_rate_limit() {
        let result = LiveBinanceUSValidationAdapter::map_response(429, "");
        assert_eq!(
            result,
            ValidationResult::TransientFailure(ValidationFailure::RateLimited)
        );
    }
}

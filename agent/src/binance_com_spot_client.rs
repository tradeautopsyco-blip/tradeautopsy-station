//! Binance Global spot read-only client — `api.binance.com`.
//!
//! Ref: `docs/reference/crypto/binance-global/spot/REST.md`
//! Scope: `GET /api/v3/account`, `GET /api/v3/myTrades` only.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const DEFAULT_BASE_URL: &str = "https://api.binance.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceComSpotError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
}

impl BinanceComSpotError {
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::Http { status: 429, .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceComAccountInfo {
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub balances: Vec<BinanceComBalance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceComBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceComMyTrade {
    pub id: i64,
    pub order_id: i64,
    pub symbol: String,
    pub price: String,
    pub qty: String,
    pub commission: String,
    pub commission_asset: String,
    pub time_ms: i64,
    pub is_buyer: bool,
}

pub struct BinanceComSpotClient {
    base_url: String,
    api_key: String,
    api_secret: String,
    client: reqwest::Client,
}

impl BinanceComSpotClient {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self::with_base_url(DEFAULT_BASE_URL, api_key, api_secret)
    }

    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            client: reqwest::Client::new(),
        }
    }

    /// `GET /api/v3/account` — USER_DATA, weight 20 (REST.md).
    pub async fn fetch_account_info(&self) -> Result<BinanceComAccountInfo, BinanceComSpotError> {
        let body = self.signed_get("/api/v3/account", &[]).await?;
        parse_account_info(&body)
    }

    /// `GET /api/v3/myTrades` — USER_DATA, weight 20 (REST.md).
    pub async fn fetch_my_trades(
        &self,
        symbol: &str,
        start_time_ms: Option<i64>,
        limit: Option<u32>,
    ) -> Result<Vec<BinanceComMyTrade>, BinanceComSpotError> {
        let mut params: Vec<(&str, String)> = vec![("symbol", symbol.to_ascii_uppercase())];
        if let Some(ms) = start_time_ms {
            params.push(("startTime", ms.to_string()));
        }
        if let Some(n) = limit {
            params.push(("limit", n.to_string()));
        }
        let param_refs: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect();
        let body = self.signed_get("/api/v3/myTrades", &param_refs).await?;
        parse_my_trades(&body)
    }

    async fn signed_get(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<String, BinanceComSpotError> {
        let timestamp = Utc::now().timestamp_millis().to_string();
        let mut pairs: Vec<(&str, &str)> = extra_params.to_vec();
        pairs.push(("timestamp", timestamp.as_str()));
        pairs.sort_by_key(|(k, _)| *k);
        let query = pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let signature = sign_query(&self.api_secret, &query);
        let url = format!("{}{path}?{query}&signature={signature}", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| BinanceComSpotError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| BinanceComSpotError::Network(e.to_string()))?;
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(BinanceComSpotError::Http { status, body })
        }
    }
}

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn parse_account_info(body: &str) -> Result<BinanceComAccountInfo, BinanceComSpotError> {
    let parsed: AccountResponse =
        serde_json::from_str(body).map_err(|e| BinanceComSpotError::Parse(e.to_string()))?;
    Ok(BinanceComAccountInfo {
        can_trade: parsed.can_trade,
        can_withdraw: parsed.can_withdraw,
        can_deposit: parsed.can_deposit,
        balances: parsed
            .balances
            .into_iter()
            .map(|b| BinanceComBalance {
                asset: b.asset,
                free: b.free,
                locked: b.locked,
            })
            .collect(),
    })
}

fn parse_my_trades(body: &str) -> Result<Vec<BinanceComMyTrade>, BinanceComSpotError> {
    let parsed: Vec<MyTradeResponse> =
        serde_json::from_str(body).map_err(|e| BinanceComSpotError::Parse(e.to_string()))?;
    Ok(parsed
        .into_iter()
        .map(|t| BinanceComMyTrade {
            id: t.id,
            order_id: t.order_id,
            symbol: t.symbol,
            price: t.price,
            qty: t.qty,
            commission: t.commission,
            commission_asset: t.commission_asset,
            time_ms: t.time,
            is_buyer: t.is_buyer,
        })
        .collect())
}

pub fn my_trade_to_broker_fill(trade: &BinanceComMyTrade) -> crate::broker::BrokerFill {
    let side = if trade.is_buyer { "BUY" } else { "SELL" }.to_string();
    let filled_at = DateTime::<Utc>::from_timestamp_millis(trade.time_ms)
        .unwrap_or_else(Utc::now);
    crate::broker::BrokerFill {
        fill_id: trade.id.to_string(),
        trade_id: trade.id.to_string(),
        symbol: trade.symbol.clone(),
        side,
        qty: trade.qty.parse().unwrap_or(0.0),
        price: trade.price.parse().unwrap_or(0.0),
        filled_at,
        broker: "binance_com".to_string(),
        fee_amount: trade.commission.parse().ok(),
        fee_asset: Some(trade.commission_asset.clone()),
        currency: Some("USDT".to_string()),
        product: None,
        exchange_segment: None,
        instrument_type: None,
        lot: None,
    }
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    #[serde(rename = "canTrade")]
    can_trade: bool,
    #[serde(rename = "canWithdraw")]
    can_withdraw: bool,
    #[serde(rename = "canDeposit")]
    can_deposit: bool,
    balances: Vec<BalanceResponse>,
}

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Debug, Deserialize)]
struct MyTradeResponse {
    id: i64,
    #[serde(rename = "orderId")]
    order_id: i64,
    symbol: String,
    price: String,
    qty: String,
    commission: String,
    #[serde(rename = "commissionAsset")]
    commission_asset: String,
    time: i64,
    #[serde(rename = "isBuyer")]
    is_buyer: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_query_matches_binance_hmac_sha256() {
        let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
        mac.update(b"symbol=BTCUSDT&timestamp=1234567890");
        assert_eq!(
            sign_query("secret", "symbol=BTCUSDT&timestamp=1234567890"),
            hex::encode(mac.finalize().into_bytes())
        );
    }

    #[test]
    fn parses_account_info_json() {
        let json = r#"{
            "canTrade": true,
            "canWithdraw": false,
            "canDeposit": true,
            "balances": [
                { "asset": "BTC", "free": "0.01000000", "locked": "0.00000000" }
            ]
        }"#;
        let info = parse_account_info(json).expect("parse");
        assert!(info.can_trade);
        assert!(!info.can_withdraw);
        assert_eq!(info.balances.len(), 1);
        assert_eq!(info.balances[0].asset, "BTC");
    }

    #[test]
    fn parses_my_trades_json() {
        let json = r#"[{
            "symbol": "BTCUSDT",
            "id": 28457,
            "orderId": 100234,
            "price": "4.00000100",
            "qty": "12.00000000",
            "commission": "0.01200000",
            "commissionAsset": "BNB",
            "time": 1499865549590,
            "isBuyer": true
        }]"#;
        let trades = parse_my_trades(json).expect("parse");
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, "BTCUSDT");
        assert!(trades[0].is_buyer);
    }
}

//! Binance Global options read-only client — `eapi.binance.com`.
//!
//! Ref: `docs/reference/crypto/binance-global/options/REST.md`
//! Scope: `GET /eapi/v1/userTrades` (Slice 4).

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const DEFAULT_BASE_URL: &str = "https://eapi.binance.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceComOptionsError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
}

impl BinanceComOptionsError {
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::Http { status: 429, .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceComUserTrade {
    pub id: i64,
    pub symbol: String,
    pub price: String,
    pub qty: String,
    pub side: String,
    pub time_ms: i64,
}

pub struct BinanceComOptionsClient {
    base_url: String,
    api_key: String,
    api_secret: String,
    transport: OptionsTransport,
}

enum OptionsTransport {
    Egress,
    Direct(reqwest::Client),
}

impl BinanceComOptionsClient {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            transport: OptionsTransport::Egress,
        }
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
            transport: OptionsTransport::Direct(crate::egress::shared_client()),
        }
    }

    /// `GET /eapi/v1/userTrades` — USER_DATA (REST.md Slice 4).
    pub async fn fetch_user_trades(
        &self,
        symbol: &str,
    ) -> Result<Vec<BinanceComUserTrade>, BinanceComOptionsError> {
        let params = [("symbol", symbol.trim())];
        let body = self.signed_get("/eapi/v1/userTrades", &params).await?;
        parse_user_trades(&body)
    }

    async fn signed_get(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<String, BinanceComOptionsError> {
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
        let signed_query = format!("{query}&signature={signature}");
        let url = format!("{}{path}?{signed_query}", self.base_url);

        let (status, body) = match &self.transport {
            OptionsTransport::Egress => {
                let call = crate::egress::EgressCall::get(
                    crate::data::BINANCE_COM_OPTIONS_BOOK_ID,
                    "eapi.binance.com",
                    path,
                    crate::egress::Lane::PrivateRead,
                )
                .with_query(signed_query)
                .with_header("X-MBX-APIKEY", &self.api_key);
                let resp = crate::egress::shared()
                    .send(&call)
                    .await
                    .map_err(|e| BinanceComOptionsError::Network(e.to_string()))?;
                (resp.status, resp.body)
            }
            OptionsTransport::Direct(client) => {
                let response = client
                    .get(&url)
                    .header("X-MBX-APIKEY", &self.api_key)
                    .send()
                    .await
                    .map_err(|e| BinanceComOptionsError::Network(e.to_string()))?;
                let status = response.status().as_u16();
                let body = response
                    .text()
                    .await
                    .map_err(|e| BinanceComOptionsError::Network(e.to_string()))?;
                (status, body)
            }
        };

        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(BinanceComOptionsError::Http { status, body })
        }
    }
}

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn parse_user_trades(body: &str) -> Result<Vec<BinanceComUserTrade>, BinanceComOptionsError> {
    let parsed: Vec<UserTradeResponse> =
        serde_json::from_str(body).map_err(|e| BinanceComOptionsError::Parse(e.to_string()))?;
    Ok(parsed
        .into_iter()
        .map(|t| BinanceComUserTrade {
            id: t.id,
            symbol: t.symbol,
            price: t.price,
            qty: t.qty,
            side: t.side,
            time_ms: t.time,
        })
        .collect())
}

pub fn user_trade_to_broker_fill(trade: &BinanceComUserTrade) -> crate::broker::BrokerFill {
    let filled_at = DateTime::<Utc>::from_timestamp_millis(trade.time_ms).unwrap_or_else(Utc::now);
    crate::broker::BrokerFill {
        fill_id: trade.id.to_string(),
        trade_id: trade.id.to_string(),
        symbol: trade.symbol.clone(),
        side: trade.side.clone(),
        qty: trade.qty.parse().unwrap_or(0.0),
        price: trade.price.parse().unwrap_or(0.0),
        filled_at,
        broker: "binance_com".to_string(),
        fee_amount: None,
        fee_asset: None,
        currency: None,
        product: None,
        exchange_segment: None,
        instrument_type: None,
        lot: None,
    }
}

#[derive(Debug, Deserialize)]
struct UserTradeResponse {
    id: i64,
    symbol: String,
    price: String,
    qty: String,
    side: String,
    time: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_query_matches_binance_hmac_sha256() {
        let mut mac = HmacSha256::new_from_slice(b"secret").unwrap();
        mac.update(b"symbol=BTC-200730-9000-C&timestamp=1234567890");
        assert_eq!(
            sign_query("secret", "symbol=BTC-200730-9000-C&timestamp=1234567890"),
            hex::encode(mac.finalize().into_bytes())
        );
    }

    #[test]
    fn parses_user_trades_json() {
        let json = r#"[{
            "id": 28457,
            "symbol": "BTC-200730-9000-C",
            "price": "1000.000",
            "qty": "0.1000",
            "side": "BUY",
            "time": 1595894400000,
            "orderId": 100234,
            "fee": "0.01200000",
            "feeAsset": "USDT"
        }]"#;
        let trades = parse_user_trades(json).expect("parse");
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].symbol, "BTC-200730-9000-C");
        assert_eq!(trades[0].side, "BUY");
        assert_eq!(trades[0].price, "1000.000");
    }
}

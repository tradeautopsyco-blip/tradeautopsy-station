//! Binance Coin-M read-only client — `dapi.binance.com`.
//!
//! Lock: `issues/compliance/locks/binance-com-coinm.md` (fetch 2026-09-15 IST;
//! path oracle 2026-09-19 IST).
//! Oracle: `/Users/bishnu/binance-connector-rust` `binance-sdk` 70.1.0 @ `592f16b`
//! feature `derivatives_trading_coin_futures`. DualNoBlend vs USDM.

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding, BrokerPositionRow};
use crate::coinm_realized_pnl::parse_position_amt;
use crate::data::BINANCE_COM_COINM_BOOK_ID;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_BASE_URL: &str = "https://dapi.binance.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceComCoinmError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
}

pub struct BinanceComCoinmClient {
    base_url: String,
    api_key: String,
    api_secret: String,
    transport: CoinmTransport,
}

enum CoinmTransport {
    Egress,
    /// Wiremock / non-venue base. Unmetered: R0 would refuse a mock host.
    Direct(reqwest::Client),
}

impl BinanceComCoinmClient {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            transport: CoinmTransport::Egress,
        }
    }

    /// Point this client at a non-venue base URL (wiremock). Prod stays `new()`.
    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            transport: CoinmTransport::Direct(crate::egress::shared_client()),
        }
    }

    pub async fn fetch_balance(&self) -> Result<BrokerBalancesSnapshot, BinanceComCoinmError> {
        let body = self.signed_get("/dapi/v1/balance", &[]).await?;
        parse_coinm_balance(&body)
    }

    pub async fn fetch_positions(&self) -> Result<Vec<BrokerPositionRow>, BinanceComCoinmError> {
        let body = self.signed_get("/dapi/v1/positionRisk", &[]).await?;
        parse_coinm_positions(&body)
    }

    pub async fn fetch_force_orders(&self) -> Result<serde_json::Value, BinanceComCoinmError> {
        let body = self.signed_get("/dapi/v1/forceOrders", &[]).await?;
        serde_json::from_str(&body).map_err(|e| BinanceComCoinmError::Parse(e.to_string()))
    }

    async fn signed_get(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<String, BinanceComCoinmError> {
        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
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
            CoinmTransport::Egress => {
                let call = crate::egress::EgressCall::get(
                    BINANCE_COM_COINM_BOOK_ID,
                    "dapi.binance.com",
                    path,
                    crate::egress::Lane::PrivateRead,
                )
                .with_query(signed_query)
                .with_header("X-MBX-APIKEY", &self.api_key);
                let resp = crate::egress::shared()
                    .send(&call)
                    .await
                    .map_err(|e| BinanceComCoinmError::Network(e.to_string()))?;
                (resp.status, resp.body)
            }
            CoinmTransport::Direct(client) => {
                let response = client
                    .get(&url)
                    .header("X-MBX-APIKEY", &self.api_key)
                    .send()
                    .await
                    .map_err(|e| BinanceComCoinmError::Network(e.to_string()))?;
                let status = response.status().as_u16();
                let body = response
                    .text()
                    .await
                    .map_err(|e| BinanceComCoinmError::Network(e.to_string()))?;
                (status, body)
            }
        };
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(BinanceComCoinmError::Http { status, body })
        }
    }
}

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn parse_coinm_balance(body: &str) -> Result<BrokerBalancesSnapshot, BinanceComCoinmError> {
    let rows: Vec<CoinmBalanceRow> =
        serde_json::from_str(body).map_err(|e| BinanceComCoinmError::Parse(e.to_string()))?;
    let holdings = rows
        .into_iter()
        .filter_map(|row| {
            let free: f64 = row.available_balance.parse().ok()?;
            if free <= 0.0 {
                return None;
            }
            Some(BrokerHolding {
                asset: row.asset,
                free,
                locked: 0.0,
            })
        })
        .collect();
    Ok(BrokerBalancesSnapshot {
        holdings,
        unrealized_pnl: None,
    })
}

pub fn parse_coinm_positions(body: &str) -> Result<Vec<BrokerPositionRow>, BinanceComCoinmError> {
    let rows: Vec<CoinmPositionRow> =
        serde_json::from_str(body).map_err(|e| BinanceComCoinmError::Parse(e.to_string()))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let net_qty = parse_position_amt(&row.position_amt)?;
            Some(BrokerPositionRow {
                symbol: row.symbol.clone(),
                exchange_segment: "coinm".into(),
                product: String::new(),
                net_qty,
                trading_symbol: row.symbol,
            })
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct CoinmBalanceRow {
    asset: String,
    #[serde(rename = "availableBalance")]
    available_balance: String,
}

#[derive(Debug, Deserialize)]
struct CoinmPositionRow {
    symbol: String,
    #[serde(rename = "positionAmt")]
    position_amt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coinm_segment_not_usdm() {
        let rows = parse_coinm_positions(r#"[{"symbol":"BTCUSD_PERP","positionAmt":"2"}]"#)
            .expect("parse");
        assert_eq!(rows[0].exchange_segment, "coinm");
        assert_ne!(rows[0].exchange_segment, "usdm");
    }
}

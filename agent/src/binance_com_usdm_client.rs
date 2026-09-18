//! Binance USDM read-only client — `fapi.binance.com`.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md` (fetch 2026-09-15 IST;
//! path oracle 2026-09-19 IST).
//! Oracle (cite, not a dep): `/Users/bishnu/binance-connector-rust`
//! `binance-sdk` 70.1.0 @ `592f16b` feature `derivatives_trading_usds_futures`.
//! USER_DATA HMAC on `/fapi/v3/balance`, `/fapi/v3/positionRisk`, `/fapi/v1/forceOrders`.
//! Do not call SDK `new_order` / `POST /fapi/v1/order`.

use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding, BrokerPositionRow};
use crate::data::BINANCE_COM_USDM_BOOK_ID;
use crate::usdm_realized_pnl::parse_position_amt;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_BASE_URL: &str = "https://fapi.binance.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceComUsdmError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
}

pub struct BinanceComUsdmClient {
    base_url: String,
    api_key: String,
    api_secret: String,
    transport: UsdmTransport,
}

enum UsdmTransport {
    Egress,
    /// Wiremock / non-venue base. Unmetered: R0 would refuse a mock host.
    Direct(reqwest::Client),
}

impl BinanceComUsdmClient {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            transport: UsdmTransport::Egress,
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
            transport: UsdmTransport::Direct(crate::egress::shared_client()),
        }
    }

    pub async fn fetch_balance(&self) -> Result<BrokerBalancesSnapshot, BinanceComUsdmError> {
        let body = self.signed_get("/fapi/v3/balance", &[]).await?;
        parse_usdm_balance(&body)
    }

    pub async fn fetch_positions(&self) -> Result<Vec<BrokerPositionRow>, BinanceComUsdmError> {
        let body = self.signed_get("/fapi/v3/positionRisk", &[]).await?;
        parse_usdm_positions(&body)
    }

    pub async fn fetch_force_orders(&self) -> Result<serde_json::Value, BinanceComUsdmError> {
        let body = self.signed_get("/fapi/v1/forceOrders", &[]).await?;
        serde_json::from_str(&body).map_err(|e| BinanceComUsdmError::Parse(e.to_string()))
    }

    async fn signed_get(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<String, BinanceComUsdmError> {
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
            UsdmTransport::Egress => {
                let call = crate::egress::EgressCall::get(
                    BINANCE_COM_USDM_BOOK_ID,
                    "fapi.binance.com",
                    path,
                    crate::egress::Lane::PrivateRead,
                )
                .with_query(signed_query)
                .with_header("X-MBX-APIKEY", &self.api_key);
                let resp = crate::egress::shared()
                    .send(&call)
                    .await
                    .map_err(|e| BinanceComUsdmError::Network(e.to_string()))?;
                (resp.status, resp.body)
            }
            UsdmTransport::Direct(client) => {
                let response = client
                    .get(&url)
                    .header("X-MBX-APIKEY", &self.api_key)
                    .send()
                    .await
                    .map_err(|e| BinanceComUsdmError::Network(e.to_string()))?;
                let status = response.status().as_u16();
                let body = response
                    .text()
                    .await
                    .map_err(|e| BinanceComUsdmError::Network(e.to_string()))?;
                (status, body)
            }
        };
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(BinanceComUsdmError::Http { status, body })
        }
    }
}

fn sign_query(api_secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(api_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub fn parse_usdm_balance(body: &str) -> Result<BrokerBalancesSnapshot, BinanceComUsdmError> {
    let rows: Vec<UsdmBalanceRow> =
        serde_json::from_str(body).map_err(|e| BinanceComUsdmError::Parse(e.to_string()))?;
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

pub fn parse_usdm_positions(body: &str) -> Result<Vec<BrokerPositionRow>, BinanceComUsdmError> {
    let rows: Vec<UsdmPositionRow> =
        serde_json::from_str(body).map_err(|e| BinanceComUsdmError::Parse(e.to_string()))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let net_qty = parse_position_amt(&row.position_amt)?;
            Some(BrokerPositionRow {
                symbol: row.symbol.clone(),
                exchange_segment: "usdm".into(),
                product: String::new(),
                net_qty,
                trading_symbol: row.symbol,
            })
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct UsdmBalanceRow {
    asset: String,
    #[serde(rename = "availableBalance")]
    available_balance: String,
}

#[derive(Debug, Deserialize)]
struct UsdmPositionRow {
    symbol: String,
    #[serde(rename = "positionAmt")]
    position_amt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_available_balance_drops_zero() {
        let snap = parse_usdm_balance(
            r#"[{"asset":"USDT","availableBalance":"23.72"},{"asset":"BNB","availableBalance":"0"}]"#,
        )
        .expect("parse");
        assert_eq!(snap.holdings.len(), 1);
        assert_eq!(snap.holdings[0].asset, "USDT");
        assert_eq!(snap.holdings[0].free, 23.72);
        assert_eq!(snap.holdings[0].locked, 0.0);
        assert!(snap.unrealized_pnl.is_none());
    }

    #[test]
    fn skips_zero_position_amt_keeps_qty_gt_1() {
        let rows = parse_usdm_positions(
            r#"[{"symbol":"BTCUSDT","positionAmt":"0"},{"symbol":"ETHUSDT","positionAmt":"2"}]"#,
        )
        .expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "ETHUSDT");
        assert_eq!(rows[0].net_qty, 2.0);
        assert_eq!(rows[0].exchange_segment, "usdm");
    }
}

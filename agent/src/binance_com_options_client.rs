//! Binance Global options read-only client — `eapi.binance.com`.
//!
//! Ref: `docs/reference/crypto/binance-global/options/REST.md`
//! Lock: `issues/compliance/locks/binance-com-options.md` (funds/positions 2026-09-15 IST;
//! path oracle 2026-09-19 IST).
//! Oracle: `/Users/bishnu/binance-connector-rust` `binance-sdk` 70.1.0 @ `592f16b`
//! feature `derivatives_trading_options`.
//! Scope: `GET /eapi/v1/userTrades` (Slice 4), `GET /eapi/v1/marginAccount`,
//! `GET /eapi/v1/position` (USER_DATA HMAC). No realized-PnL owner.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding, BrokerPositionRow};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_BASE_URL: &str = "https://eapi.binance.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinanceComOptionsError {
    Http { status: u16, body: String },
    Network(String),
    Parse(String),
}

impl BinanceComOptionsError {
    #[cfg(test)]
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
    /// Wiremock / non-venue base. Unmetered: R0 would refuse a mock host.
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

    /// `GET /eapi/v1/marginAccount` — USER_DATA. Never `/api/v3/account`.
    pub async fn fetch_margin_account(
        &self,
    ) -> Result<BrokerBalancesSnapshot, BinanceComOptionsError> {
        let body = self.signed_get("/eapi/v1/marginAccount", &[]).await?;
        parse_margin_account(&body)
    }

    /// `GET /eapi/v1/position` — USER_DATA. Never `/fapi/`.
    pub async fn fetch_positions(&self) -> Result<Vec<BrokerPositionRow>, BinanceComOptionsError> {
        let body = self.signed_get("/eapi/v1/position", &[]).await?;
        parse_option_positions(&body)
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

pub fn parse_margin_account(body: &str) -> Result<BrokerBalancesSnapshot, BinanceComOptionsError> {
    let parsed: MarginAccountResponse =
        serde_json::from_str(body).map_err(|e| BinanceComOptionsError::Parse(e.to_string()))?;
    let remaining: Vec<(BrokerHolding, Option<f64>)> = parsed
        .asset
        .into_iter()
        .filter_map(|row| {
            let free: f64 = row.available.parse().ok()?;
            let locked: f64 = row.initial_margin.parse().ok()?;
            if free <= 0.0 && locked <= 0.0 {
                return None;
            }
            let row_unrealized = row.unrealized_pnl.and_then(|s| s.parse().ok());
            Some((
                BrokerHolding {
                    asset: row.asset,
                    free,
                    locked,
                },
                row_unrealized,
            ))
        })
        .collect();
    let unrealized_pnl = if remaining.len() == 1 {
        remaining[0].1
    } else {
        None
    };
    Ok(BrokerBalancesSnapshot {
        holdings: remaining.into_iter().map(|(h, _)| h).collect(),
        unrealized_pnl,
    })
}

pub fn parse_option_positions(
    body: &str,
) -> Result<Vec<BrokerPositionRow>, BinanceComOptionsError> {
    let parsed: Vec<OptionPositionResponse> =
        serde_json::from_str(body).map_err(|e| BinanceComOptionsError::Parse(e.to_string()))?;
    Ok(parsed
        .into_iter()
        .filter_map(|row| {
            let net_qty: f64 = row.quantity.parse().ok()?;
            if net_qty == 0.0 {
                return None;
            }
            Some(BrokerPositionRow {
                symbol: row.symbol.clone(),
                exchange_segment: "options".into(),
                product: String::new(),
                net_qty,
                trading_symbol: row.symbol,
            })
        })
        .collect())
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
struct MarginAccountResponse {
    #[serde(default)]
    asset: Vec<MarginAssetResponse>,
}

#[derive(Debug, Deserialize)]
struct MarginAssetResponse {
    asset: String,
    available: String,
    #[serde(rename = "initialMargin")]
    initial_margin: String,
    #[serde(rename = "unrealizedPNL")]
    unrealized_pnl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OptionPositionResponse {
    symbol: String,
    quantity: String,
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

    /// Lock: `issues/compliance/locks/binance-com-options.md` (funds fetch 2026-09-15 IST).
    /// `available` → free, `initialMargin` → locked. Drop when both parse `<= 0`.
    /// Snapshot `unrealized_pnl` copies `unrealizedPNL` when exactly one row remains.
    #[test]
    fn parse_margin_account_one_remaining_row_copies_unrealized_pnl() {
        let snap = parse_margin_account(
            r#"{"asset":[{"asset":"USDT","available":"12.5","initialMargin":"3.25","unrealizedPNL":"1.1"},{"asset":"BNB","available":"0","initialMargin":"0","unrealizedPNL":"9"}]}"#,
        )
        .expect("parse");
        assert_eq!(snap.holdings.len(), 1);
        assert_eq!(snap.holdings[0].asset, "USDT");
        assert_eq!(snap.holdings[0].free, 12.5);
        assert_eq!(snap.holdings[0].locked, 3.25);
        assert_eq!(snap.unrealized_pnl, Some(1.1));
    }

    /// Two remaining rows after drop: do not invent a sum of `unrealizedPNL`.
    #[test]
    fn parse_margin_account_two_remaining_rows_unrealized_pnl_is_none() {
        let snap = parse_margin_account(
            r#"{"asset":[{"asset":"USDT","available":"12.5","initialMargin":"3.25","unrealizedPNL":"1.1"},{"asset":"BNB","available":"0.5","initialMargin":"0.25","unrealizedPNL":"9"}]}"#,
        )
        .expect("parse");
        assert_eq!(snap.holdings.len(), 2);
        assert!(snap.unrealized_pnl.is_none());
        assert_ne!(snap.unrealized_pnl, Some(10.1));
    }

    /// `greek[]` stays off BrokerBalancesSnapshot — optiongreeks is `GET /eapi/v1/mark`.
    #[test]
    fn parse_margin_account_does_not_map_greek_array() {
        let snap = parse_margin_account(
            r#"{"asset":[{"asset":"USDT","available":"12.5","initialMargin":"3.25","unrealizedPNL":"1.1"}],"greek":[{"underlying":"BTCUSDT","delta":"0.55937056","theta":"-1","gamma":"0.0001","vega":"2"}]}"#,
        )
        .expect("parse");
        assert_eq!(snap.holdings.len(), 1);
        assert_eq!(snap.holdings[0].asset, "USDT");
        assert_eq!(snap.unrealized_pnl, Some(1.1));
        assert_ne!(snap.unrealized_pnl, Some(0.55937056));
        assert!(
            snap.holdings
                .iter()
                .all(|h| h.asset != "BTCUSDT" && h.asset != "delta"),
            "greek[] must not land as holdings"
        );
        let debug = format!("{snap:?}");
        assert!(
            !debug.contains("0.55937056"),
            "venue greek delta must not appear on the funds snapshot"
        );
    }

    /// Lock: mixed-case `symbol`, `quantity` → net_qty, skip 0. Never lowercase, never qty=1 smash.
    #[test]
    fn parse_option_positions_keeps_mixed_case_and_qty_gt_1() {
        let rows = parse_option_positions(
            r#"[{"symbol":"BTC-260925-145000-C","quantity":"2"},{"symbol":"ETH-260925-3000-P","quantity":"0"}]"#,
        )
        .expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC-260925-145000-C");
        assert_ne!(rows[0].symbol, "btc-260925-145000-c");
        assert_eq!(rows[0].net_qty, 2.0);
        assert_ne!(rows[0].net_qty, 1.0);
        assert_eq!(rows[0].exchange_segment, "options");
        assert_ne!(rows[0].exchange_segment, "usdm");
        assert_ne!(rows[0].exchange_segment, "spot");
        assert_ne!(rows[0].exchange_segment, "nfo");
        assert_eq!(rows[0].trading_symbol, "BTC-260925-145000-C");
    }
}

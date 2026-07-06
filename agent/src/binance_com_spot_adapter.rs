//! Binance Global spot broker adapter — read-only poll via [`BinanceComSpotClient`].

use crate::binance_com_spot_client::{
    my_trade_to_broker_fill, BinanceComSpotClient, BinanceComSpotError,
};
use crate::broker::{BrokerAdapter, BrokerError, BrokerFill};
use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct BinanceComSpotBrokerAdapter {
    client: BinanceComSpotClient,
}

impl BinanceComSpotBrokerAdapter {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            client: BinanceComSpotClient::new(api_key, api_secret),
        }
    }
}

#[async_trait]
impl BrokerAdapter for BinanceComSpotBrokerAdapter {
    fn name(&self) -> &'static str {
        "binance_com_spot"
    }

    async fn poll_fills(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        let account = self
            .client
            .fetch_account_info()
            .await
            .map_err(map_spot_error)?;

        let symbols = trade_symbols_from_balances(&account.balances);
        let start_time_ms = since.map(|t| t.timestamp_millis());
        let mut fills = Vec::new();

        for symbol in symbols {
            match self
                .client
                .fetch_my_trades(&symbol, start_time_ms, Some(500))
                .await
            {
                Ok(trades) => {
                    fills.extend(trades.iter().map(my_trade_to_broker_fill));
                }
                Err(BinanceComSpotError::Http { status: 400, .. }) => continue,
                Err(e) => return Err(map_spot_error(e)),
            }
        }

        fills.sort_by_key(|f| f.filled_at);
        Ok(fills)
    }

    async fn poll_balances_holdings(&self) -> Result<BrokerBalancesSnapshot, BrokerError> {
        let account = self
            .client
            .fetch_account_info()
            .await
            .map_err(map_spot_error)?;

        let holdings = account
            .balances
            .into_iter()
            .filter_map(|b| {
                let free: f64 = b.free.parse().ok()?;
                let locked: f64 = b.locked.parse().ok()?;
                if free + locked <= 0.0 {
                    return None;
                }
                Some(BrokerHolding {
                    asset: b.asset,
                    free,
                    locked,
                })
            })
            .collect();

        Ok(BrokerBalancesSnapshot {
            holdings,
            unrealized_pnl: None,
        })
    }
}

fn map_spot_error(err: BinanceComSpotError) -> BrokerError {
    match err {
        BinanceComSpotError::Http { status, .. } if status == 429 => BrokerError::RateLimited {
            retry_after_ms: Some(60_000),
        },
        BinanceComSpotError::Network(msg) => BrokerError::Http(msg),
        other => BrokerError::Http(format!("{other:?}")),
    }
}

/// Derive likely USDT spot symbols from non-zero balances (read-only sync bootstrap).
fn trade_symbols_from_balances(balances: &[crate::binance_com_spot_client::BinanceComBalance]) -> Vec<String> {
    balances
        .iter()
        .filter_map(|b| {
            let free: f64 = b.free.parse().ok()?;
            let locked: f64 = b.locked.parse().ok()?;
            if free + locked <= 0.0 {
                return None;
            }
            if matches!(b.asset.as_str(), "USDT" | "USDC" | "BUSD" | "USD") {
                return None;
            }
            Some(format!("{}USDT", b.asset))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binance_com_spot_client::BinanceComBalance;

    #[test]
    fn trade_symbols_skip_stables_and_zero_balances() {
        let symbols = trade_symbols_from_balances(&[
            BinanceComBalance {
                asset: "BTC".into(),
                free: "0.01".into(),
                locked: "0".into(),
            },
            BinanceComBalance {
                asset: "USDT".into(),
                free: "100".into(),
                locked: "0".into(),
            },
            BinanceComBalance {
                asset: "ETH".into(),
                free: "0".into(),
                locked: "0".into(),
            },
        ]);
        assert_eq!(symbols, vec!["BTCUSDT"]);
    }
}

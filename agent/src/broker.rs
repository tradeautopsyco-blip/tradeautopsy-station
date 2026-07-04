//! Broker polling adapter boundary (design §8.3–8.4, Phase 6).

use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerOpenOrdersSnapshot};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// One executed fill surfaced to the toolbar (diff input + persistence row).
#[derive(Debug, Clone, PartialEq)]
pub struct BrokerFill {
    pub fill_id: String,
    pub trade_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub price: f64,
    pub filled_at: DateTime<Utc>,
    pub broker: String,
    pub fee_amount: Option<f64>,
    pub fee_asset: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BrokerError {
    Http(String),
    RateLimited {
        retry_after_ms: Option<i64>,
    },
}

impl BrokerError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Http(_) => "http_error",
            Self::RateLimited { .. } => "rate_limited",
        }
    }
}

#[async_trait]
pub trait BrokerAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn poll_fills(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError>;
    async fn poll_balances_holdings(&self) -> Result<BrokerBalancesSnapshot, BrokerError> {
        Ok(BrokerBalancesSnapshot::default())
    }
    async fn poll_open_orders(&self) -> Result<BrokerOpenOrdersSnapshot, BrokerError> {
        Ok(BrokerOpenOrdersSnapshot::empty())
    }
}

/// Test-double: returns queued results per poll round-robin (Phase 6 TDD harness).
#[derive(Clone)]
pub struct SeqMockBrokerAdapter {
    pub calls: Arc<Mutex<VecDeque<Result<Vec<BrokerFill>, BrokerError>>>>,
}

/// Test-double: counts poll invocations (issue #14 runtime control tests).
#[derive(Clone)]
pub struct CountingPollAdapter {
    poll_count: Arc<std::sync::atomic::AtomicU32>,
}

impl CountingPollAdapter {
    pub fn new() -> Self {
        Self {
            poll_count: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    pub fn poll_count(&self) -> u32 {
        self.poll_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait]
impl BrokerAdapter for CountingPollAdapter {
    fn name(&self) -> &'static str {
        "counting_poll"
    }

    async fn poll_fills(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        self.poll_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(Vec::new())
    }
}

/// Test-double: per-round configurable outcomes for all v1 data classes (#16/#17).
#[derive(Clone)]
pub struct ConfigurableDataClassAdapter {
    rounds: Arc<Mutex<VecDeque<DataClassPollRound>>>,
    current: Arc<Mutex<Option<DataClassPollRound>>>,
}

#[derive(Debug, Clone)]
pub struct DataClassPollRound {
    pub fills: Result<Vec<BrokerFill>, BrokerError>,
    pub balances: Result<BrokerBalancesSnapshot, BrokerError>,
    pub open_orders: Result<BrokerOpenOrdersSnapshot, BrokerError>,
}

impl ConfigurableDataClassAdapter {
    pub fn new(rounds: Vec<DataClassPollRound>) -> Self {
        Self {
            rounds: Arc::new(Mutex::new(rounds.into())),
            current: Arc::new(Mutex::new(None)),
        }
    }

    pub fn all_ok() -> Self {
        Self::new(vec![DataClassPollRound {
            fills: Ok(vec![]),
            balances: Ok(BrokerBalancesSnapshot::default()),
            open_orders: Ok(BrokerOpenOrdersSnapshot::empty()),
        }])
    }

    fn take_round(&self) -> DataClassPollRound {
        let mut current = self.current.lock().expect("current");
        if let Some(round) = current.take() {
            return round;
        }
        let mut rounds = self.rounds.lock().expect("rounds");
        rounds
            .pop_front()
            .unwrap_or(DataClassPollRound {
                fills: Ok(vec![]),
                balances: Ok(BrokerBalancesSnapshot::default()),
                open_orders: Ok(BrokerOpenOrdersSnapshot::empty()),
            })
    }
}

#[async_trait]
impl BrokerAdapter for ConfigurableDataClassAdapter {
    fn name(&self) -> &'static str {
        "configurable_data_class"
    }

    async fn poll_fills(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        let round = self.take_round();
        let result = round.fills.clone();
        *self.current.lock().expect("current") = Some(round);
        result
    }

    async fn poll_balances_holdings(&self) -> Result<BrokerBalancesSnapshot, BrokerError> {
        let round = self.current.lock().expect("current").clone();
        round
            .map(|r| r.balances)
            .unwrap_or(Ok(BrokerBalancesSnapshot::default()))
    }

    async fn poll_open_orders(&self) -> Result<BrokerOpenOrdersSnapshot, BrokerError> {
        let round = self.current.lock().expect("current").take();
        round
            .map(|r| r.open_orders)
            .unwrap_or(Ok(BrokerOpenOrdersSnapshot::empty()))
    }
}

#[async_trait]
impl BrokerAdapter for SeqMockBrokerAdapter {
    fn name(&self) -> &'static str {
        "seq_mock"
    }

    async fn poll_fills(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        let mut guard = self.calls.lock().expect("calls mutex");
        guard
            .pop_front()
            .unwrap_or_else(|| Ok(Vec::new()))
            .map_err(|e| match e {
                BrokerError::Http(s) => BrokerError::Http(s),
                BrokerError::RateLimited { retry_after_ms } => BrokerError::RateLimited {
                    retry_after_ms,
                },
            })
    }
}

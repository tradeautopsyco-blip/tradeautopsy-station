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
    /// Venue quote currency (I-N3). Kotak cash is `INR` — must not be dropped.
    pub currency: Option<String>,
    /// Venue product (I-N3 / I-N4). v1 cash lock: `CNC` | `MIS` only.
    pub product: Option<String>,
    /// Venue segment (I-N3). Kotak cash is `nse_cm` / `bse_cm`.
    pub exchange_segment: Option<String>,
    /// NFO only: CE / PE / FUT from master `pOptionType` or fill symbol — never product, never `"NSE"`.
    pub instrument_type: Option<String>,
    /// NFO only: lot from slice-2 scrip master row. Never default `1`. Never copy onto COM/spot.
    pub lot: Option<i64>,
}

impl Default for BrokerFill {
    fn default() -> Self {
        Self {
            fill_id: String::new(),
            trade_id: String::new(),
            symbol: String::new(),
            side: String::new(),
            qty: 0.0,
            price: 0.0,
            filled_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now),
            broker: String::new(),
            fee_amount: None,
            fee_asset: None,
            currency: None,
            product: None,
            exchange_segment: None,
            instrument_type: None,
            lot: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BrokerError {
    Http(String),
    RateLimited {
        retry_after_ms: Option<i64>,
    },
    /// Admit refused this venue (`venue_banned` / `venue_frozen`). Clock is on
    /// the engine (`posture_for`); `until_ms` is None at classify time.
    VenueStopped {
        until_ms: Option<i64>,
    },
}

impl BrokerError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Http(_) => "http_error",
            Self::RateLimited { .. } => "rate_limited",
            Self::VenueStopped { .. } => "venue_stopped",
        }
    }
}

/// Adapter name / error text → egress slot. `binance_com` wins before `kotak`
/// so a combined string cannot steal COM into the cash slot.
pub(crate) fn slug_to_slot(name: &str) -> Option<&'static str> {
    if name.contains("binance_com") {
        Some("binance_com")
    } else if name.contains("kotak_neo") || name.contains("kotak") {
        Some("kotak_neo")
    } else {
        None
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
        self.poll_count.load(std::sync::atomic::Ordering::Relaxed)
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
        rounds.pop_front().unwrap_or(DataClassPollRound {
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
                BrokerError::RateLimited { retry_after_ms } => {
                    BrokerError::RateLimited { retry_after_ms }
                }
                BrokerError::VenueStopped { until_ms } => BrokerError::VenueStopped { until_ms },
            })
    }
}

//! Broker polling adapter boundary (design §8.3–8.4, Phase 6).

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
}

#[derive(Debug, Clone)]
pub enum BrokerError {
    Http(String),
}

#[async_trait]
pub trait BrokerAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    async fn poll_fills(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError>;
}

/// Test-double: returns queued results per poll round-robin (Phase 6 TDD harness).
#[derive(Clone)]
pub struct SeqMockBrokerAdapter {
    pub calls: Arc<Mutex<VecDeque<Result<Vec<BrokerFill>, BrokerError>>>>,
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
            })
    }
}

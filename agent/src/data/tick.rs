//! Fixture quote tick for S1 lab. No vendor session.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How the tick arrived. REST is closed once the instrument is subscribed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Fixture,
    Rest,
    Stream,
}

/// Session bar from REST `quote_type=ohlc` / `all`. One latest_state slice — not history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOhlc {
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
}

/// One latest-state quote observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteTick {
    pub instrument_id: String,
    /// Decimal string, e.g. `"1400.50"`.
    pub last: String,
    pub as_of: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub age_unknown: bool,
    pub transport: Transport,
    /// Must match a loaded `market/quote/latest_state` descriptor.
    pub adapter_id: String,
    /// Optional session OHLC. Rides the quote envelope, not `/api/station/history`.
    pub session_ohlc: Option<SessionOhlc>,
}

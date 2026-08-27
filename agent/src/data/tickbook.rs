//! In-memory latest-state quote book. Single-threaded S1 lab (no Mutex).

use super::tick::{QuoteTick, SessionOhlc, Transport};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

/// Stored latest quote row for one instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredQuote {
    pub last: String,
    pub as_of: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub age_unknown: bool,
    pub adapter_id: String,
    pub transport: Transport,
    pub session_ohlc: Option<SessionOhlc>,
}

/// Latest-wins quote map plus the subscribe set that closes REST.
#[derive(Debug, Clone, Default)]
pub struct TickBook {
    quotes: HashMap<String, StoredQuote>,
    subscribed: HashSet<String>,
}

/// Result of an internal latest-wins upsert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpsertOutcome {
    Applied,
    IgnoredOlder,
}

impl TickBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, instrument_id: impl Into<String>) {
        self.subscribed.insert(instrument_id.into());
    }

    pub fn unsubscribe(&mut self, instrument_id: &str) {
        self.subscribed.remove(instrument_id);
    }

    pub fn is_subscribed(&self, instrument_id: &str) -> bool {
        self.subscribed.contains(instrument_id)
    }

    pub fn get(&self, instrument_id: &str) -> Option<&StoredQuote> {
        self.quotes.get(instrument_id)
    }

    /// Replace when incoming `as_of` is equal or newer; keep stored when older.
    pub(crate) fn upsert(&mut self, tick: QuoteTick) -> UpsertOutcome {
        if let Some(stored) = self.quotes.get(&tick.instrument_id) {
            if tick.as_of < stored.as_of {
                return UpsertOutcome::IgnoredOlder;
            }
        }
        self.quotes.insert(
            tick.instrument_id.clone(),
            StoredQuote {
                last: tick.last,
                as_of: tick.as_of,
                received_at: tick.received_at,
                age_unknown: tick.age_unknown,
                adapter_id: tick.adapter_id,
                transport: tick.transport,
                session_ohlc: tick.session_ohlc,
            },
        );
        UpsertOutcome::Applied
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &StoredQuote)> {
        self.quotes.iter()
    }

    pub fn has_positive_tick_for(&self, adapter_id: &str) -> bool {
        self.quotes.values().any(|row| {
            row.adapter_id == adapter_id
                && row.last.parse::<f64>().ok().is_some_and(|last| last > 0.0)
        })
    }
}

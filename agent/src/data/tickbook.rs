//! In-memory latest-state quote book. Single-threaded S1 lab (no Mutex).
//!
//! Keys are `{adapter}\0{instrument}` (same idea as HistoryBook). Two adapters
//! cannot occupy one slot — a future USD-M BTCUSDT subscribe must not close
//! spot REST for that ticker. Locks: binance-com-spot + kotak-nse-bse-cash
//! (fetch 2026-08-22 IST).

use super::binance_public::normalize_quote_instrument;
use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::tick::{QuoteTick, SessionOhlc, Transport};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub(crate) fn tick_key(adapter_id: &str, instrument_id: &str) -> String {
    format!(
        "{}\0{}",
        adapter_id.trim(),
        normalize_tick_instrument(adapter_id, instrument_id)
    )
}

fn normalize_tick_instrument(adapter_id: &str, instrument_id: &str) -> String {
    if adapter_id.trim() == BINANCE_COM_ADAPTER_ID {
        normalize_quote_instrument(instrument_id)
    } else {
        instrument_id.trim().to_string()
    }
}

/// Stored latest quote row for one instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredQuote {
    pub instrument_id: String,
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

    pub fn subscribe(&mut self, adapter_id: &str, instrument_id: impl Into<String>) {
        self.subscribed
            .insert(tick_key(adapter_id, &instrument_id.into()));
    }

    pub fn unsubscribe(&mut self, adapter_id: &str, instrument_id: &str) {
        self.subscribed.remove(&tick_key(adapter_id, instrument_id));
    }

    pub fn is_subscribed(&self, adapter_id: &str, instrument_id: &str) -> bool {
        self.subscribed
            .contains(&tick_key(adapter_id, instrument_id))
    }

    pub fn get(&self, adapter_id: &str, instrument_id: &str) -> Option<&StoredQuote> {
        self.quotes.get(&tick_key(adapter_id, instrument_id))
    }

    /// Replace when incoming `as_of` is equal or newer; keep stored when older.
    pub(crate) fn upsert(&mut self, tick: QuoteTick) -> UpsertOutcome {
        let key = tick_key(&tick.adapter_id, &tick.instrument_id);
        if let Some(stored) = self.quotes.get(&key) {
            if tick.as_of < stored.as_of {
                return UpsertOutcome::IgnoredOlder;
            }
        }
        let instrument_id = normalize_tick_instrument(&tick.adapter_id, &tick.instrument_id);
        self.quotes.insert(
            key,
            StoredQuote {
                instrument_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn tick(adapter: &str, instrument: &str, last: &str) -> QuoteTick {
        QuoteTick {
            instrument_id: instrument.to_string(),
            last: last.to_string(),
            as_of: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            received_at: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            age_unknown: false,
            transport: Transport::Fixture,
            adapter_id: adapter.to_string(),
            session_ohlc: None,
        }
    }

    #[test]
    fn two_adapters_do_not_share_a_tick_slot() {
        let mut book = TickBook::new();
        assert_eq!(
            book.upsert(tick("binance_com", "BTCUSDT", "1")),
            UpsertOutcome::Applied
        );
        assert_eq!(
            book.upsert(tick("other", "BTCUSDT", "2")),
            UpsertOutcome::Applied
        );
        assert_eq!(book.get("binance_com", "btcusdt").unwrap().last, "1");
        assert_eq!(book.get("other", "BTCUSDT").unwrap().last, "2");
        assert!(book.has_positive_tick_for("binance_com"));
    }

    #[test]
    fn subscribe_is_per_adapter_and_instrument() {
        let mut book = TickBook::new();
        book.subscribe("other", "btcusdt");
        assert!(book.is_subscribed("other", "btcusdt"));
        assert!(!book.is_subscribed("binance_com", "btcusdt"));
        book.unsubscribe("other", "btcusdt");
        assert!(!book.is_subscribed("other", "btcusdt"));
    }
}

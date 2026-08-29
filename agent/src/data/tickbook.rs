//! In-memory latest-state quote book. Single-threaded S1 lab (no Mutex).
//!
//! Keys are `{book_id}\0{instrument}` (same idea as HistoryBook). Two books can
//! share adapter `binance_com` and BTCUSDT without overwriting — a future USD-M
//! BTCUSDT subscribe must not close spot REST for that ticker. Locks:
//! binance-com-spot + kotak-nse-bse-cash (fetch 2026-08-22 IST).

use super::binance_public::normalize_quote_instrument;
use super::descriptor::BINANCE_COM_SPOT_BOOK_ID;
use super::tick::{QuoteTick, SessionOhlc, Transport};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

pub(crate) fn tick_key(book_id: &str, instrument_id: &str) -> String {
    format!(
        "{}\0{}",
        book_id.trim(),
        normalize_tick_instrument(book_id, instrument_id)
    )
}

fn normalize_tick_instrument(book_id: &str, instrument_id: &str) -> String {
    if book_id.trim() == BINANCE_COM_SPOT_BOOK_ID {
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
    pub book_id: String,
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

    pub fn subscribe(&mut self, book_id: &str, instrument_id: impl Into<String>) {
        self.subscribed
            .insert(tick_key(book_id, &instrument_id.into()));
    }

    pub fn unsubscribe(&mut self, book_id: &str, instrument_id: &str) {
        self.subscribed.remove(&tick_key(book_id, instrument_id));
    }

    pub fn is_subscribed(&self, book_id: &str, instrument_id: &str) -> bool {
        self.subscribed.contains(&tick_key(book_id, instrument_id))
    }

    pub fn get(&self, book_id: &str, instrument_id: &str) -> Option<&StoredQuote> {
        self.quotes.get(&tick_key(book_id, instrument_id))
    }

    /// Replace when incoming `as_of` is equal or newer; keep stored when older.
    pub(crate) fn upsert(&mut self, tick: QuoteTick) -> UpsertOutcome {
        let key = tick_key(&tick.book_id, &tick.instrument_id);
        if let Some(stored) = self.quotes.get(&key) {
            if tick.as_of < stored.as_of {
                return UpsertOutcome::IgnoredOlder;
            }
        }
        let instrument_id = normalize_tick_instrument(&tick.book_id, &tick.instrument_id);
        self.quotes.insert(
            key,
            StoredQuote {
                instrument_id,
                last: tick.last,
                as_of: tick.as_of,
                received_at: tick.received_at,
                age_unknown: tick.age_unknown,
                adapter_id: tick.adapter_id,
                book_id: tick.book_id,
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

    fn tick(book_id: &str, instrument: &str, last: &str) -> QuoteTick {
        let adapter_id = if book_id == "binance-com-spot"
            || book_id == "binance-com-usdm"
            || book_id == "binance-com-options"
        {
            "binance_com"
        } else {
            book_id
        };
        QuoteTick {
            instrument_id: instrument.to_string(),
            last: last.to_string(),
            as_of: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            received_at: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            age_unknown: false,
            transport: Transport::Fixture,
            adapter_id: adapter_id.to_string(),
            book_id: book_id.to_string(),
            session_ohlc: None,
        }
    }

    #[test]
    fn two_adapters_do_not_share_a_tick_slot() {
        let mut book = TickBook::new();
        assert_eq!(
            book.upsert(tick("binance-com-spot", "BTCUSDT", "1")),
            UpsertOutcome::Applied
        );
        assert_eq!(
            book.upsert(tick("other", "BTCUSDT", "2")),
            UpsertOutcome::Applied
        );
        assert_eq!(book.get("binance-com-spot", "btcusdt").unwrap().last, "1");
        assert_eq!(book.get("other", "BTCUSDT").unwrap().last, "2");
        assert!(book.has_positive_tick_for("binance_com"));
    }

    #[test]
    fn subscribe_is_per_adapter_and_instrument() {
        let mut book = TickBook::new();
        book.subscribe("other", "btcusdt");
        assert!(book.is_subscribed("other", "btcusdt"));
        assert!(!book.is_subscribed("binance-com-spot", "btcusdt"));
        book.unsubscribe("other", "btcusdt");
        assert!(!book.is_subscribed("other", "btcusdt"));
    }

    #[test]
    fn tick_key_spot_and_usdm_prefixes_are_unequal() {
        // `binance-com-usdm` is a slot prefix only — not a live USD-M book.
        assert_ne!(
            tick_key("binance-com-spot", "btcusdt"),
            tick_key("binance-com-usdm", "btcusdt")
        );
    }

    #[test]
    fn tick_key_spot_and_options_prefixes_are_unequal() {
        assert_ne!(
            tick_key("binance-com-spot", "btcusdt"),
            tick_key("binance-com-options", "btcusdt")
        );
        assert_ne!(
            tick_key("binance-com-spot", "BTC-200730-9000-C"),
            tick_key("binance-com-options", "BTC-200730-9000-C")
        );
    }

    #[test]
    fn options_instrument_is_not_lowercased() {
        let mut book = TickBook::new();
        assert_eq!(
            book.upsert(tick("binance-com-options", "BTC-200730-9000-C", "1.23")),
            UpsertOutcome::Applied
        );
        let row = book
            .get("binance-com-options", "BTC-200730-9000-C")
            .expect("mixed-case options contract");
        assert_eq!(row.instrument_id, "BTC-200730-9000-C");
        assert_eq!(row.last, "1.23");
        assert_eq!(row.book_id, "binance-com-options");
        assert!(book
            .get("binance-com-options", "btc-200730-9000-c")
            .is_none());
        assert_ne!(
            tick_key("binance-com-spot", "BTC-200730-9000-C"),
            tick_key("binance-com-options", "BTC-200730-9000-C")
        );
        assert_eq!(
            tick_key("binance-com-spot", "BTCUSDT"),
            tick_key("binance-com-spot", "btcusdt")
        );
    }

    #[test]
    fn two_book_id_prefixes_do_not_share_a_tick_slot() {
        // Two rows share adapter `binance_com` and BTCUSDT without overwrite.
        // Spot book id normalizes the instrument; usdm does not (fake key only).
        let mut book = TickBook::new();
        assert_eq!(
            book.upsert(tick("binance-com-spot", "BTCUSDT", "1")),
            UpsertOutcome::Applied
        );
        assert_eq!(
            book.upsert(tick("binance-com-usdm", "btcusdt", "2")),
            UpsertOutcome::Applied
        );
        let spot = book.get("binance-com-spot", "btcusdt").unwrap();
        let usdm = book.get("binance-com-usdm", "btcusdt").unwrap();
        assert_eq!(spot.last, "1");
        assert_eq!(usdm.last, "2");
        assert_eq!(spot.adapter_id, "binance_com");
        assert_eq!(usdm.adapter_id, "binance_com");
        assert_eq!(spot.book_id, "binance-com-spot");
        assert_eq!(usdm.book_id, "binance-com-usdm");
    }

    #[test]
    fn subscribe_is_per_book_slot() {
        let mut book = TickBook::new();
        book.subscribe("binance-com-spot", "btcusdt");
        assert!(book.is_subscribed("binance-com-spot", "btcusdt"));
        assert!(!book.is_subscribed("binance-com-usdm", "btcusdt"));
    }
}

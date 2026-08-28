//! Quote extract envelope. Freshness copied from JS `extractQuote` (issue #361).

use super::apply::quote_latest_state;
use super::descriptor::DelayClass;
use super::registry::Registry;
use super::rights::Rights;
use super::tick::{SessionOhlc, Transport};
use super::tickbook::TickBook;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use std::time::Duration;

/// Envelope freshness. Empty book is `unavailable`, not an invented last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteStatus {
    Fresh,
    Stale,
    Unknown,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct QuoteData {
    pub last: String,
    pub as_of: String,
    /// Session OHLC from the quote envelope. Never a historical_series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ohlc: Option<SessionOhlc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct QuoteProvenance {
    pub adapter_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Transport>,
    pub delay_class: DelayClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct QuoteEnvelope {
    pub identity: super::identity::Identity,
    pub instrument_id: String,
    pub status: QuoteStatus,
    pub data: Option<QuoteData>,
    pub provenance: QuoteProvenance,
    pub rights: Rights,
    pub ineligible: Vec<String>,
    pub research: bool,
    pub canonical: bool,
    pub persist_canonical: bool,
}

/// Alias used by callers/tests.
pub type QuoteExtract = QuoteEnvelope;

/// Extract latest quote. Delay class comes from the loaded descriptor, not freshness.
pub fn extract_quote(
    registry: &Registry,
    book: &TickBook,
    instrument_id: &str,
    now: DateTime<Utc>,
    freshness: Duration,
) -> QuoteEnvelope {
    extract_quote_for(registry, book, instrument_id, now, freshness, None)
}

/// Like [`extract_quote`], but provenance and book rows prefer `adapter_id`.
pub fn extract_quote_for(
    registry: &Registry,
    book: &TickBook,
    instrument_id: &str,
    now: DateTime<Utc>,
    freshness: Duration,
    adapter_id: Option<&str>,
) -> QuoteEnvelope {
    let identity = quote_latest_state();
    let descriptor = adapter_id
        .and_then(|id| registry.get_for_adapter(&identity, id))
        .or_else(|| registry.get(&identity));
    let rights = descriptor
        .and_then(|binding| binding.rights)
        .unwrap_or_else(Rights::research_fetch_only);
    let delay_class = descriptor
        .map(|binding| binding.delay_class)
        .unwrap_or(DelayClass::Unknown);

    let Some(row) = (match adapter_id {
        Some(id) => book.get(id, instrument_id),
        None => book
            .iter()
            .find_map(|(_, row)| (row.instrument_id == instrument_id).then_some(row)),
    }) else {
        return QuoteEnvelope {
            identity,
            instrument_id: instrument_id.to_string(),
            status: QuoteStatus::Unavailable,
            data: None,
            provenance: QuoteProvenance {
                adapter_id: adapter_id
                    .map(str::to_string)
                    .or_else(|| descriptor.map(|binding| binding.adapter_id.clone()))
                    .unwrap_or_default(),
                transport: None,
                delay_class,
            },
            rights,
            ineligible: Vec::new(),
            research: true,
            canonical: false,
            persist_canonical: false,
        };
    };

    let status = if row.age_unknown {
        QuoteStatus::Unknown
    } else {
        let age = now - row.as_of;
        let freshness_chrono =
            chrono::Duration::from_std(freshness).unwrap_or_else(|_| chrono::Duration::MAX);
        if age <= freshness_chrono {
            QuoteStatus::Fresh
        } else {
            QuoteStatus::Stale
        }
    };

    QuoteEnvelope {
        identity,
        instrument_id: instrument_id.to_string(),
        status,
        data: Some(QuoteData {
            last: row.last.clone(),
            as_of: row.as_of.to_rfc3339_opts(SecondsFormat::Millis, true),
            session_ohlc: row.session_ohlc.clone(),
        }),
        provenance: QuoteProvenance {
            adapter_id: row.adapter_id.clone(),
            transport: Some(row.transport),
            delay_class,
        },
        rights,
        ineligible: Vec::new(),
        research: true,
        canonical: false,
        persist_canonical: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::apply::{apply_quote, ApplyOutcome};
    use crate::data::descriptor::fixture_quote_descriptor;
    use crate::data::tick::{QuoteTick, Transport};
    use crate::data::tickbook::TickBook;
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    const INSTRUMENT: &str = "instr.fixture.1";
    const LAST: &str = "1400.50";
    const FRESHNESS_MS: u64 = 1000;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0).unwrap()
    }

    fn lab_registry() -> Registry {
        Registry::load(&[fixture_quote_descriptor()]).expect("fixture quote loads")
    }

    fn tick(as_of: DateTime<Utc>, age_unknown: bool) -> QuoteTick {
        QuoteTick {
            instrument_id: INSTRUMENT.to_string(),
            last: LAST.to_string(),
            as_of,
            received_at: now(),
            age_unknown,
            transport: Transport::Fixture,
            adapter_id: "fixture_equity_quote".to_string(),
            session_ohlc: None,
        }
    }

    fn apply_then_extract(as_of: DateTime<Utc>, age_unknown: bool) -> QuoteEnvelope {
        let registry = lab_registry();
        let mut book = TickBook::new();
        let outcome = apply_quote(&registry, &mut book, tick(as_of, age_unknown)).unwrap();
        assert_eq!(outcome, ApplyOutcome::Applied);
        extract_quote(
            &registry,
            &book,
            INSTRUMENT,
            now(),
            Duration::from_millis(FRESHNESS_MS),
        )
    }

    #[test]
    fn extract_fresh() {
        let envelope = apply_then_extract(now() - ChronoDuration::milliseconds(400), false);
        assert_eq!(envelope.status, QuoteStatus::Fresh);
        assert_eq!(envelope.data.as_ref().map(|d| d.last.as_str()), Some(LAST));
        assert_eq!(
            envelope.data.as_ref().map(|d| d.as_of.as_str()),
            Some("2026-08-25T11:59:59.600Z")
        );
        assert_eq!(envelope.identity, quote_latest_state());
        assert_eq!(envelope.instrument_id, INSTRUMENT);
        assert_eq!(envelope.provenance.delay_class, DelayClass::Realtime);
        assert_eq!(envelope.provenance.adapter_id, "fixture_equity_quote");
        assert_eq!(envelope.provenance.transport, Some(Transport::Fixture));
        assert_eq!(envelope.rights, Rights::research_fetch_only());
        assert!(envelope.research);
        assert!(!envelope.canonical);
        assert!(!envelope.persist_canonical);
        assert!(envelope.ineligible.is_empty());
    }

    #[test]
    fn extract_stale() {
        let envelope = apply_then_extract(now() - ChronoDuration::milliseconds(5000), false);
        assert_eq!(envelope.status, QuoteStatus::Stale);
        assert_eq!(envelope.data.as_ref().map(|d| d.last.as_str()), Some(LAST));
        assert_eq!(
            envelope.data.as_ref().map(|d| d.as_of.as_str()),
            Some("2026-08-25T11:59:55.000Z")
        );
        assert_eq!(envelope.provenance.delay_class, DelayClass::Realtime);
    }

    #[test]
    fn extract_unknown() {
        let envelope = apply_then_extract(now() - ChronoDuration::milliseconds(400), true);
        assert_eq!(envelope.status, QuoteStatus::Unknown);
        assert_eq!(envelope.data.as_ref().map(|d| d.last.as_str()), Some(LAST));
    }

    #[test]
    fn empty_extract() {
        let registry = lab_registry();
        let book = TickBook::new();
        let envelope = extract_quote(
            &registry,
            &book,
            INSTRUMENT,
            now(),
            Duration::from_millis(FRESHNESS_MS),
        );
        assert_eq!(envelope.status, QuoteStatus::Unavailable);
        assert!(envelope.data.is_none());
        assert!(envelope.ineligible.is_empty());
        assert_eq!(envelope.instrument_id, INSTRUMENT);
        assert_eq!(envelope.provenance.delay_class, DelayClass::Realtime);
        assert!(!envelope.persist_canonical);
    }
}

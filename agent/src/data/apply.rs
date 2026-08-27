//! Apply rules for S1 lab: registered binding, REST closed when subscribed, latest-wins.

use super::identity::{CapabilityId, Family, Identity, Physics};
use super::registry::Registry;
use super::tick::{QuoteTick, Transport};
use super::tickbook::{TickBook, UpsertOutcome};
use std::fmt;

/// Successful apply (including ignored older ticks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    IgnoredOlder,
}

/// Apply refused. Book is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    UnregisteredBinding,
    RestClosed { instrument_id: String },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyError::UnregisteredBinding => {
                write!(f, "no market/quote/latest_state binding for tick adapter")
            }
            ApplyError::RestClosed { instrument_id } => {
                write!(
                    f,
                    "REST path closed for subscribed instrument `{instrument_id}`"
                )
            }
        }
    }
}

impl std::error::Error for ApplyError {}

pub(crate) fn quote_latest_state() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("quote").expect("canonical quote id"),
        Physics::LatestState,
    )
}

fn registered_quote_adapter(registry: &Registry, adapter_id: &str) -> bool {
    let identity = quote_latest_state();
    registry
        .iter()
        .any(|descriptor| descriptor.identity == identity && descriptor.adapter_id == adapter_id)
}

/// Apply a quote tick. Fixture and stream always attempt upsert; REST is refused
/// when that instrument is in the subscribe set.
pub fn apply_quote(
    registry: &Registry,
    book: &mut TickBook,
    tick: QuoteTick,
) -> Result<ApplyOutcome, ApplyError> {
    if !registered_quote_adapter(registry, &tick.adapter_id) {
        return Err(ApplyError::UnregisteredBinding);
    }
    if tick.transport == Transport::Rest && book.is_subscribed(&tick.instrument_id) {
        return Err(ApplyError::RestClosed {
            instrument_id: tick.instrument_id,
        });
    }
    match book.upsert(tick) {
        UpsertOutcome::Applied => Ok(ApplyOutcome::Applied),
        UpsertOutcome::IgnoredOlder => Ok(ApplyOutcome::IgnoredOlder),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::descriptor::fixture_quote_descriptor;
    use crate::data::tick::Transport;
    use chrono::{Duration, TimeZone, Utc};

    const INSTRUMENT: &str = "instr.fixture.1";
    const LAST: &str = "1400.50";
    const OLDER_LAST: &str = "1390.00";

    fn now() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0).unwrap()
    }

    fn lab_registry() -> Registry {
        Registry::load(&[fixture_quote_descriptor()]).expect("fixture quote loads")
    }

    fn tick(
        last: &str,
        as_of: chrono::DateTime<Utc>,
        transport: Transport,
        age_unknown: bool,
    ) -> QuoteTick {
        QuoteTick {
            instrument_id: INSTRUMENT.to_string(),
            last: last.to_string(),
            as_of,
            received_at: now(),
            age_unknown,
            transport,
            adapter_id: "fixture_equity_quote".to_string(),
            session_ohlc: None,
        }
    }

    #[test]
    fn rest_closed_when_subscribed() {
        let registry = lab_registry();
        let mut book = TickBook::new();
        book.subscribe(INSTRUMENT);

        let err = apply_quote(
            &registry,
            &mut book,
            tick(
                LAST,
                now() - Duration::milliseconds(400),
                Transport::Rest,
                false,
            ),
        )
        .expect_err("REST must close when subscribed");

        assert_eq!(
            err,
            ApplyError::RestClosed {
                instrument_id: INSTRUMENT.to_string()
            }
        );
        assert!(book.get(INSTRUMENT).is_none());

        apply_quote(
            &registry,
            &mut book,
            tick(
                LAST,
                now() - Duration::milliseconds(400),
                Transport::Fixture,
                false,
            ),
        )
        .unwrap();
        book.subscribe(INSTRUMENT);
        let before = book.get(INSTRUMENT).cloned().unwrap();
        let err = apply_quote(
            &registry,
            &mut book,
            tick(
                OLDER_LAST,
                now() - Duration::milliseconds(100),
                Transport::Rest,
                false,
            ),
        )
        .expect_err("REST still closed with a stored row");
        assert!(matches!(err, ApplyError::RestClosed { .. }));
        assert_eq!(book.get(INSTRUMENT), Some(&before));
    }

    #[test]
    fn stream_applies_when_subscribed() {
        let registry = lab_registry();
        let mut book = TickBook::new();
        book.subscribe(INSTRUMENT);

        let outcome = apply_quote(
            &registry,
            &mut book,
            tick(
                LAST,
                now() - Duration::milliseconds(400),
                Transport::Stream,
                false,
            ),
        )
        .unwrap();
        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(
            book.get(INSTRUMENT).map(|row| row.last.as_str()),
            Some(LAST)
        );
    }

    #[test]
    fn latest_wins() {
        let registry = lab_registry();
        let mut book = TickBook::new();
        let older_at = now() - Duration::milliseconds(5000);
        let newer_at = now() - Duration::milliseconds(400);

        assert_eq!(
            apply_quote(
                &registry,
                &mut book,
                tick(OLDER_LAST, older_at, Transport::Fixture, false)
            )
            .unwrap(),
            ApplyOutcome::Applied
        );
        assert_eq!(
            apply_quote(
                &registry,
                &mut book,
                tick(LAST, newer_at, Transport::Fixture, false)
            )
            .unwrap(),
            ApplyOutcome::Applied
        );
        assert_eq!(
            book.get(INSTRUMENT).map(|row| row.last.as_str()),
            Some(LAST)
        );

        let outcome = apply_quote(
            &registry,
            &mut book,
            tick(OLDER_LAST, older_at, Transport::Fixture, false),
        )
        .unwrap();
        assert_eq!(outcome, ApplyOutcome::IgnoredOlder);
        assert_eq!(
            book.get(INSTRUMENT).map(|row| row.last.as_str()),
            Some(LAST)
        );
        assert_eq!(book.get(INSTRUMENT).map(|row| row.as_of), Some(newer_at));
    }

    #[test]
    fn registry_required() {
        let empty = Registry::load(&[]).expect("empty load is ok");
        let mut book = TickBook::new();
        let err = apply_quote(
            &empty,
            &mut book,
            tick(LAST, now(), Transport::Fixture, false),
        )
        .expect_err("empty registry must refuse");
        assert_eq!(err, ApplyError::UnregisteredBinding);
        assert!(book.get(INSTRUMENT).is_none());

        let account_only =
            Registry::load(&[crate::data::descriptor::fixture_account_descriptor()]).unwrap();
        let err = apply_quote(
            &account_only,
            &mut book,
            tick(LAST, now(), Transport::Fixture, false),
        )
        .expect_err("account binding is not quote");
        assert_eq!(err, ApplyError::UnregisteredBinding);
    }
}

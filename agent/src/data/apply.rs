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
    if tick.transport == Transport::Rest && book.is_subscribed(&tick.book_id, &tick.instrument_id) {
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
            book_id: "fixture_equity_quote".to_string(),
            session_ohlc: None,
        }
    }

    #[test]
    fn rest_closed_when_subscribed() {
        let registry = lab_registry();
        let mut book = TickBook::new();
        book.subscribe("fixture_equity_quote", INSTRUMENT);

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
        assert!(book.get("fixture_equity_quote", INSTRUMENT).is_none());

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
        book.subscribe("fixture_equity_quote", INSTRUMENT);
        let before = book
            .get("fixture_equity_quote", INSTRUMENT)
            .cloned()
            .unwrap();
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
        assert_eq!(book.get("fixture_equity_quote", INSTRUMENT), Some(&before));
    }

    #[test]
    fn stream_applies_when_subscribed() {
        let registry = lab_registry();
        let mut book = TickBook::new();
        book.subscribe("fixture_equity_quote", INSTRUMENT);

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
            book.get("fixture_equity_quote", INSTRUMENT)
                .map(|row| row.last.as_str()),
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
            book.get("fixture_equity_quote", INSTRUMENT)
                .map(|row| row.last.as_str()),
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
            book.get("fixture_equity_quote", INSTRUMENT)
                .map(|row| row.last.as_str()),
            Some(LAST)
        );
        assert_eq!(
            book.get("fixture_equity_quote", INSTRUMENT)
                .map(|row| row.as_of),
            Some(newer_at)
        );
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
        assert!(book.get("fixture_equity_quote", INSTRUMENT).is_none());

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

    #[test]
    fn rest_closed_is_per_adapter_and_instrument() {
        use crate::data::descriptor::binance_com_quote_descriptor;
        let registry = Registry::load(&[binance_com_quote_descriptor()]).expect("spot quote");
        let mut book = TickBook::new();
        book.subscribe("other", "btcusdt");
        let rest = QuoteTick {
            instrument_id: "btcusdt".to_string(),
            last: "1".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "binance_com".to_string(),
            book_id: "binance-com-spot".to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, rest).unwrap();
        assert_eq!(book.get("binance-com-spot", "btcusdt").unwrap().last, "1");
        book.subscribe("binance-com-spot", "btcusdt");
        let rest2 = QuoteTick {
            instrument_id: "btcusdt".to_string(),
            last: "2".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "binance_com".to_string(),
            book_id: "binance-com-spot".to_string(),
            session_ohlc: None,
        };
        let err = apply_quote(&registry, &mut book, rest2).unwrap_err();
        assert!(matches!(err, ApplyError::RestClosed { .. }));
        assert_eq!(book.get("binance-com-spot", "btcusdt").unwrap().last, "1");
    }

    #[test]
    fn rest_closed_is_per_book_slot() {
        use crate::data::descriptor::binance_com_quote_descriptor;
        let registry = Registry::load(&[binance_com_quote_descriptor()]).expect("spot quote");
        let mut book = TickBook::new();
        book.subscribe("binance-com-usdm", "btcusdt");
        let rest = QuoteTick {
            instrument_id: "btcusdt".to_string(),
            last: "1".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "binance_com".to_string(),
            book_id: "binance-com-spot".to_string(),
            session_ohlc: None,
        };
        let outcome = apply_quote(&registry, &mut book, rest).unwrap();
        assert_eq!(outcome, ApplyOutcome::Applied);
        // usdm subscribe must not close spot REST; lookup is by book_id.
        assert_eq!(book.get("binance-com-spot", "btcusdt").unwrap().last, "1");
    }

    #[test]
    fn subscribe_nfo_does_not_close_spot_or_cash_rest() {
        use crate::data::descriptor::{binance_com_quote_descriptor, kotak_neo_quote_descriptor};
        let registry =
            Registry::load(&[binance_com_quote_descriptor(), kotak_neo_quote_descriptor()])
                .expect("spot + kotak quote");
        let mut book = TickBook::new();
        book.subscribe("kotak-nse-nfo", "nse_fo|12345");

        let spot = QuoteTick {
            instrument_id: "btcusdt".to_string(),
            last: "65000".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "binance_com".to_string(),
            book_id: "binance-com-spot".to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, spot).unwrap();
        assert_eq!(
            book.get("binance-com-spot", "btcusdt").unwrap().last,
            "65000"
        );

        let cash = QuoteTick {
            instrument_id: "nse_cm|2885".to_string(),
            last: "1400.50".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "kotak_neo".to_string(),
            book_id: "kotak-nse-bse-cash".to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, cash).unwrap();
        assert_eq!(
            book.get("kotak-nse-bse-cash", "nse_cm|2885").unwrap().last,
            "1400.50"
        );

        let nfo = QuoteTick {
            instrument_id: "nse_fo|12345".to_string(),
            last: "10.00".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "kotak_neo".to_string(),
            book_id: "kotak-nse-nfo".to_string(),
            session_ohlc: None,
        };
        let err = apply_quote(&registry, &mut book, nfo).unwrap_err();
        assert!(matches!(err, ApplyError::RestClosed { .. }));
        assert!(book.get("kotak-nse-nfo", "nse_fo|12345").is_none());
    }

    #[test]
    fn subscribe_cash_does_not_close_nfo_rest() {
        use crate::data::descriptor::kotak_neo_quote_descriptor;
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).expect("kotak quote");
        let mut book = TickBook::new();
        book.subscribe("kotak-nse-bse-cash", "nse_cm|2885");

        let nfo = QuoteTick {
            instrument_id: "nse_fo|12345".to_string(),
            last: "10.00".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "kotak_neo".to_string(),
            book_id: "kotak-nse-nfo".to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, nfo).unwrap();
        assert_eq!(
            book.get("kotak-nse-nfo", "nse_fo|12345").unwrap().last,
            "10.00"
        );

        let cash = QuoteTick {
            instrument_id: "nse_cm|2885".to_string(),
            last: "1400.50".to_string(),
            as_of: now(),
            received_at: now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "kotak_neo".to_string(),
            book_id: "kotak-nse-bse-cash".to_string(),
            session_ohlc: None,
        };
        let err = apply_quote(&registry, &mut book, cash).unwrap_err();
        assert!(matches!(err, ApplyError::RestClosed { .. }));
        assert!(book.get("kotak-nse-bse-cash", "nse_cm|2885").is_none());
    }
}

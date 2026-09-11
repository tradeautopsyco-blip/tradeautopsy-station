//! S1 desk: one unsigned `GET /eapi/v1/ticker?symbol=` so Last lands on a dated bind.
//!
//! Lock (`binance-com-options.md`): public last is REST `lastPrice`, mixed-case
//! `?symbol=`, host `eapi.binance.com`, no HMAC. Never `/api/v3/ticker/price`.
//! WS last field is NOT SPECIFIED.

use super::apply::apply_quote;
use super::binance_options_public::{
    is_dated_option_contract, normalize_options_instrument, options_ticker_query,
    quote_tick_from_options_ticker_json_for_symbol, OPTIONS_EAPI_HOST, OPTIONS_TICKER_PATH,
};
use super::descriptor::BINANCE_COM_OPTIONS_BOOK_ID;
use super::registry::Registry;
use super::tickbook::TickBook;
use crate::egress::{EgressCall, EgressError, EgressResponse, Lane};
use crate::kotak_rest_quotes::{QuoteFetchErrorClass, QuoteFetchErrorMap};
use chrono::Utc;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Two loopback quotes on the same id inside this window share one eapi call.
const TICKER_MAX_AGE_MS: i64 = 1_000;

/// The one call this module is allowed to make, or `None` when the id is not a
/// dated options contract. Returning `None` is how a leftover `BTCUSDT` or a
/// Kotak `seg|token` never reaches the engine at all.
pub fn binance_options_ticker_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_options_instrument(instrument_id);
    if !is_dated_option_contract(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_OPTIONS_BOOK_ID,
            OPTIONS_EAPI_HOST,
            OPTIONS_TICKER_PATH,
            Lane::MarketData,
        )
        .with_query(options_ticker_query(&id))
        .with_max_age_ms(TICKER_MAX_AGE_MS),
    )
}

/// Should the desk spend an eapi call on this id right now?
///
/// TickBook already holding a last for this book+id is the Kotak/spot early
/// return. Options last has no specified WS field, so subscribe does not close
/// REST — a resident last is the only skip.
pub fn should_prime_binance_options_ticker(book: &TickBook, instrument_id: &str) -> bool {
    let id = normalize_options_instrument(instrument_id);
    if binance_options_ticker_call(&id).is_none() {
        return false;
    }
    book.get(BINANCE_COM_OPTIONS_BOOK_ID, &id).is_none()
}

/// One REST last for `instrument_id`, awaited, before the caller extracts.
///
/// Mirrors `await_binance_spot_ticker_price`: skip when the book already has a
/// last, de-duplicate concurrent callers through `inflight`, and never poll — a
/// 429 or 418 is the engine's to record, not ours to retry around.
pub async fn await_binance_options_ticker(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    instrument_id: &str,
) {
    let id = normalize_options_instrument(instrument_id);
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if !should_prime_binance_options_ticker(&guard, &id) {
            return;
        }
    }
    let Some(call) = binance_options_ticker_call(&id) else {
        return;
    };
    let we_own = {
        let mut guard = inflight.lock().expect("com ticker inflight poisoned");
        guard.insert(id.clone())
    };
    if !we_own {
        wait_for_inflight_options_ticker(&book, &inflight, &id).await;
        return;
    }

    let result = crate::egress::shared().send(&call).await;
    inflight
        .lock()
        .expect("com ticker inflight poisoned")
        .remove(&id);

    apply_options_ticker_result(registry, book, quote_fetch_error, &id, result);
}

fn apply_options_ticker_result(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    quote_fetch_error: QuoteFetchErrorMap,
    id: &str,
    result: Result<EgressResponse, EgressError>,
) {
    let class = match result {
        Ok(resp) if resp.is_success() => {
            match quote_tick_from_options_ticker_json_for_symbol(&resp.body, id, Utc::now()) {
                Some(tick) => {
                    let mut guard = book.lock().expect("tickbook mutex poisoned");
                    match apply_quote(registry.as_ref(), &mut guard, tick) {
                        Ok(_) => {
                            drop(guard);
                            quote_fetch_error
                                .lock()
                                .expect("quote fetch error poisoned")
                                .remove(id);
                            tracing::info!(
                                instrument = %id,
                                "s1 desk: binance_com options REST ticker applied"
                            );
                            return;
                        }
                        Err(err) => {
                            tracing::warn!(
                                instrument = %id,
                                error = %err,
                                "s1 desk: binance_com options REST ticker refused"
                            );
                            None
                        }
                    }
                }
                None => Some(QuoteFetchErrorClass::QuotesUnusable),
            }
        }
        Ok(resp) => {
            tracing::warn!(
                instrument = %id,
                status = resp.status,
                "s1 desk: binance_com options REST ticker http"
            );
            Some(QuoteFetchErrorClass::QuotesHttp)
        }
        Err(err) => {
            tracing::warn!(
                instrument = %id,
                error = %err,
                "s1 desk: binance_com options REST ticker failed"
            );
            Some(QuoteFetchErrorClass::QuotesHttp)
        }
    };
    if let Some(class) = class {
        quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .insert(id.to_string(), class.as_str().to_string());
    }
}

/// A second caller for the same mixed-case id waits on the owner's GET rather
/// than dialling eapi twice. Bounded, and it never itself calls the venue.
async fn wait_for_inflight_options_ticker(
    book: &Arc<Mutex<TickBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    instrument_id: &str,
) {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let guard = book.lock().expect("tickbook mutex poisoned");
            if guard
                .get(BINANCE_COM_OPTIONS_BOOK_ID, instrument_id)
                .is_some()
            {
                return;
            }
        }
        if !inflight
            .lock()
            .expect("com ticker inflight poisoned")
            .contains(instrument_id)
        {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_options_public::quote_tick_from_options_ticker_json;
    use crate::data::descriptor::binance_com_quote_descriptor;
    use crate::egress::Lane;
    use chrono::{TimeZone, Utc};

    fn desk_registry() -> Registry {
        Registry::load(&[binance_com_quote_descriptor()]).expect("binance_com quote loads")
    }

    fn received() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap()
    }

    fn http(status: u16, body: &str) -> EgressResponse {
        EgressResponse {
            status,
            headers: Vec::new(),
            body: body.to_string(),
            coalesced: false,
        }
    }

    /// Official CLI example (`BTC-200730-9000-C`): mixed-case query, empty headers
    /// (HMAC would be a private credential on a public path — a refusal, not an
    /// upgrade), never the spot ticker.
    #[test]
    fn dated_contract_builds_unsigned_mixed_case_eapi_ticker_get() {
        let call =
            binance_options_ticker_call("BTC-200730-9000-C").expect("dated id builds a call");
        assert_eq!(call.method, "GET");
        assert_eq!(call.host, "eapi.binance.com");
        assert_eq!(call.path, "/eapi/v1/ticker");
        assert_eq!(call.query, "symbol=BTC-200730-9000-C");
        assert_ne!(call.query, "symbol=btc-200730-9000-c");
        assert_eq!(call.lane, Lane::MarketData);
        assert_eq!(call.book_id, "binance-com-options");
        assert!(
            call.headers.is_empty(),
            "public ticker must not carry HMAC headers"
        );
        assert!(!call.query.contains("signature="));
        assert_ne!(call.path, "/api/v3/ticker/price");
        assert_ne!(call.host, "api.binance.com");
    }

    /// A leftover spot id or Kotak token never becomes an eapi ticker GET.
    #[test]
    fn ids_with_no_call_are_not_primed() {
        let book = TickBook::new();
        assert!(!should_prime_binance_options_ticker(&book, "BTCUSDT"));
        assert!(!should_prime_binance_options_ticker(&book, "nse_cm|2885"));
        assert!(!should_prime_binance_options_ticker(&book, ""));
        assert!(binance_options_ticker_call("BTCUSDT").is_none());
    }

    /// Kotak's `get` early return: a book that already has a last spends no call.
    #[test]
    fn id_with_a_last_is_not_primed_again() {
        let registry = desk_registry();
        let mut book = TickBook::new();
        assert!(should_prime_binance_options_ticker(
            &book,
            "BTC-200730-9000-C"
        ));
        let tick = quote_tick_from_options_ticker_json(
            r#"{"symbol":"BTC-200730-9000-C","lastPrice":"1.23"}"#,
            received(),
        )
        .expect("rest ticker parses");
        apply_quote(&registry, &mut book, tick).expect("rest applies");
        assert!(!should_prime_binance_options_ticker(
            &book,
            "BTC-200730-9000-C"
        ));
        // A *different* contract in the same book is still cold.
        assert!(should_prime_binance_options_ticker(
            &book,
            "BTC-200730-9500-C"
        ));
        // Lowercase smash must not hit the mixed-case slot.
        assert!(should_prime_binance_options_ticker(
            &book,
            "btc-200730-9000-c"
        ));
    }

    /// An id whose last is already in the book is not re-fetched, so the stored
    /// price survives the second quote GET untouched.
    #[tokio::test]
    async fn prime_skips_an_id_that_already_has_a_last() {
        let registry = Arc::new(desk_registry());
        let mut warm = TickBook::new();
        let tick = quote_tick_from_options_ticker_json(
            r#"{"symbol":"BTC-200730-9000-C","lastPrice":"1.23"}"#,
            received(),
        )
        .expect("rest ticker parses");
        apply_quote(registry.as_ref(), &mut warm, tick).expect("applies");
        let book = Arc::new(Mutex::new(warm));
        let inflight = Arc::new(Mutex::new(HashSet::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        await_binance_options_ticker(
            registry,
            book.clone(),
            inflight.clone(),
            errors.clone(),
            "BTC-200730-9000-C",
        )
        .await;

        assert!(inflight.lock().expect("inflight").is_empty());
        assert_eq!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
                .map(|row| row.last.as_str()),
            Some("1.23")
        );
        assert_ne!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
                .map(|row| row.last.as_str()),
            Some("0")
        );
    }

    /// The owner already marked inflight: a second caller waits rather than
    /// starting a GET. Proof: the key is still present (this caller did not
    /// become owner and did not remove it) and the options book stayed empty.
    #[tokio::test]
    async fn a_second_caller_joins_an_in_flight_get_instead_of_dialling() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let inflight = Arc::new(Mutex::new(HashSet::from(["BTC-200730-9000-C".to_string()])));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let waiter = tokio::spawn({
            let registry = registry.clone();
            let book = book.clone();
            let inflight = inflight.clone();
            let errors = errors.clone();
            async move {
                await_binance_options_ticker(registry, book, inflight, errors, "BTC-200730-9000-C")
                    .await;
            }
        });

        // Drop the owner's claim so the waiter is not stuck for the full 2s.
        tokio::time::sleep(Duration::from_millis(20)).await;
        inflight
            .lock()
            .expect("inflight")
            .remove("BTC-200730-9000-C");
        waiter.await.expect("waiter");

        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
            .is_none());
        assert!(errors.lock().expect("errors").is_empty());
    }

    /// HTTP 429 leaves Last a typed hole (`quotes_http`), never last `"0"`.
    #[test]
    fn http_429_records_quotes_http_and_leaves_the_book_empty() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        apply_options_ticker_result(
            registry,
            book.clone(),
            errors.clone(),
            "BTC-200730-9000-C",
            Ok(http(429, "too many")),
        );
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("BTC-200730-9000-C")
                .map(String::as_str),
            Some("quotes_http")
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
            .is_none());
    }

    /// HTTP 418 (IP banned) is the same class: Last can say why, not stay silent.
    #[test]
    fn http_418_records_quotes_http_and_leaves_the_book_empty() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        apply_options_ticker_result(
            registry,
            book.clone(),
            errors.clone(),
            "BTC-200730-9000-C",
            Ok(http(418, "banned")),
        );
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("BTC-200730-9000-C")
                .map(String::as_str),
            Some("quotes_http")
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
            .is_none());
    }

    /// An unusable success body (zero / missing lastPrice) does not plant `"0"`.
    #[test]
    fn empty_or_zero_last_stays_unavailable_not_zero() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        apply_options_ticker_result(
            registry,
            book.clone(),
            errors.clone(),
            "BTC-200730-9000-C",
            Ok(http(
                200,
                r#"{"symbol":"BTC-200730-9000-C","lastPrice":"0"}"#,
            )),
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
            .is_none());
        assert_ne!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-200730-9000-C")
                .map(|row| row.last.as_str()),
            Some("0")
        );
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("BTC-200730-9000-C")
                .map(String::as_str),
            Some("quotes_unusable")
        );
    }
}

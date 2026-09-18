//! S1 desk: one unsigned `GET /fapi/v1/ticker/price?symbol=` so Last lands on
//! book `binance-com-usdm`.
//!
//! Lock (`binance-com-usdm.md`): public last is REST field `price` (string),
//! host `fapi.binance.com`, no HMAC. Never `/api/v3/ticker/price`. Never eapi
//! `lastPrice`. Never `/fapi/v2/ticker/price`. TickBook key preserves venue
//! symbol case (`BTCUSDT`) — do not lowercase like spot. REST last only; no
//! fapi trade-stream subscribe this slice.

use super::apply::apply_quote;
use super::binance_options_public::is_dated_option_contract;
use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_USDM_BOOK_ID};
use super::registry::Registry;
use super::tick::{QuoteTick, Transport};
use super::tickbook::TickBook;
use crate::egress::{EgressCall, EgressError, EgressResponse, Lane};
use crate::kotak_rest_quotes::{QuoteFetchErrorClass, QuoteFetchErrorMap};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Two loopback quotes on the same id inside this window share one fapi call.
const TICKER_MAX_AGE_MS: i64 = 1_000;

pub const USDM_FAPI_HOST: &str = "fapi.binance.com";
pub const USDM_TICKER_PATH: &str = "/fapi/v1/ticker/price";

/// Venue symbol case (typically `BTCUSDT`). Trim + uppercase; never spot lowercase.
pub fn normalize_usdm_instrument(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn is_usdm_pair(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('|')
        && !is_dated_option_contract(id)
        && id.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The one call this module is allowed to make, or `None` when the id is not a
/// USDM pair. Returning `None` is how a leftover dated contract, a Kotak
/// `seg|token`, or an empty id never reaches the engine at all.
pub fn binance_usdm_ticker_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_usdm_instrument(instrument_id);
    if !is_usdm_pair(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_USDM_BOOK_ID,
            USDM_FAPI_HOST,
            USDM_TICKER_PATH,
            Lane::MarketData,
        )
        .with_query_pairs(&[("symbol", id.as_str())])
        .with_max_age_ms(TICKER_MAX_AGE_MS),
    )
}

/// Should the desk spend an fapi call on this id right now?
///
/// TickBook already holding a last for this book+id is the early return.
/// USDM last has no trade-stream subscribe this slice, so REST stays open —
/// a resident last is the only skip.
pub fn should_prime_binance_usdm_ticker(book: &TickBook, instrument_id: &str) -> bool {
    let id = normalize_usdm_instrument(instrument_id);
    if binance_usdm_ticker_call(&id).is_none() {
        return false;
    }
    book.get(BINANCE_COM_USDM_BOOK_ID, &id).is_none()
}

/// One REST last for `instrument_id`, awaited, before the caller extracts.
///
/// Mirrors `await_binance_options_ticker`: skip when the book already has a
/// last, de-duplicate concurrent callers through `inflight`, and never poll —
/// a 429 or 418 is the engine's to record, not ours to retry around. Never
/// HMAC. Never a spot WS subscribe.
pub async fn await_binance_usdm_ticker(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    instrument_id: &str,
) {
    let id = normalize_usdm_instrument(instrument_id);
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if !should_prime_binance_usdm_ticker(&guard, &id) {
            return;
        }
    }
    let Some(call) = binance_usdm_ticker_call(&id) else {
        return;
    };
    let we_own = {
        let mut guard = inflight.lock().expect("usdm ticker inflight poisoned");
        guard.insert(id.clone())
    };
    if !we_own {
        wait_for_inflight_usdm_ticker(&book, &inflight, &id).await;
        return;
    }

    let result = crate::egress::shared().send(&call).await;
    inflight
        .lock()
        .expect("usdm ticker inflight poisoned")
        .remove(&id);

    apply_usdm_ticker_result(registry, book, quote_fetch_error, &id, result);
}

fn apply_usdm_ticker_result(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    quote_fetch_error: QuoteFetchErrorMap,
    id: &str,
    result: Result<EgressResponse, EgressError>,
) {
    let class = match result {
        Ok(resp) if resp.is_success() => {
            match quote_tick_from_usdm_ticker_json_for_symbol(&resp.body, id, Utc::now()) {
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
                                "s1 desk: binance_com usdm REST ticker applied"
                            );
                            return;
                        }
                        Err(err) => {
                            tracing::warn!(
                                instrument = %id,
                                error = %err,
                                "s1 desk: binance_com usdm REST ticker refused"
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
                "s1 desk: binance_com usdm REST ticker http"
            );
            Some(QuoteFetchErrorClass::QuotesHttp)
        }
        Err(err) => {
            tracing::warn!(
                instrument = %id,
                error = %err,
                "s1 desk: binance_com usdm REST ticker failed"
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

/// A second caller for the same venue-case id waits on the owner's GET rather
/// than dialling fapi twice. Bounded, and it never itself calls the venue.
async fn wait_for_inflight_usdm_ticker(
    book: &Arc<Mutex<TickBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    instrument_id: &str,
) {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let guard = book.lock().expect("tickbook mutex poisoned");
            if guard.get(BINANCE_COM_USDM_BOOK_ID, instrument_id).is_some() {
                return;
            }
        }
        if !inflight
            .lock()
            .expect("usdm ticker inflight poisoned")
            .contains(instrument_id)
        {
            return;
        }
    }
}

fn tick_from_ticker_object(value: &Value, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    let symbol = value.get("symbol").and_then(Value::as_str)?;
    let instrument_id = symbol.trim().to_string();
    if !is_usdm_pair(&normalize_usdm_instrument(&instrument_id)) {
        return None;
    }
    // Wire last is `price`. Options `lastPrice` is a different book — never a fallback.
    let last = value.get("price").and_then(Value::as_str)?.to_string();
    let last_n: f64 = last.parse().ok()?;
    if last_n <= 0.0 {
        return None;
    }
    Some(QuoteTick {
        instrument_id,
        last,
        as_of: received_at,
        received_at,
        age_unknown: true,
        transport: Transport::Rest,
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        book_id: BINANCE_COM_USDM_BOOK_ID.to_string(),
        session_ohlc: None,
    })
}

/// Map REST ticker JSON to a quote tick. Last = `price`, never `lastPrice`.
pub fn quote_tick_from_usdm_ticker_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Option<QuoteTick> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let obj = if value.is_object() {
        &value
    } else {
        return None;
    };
    tick_from_ticker_object(obj, received_at)
}

/// One last for exactly this venue-case symbol. A body whose `symbol` is a
/// different pair, a leftover dated contract, or a zero `price` is a miss.
pub fn quote_tick_from_usdm_ticker_json_for_symbol(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<QuoteTick> {
    let want = normalize_usdm_instrument(symbol);
    if !is_usdm_pair(&want) {
        return None;
    }
    let tick = quote_tick_from_usdm_ticker_json(raw, received_at)?;
    (normalize_usdm_instrument(&tick.instrument_id) == want).then_some(tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::descriptor::binance_com_quote_descriptor;
    use crate::egress::Lane;
    use chrono::TimeZone;

    fn desk_registry() -> Registry {
        Registry::load(&[binance_com_quote_descriptor()]).expect("binance_com quote loads")
    }

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 19, 12, 0, 0).unwrap()
    }

    fn http(status: u16, body: &str) -> EgressResponse {
        EgressResponse {
            status,
            headers: Vec::new(),
            body: body.to_string(),
            coalesced: false,
        }
    }

    /// Official USDM ticker: unsigned `?symbol=BTCUSDT`, empty headers (HMAC on
    /// a public path is a refusal), never the spot ticker, never eapi, never v2.
    #[test]
    fn usdm_pair_builds_unsigned_fapi_ticker_get() {
        let call = binance_usdm_ticker_call("BTCUSDT").expect("usdm id builds a call");
        assert_eq!(call.method, "GET");
        assert_eq!(call.host, "fapi.binance.com");
        assert_eq!(call.path, "/fapi/v1/ticker/price");
        assert_eq!(call.query, "symbol=BTCUSDT");
        assert_ne!(call.query, "symbol=btcusdt");
        assert_eq!(call.lane, Lane::MarketData);
        assert_eq!(call.book_id, "binance-com-usdm");
        assert!(
            call.headers.is_empty(),
            "public ticker must not carry HMAC headers"
        );
        assert!(!call.query.contains("signature="));
        assert_ne!(call.path, "/api/v3/ticker/price");
        assert_ne!(call.path, "/fapi/v2/ticker/price");
        assert_ne!(call.path, "/eapi/v1/ticker");
        assert_ne!(call.host, "api.binance.com");
        assert_ne!(call.host, "eapi.binance.com");
    }

    /// Venue case is uppercase. Spot's TickBook smash to `btcusdt` is refused.
    #[test]
    fn symbol_preserves_venue_case_not_spot_lowercase() {
        let call = binance_usdm_ticker_call("  btcusdt  ").expect("call");
        assert_eq!(call.query, "symbol=BTCUSDT");
        assert_ne!(call.query, "symbol=btcusdt");
        assert_eq!(normalize_usdm_instrument("BTCUSDT"), "BTCUSDT");
        assert_ne!(normalize_usdm_instrument("BTCUSDT"), "btcusdt");
    }

    /// A leftover dated contract, Kotak token, or empty never becomes an fapi GET.
    #[test]
    fn leftover_dated_kotak_or_empty_never_builds_a_call() {
        let book = TickBook::new();
        assert!(!should_prime_binance_usdm_ticker(
            &book,
            "BTC-200730-9000-C"
        ));
        assert!(!should_prime_binance_usdm_ticker(&book, "nse_cm|2885"));
        assert!(!should_prime_binance_usdm_ticker(&book, "nse_fo|12345"));
        assert!(!should_prime_binance_usdm_ticker(&book, ""));
        assert!(!should_prime_binance_usdm_ticker(&book, "   "));
        assert!(binance_usdm_ticker_call("BTC-200730-9000-C").is_none());
        assert!(binance_usdm_ticker_call("nse_cm|2885").is_none());
        assert!(binance_usdm_ticker_call("nse_fo|12345").is_none());
        assert!(binance_usdm_ticker_call("").is_none());
        assert!(binance_usdm_ticker_call("   ").is_none());
    }

    /// Kotak's `get` early return: a book that already has a last spends no call.
    #[test]
    fn id_with_a_last_is_not_primed_again() {
        let registry = desk_registry();
        let mut book = TickBook::new();
        assert!(should_prime_binance_usdm_ticker(&book, "BTCUSDT"));
        let tick = quote_tick_from_usdm_ticker_json(
            r#"{"symbol":"BTCUSDT","price":"65000.10","time":1}"#,
            received(),
        )
        .expect("rest ticker parses");
        apply_quote(&registry, &mut book, tick).expect("rest applies");
        assert!(!should_prime_binance_usdm_ticker(&book, "BTCUSDT"));
        assert!(should_prime_binance_usdm_ticker(&book, "ETHUSDT"));
        // Spot-lowercase smash must not hit the venue-case slot.
        assert!(!should_prime_binance_usdm_ticker(&book, "btcusdt"));
    }

    /// An id whose last is already in the book is not re-fetched.
    #[tokio::test]
    async fn prime_skips_an_id_that_already_has_a_last() {
        let registry = Arc::new(desk_registry());
        let mut warm = TickBook::new();
        let tick = quote_tick_from_usdm_ticker_json(
            r#"{"symbol":"BTCUSDT","price":"65000.10","time":1}"#,
            received(),
        )
        .expect("rest ticker parses");
        apply_quote(registry.as_ref(), &mut warm, tick).expect("applies");
        let book = Arc::new(Mutex::new(warm));
        let inflight = Arc::new(Mutex::new(HashSet::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        await_binance_usdm_ticker(
            registry,
            book.clone(),
            inflight.clone(),
            errors.clone(),
            "BTCUSDT",
        )
        .await;

        assert!(inflight.lock().expect("inflight").is_empty());
        assert_eq!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
                .map(|row| row.last.as_str()),
            Some("65000.10")
        );
        assert_ne!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
                .map(|row| row.last.as_str()),
            Some("0")
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_USDM_BOOK_ID, "btcusdt")
            .is_none());
    }

    /// The owner already marked inflight: a second caller waits rather than
    /// starting a GET.
    #[tokio::test]
    async fn a_second_caller_joins_an_in_flight_get_instead_of_dialling() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let inflight = Arc::new(Mutex::new(HashSet::from(["BTCUSDT".to_string()])));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let waiter = tokio::spawn({
            let registry = registry.clone();
            let book = book.clone();
            let inflight = inflight.clone();
            let errors = errors.clone();
            async move {
                await_binance_usdm_ticker(registry, book, inflight, errors, "BTCUSDT").await;
            }
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        inflight.lock().expect("inflight").remove("BTCUSDT");
        waiter.await.expect("waiter");

        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
            .is_none());
        assert!(errors.lock().expect("errors").is_empty());
    }

    /// Guarded leftover ids never mark inflight (no GET started).
    #[tokio::test]
    async fn prime_makes_no_call_for_leftover_ids() {
        let registry = Arc::new(desk_registry());
        let inflight = Arc::new(Mutex::new(HashSet::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let book = Arc::new(Mutex::new(TickBook::new()));

        for id in ["BTC-200730-9000-C", "nse_cm|2885", "", "   "] {
            await_binance_usdm_ticker(
                registry.clone(),
                book.clone(),
                inflight.clone(),
                errors.clone(),
                id,
            )
            .await;
        }

        assert!(
            inflight.lock().expect("inflight").is_empty(),
            "no leftover id may start a fapi GET"
        );
        assert!(errors.lock().expect("errors").is_empty());
    }

    /// Official CI fixture: `price`, venue-case `BTCUSDT`, USDM book. Not `lastPrice`.
    #[test]
    fn ticker_price_body_parses_as_a_rest_last_on_the_usdm_book() {
        let json = include_str!("../../fixtures/binance/usdm_ticker.json");
        let tick = quote_tick_from_usdm_ticker_json(json, received()).expect("parses");
        assert_eq!(tick.instrument_id, "BTCUSDT");
        assert_ne!(tick.instrument_id, "btcusdt");
        assert_eq!(tick.last, "65000.10");
        assert_ne!(tick.last, "0");
        assert_eq!(tick.transport, Transport::Rest);
        assert_eq!(tick.book_id, BINANCE_COM_USDM_BOOK_ID);
        assert_eq!(tick.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert!(tick.age_unknown);
    }

    /// Options wire `lastPrice` is not USDM last. A body with only that field is a miss.
    #[test]
    fn last_price_field_is_not_usdm_last() {
        assert!(quote_tick_from_usdm_ticker_json(
            r#"{"symbol":"BTCUSDT","lastPrice":"65000.10"}"#,
            received(),
        )
        .is_none());
        let mixed = quote_tick_from_usdm_ticker_json(
            r#"{"symbol":"BTCUSDT","price":"65000.10","lastPrice":"1.23"}"#,
            received(),
        )
        .expect("price wins");
        assert_eq!(mixed.last, "65000.10");
        assert_ne!(mixed.last, "1.23");
    }

    /// Zero / missing `price` does not plant `"0"`.
    #[test]
    fn empty_or_zero_price_stays_unavailable_not_zero() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        apply_usdm_ticker_result(
            registry,
            book.clone(),
            errors.clone(),
            "BTCUSDT",
            Ok(http(200, r#"{"symbol":"BTCUSDT","price":"0"}"#)),
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
            .is_none());
        assert_ne!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
                .map(|row| row.last.as_str()),
            Some("0")
        );
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("BTCUSDT")
                .map(String::as_str),
            Some("quotes_unusable")
        );
    }

    /// HTTP 429 leaves Last a typed hole (`quotes_http`), never last `"0"`.
    #[test]
    fn http_429_records_quotes_http_and_leaves_the_book_empty() {
        let registry = Arc::new(desk_registry());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));
        apply_usdm_ticker_result(
            registry,
            book.clone(),
            errors.clone(),
            "BTCUSDT",
            Ok(http(429, "too many")),
        );
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("BTCUSDT")
                .map(String::as_str),
            Some("quotes_http")
        );
        assert!(book
            .lock()
            .expect("book")
            .get(BINANCE_COM_USDM_BOOK_ID, "BTCUSDT")
            .is_none());
    }
}

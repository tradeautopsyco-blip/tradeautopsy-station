//! S1 desk: one unsigned `GET /api/v3/ticker/price` so Last lands before the
//! WS subscribe closes REST for that instrument.
//!
//! REST.md (fetched 2026-08-30): Security NONE, host `api.binance.com`,
//! weight 2 with `symbol=`, 4 without. Always send `symbol=` — never the
//! no-symbol form. Not `/ticker/24hr` (80 / 2). Not `eapi` (that book has its
//! own last). Never HMAC.
//!
//! Ordering is load-bearing: `apply_quote` refuses `Transport::Rest` once the
//! instrument is subscribed (`ApplyError::RestClosed`), so this must land
//! *before* `subscribe_instrument` binds the trade stream.

use super::apply::apply_quote;
use super::binance_options_public::is_dated_option_contract;
use super::binance_public::normalize_quote_instrument;
use super::binance_public::quote_tick_from_binance_json;
use super::descriptor::BINANCE_COM_SPOT_BOOK_ID;
use super::registry::Registry;
use super::tickbook::TickBook;
use crate::egress::{EgressCall, Lane};
use crate::kotak_rest_quotes::{QuoteFetchErrorClass, QuoteFetchErrorMap};
use chrono::Utc;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Two loopback quotes on the same id inside this window share one COM call.
const TICKER_MAX_AGE_MS: i64 = 1_000;

/// The one call this module is allowed to make, or `None` when the id is not a
/// COM spot symbol. Returning `None` is how a dated contract or a Kotak `seg|token`
/// id never reaches the engine at all — the engine would refuse it `WrongBook`,
/// but a call that is never built cannot be mispriced either.
pub fn binance_spot_ticker_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_quote_instrument(instrument_id);
    if id.is_empty() || id.contains('|') || is_dated_option_contract(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_SPOT_BOOK_ID,
            "api.binance.com",
            "/api/v3/ticker/price",
            Lane::MarketData,
        )
        .with_query(format!("symbol={}", id.to_ascii_uppercase()))
        .with_max_age_ms(TICKER_MAX_AGE_MS),
    )
}

/// Should the desk spend a COM call on this id right now?
///
/// Two reasons not to, both matching the Kotak template's early return:
/// TickBook already holds a last for this book+id, or the instrument is already
/// subscribed — in which case `apply_quote` would refuse the REST tick
/// `RestClosed` and the call would be spent for nothing.
pub fn should_prime_binance_spot_ticker(book: &TickBook, instrument_id: &str) -> bool {
    let id = normalize_quote_instrument(instrument_id);
    if binance_spot_ticker_call(&id).is_none() {
        return false;
    }
    if book.get(BINANCE_COM_SPOT_BOOK_ID, &id).is_some() {
        return false;
    }
    !book.is_subscribed(BINANCE_COM_SPOT_BOOK_ID, &id)
}

/// One REST last for `instrument_id`, awaited, before the caller subscribes.
///
/// Mirrors `await_kotak_rest_quote`: skip when the book already has a last,
/// de-duplicate concurrent callers through `inflight`, and never poll — a 429 or
/// 418 is the engine's to record, not ours to retry around.
///
/// The caller must `await` this *before* `subscribe_instrument`. After the trade
/// stream subscribes the id, `apply_quote` refuses this tick `RestClosed` and the
/// call is spent for nothing.
pub async fn await_binance_spot_ticker_price(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    instrument_id: &str,
) {
    let id = normalize_quote_instrument(instrument_id);
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if !should_prime_binance_spot_ticker(&guard, &id) {
            return;
        }
    }
    let Some(call) = binance_spot_ticker_call(&id) else {
        return;
    };
    let we_own = {
        let mut guard = inflight.lock().expect("com ticker inflight poisoned");
        guard.insert(id.clone())
    };
    if !we_own {
        wait_for_inflight_ticker(&book, &inflight, &id).await;
        return;
    }

    let result = crate::egress::shared().send(&call).await;
    inflight
        .lock()
        .expect("com ticker inflight poisoned")
        .remove(&id);

    let class = match result {
        Ok(resp) if resp.is_success() => {
            match quote_tick_from_binance_json(&resp.body, Utc::now()) {
                Some(tick) => {
                    let mut guard = book.lock().expect("tickbook mutex poisoned");
                    match apply_quote(registry.as_ref(), &mut guard, tick) {
                        Ok(_) => {
                            drop(guard);
                            quote_fetch_error
                                .lock()
                                .expect("quote fetch error poisoned")
                                .remove(&id);
                            tracing::info!(
                                instrument = %id,
                                "s1 desk: binance_com REST ticker applied"
                            );
                            return;
                        }
                        Err(err) => {
                            tracing::warn!(
                                instrument = %id,
                                error = %err,
                                "s1 desk: binance_com REST ticker refused"
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
                "s1 desk: binance_com REST ticker http"
            );
            Some(QuoteFetchErrorClass::QuotesHttp)
        }
        Err(err) => {
            tracing::warn!(
                instrument = %id,
                error = %err,
                "s1 desk: binance_com REST ticker failed"
            );
            Some(QuoteFetchErrorClass::QuotesHttp)
        }
    };
    if let Some(class) = class {
        quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .insert(id, class.as_str().to_string());
    }
}

/// A second caller for the same id waits on the owner's GET rather than dialling
/// COM twice. Bounded, and it never itself calls the venue.
async fn wait_for_inflight_ticker(
    book: &Arc<Mutex<TickBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    instrument_id: &str,
) {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let guard = book.lock().expect("tickbook mutex poisoned");
            if guard.get(BINANCE_COM_SPOT_BOOK_ID, instrument_id).is_some() {
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
    use crate::data::apply::apply_quote;
    use crate::data::binance_public::quote_tick_from_binance_json;
    use crate::data::descriptor::binance_com_quote_descriptor;
    use crate::data::registry::Registry;
    use chrono::{TimeZone, Utc};

    fn desk_registry() -> Registry {
        Registry::load(&[binance_com_quote_descriptor()]).expect("binance_com quote loads")
    }

    fn received() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 30, 12, 0, 0).unwrap()
    }

    /// A cold book on an unsubscribed id is exactly the case the prime exists for.
    #[test]
    fn cold_unsubscribed_spot_id_is_primed() {
        let book = TickBook::new();
        assert!(should_prime_binance_spot_ticker(&book, "ethusdt"));
    }

    /// Kotak's `get` early return: a book that already has a last spends no call.
    #[test]
    fn id_with_a_last_is_not_primed_again() {
        let registry = desk_registry();
        let mut book = TickBook::new();
        let tick =
            quote_tick_from_binance_json(r#"{"symbol":"ETHUSDT","price":"3500.12"}"#, received())
                .expect("rest ticker parses");
        apply_quote(&registry, &mut book, tick).expect("rest applies to an unsubscribed id");
        assert!(!should_prime_binance_spot_ticker(&book, "ethusdt"));
        // A *different* id in the same book is still cold.
        assert!(should_prime_binance_spot_ticker(&book, "btcusdt"));
    }

    /// Once subscribed, `apply_quote` returns `RestClosed`: do not fight it.
    /// This is why boot's bound `btcusdt` stream must not trigger a ticker.
    #[test]
    fn subscribed_id_is_not_primed() {
        let mut book = TickBook::new();
        book.subscribe(BINANCE_COM_SPOT_BOOK_ID, "btcusdt");
        assert!(!should_prime_binance_spot_ticker(&book, "btcusdt"));
        assert!(!should_prime_binance_spot_ticker(&book, "BTCUSDT"));
        assert!(should_prime_binance_spot_ticker(&book, "ethusdt"));
    }

    /// An id that builds no call is never worth priming either.
    #[test]
    fn ids_with_no_call_are_not_primed() {
        let book = TickBook::new();
        assert!(!should_prime_binance_spot_ticker(
            &book,
            "BTC-200730-9000-C"
        ));
        assert!(!should_prime_binance_spot_ticker(&book, "nse_cm|2885"));
        assert!(!should_prime_binance_spot_ticker(&book, ""));
    }

    /// The live spot call: symbol-scoped (weight 2), MarketData lane, COM spot book.
    #[test]
    fn spot_symbol_builds_a_symbol_scoped_market_data_get() {
        let call = binance_spot_ticker_call("ethusdt").expect("spot id builds a call");
        assert_eq!(call.method, "GET");
        assert_eq!(call.host, "api.binance.com");
        assert_eq!(call.path, "/api/v3/ticker/price");
        assert_eq!(call.query, "symbol=ETHUSDT");
        assert_eq!(call.lane, Lane::MarketData);
        assert_eq!(call.book_id, BINANCE_COM_SPOT_BOOK_ID);
        // Never the no-symbol form (weight 4) and never the 24hr endpoint (weight 80).
        assert!(!call.query.is_empty());
        assert_ne!(call.path, "/api/v3/ticker/24hr");
        assert_ne!(call.path, "/eapi/v1/ticker");
    }

    /// Mixed case in, upper-case `symbol=` out — COM matches symbols case-sensitively.
    #[test]
    fn symbol_is_upper_cased_and_trimmed() {
        let call = binance_spot_ticker_call("  EthUsdt  ").expect("call");
        assert_eq!(call.query, "symbol=ETHUSDT");
    }

    /// A dated contract belongs to the options book, which has its own last.
    /// No call is built at all, so nothing reaches the engine to be refused.
    #[test]
    fn dated_option_contract_builds_no_call() {
        assert!(binance_spot_ticker_call("BTC-200730-9000-C").is_none());
        assert!(binance_spot_ticker_call("btc-260925-145000-c").is_none());
    }

    /// Kotak `seg|token` ids are not COM symbols.
    #[test]
    fn kotak_pipe_id_builds_no_call() {
        assert!(binance_spot_ticker_call("nse_cm|2885").is_none());
        assert!(binance_spot_ticker_call("nse_fo|12345").is_none());
    }

    /// Empty / blank must never become a no-symbol `GET /ticker/price` (weight 4).
    #[test]
    fn empty_instrument_builds_no_call() {
        assert!(binance_spot_ticker_call("").is_none());
        assert!(binance_spot_ticker_call("   ").is_none());
    }

    /// The guard runs before anything is dialled: a subscribed id, an id that
    /// already has a last, and a non-spot id all return without touching
    /// `inflight` — which is the observable proof no GET was started, since the
    /// owner marks `inflight` before it calls `send`.
    #[tokio::test]
    async fn prime_makes_no_call_for_guarded_ids() {
        let registry = Arc::new(desk_registry());
        let inflight = Arc::new(Mutex::new(HashSet::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let mut cold = TickBook::new();
        cold.subscribe(BINANCE_COM_SPOT_BOOK_ID, "btcusdt");
        let book = Arc::new(Mutex::new(cold));

        for id in ["btcusdt", "BTC-200730-9000-C", "nse_cm|2885", "", "   "] {
            await_binance_spot_ticker_price(
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
            "no guarded id may start a COM GET"
        );
        assert!(errors.lock().expect("errors").is_empty());
    }

    /// An id whose last is already in the book is not re-fetched, so the stored
    /// price survives the second quote GET untouched.
    #[tokio::test]
    async fn prime_skips_an_id_that_already_has_a_last() {
        let registry = Arc::new(desk_registry());
        let mut warm = TickBook::new();
        let tick =
            quote_tick_from_binance_json(r#"{"symbol":"ETHUSDT","price":"3500.12"}"#, received())
                .expect("rest ticker parses");
        apply_quote(registry.as_ref(), &mut warm, tick).expect("applies before subscribe");
        let book = Arc::new(Mutex::new(warm));
        let inflight = Arc::new(Mutex::new(HashSet::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

        await_binance_spot_ticker_price(
            registry.clone(),
            book.clone(),
            inflight.clone(),
            errors.clone(),
            "ETHUSDT",
        )
        .await;

        assert!(inflight.lock().expect("inflight").is_empty());
        assert_eq!(
            book.lock()
                .expect("book")
                .get(BINANCE_COM_SPOT_BOOK_ID, "ethusdt")
                .map(|row| row.last.as_str()),
            Some("3500.12")
        );
    }

    /// The invariant the call-site ordering exists to protect.
    ///
    /// REST *before* the subscribe lands a last; the same tick *after* the
    /// subscribe is refused `RestClosed` and the book stays empty. This is why
    /// the quote handler awaits the prime and only then calls
    /// `subscribe_instrument` — reverse the two and ETHUSDT paints no Last.
    #[test]
    fn rest_lands_before_subscribe_and_is_refused_after() {
        let registry = desk_registry();
        let body = r#"{"symbol":"ETHUSDT","price":"3500.12"}"#;

        // Order the handler uses: prime, then subscribe.
        let mut primed = TickBook::new();
        apply_quote(
            &registry,
            &mut primed,
            quote_tick_from_binance_json(body, received()).expect("parses"),
        )
        .expect("rest applies while unsubscribed");
        primed.subscribe(BINANCE_COM_SPOT_BOOK_ID, "ethusdt");
        assert_eq!(
            primed
                .get(BINANCE_COM_SPOT_BOOK_ID, "ethusdt")
                .map(|row| row.last.as_str()),
            Some("3500.12"),
            "a last applied before the subscribe survives it"
        );

        // Reversed order: the same tick is refused and Last stays empty.
        let mut reversed = TickBook::new();
        reversed.subscribe(BINANCE_COM_SPOT_BOOK_ID, "ethusdt");
        let err = apply_quote(
            &registry,
            &mut reversed,
            quote_tick_from_binance_json(body, received()).expect("parses"),
        )
        .expect_err("rest is closed once subscribed");
        assert_eq!(
            err,
            crate::data::apply::ApplyError::RestClosed {
                instrument_id: "ethusdt".to_string()
            }
        );
        assert!(reversed.get(BINANCE_COM_SPOT_BOOK_ID, "ethusdt").is_none());
    }

    /// The parse this module depends on: `symbol` + `price` is a REST last on the
    /// spot book, keyed by the lower-cased id the WS stream also uses.
    #[test]
    fn ticker_price_body_parses_as_a_rest_last_on_the_spot_book() {
        let tick =
            quote_tick_from_binance_json(r#"{"symbol":"ETHUSDT","price":"3500.12"}"#, received())
                .expect("parses");
        assert_eq!(tick.instrument_id, "ethusdt");
        assert_eq!(tick.last, "3500.12");
        assert_eq!(tick.transport, crate::data::tick::Transport::Rest);
        assert_eq!(tick.book_id, BINANCE_COM_SPOT_BOOK_ID);
        assert!(tick.age_unknown, "ticker/price carries no venue timestamp");
    }
}

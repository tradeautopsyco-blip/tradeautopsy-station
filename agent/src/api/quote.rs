//! S1 desk extract on Station loopback. TickBook ≠ BAR LiveBook. No ingestSignal. No Neon.

use crate::api::AppState;
use crate::data::{
    ensure_binance_com_options_quote, extract_quote_for_book, is_dated_option_contract,
    normalize_options_instrument, parse_nfo_instrument_id, QuoteEnvelope, QuoteStatus,
    BINANCE_COM_OPTIONS_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct QuoteQuery {
    pub instrument: Option<String>,
    /// Named TickBook slot. Only `binance-com-options` is honoured; see `named_book_for`.
    pub book: Option<String>,
}

/// Named TickBook slot for the extract. `binance-com-options` is the only book a
/// caller may request by name; every other `book=` value — `binance-com-spot`
/// included — is ignored, so spot / NFO routing stays exactly as it was.
/// A dated contract routes to the options book with no `book=` at all, matching
/// the desk bind (`DeskInstrumentBind` resolves `BTC-200730-9000-C` with `bookId == nil`).
/// The options book wins over the `nse_fo|` shape: the options lookup misses and
/// the envelope is `unavailable` rather than NFO data.
pub(crate) fn named_book_for(
    instrument: &str,
    requested_book: Option<&str>,
) -> Option<&'static str> {
    if requested_book.map(str::trim) == Some(BINANCE_COM_OPTIONS_BOOK_ID)
        || is_dated_option_contract(instrument)
    {
        return Some(BINANCE_COM_OPTIONS_BOOK_ID);
    }
    parse_nfo_instrument_id(instrument).map(|_| KOTAK_NSE_NFO_BOOK_ID)
}

/// What the quote route may open for `id`. Dated option contracts take the
/// options-quote record only — never spot `@trade` / depth / klines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteSubscription {
    OptionsQuote,
    Desk,
    None,
}

/// Way 3 last-only: shape, not `book=`, decides the stream. `desk_would_subscribe`
/// is `AppState::should_subscribe_quote`.
pub(crate) fn quote_subscription_for(id: &str, desk_would_subscribe: bool) -> QuoteSubscription {
    if is_dated_option_contract(id) {
        return QuoteSubscription::OptionsQuote;
    }
    if desk_would_subscribe {
        return QuoteSubscription::Desk;
    }
    QuoteSubscription::None
}

pub async fn handler(
    State(state): State<AppState>,
    Query(query): Query<QuoteQuery>,
) -> Json<QuoteEnvelope> {
    let raw = query
        .instrument
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_default();
    let adapter = state.active_adapter_id();
    if raw.trim().is_empty() {
        let book = state.tickbook.lock().expect("tickbook mutex poisoned");
        return Json(extract_quote_for_book(
            state.quote_registry.as_ref(),
            &book,
            "",
            Utc::now(),
            state.quote_freshness,
            adapter.as_deref(),
            None,
        ));
    }

    let instrument = match state.validated_quote_id(&raw) {
        Some(id) => {
            let nfo = parse_nfo_instrument_id(&id).is_some();
            match quote_subscription_for(&id, state.should_subscribe_quote(&id)) {
                QuoteSubscription::OptionsQuote => {
                    *state
                        .selected_quote_instrument
                        .lock()
                        .expect("selected quote instrument poisoned") = Some(id.clone());
                    ensure_binance_com_options_quote(&state.quote_streams, &id);
                }
                QuoteSubscription::Desk => {
                    *state
                        .selected_quote_instrument
                        .lock()
                        .expect("selected quote instrument poisoned") = Some(id.clone());
                    if state.is_kotak_neo_desk() {
                        state.prime_kotak_quote(&id).await;
                    } else {
                        state.subscribe_instrument(&id);
                    }
                }
                QuoteSubscription::None => {
                    if state.is_kotak_neo_desk() && nfo {
                        *state
                            .selected_quote_instrument
                            .lock()
                            .expect("selected quote instrument poisoned") = Some(id.clone());
                        state.prime_kotak_quote(&id).await;
                    }
                }
            }
            id
        }
        None => {
            if state.is_kotak_neo_desk() {
                raw.trim().to_string()
            } else if is_dated_option_contract(&raw) {
                normalize_options_instrument(&raw)
            } else {
                state.resolve_instrument(&raw)
            }
        }
    };

    let named_book = named_book_for(&instrument, query.book.as_deref());
    let book = state.tickbook.lock().expect("tickbook mutex poisoned");
    let mut env = extract_quote_for_book(
        state.quote_registry.as_ref(),
        &book,
        &instrument,
        Utc::now(),
        state.quote_freshness,
        adapter.as_deref(),
        named_book,
    );
    if env.status == QuoteStatus::Unavailable {
        if let Some(class) = state
            .quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .get(&instrument)
            .cloned()
        {
            env.ineligible = vec![class];
        }
    }
    Json(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_instrument_query_is_unresolved() {
        let none = QuoteQuery {
            instrument: None,
            book: None,
        };
        let blank = QuoteQuery {
            instrument: Some("  ".into()),
            book: None,
        };
        let present = QuoteQuery {
            instrument: Some("nse_cm|2885".into()),
            book: None,
        };
        assert!(none.instrument.filter(|s| !s.trim().is_empty()).is_none());
        assert!(blank.instrument.filter(|s| !s.trim().is_empty()).is_none());
        assert_eq!(
            present
                .instrument
                .filter(|s| !s.trim().is_empty())
                .as_deref(),
            Some("nse_cm|2885")
        );
    }

    #[test]
    fn nse_fo_extract_uses_nfo_book_spot_does_not() {
        assert_eq!(
            parse_nfo_instrument_id("nse_fo|12345").map(|_| KOTAK_NSE_NFO_BOOK_ID),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
        assert!(parse_nfo_instrument_id("nse_cm|2885")
            .map(|_| KOTAK_NSE_NFO_BOOK_ID)
            .is_none());
        assert!(parse_nfo_instrument_id("btcusdt")
            .map(|_| KOTAK_NSE_NFO_BOOK_ID)
            .is_none());
    }

    #[test]
    fn named_book_for_only_honours_the_options_book() {
        assert_eq!(
            named_book_for("BTC-200730-9000-C", Some(BINANCE_COM_OPTIONS_BOOK_ID)),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
        assert_eq!(
            named_book_for("nse_fo|12345", None),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
        assert_eq!(named_book_for("nse_cm|2885", None), None);
        assert_eq!(named_book_for("btcusdt", None), None);
        // No `book=` on a dated contract still routes to the options book — the
        // desk binds these with `bookId == nil`. Shape decides, not the query.
        assert_eq!(
            named_book_for("BTC-200730-9000-C", None),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
        // An ignored `book=` does not drag a dated contract onto the spot slot.
        assert_eq!(
            named_book_for("BTC-200730-9000-C", Some("binance-com-spot")),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
        // Unknown / other book names are ignored — never a silent named book.
        assert_eq!(named_book_for("btcusdt", Some("binance-com-spot")), None);
        assert_eq!(named_book_for("btcusdt", Some("kotak-nse-nfo")), None);
        assert_eq!(named_book_for("btcusdt", Some("nonsense")), None);
        assert_eq!(named_book_for("btcusdt", Some("")), None);
        // Options book wins over the NFO shape; the lookup misses rather than
        // serving NFO data.
        assert_eq!(
            named_book_for("nse_fo|12345", Some(BINANCE_COM_OPTIONS_BOOK_ID)),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
        // An unknown book on an `nse_fo|` id keeps today's NFO routing.
        assert_eq!(
            named_book_for("nse_fo|12345", Some("nonsense")),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
    }

    #[test]
    fn options_id_reaches_the_extract_in_verbatim_case() {
        let id = crate::data::normalize_options_instrument("  BTC-200730-9000-C  ");
        assert_eq!(id, "BTC-200730-9000-C");
        assert_ne!(id, "btc-200730-9000-c");
        assert_ne!(id, crate::data::normalize_quote_instrument("BTC-200730-9000-C"));
        assert_eq!(
            named_book_for(&id, Some(BINANCE_COM_OPTIONS_BOOK_ID)),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
    }

    /// The seam for "no spot stream opens": a dated contract never reaches
    /// `subscribe_instrument`, and the only key it records is `{book}\0{symbol}`.
    #[test]
    fn dated_contract_records_options_key_and_no_spot_key() {
        assert_eq!(
            quote_subscription_for("BTC-200730-9000-C", true),
            QuoteSubscription::OptionsQuote
        );
        assert_eq!(
            quote_subscription_for("BTC-200730-9000-C", false),
            QuoteSubscription::OptionsQuote
        );
        assert_eq!(
            quote_subscription_for("btcusdt", true),
            QuoteSubscription::Desk
        );
        assert_eq!(
            quote_subscription_for("btcusdt", false),
            QuoteSubscription::None
        );
        assert_eq!(
            quote_subscription_for("nse_fo|12345", true),
            QuoteSubscription::Desk
        );

        let spawned = std::sync::Arc::new(std::sync::Mutex::new(
            std::collections::HashSet::<String>::new(),
        ));
        ensure_binance_com_options_quote(&spawned, "BTC-200730-9000-C");
        let set = spawned.lock().expect("quote stream set poisoned");
        assert_eq!(set.len(), 1);
        assert!(set.contains(&format!(
            "{BINANCE_COM_OPTIONS_BOOK_ID}\0BTC-200730-9000-C"
        )));
        // `ensure_binance_com_trade_stream` inserts the bare lowercased spot id.
        assert!(!set.contains("btc-200730-9000-c"));
        assert!(!set.contains("BTC-200730-9000-C"));
        assert!(!set.contains("btcusdt"));
    }
}

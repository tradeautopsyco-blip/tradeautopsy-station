//! S1 desk extract on Station loopback. TickBook ≠ BAR LiveBook. No ingestSignal. No Neon.

use crate::api::quote_selection::QuoteBindError;
use crate::api::AppState;
use crate::data::{
    extract_quote_for_book, is_dated_option_contract, kotak_quote_book_id, parse_nfo_instrument_id,
    refused_quote_binding, QuoteEnvelope, QuoteStatus, BINANCE_COM_COINM_BOOK_ID,
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, BINANCE_COM_USDM_BOOK_ID,
    KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct QuoteQuery {
    pub instrument: Option<String>,
    /// Named TickBook slot (`binance-com-options`, `binance-com-spot`, etc.).
    pub book: Option<String>,
}

fn inferred_quote_book(raw: &str) -> Option<&'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if is_dated_option_contract(raw) {
        return Some(BINANCE_COM_OPTIONS_BOOK_ID);
    }
    if parse_nfo_instrument_id(raw).is_some() {
        return Some(KOTAK_NSE_NFO_BOOK_ID);
    }
    if kotak_quote_book_id(raw) == Some(KOTAK_NSE_BSE_CASH_BOOK_ID) {
        return Some(KOTAK_NSE_BSE_CASH_BOOK_ID);
    }
    Some(BINANCE_COM_SPOT_BOOK_ID)
}

fn refused_quote_envelope(
    state: &AppState,
    err: QuoteBindError,
    requested_book: Option<&str>,
    raw: &str,
) -> QuoteEnvelope {
    let book_id = err.book_id_for_refusal(requested_book, inferred_quote_book(raw));
    refused_quote_binding(
        state.quote_registry.as_ref(),
        state.active_adapter_id().as_deref(),
        &book_id,
        err.refusal_class(),
    )
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

    let binding = match state.validate_quote_binding(&raw, query.book.as_deref()) {
        Ok(binding) => binding,
        Err(err) => {
            return Json(refused_quote_envelope(
                &state,
                err,
                query.book.as_deref(),
                &raw,
            ));
        }
    };

    state.bind_quote_selection(&binding).await;

    let book = state.tickbook.lock().expect("tickbook mutex poisoned");
    let mut env = extract_quote_for_book(
        state.quote_registry.as_ref(),
        &book,
        &binding.instrument_id,
        Utc::now(),
        state.quote_freshness,
        adapter.as_deref(),
        Some(binding.book_id),
    );
    env.book_id = Some(binding.book_id.to_string());
    env.bind_status = Some("bound".to_string());
    match binding.book_id {
        BINANCE_COM_USDM_BOOK_ID => {
            env.tick_size = crate::data::tick_size_for(&state, &binding.instrument_id);
            env.step_size = crate::data::step_size_for(&state, &binding.instrument_id);
        }
        BINANCE_COM_COINM_BOOK_ID => {
            env.tick_size = crate::data::coinm_tick_size_for(&state, &binding.instrument_id);
            env.step_size = crate::data::coinm_step_size_for(&state, &binding.instrument_id);
        }
        _ => {}
    }
    if env.status == QuoteStatus::Unavailable {
        if let Some(class) = state
            .quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .get(&binding.instrument_id)
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
    use crate::api::desk::bind_spot_market_ids;
    use crate::data::{
        quote_subscription_for, MarketBind, QuoteSubscription, BINANCE_COM_USDM_BOOK_ID,
    };

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
    fn inferred_quote_book_routes_options_nfo_cash_and_spot() {
        assert_eq!(
            inferred_quote_book("BTC-200730-9000-C"),
            Some(BINANCE_COM_OPTIONS_BOOK_ID)
        );
        assert_eq!(
            inferred_quote_book("nse_fo|12345"),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
        assert_eq!(
            inferred_quote_book("nse_cm|2885"),
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID)
        );
        assert_eq!(
            inferred_quote_book("btcusdt"),
            Some(BINANCE_COM_SPOT_BOOK_ID)
        );
        assert_eq!(
            inferred_quote_book("BTCUSDT"),
            Some(BINANCE_COM_SPOT_BOOK_ID)
        );
        assert_ne!(
            inferred_quote_book("BTCUSDT"),
            Some(BINANCE_COM_USDM_BOOK_ID)
        );
        assert_eq!(inferred_quote_book(""), None);
        assert_eq!(inferred_quote_book("  "), None);
    }

    use crate::data::normalize_options_instrument;

    #[test]
    fn options_id_reaches_the_extract_in_verbatim_case() {
        let id = normalize_options_instrument("  BTC-200730-9000-C  ");
        assert_eq!(id, "BTC-200730-9000-C");
        assert_ne!(id, "btc-200730-9000-c");
        assert_ne!(
            id,
            crate::data::normalize_quote_instrument("BTC-200730-9000-C")
        );
        assert_eq!(inferred_quote_book(&id), Some(BINANCE_COM_OPTIONS_BOOK_ID));
    }

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
        let trade = MarketBind::new();
        let depth = MarketBind::new();
        bind_spot_market_ids("BTC-200730-9000-C", &trade, &depth, &spawned);
        let set = spawned.lock().expect("quote stream set poisoned");
        assert_eq!(set.len(), 1);
        assert!(set.contains(&format!("{BINANCE_COM_OPTIONS_BOOK_ID}\0BTC-200730-9000-C")));
        assert!(trade.current().is_none());
        assert!(depth.current().is_none());
        assert_ne!(trade.current().as_deref(), Some("btc-200730-9000-c"));
        assert!(!set.contains("btc-200730-9000-c"));
        assert!(!set.contains("BTC-200730-9000-C"));
        assert!(!set.contains("btcusdt"));
    }
}

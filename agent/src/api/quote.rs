//! S1 desk extract on Station loopback. TickBook ≠ BAR LiveBook. No ingestSignal. No Neon.

use crate::api::AppState;
use crate::data::{
    extract_quote_for_book, parse_nfo_instrument_id, QuoteEnvelope, QuoteStatus,
    KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct QuoteQuery {
    pub instrument: Option<String>,
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
            if state.should_subscribe_quote(&id) {
                *state
                    .selected_quote_instrument
                    .lock()
                    .expect("selected quote instrument poisoned") = Some(id.clone());
                if state.is_kotak_neo_desk() {
                    state.prime_kotak_quote(&id).await;
                } else {
                    state.subscribe_instrument(&id);
                }
            } else if state.is_kotak_neo_desk() && nfo {
                *state
                    .selected_quote_instrument
                    .lock()
                    .expect("selected quote instrument poisoned") = Some(id.clone());
                state.prime_kotak_quote(&id).await;
            }
            id
        }
        None => {
            if state.is_kotak_neo_desk() {
                raw.trim().to_string()
            } else {
                state.resolve_instrument(&raw)
            }
        }
    };

    let named_book = parse_nfo_instrument_id(&instrument).map(|_| KOTAK_NSE_NFO_BOOK_ID);
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
        let none = QuoteQuery { instrument: None };
        let blank = QuoteQuery {
            instrument: Some("  ".into()),
        };
        let present = QuoteQuery {
            instrument: Some("nse_cm|2885".into()),
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
}

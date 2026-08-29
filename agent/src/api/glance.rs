//! Options chain / OI extracts. Wire `status` is unavailable until a named book
//! can Success. Do not default `s1_desk_symbol` (spot) as an NFO underlying.

use crate::api::AppState;
use crate::data::{
    extract_chain_from, extract_open_interest_from, parse_nfo_instrument_id, ChainRow,
    GlanceEnvelope, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GlanceQuery {
    pub book: Option<String>,
    pub instrument: Option<String>,
}

fn query_book(query: &GlanceQuery) -> Option<String> {
    query
        .book
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn query_instrument(query: &GlanceQuery) -> String {
    query
        .instrument
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

fn nfo_chain_rows(state: &AppState, instrument: &str) -> Vec<ChainRow> {
    let master = state
        .kotak_nfo_scrip_master
        .lock()
        .expect("kotak nfo scrip master mutex poisoned");
    master
        .rows_for_underlying(instrument)
        .into_iter()
        .map(|row| row.to_chain_row())
        .collect()
}

fn kick_nfo_visible_quotes(state: &AppState, rows: &[ChainRow]) {
    for row in rows {
        if parse_nfo_instrument_id(&row.instrument_id).is_none() {
            continue;
        }
        state.kick_kotak_rest_quote(&row.instrument_id);
    }
}

pub async fn chain_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    match book.as_deref() {
        Some(id) if id == KOTAK_NSE_NFO_BOOK_ID => {
            let rows = nfo_chain_rows(&state, &instrument);
            kick_nfo_visible_quotes(&state, &rows);
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            Json(extract_chain_from(
                Some(KOTAK_NSE_NFO_BOOK_ID),
                &instrument,
                Some(rows.as_slice()),
                Some(&tickbook),
            ))
        }
        Some(id) if id == KOTAK_NSE_BSE_CASH_BOOK_ID => Json(extract_chain_from(
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID),
            &instrument,
            None,
            None,
        )),
        other => Json(extract_chain_from(other, &instrument, None, None)),
    }
}

pub async fn oi_handler(
    State(_state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    Json(extract_open_interest_from(book.as_deref(), &instrument))
}

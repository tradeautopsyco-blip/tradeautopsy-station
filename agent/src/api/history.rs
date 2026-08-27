//! S2 history extract on Station loopback. No Neon. No ingestSignal.
//!
//! `source=yahoo` stays unit-test-only (`rights_forbid_canonical`).
//! Licensed product path is Binance.com klines when the desk is not Kotak.

use crate::api::AppState;
use crate::data::{
    extract_history, extract_licensed_history, HistoryEnvelope, DEFAULT_HISTORY_INTERVAL,
};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub instrument: Option<String>,
    pub source: Option<String>,
    pub interval: Option<String>,
    pub limit: Option<u32>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Json<HistoryEnvelope> {
    let raw = query
        .instrument
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| state.s1_desk_symbol.clone())
        .unwrap_or_default();
    let instrument = state.resolve_instrument(&raw);
    Json(extract_station_history(&state, &instrument, &query))
}

fn extract_station_history(
    state: &AppState,
    instrument: &str,
    query: &HistoryQuery,
) -> HistoryEnvelope {
    let source = query.source.as_deref();
    let yahoo = source
        .map(|s| {
            let n = s.trim().to_ascii_lowercase();
            n == "yahoo" || n == "yahoo_chart"
        })
        .unwrap_or(false);
    if yahoo {
        return extract_history(instrument, source);
    }
    if state.is_kotak_neo_desk() {
        return extract_history(instrument, None);
    }
    let interval = query
        .interval
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_HISTORY_INTERVAL);
    let book = state
        .historybook
        .lock()
        .expect("historybook mutex poisoned");
    extract_licensed_history(&book, instrument, Some(interval), query.limit)
}

//! S2 history extract on Station loopback. No Neon. No ingestSignal.
//!
//! `source=yahoo` stays unit-test-only (`rights_forbid_canonical`).
//! Licensed product path is Binance.com klines when the desk is not Kotak.

use crate::api::AppState;
use crate::data::{
    extract_history, extract_licensed_history, extract_options_history, is_dated_option_contract,
    normalize_options_instrument, overlay_json_candles, HistoryEnvelope, BINANCE_COM_ADAPTER_ID,
    DEFAULT_HISTORY_INTERVAL, DEFAULT_OPTIONS_HISTORY_INTERVAL,
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
    if crate::data::is_dated_option_contract(&raw) {
        let instrument = crate::data::normalize_options_instrument(&raw);
        return Json(extract_station_history(&state, &instrument, &query));
    }
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
    if is_dated_option_contract(instrument) {
        let instrument = normalize_options_instrument(instrument);
        let interval = query
            .interval
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_OPTIONS_HISTORY_INTERVAL);
        let book = state
            .historybook
            .lock()
            .expect("historybook mutex poisoned");
        let mut env = extract_options_history(&book, &instrument, Some(interval), query.limit);
        if let Some(data) = env.data.as_mut() {
            let builders = state
                .candle_builders
                .lock()
                .expect("candle builders mutex poisoned");
            overlay_json_candles(
                data,
                &builders,
                BINANCE_COM_ADAPTER_ID,
                &instrument,
                interval,
            );
        }
        return env;
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
    let mut env = extract_licensed_history(&book, instrument, Some(interval), query.limit);
    if let Some(data) = env.data.as_mut() {
        let builders = state
            .candle_builders
            .lock()
            .expect("candle builders mutex poisoned");
        overlay_json_candles(
            data,
            &builders,
            BINANCE_COM_ADAPTER_ID,
            instrument,
            interval,
        );
    }
    env
}

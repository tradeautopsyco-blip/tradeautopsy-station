//! Options chain / OI extracts. Wire `status` is unavailable until a named book
//! can Success. Do not default `s1_desk_symbol` (spot) as an NFO underlying.

use crate::api::AppState;
use crate::data::{
    chain_rows_for_contract, expiration_from_dated_contract, extract_chain_from,
    extract_open_interest, extract_open_interest_from, oi_rows_from_json,
    option_symbols_from_exchange_info_json, parse_nfo_instrument_id,
    underlying_asset_from_dated_contract, ChainRow, GlanceEnvelope, OptionsOiRow,
    BINANCE_COM_OPTIONS_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use std::time::Duration;

use crate::egress::{EgressCall, Lane};

/// Options contract lists change on expiry boundaries, not per request.
const EXCHANGE_INFO_MAX_AGE_MS: i64 = 300_000;
/// Open interest is a slow series; one call serves every panel asking at once.
const OPEN_INTEREST_MAX_AGE_MS: i64 = 5_000;

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

fn options_chain_rows(state: &AppState, instrument: &str) -> Vec<ChainRow> {
    let master = state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned");
    chain_rows_for_contract(instrument, &master)
}

pub(crate) async fn ensure_options_master(state: &AppState) {
    {
        let master = state
            .options_option_symbols
            .lock()
            .expect("options option symbols mutex poisoned");
        if !master.is_empty() || !state.eapi_public_fetch {
            return;
        }
    }
    // The engine runs the same `authorize_book_call` fence before it charges
    // anything, so the check is not repeated here.
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "/eapi/v1/exchangeInfo",
        Lane::MarketData,
    )
    .with_max_age_ms(EXCHANGE_INFO_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let rows = option_symbols_from_exchange_info_json(&resp.body);
    if rows.is_empty() {
        return;
    }
    *state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned") = rows;
}

pub(crate) async fn fetch_options_oi(state: &AppState, instrument: &str) -> Vec<OptionsOiRow> {
    {
        let planted = state
            .options_oi_rows
            .lock()
            .expect("options oi mutex poisoned");
        if !planted.is_empty() {
            return planted.clone();
        }
        if !state.eapi_public_fetch {
            return Vec::new();
        }
    }
    let Some(underlying) = underlying_asset_from_dated_contract(instrument) else {
        return Vec::new();
    };
    let Some(expiration) = expiration_from_dated_contract(instrument) else {
        return Vec::new();
    };
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "/eapi/v1/openInterest",
        Lane::MarketData,
    )
    .with_query(format!(
        "underlyingAsset={underlying}&expiration={expiration}"
    ))
    .with_max_age_ms(OPEN_INTEREST_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return Vec::new();
    };
    if !resp.is_success() {
        return Vec::new();
    }
    oi_rows_from_json(&resp.body)
}

/// Obtain enriches synchronously, but the OI snapshot is an async fetch. Land
/// the rows in the shared store so the enricher only ever reads state.
pub(crate) async fn ensure_options_oi(state: &AppState, instrument: &str) {
    {
        let planted = state
            .options_oi_rows
            .lock()
            .expect("options oi mutex poisoned");
        if !planted.is_empty() {
            return;
        }
    }
    let rows = fetch_options_oi(state, instrument).await;
    if rows.is_empty() {
        return;
    }
    *state
        .options_oi_rows
        .lock()
        .expect("options oi mutex poisoned") = rows;
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
        Some(id) if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            ensure_options_master(&state).await;
            let rows = options_chain_rows(&state, &instrument);
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            Json(extract_chain_from(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                &instrument,
                if rows.is_empty() {
                    None
                } else {
                    Some(rows.as_slice())
                },
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
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    match book.as_deref() {
        Some(id) if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            let rows = fetch_options_oi(&state, &instrument).await;
            Json(extract_open_interest_from(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                &instrument,
                if rows.is_empty() {
                    None
                } else {
                    Some(rows.as_slice())
                },
            ))
        }
        other => Json(extract_open_interest(other, &instrument)),
    }
}

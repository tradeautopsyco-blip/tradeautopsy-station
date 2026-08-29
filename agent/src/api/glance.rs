//! Options chain / OI extracts. Wire `status` is unavailable until a named book
//! can Success. Do not default `s1_desk_symbol` (spot) as an NFO underlying.

use crate::api::AppState;
use crate::data::{
    authorize_book_call, chain_rows_for_contract, expiration_from_dated_contract,
    extract_chain_from, extract_open_interest, extract_open_interest_from, oi_rows_from_json,
    option_symbols_from_exchange_info_json, parse_nfo_instrument_id,
    underlying_asset_from_dated_contract, ChainRow, GlanceEnvelope, OptionsOiRow,
    BINANCE_COM_OPTIONS_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use std::time::Duration;

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

async fn ensure_options_master(state: &AppState) {
    {
        let master = state
            .options_option_symbols
            .lock()
            .expect("options option symbols mutex poisoned");
        if !master.is_empty() || !state.eapi_public_fetch {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "GET",
        "/eapi/v1/exchangeInfo",
        false,
    )
    .is_err()
    {
        return;
    }
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    else {
        return;
    };
    let Ok(resp) = client
        .get("https://eapi.binance.com/eapi/v1/exchangeInfo")
        .send()
        .await
    else {
        return;
    };
    if !resp.status().is_success() {
        return;
    }
    let Ok(body) = resp.text().await else {
        return;
    };
    let rows = option_symbols_from_exchange_info_json(&body);
    if rows.is_empty() {
        return;
    }
    *state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned") = rows;
}

async fn fetch_options_oi(state: &AppState, instrument: &str) -> Vec<OptionsOiRow> {
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
    let path =
        format!("/eapi/v1/openInterest?underlyingAsset={underlying}&expiration={expiration}");
    if authorize_book_call(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "GET",
        &path,
        false,
    )
    .is_err()
    {
        return Vec::new();
    }
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
    else {
        return Vec::new();
    };
    let url = format!("https://eapi.binance.com{path}");
    let Ok(resp) = client.get(&url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(body) = resp.text().await else {
        return Vec::new();
    };
    oi_rows_from_json(&body)
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

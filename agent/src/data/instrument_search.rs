//! Obtain `search` — local projection from resident masters (no vendor hop).
//!
//! Same row shapes as `GET /instruments/search`; `q` len < 2 → empty success.

use crate::api::AppState;
use crate::data::{
    extract_quote, extract_quote_for, extract_quote_for_book, normalize_quote_instrument,
    BINANCE_COM_COINM_BOOK_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
    BINANCE_COM_USDM_BOOK_ID, KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use chrono::Utc;
use serde_json::{json, Value};

const SEARCH_LIMIT: usize = 10;

pub fn search_identity() -> Value {
    json!({
        "family": "reference",
        "capability_id": "instrument_search",
        "physics": "bounded_snapshot",
    })
}

/// Book-keyed prefix search. `q` trimmed; len < 2 → `[]`.
pub fn search_rows_for_book(state: &AppState, book_id: &str, q: &str) -> Vec<Value> {
    let q = q.trim();
    if q.len() < 2 {
        return Vec::new();
    }
    match book_id {
        id if id == BINANCE_COM_SPOT_BOOK_ID => binance_spot_rows(state, q),
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => kotak_cash_rows(state, q),
        id if id == KOTAK_NSE_NFO_BOOK_ID => kotak_nfo_rows(state, q),
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => binance_options_rows(state, q),
        id if id == BINANCE_COM_USDM_BOOK_ID => binance_usdm_rows(state, q),
        id if id == BINANCE_COM_COINM_BOOK_ID => binance_coinm_rows(state, q),
        _ => Vec::new(),
    }
}

fn binance_spot_rows(state: &AppState, q: &str) -> Vec<Value> {
    let master = state
        .instrument_master
        .lock()
        .expect("instrument master mutex poisoned");
    let mut hits = master.search_trading(q, SEARCH_LIMIT);
    drop(master);
    if hits.is_empty() {
        hits = state
            .resolve_candidates()
            .into_iter()
            .filter(|id| crate::data::resolve_among(q, std::iter::once(id.as_str())).is_some())
            .map(|id| crate::exchange_info::InstrumentSearchHit {
                symbol: id.to_ascii_uppercase(),
                base_asset: id.to_ascii_uppercase(),
                quote_asset: String::new(),
            })
            .take(SEARCH_LIMIT)
            .collect();
    }
    hits.into_iter()
        .map(|hit| {
            let instrument_id = normalize_quote_instrument(&hit.symbol);
            let last_price =
                last_price_for(state, &instrument_id, Some(BINANCE_COM_SPOT_BOOK_ID), None);
            json!({
                "trading_symbol": hit.symbol,
                "name": hit.base_asset,
                "exchange": "binance_com",
                "segment": "SPOT",
                "instrument_token": 0,
                "last_price": last_price,
            })
        })
        .collect()
}

fn kotak_cash_rows(state: &AppState, q: &str) -> Vec<Value> {
    let master = state
        .kotak_scrip_master
        .lock()
        .expect("kotak scrip master mutex poisoned");
    let hits = master.search(q, SEARCH_LIMIT);
    drop(master);
    hits.into_iter()
        .map(|hit| {
            let instrument_id =
                crate::kotak_scrip_master::instrument_id(&hit.segment, hit.instrument_token);
            let last_price = last_price_for(
                state,
                &instrument_id,
                Some(KOTAK_NSE_BSE_CASH_BOOK_ID),
                Some(KOTAK_NEO_ADAPTER_ID),
            );
            json!({
                "trading_symbol": hit.trading_symbol,
                "name": hit.name,
                "exchange": hit.exchange,
                "segment": hit.segment,
                "instrument_token": hit.instrument_token,
                "last_price": last_price,
            })
        })
        .collect()
}

fn kotak_nfo_rows(state: &AppState, q: &str) -> Vec<Value> {
    let master = state
        .kotak_nfo_scrip_master
        .lock()
        .expect("kotak nfo scrip master mutex poisoned");
    let hits: Vec<_> = master
        .search(q, SEARCH_LIMIT)
        .into_iter()
        .map(|row| {
            (
                row.trading_symbol.clone(),
                row.name.clone(),
                row.instrument_token,
            )
        })
        .collect();
    drop(master);
    hits.into_iter()
        .map(|(trading_symbol, name, instrument_token)| {
            let instrument_id = format!("nse_fo|{instrument_token}");
            let last_price = last_price_for(
                state,
                &instrument_id,
                Some(KOTAK_NSE_NFO_BOOK_ID),
                Some(KOTAK_NEO_ADAPTER_ID),
            );
            json!({
                "trading_symbol": trading_symbol,
                "name": name,
                "exchange": "kotak_neo",
                "segment": "nse_fo",
                "instrument_token": instrument_token,
                "last_price": last_price,
            })
        })
        .collect()
}

fn binance_options_rows(state: &AppState, q: &str) -> Vec<Value> {
    let master = state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned");
    let q_upper = q.to_ascii_uppercase();
    let hits: Vec<_> = master
        .iter()
        .filter(|row| {
            row.symbol.to_ascii_uppercase().contains(&q_upper)
                || row.underlying.to_ascii_uppercase().contains(&q_upper)
        })
        .take(SEARCH_LIMIT)
        .map(|row| (row.symbol.clone(), row.underlying.clone()))
        .collect();
    drop(master);
    hits.into_iter()
        .map(|(symbol, underlying)| {
            let last_price = last_price_for(
                state,
                &symbol,
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                Some("binance_com"),
            );
            json!({
                "trading_symbol": symbol,
                "name": underlying,
                "exchange": "binance_com",
                "segment": "OPTIONS",
                "instrument_token": 0,
                "last_price": last_price,
            })
        })
        .collect()
}

fn binance_usdm_rows(state: &AppState, q: &str) -> Vec<Value> {
    let hits = state
        .usdm_exchange_info
        .lock()
        .expect("usdm exchange info mutex poisoned")
        .search_symbols(q, SEARCH_LIMIT);
    hits.into_iter()
        .map(|symbol| {
            let last_price = last_price_for_book(state, &symbol, BINANCE_COM_USDM_BOOK_ID);
            json!({
                "trading_symbol": symbol,
                "name": symbol,
                "exchange": "binance_com",
                "segment": "USDM",
                "instrument_token": 0,
                "last_price": last_price,
            })
        })
        .collect()
}

fn binance_coinm_rows(state: &AppState, q: &str) -> Vec<Value> {
    let hits = state
        .coinm_exchange_info
        .lock()
        .expect("coinm exchange info mutex poisoned")
        .search_symbols(q, SEARCH_LIMIT);
    hits.into_iter()
        .map(|symbol| {
            let last_price = last_price_for_book(state, &symbol, BINANCE_COM_COINM_BOOK_ID);
            json!({
                "trading_symbol": symbol,
                "name": symbol,
                "exchange": "binance_com",
                "segment": "COINM",
                "instrument_token": 0,
                "last_price": last_price,
            })
        })
        .collect()
}

fn last_price_for_book(state: &AppState, instrument_id: &str, book_id: &str) -> f64 {
    let book = state.tickbook.lock().expect("tickbook mutex poisoned");
    let env = extract_quote_for_book(
        state.quote_registry.as_ref(),
        &book,
        instrument_id,
        Utc::now(),
        state.quote_freshness,
        Some("binance_com"),
        Some(book_id),
    );
    env.data
        .and_then(|d| d.last.parse::<f64>().ok())
        .filter(|p| *p > 0.0)
        .unwrap_or(0.0)
}

fn last_price_for(
    state: &AppState,
    instrument_id: &str,
    book_id: Option<&str>,
    adapter_id: Option<&str>,
) -> f64 {
    let book = state.tickbook.lock().expect("tickbook mutex poisoned");
    let env = if let Some(adapter) = adapter_id {
        extract_quote_for(
            state.quote_registry.as_ref(),
            &book,
            instrument_id,
            Utc::now(),
            state.quote_freshness,
            Some(adapter),
        )
    } else {
        extract_quote(
            state.quote_registry.as_ref(),
            &book,
            instrument_id,
            Utc::now(),
            state.quote_freshness,
        )
    };
    let _ = book_id;
    env.data
        .and_then(|d| d.last.parse::<f64>().ok())
        .filter(|p| *p > 0.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_identity_is_bounded_snapshot() {
        let id = search_identity();
        assert_eq!(id["family"], "reference");
        assert_eq!(id["capability_id"], "instrument_search");
        assert_eq!(id["physics"], "bounded_snapshot");
    }
}

use crate::api::AppState;
use crate::broker_sync_control::BrokerSyncStartRequest;
use crate::data::{BrokerConnectionRuntime, InstrumentMasterPhase};
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

fn credential_handle(slug: &str, connection_id: &str) -> String {
    format!(
        "keychain:{}:{connection_id}",
        crate::ubi::keychain_service_for(slug)
    )
}

/// Named-book runtime keyed by `book_id`. Never overwrites the shipping slug entry.
fn insert_named_book_runtime(
    map: &mut std::collections::HashMap<String, BrokerConnectionRuntime>,
    slug: &str,
    book_id: &str,
    handle: Option<String>,
    budget: u32,
) {
    if book_id == slug {
        return;
    }
    match BrokerConnectionRuntime::on_start_for_book(slug, book_id, handle, budget) {
        Ok(runtime) => {
            map.insert(book_id.to_string(), runtime);
        }
        Err(err) => tracing::debug!(slug, book_id, error = %err, "named book runtime skipped"),
    }
}

pub async fn start_handler(
    State(state): State<AppState>,
    Json(body): Json<BrokerSyncStartRequest>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    state.broker_sync_control.start(&body).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    let slug = body.broker_slug.trim().to_ascii_lowercase();
    let budget = state
        .source_manifests
        .iter()
        .find(|manifest| manifest.adapter_id == slug)
        .map(crate::data::shared_budget)
        .unwrap_or(60);
    let runtime = BrokerConnectionRuntime::on_start(
        slug.clone(),
        Some(credential_handle(&slug, &body.broker_connection_id)),
        budget,
    );
    {
        let mut map = state
            .broker_connections
            .lock()
            .expect("broker connections mutex poisoned");
        map.insert(slug.clone(), runtime);
        if slug == "binance_com" {
            if let Some(runtime) = map.get_mut(&slug) {
                if let Some(default) = state.s1_desk_symbol.as_deref() {
                    runtime.subscribe(default);
                }
            }
            let options_budget = state
                .source_manifests
                .iter()
                .find(|manifest| manifest.book_id == crate::data::BINANCE_COM_OPTIONS_BOOK_ID)
                .map(crate::data::shared_budget)
                .unwrap_or(60);
            insert_named_book_runtime(
                &mut map,
                &slug,
                crate::data::BINANCE_COM_OPTIONS_BOOK_ID,
                Some(credential_handle(&slug, &body.broker_connection_id)),
                options_budget,
            );
            if let Some(opt_sym) = state.s1_options_symbol.as_deref() {
                if let Some(opt_rt) = map.get_mut(crate::data::BINANCE_COM_OPTIONS_BOOK_ID) {
                    opt_rt.subscribe(opt_sym);
                }
            }
        }
        if slug == "kotak_neo" {
            let nfo_budget = state
                .source_manifests
                .iter()
                .find(|m| m.book_id == crate::data::KOTAK_NSE_NFO_BOOK_ID)
                .map(crate::data::shared_budget)
                .unwrap_or(60);
            insert_named_book_runtime(
                &mut map,
                &slug,
                crate::data::KOTAK_NSE_NFO_BOOK_ID,
                Some(credential_handle(&slug, &body.broker_connection_id)),
                nfo_budget,
            );
        }
    }
    state
        .instrument_master_cancel
        .store(false, Ordering::SeqCst);
    crate::api::desk::spawn_instrument_master_refresh(
        &state,
        &slug,
        &body.environment,
        &body.broker_connection_id,
    );
    if slug == "kotak_neo" {
        crate::api::desk::spawn_nfo_master_refresh(
            &state,
            &body.environment,
            &body.broker_connection_id,
        );
    }
    if slug == "binance_com" {
        if let Some(default) = state.s1_desk_symbol.clone() {
            state.bind_spot_market(&default);
        }
        if let Some(opt_sym) = state.s1_options_symbol.as_deref() {
            crate::data::ensure_binance_com_options_quote(&state.quote_streams, opt_sym);
        }
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn retry_handler(State(state): State<AppState>) -> Json<Value> {
    let _ = state.broker_sync_control.retry_failed_classes();
    if let Some(slug) = state.active_adapter_id() {
        let (environment, connection_id) = if slug == "kotak_neo" {
            state
                .kotak_session_locator
                .lock()
                .expect("kotak session locator poisoned")
                .clone()
                .unwrap_or_default()
        } else {
            (String::new(), String::new())
        };
        state
            .instrument_master_cancel
            .store(false, Ordering::SeqCst);
        crate::api::desk::spawn_instrument_master_refresh(
            &state,
            &slug,
            &environment,
            &connection_id,
        );
        if slug == "kotak_neo" {
            crate::api::desk::spawn_nfo_master_refresh(&state, &environment, &connection_id);
        }
    }
    Json(json!({ "ok": true }))
}

pub async fn stop_handler(State(state): State<AppState>) -> Json<Value> {
    let slug = state
        .broker_status
        .lock()
        .ok()
        .and_then(|st| st.active_broker_slug.clone());
    let _ = state.broker_sync_control.stop();
    state.instrument_master_cancel.store(true, Ordering::SeqCst);
    {
        let mut st = state
            .instrument_master_status
            .lock()
            .expect("instrument master status poisoned");
        if st.phase != InstrumentMasterPhase::Idle {
            st.mark_idle();
        }
    }
    let mut map = state
        .broker_connections
        .lock()
        .expect("broker connections mutex poisoned");
    if let Some(slug) = slug {
        map.remove(&slug);
        if slug == "binance_com" {
            map.remove(crate::data::BINANCE_COM_OPTIONS_BOOK_ID);
        }
        if slug == "kotak_neo" {
            map.remove(crate::data::KOTAK_NSE_NFO_BOOK_ID);
            drop(map);
            *state
                .kotak_session_locator
                .lock()
                .expect("kotak session locator poisoned") = None;
        }
    } else {
        map.clear();
        drop(map);
        *state
            .kotak_session_locator
            .lock()
            .expect("kotak session locator poisoned") = None;
    }
    *state
        .selected_quote_instrument
        .lock()
        .expect("selected quote instrument poisoned") = None;
    Json(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID};
    use std::collections::HashMap;

    #[test]
    fn kotak_start_map_keeps_cash_slug_and_named_nfo() {
        let slug = "kotak_neo".to_string();
        let handle = Some("keychain:test:conn-1".to_string());
        let mut map = HashMap::new();
        let runtime = BrokerConnectionRuntime::on_start(slug.clone(), handle.clone(), 60);
        map.insert(slug.clone(), runtime);
        insert_named_book_runtime(&mut map, &slug, KOTAK_NSE_NFO_BOOK_ID, handle, 60);
        let cash = map.get(&slug).expect("shipping slug stays");
        assert_eq!(cash.book_id, KOTAK_NSE_BSE_CASH_BOOK_ID);
        let nfo = map.get(KOTAK_NSE_NFO_BOOK_ID).expect("named NFO key");
        assert_eq!(nfo.book_id, KOTAK_NSE_NFO_BOOK_ID);
        assert_eq!(nfo.adapter_id, "kotak_neo");
        assert_ne!(slug.as_str(), KOTAK_NSE_NFO_BOOK_ID);
    }
}

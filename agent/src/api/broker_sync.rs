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
    if slug == "binance_com" {
        if let Some(default) = state.s1_desk_symbol.clone() {
            crate::data::ensure_binance_com_trade_stream(
                state.quote_registry.clone(),
                state.tickbook.clone(),
                &state.quote_streams,
                &default,
            );
            crate::data::ensure_binance_com_depth_stream(
                state.depthbook.clone(),
                &state.depth_streams,
                &default,
            );
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
        if slug == "kotak_neo" {
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

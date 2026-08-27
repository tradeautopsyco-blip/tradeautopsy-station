//! Options chain / OI holes. Unavailable until S3 ordered-state.

use crate::api::AppState;
use crate::data::{extract_chain, extract_open_interest, GlanceEnvelope};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GlanceQuery {
    pub instrument: Option<String>,
}

fn instrument_id(state: &AppState, query: GlanceQuery) -> String {
    let raw = query
        .instrument
        .filter(|s| !s.trim().is_empty())
        .or_else(|| state.s1_desk_symbol.clone())
        .unwrap_or_default();
    state.resolve_instrument(&raw)
}

pub async fn chain_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    Json(extract_chain(&instrument_id(&state, query)))
}

pub async fn oi_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    Json(extract_open_interest(&instrument_id(&state, query)))
}

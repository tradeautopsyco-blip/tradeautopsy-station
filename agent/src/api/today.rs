//! GET `/api/daemon/today` — session mirror payload for Station Today + pulse strip.

use crate::api::AppState;
use crate::today::TodayPayload;
use axum::extract::State;
use axum::Json;

pub async fn handler(State(state): State<AppState>) -> Json<TodayPayload> {
    let payload = state
        .today_service
        .build_payload()
        .unwrap_or_else(|_| TodayPayload::unavailable(crate::today::TodayDegradedReason::SyncUnavailable));
    Json(payload)
}

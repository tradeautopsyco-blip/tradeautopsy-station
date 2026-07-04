//! GET `/api/daemon/today` — session mirror payload for Station Today + pulse strip.

use crate::api::AppState;
use axum::extract::State;
use axum::Json;

pub async fn handler(State(state): State<AppState>) -> Json<crate::today::TodayPayload> {
    let payload = state
        .today_service
        .build_payload()
        .unwrap_or_else(|_| crate::today::TodayPayload {
            local_date: chrono::Local::now().date_naive().format("%Y-%m-%d").to_string(),
            performance_basis_not_tax: true,
            degraded_reason: Some(crate::today::TodayDegradedReason::SyncUnavailable),
            learning_baseline: true,
            hero: crate::today::TodayHeroPayload {
                pnl_today_usd: None,
                trades_today: None,
                win_rate: None,
            },
            top_signals: vec![],
            trades: vec![],
            open_position_count: 0,
        });
    Json(payload)
}

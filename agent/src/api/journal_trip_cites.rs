//! GET `/api/daemon/journal/trip-cites` — Station-owned closed-trip P&L keyed by declaration id (Wave J cite).

use crate::api::AppState;
use axum::extract::{Query, State};
use axum::Json;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize, Default)]
pub struct JournalTripCitesParams {
    /// Inclusive local close date `YYYY-MM-DD` (IST for India book).
    week_start: Option<String>,
    /// Inclusive local close date `YYYY-MM-DD`.
    week_end: Option<String>,
}

fn parse_local_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub async fn handler(
    Query(params): Query<JournalTripCitesParams>,
    State(state): State<AppState>,
) -> Json<Value> {
    let (start, end) = match (
        params.week_start.as_deref().and_then(parse_local_date),
        params.week_end.as_deref().and_then(parse_local_date),
    ) {
        (Some(s), Some(e)) if s <= e => (s, e),
        _ => {
            let today = chrono::Local::now().date_naive();
            (today, today)
        }
    };

    let items = state
        .today_service
        .journal_trip_cites_for_local_dates(start, end)
        .unwrap_or_default();

    let rows: Vec<Value> = items
        .into_iter()
        .map(|row| {
            json!({
                "declarationId": row.declaration_id,
                "net": row.net,
                "currency": row.currency,
            })
        })
        .collect();

    Json(json!({ "ok": true, "scope": "week", "items": rows }))
}

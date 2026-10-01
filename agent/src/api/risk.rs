use crate::api::AppState;
use crate::risk::{compute_preview, RiskPreviewInput};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RiskPreviewBody {
    pub book_id: Option<String>,
    pub side: Option<String>,
    pub budget_mode: Option<String>,
    pub budget_value: Option<f64>,
    pub entry: Option<f64>,
    pub stop: Option<f64>,
    pub target: Option<f64>,
    pub override_qty: Option<f64>,
    pub funds_lit: Option<bool>,
    pub funds_balance: Option<f64>,
}

pub async fn preview_handler(
    State(_state): State<AppState>,
    Json(body): Json<RiskPreviewBody>,
) -> Json<Value> {
    let input = RiskPreviewInput {
        book_id: body.book_id.unwrap_or_default(),
        side: body.side.unwrap_or_else(|| "BUY".into()),
        budget_mode: body.budget_mode.unwrap_or_else(|| "risk_percent".into()),
        budget_value: body.budget_value,
        entry: body.entry,
        stop: body.stop,
        target: body.target,
        override_qty: body.override_qty,
        funds_lit: body.funds_lit.unwrap_or(false),
        funds_balance: body.funds_balance,
    };
    Json(compute_preview(input))
}

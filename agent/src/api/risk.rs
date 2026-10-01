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
    pub price_increment: Option<f64>,
    pub multiplier: Option<f64>,
    pub unit_batch_size: Option<f64>,
    pub instrument_role: Option<String>,
    pub leverage: Option<f64>,
    pub symbol: Option<String>,
}

pub async fn preview_handler(
    State(state): State<AppState>,
    Json(body): Json<RiskPreviewBody>,
) -> Json<Value> {
    let book_id = body.book_id.unwrap_or_default();
    let symbol = body.symbol.unwrap_or_default();
    let role = body.instrument_role.unwrap_or_default();
    let mut price_increment = body.price_increment;
    let mut multiplier = body.multiplier;
    if role == "nfo_future" && !symbol.trim().is_empty() {
        if let Ok(Some((lot, tick))) = state.instruments.lot_size_and_tick(symbol.trim()) {
            if multiplier.filter(|m| m.is_finite() && *m > 0.0).is_none() && lot > 0.0 {
                multiplier = Some(lot);
            }
            if price_increment
                .filter(|t| t.is_finite() && *t > 0.0)
                .is_none()
                && tick > 0.0
            {
                price_increment = Some(tick);
            }
        }
    }
    let input = RiskPreviewInput {
        book_id,
        side: body.side.unwrap_or_else(|| "BUY".into()),
        budget_mode: body.budget_mode.unwrap_or_else(|| "risk_percent".into()),
        budget_value: body.budget_value,
        entry: body.entry,
        stop: body.stop,
        target: body.target,
        override_qty: body.override_qty,
        funds_lit: body.funds_lit.unwrap_or(false),
        funds_balance: body.funds_balance,
        price_increment,
        multiplier,
        unit_batch_size: body.unit_batch_size,
        instrument_role: role,
        leverage: body.leverage,
        symbol,
    };
    Json(compute_preview(input))
}

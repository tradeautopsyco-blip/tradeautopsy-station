//! GET `/api/daemon/morning-brief` — desk facts for Brief tab (header B + gate D on client).

use crate::api::AppState;
use axum::extract::State;
use axum::Json;
use chrono::Local;
use serde_json::{json, Value};

pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let payload = state.today_service.build_payload().ok();
    let today = payload
        .as_ref()
        .and_then(|p| p.hero.pnl_today_usd.or(p.hero.pnl_today_inr));
    let trade_count = payload
        .as_ref()
        .and_then(|p| p.hero.trades_today)
        .unwrap_or(0);

    let date_line = Local::now().format("%A · %d %b").to_string();
    let briefing = json!({
        "summary": "Desk stack — book, clock, and yesterday from Today. No demo chart.",
        "briefing": "Honest morning brief from this Mac. Behavioral patterns need history on Console.",
        "trade_count": trade_count,
        "is_new_user": trade_count < 30,
        "patterns": [],
        "nifty_futures": 0,
        "banknifty_futures": 0,
        "nifty_change_pct": 0,
        "banknifty_change_pct": 0,
        "vix": 0,
        "edge_symbols": [],
        "caution_symbols": [],
        "recommendation": "",
        "behavioral_date_line": date_line,
        "behavioral_headline": "",
        "session_pnl_kpi": today,
        "plan_adherence_pct": Value::Null,
        "win_rate_kpi": Value::Null,
        "left_on_table_inr": Value::Null,
        "non_negotiable_rule": Value::Null,
        "own_metrics": Value::Null,
    });

    Json(json!({
        "briefing_contract_version": 1,
        "briefing": briefing,
    }))
}

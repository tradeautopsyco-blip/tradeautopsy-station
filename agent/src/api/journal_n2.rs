//! Journal N2 overlay — condition fire log + week declaration enrichment (Wave 7).

use crate::api::AppState;
use crate::journal_n2::JournalN2Store;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct ConditionFireBody {
    declaration_id: Option<String>,
    rule_id: Option<String>,
    fired_at_ms: Option<i64>,
    working: Option<Value>,
}

pub async fn condition_fire_handler(
    State(state): State<AppState>,
    Json(body): Json<ConditionFireBody>,
) -> (StatusCode, Json<Value>) {
    let decl = body.declaration_id.as_deref().unwrap_or("").trim();
    let rule = body.rule_id.as_deref().unwrap_or("").trim();
    if decl.is_empty() || rule.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "declaration_id and rule_id required" })),
        );
    }
    let fired_at_ms = body
        .fired_at_ms
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let local_date = Local::now().format("%Y-%m-%d").to_string();
    if let Err(e) = state.journal_n2.record_condition_fire(
        decl,
        &local_date,
        rule,
        fired_at_ms,
        body.working,
    ) {
        tracing::warn!("journal n2 condition fire: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": "store_failed" })),
        );
    }
    (StatusCode::OK, Json(json!({ "ok": true })))
}

/// Merge local `n2_day_sheet` into each week-list item when upstream JSON parses.
pub fn enrich_week_declarations(body_text: &str, store: &JournalN2Store) -> String {
    let mut root: Value = match serde_json::from_str(body_text) {
        Ok(v) => v,
        Err(_) => return body_text.to_string(),
    };
    let items = root
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| root.as_array().cloned());
    let items = match items {
        Some(i) => i,
        None => return body_text.to_string(),
    };
    let ids: Vec<String> = items
        .iter()
        .filter_map(|it| it.get("id").and_then(Value::as_str).map(String::from))
        .collect();
    let rows = store.rows_for_ids(&ids).unwrap_or_default();
    if rows.is_empty() {
        return body_text.to_string();
    }
    let enriched: Vec<Value> = items
        .into_iter()
        .map(|mut it| {
            let id = it
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if let Some(row) = rows.get(&id) {
                let plan = it.get("snapshot").cloned();
                it.as_object_mut().map(|o| {
                    o.insert(
                        "n2_day_sheet".into(),
                        JournalN2Store::n2_day_sheet_json(row, plan.as_ref()),
                    );
                });
            }
            it
        })
        .collect();
    if root.get("items").is_some() {
        if let Some(obj) = root.as_object_mut() {
            obj.insert("items".into(), Value::Array(enriched));
        }
    } else {
        root = Value::Array(enriched);
    }
    serde_json::to_string(&root).unwrap_or_else(|_| body_text.to_string())
}

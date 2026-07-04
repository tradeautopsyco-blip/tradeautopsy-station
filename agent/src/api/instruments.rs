use crate::api::AppState;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::collections::HashMap;

pub async fn search_instruments(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").cloned().unwrap_or_default();
    if q.len() < 2 {
        return Json(json!({ "symbols": [] }));
    }
    match state.instruments.search(&q, 10) {
        Ok(results) => Json(json!({ "symbols": results })),
        Err(e) => {
            tracing::error!("instrument search error: {}", e);
            Json(json!({ "symbols": [] }))
        }
    }
}

pub async fn get_ltp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let symbol = params.get("symbol").cloned().unwrap_or_default();
    let exchange = params
        .get("exchange")
        .cloned()
        .unwrap_or_else(|| "NSE".to_string());
    let segment = params
        .get("segment")
        .cloned()
        .unwrap_or_else(|| "NSE".to_string());

    if symbol.is_empty() {
        return Json(json!({ "ltp": null, "source": "none" }));
    }

    if let Ok(Some(price)) = state.instruments.get_last_price(&symbol) {
        if price > 0.0 {
            return Json(json!({ "ltp": price, "source": "cache" }));
        }
    }

    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            headers
                .get("x-daemon-user-id")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("");

    if user_id.is_empty() {
        return Json(json!({ "ltp": null, "source": "none" }));
    }

    let url = format!("{}/api/bar/v1/broker/ltp", state.upstream.config.base_url);
    let body = json!({
        "trading_symbol": symbol,
        "exchange": exchange,
        "segment": segment,
    });

    match state
        .upstream
        .http
        .post(&url)
        .header(
            reqwest::header::HeaderName::from_static("x-daemon-secret"),
            state.upstream.config.daemon_secret.as_str(),
        )
        .header("x-user-id", user_id)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(ltp) = data["ltp"].as_f64() {
                    if ltp > 0.0 {
                        let _ = state.instruments.upsert_last_price(&symbol, ltp);
                        return Json(json!({ "ltp": ltp, "source": "broker" }));
                    }
                }
                if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                    return Json(json!({
                        "ltp": null,
                        "source": "none",
                        "error": err,
                    }));
                }
            }
            Json(json!({ "ltp": null, "source": "none" }))
        }
        Err(e) => {
            tracing::debug!("broker ltp upstream failed: {e}");
            Json(json!({ "ltp": null, "source": "none" }))
        }
    }
}

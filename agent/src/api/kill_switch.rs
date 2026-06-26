//! Station Slice B — kill switch fire + dismiss (L3 DNS in agent).

use crate::api::AppState;
use crate::dns_block;
use crate::AgentEvent;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tracing::{info, warn};

pub const L3_COUNTDOWN_SECS: u32 = 90;

fn broker_from_body(body: &Value) -> String {
    body.get("broker")
        .and_then(|v| v.as_str())
        .unwrap_or("zerodha")
        .to_string()
}

fn level_from_body(body: &Value) -> u64 {
    body.get("level").and_then(|v| v.as_u64()).unwrap_or(0)
}

/// Shared enforcement path for HTTP handler + brain `fog_of_war` commands (#190).
pub async fn apply_kill_switch_level(
    state: &AppState,
    level: u64,
    broker: &str,
    trigger: Option<&str>,
) -> Result<(), String> {
    info!(
        "Kill switch apply level={level} broker={broker} trigger={}",
        trigger.unwrap_or("—")
    );

    if level <= 1 {
        state.fog_active.store(true, Ordering::SeqCst);
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L1".to_string()),
            countdown_secs: None,
            requires_ack: false,
        });
        return Ok(());
    }

    if level == 2 {
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L2".to_string()),
            countdown_secs: Some(L3_COUNTDOWN_SECS),
            requires_ack: false,
        });
        return Ok(());
    }

    if level >= 3 {
        let broker = broker.to_string();
        let block_result = tokio::task::spawn_blocking(move || dns_block::apply_hosts_block(&broker))
            .await
            .map_err(|e| format!("hosts block task: {e}"))?;
        block_result?;
        state.fog_active.store(true, Ordering::SeqCst);
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L3".to_string()),
            countdown_secs: Some(L3_COUNTDOWN_SECS),
            requires_ack: true,
        });
    }

    Ok(())
}

pub async fn dismiss_kill_switch_state(state: &AppState) -> Result<(), String> {
    let dns = tokio::task::spawn_blocking(dns_block::disable_block).await;
    match dns {
        Ok(Ok(())) => info!("KillSwitch DNS block removed"),
        Ok(Err(e)) => warn!("KillSwitch DNS unblock failed: {e}"),
        Err(e) => warn!("KillSwitch DNS unblock join error: {e}"),
    }
    state.fog_active.store(false, Ordering::SeqCst);
    state.event_bus.publish(AgentEvent::KillSwitchState {
        active: false,
        level: None,
        countdown_secs: None,
        requires_ack: false,
    });
    Ok(())
}

pub async fn kill_switch_handler(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let level = level_from_body(&body);
    let broker = broker_from_body(&body);
    let trigger = body.get("reason").and_then(|v| v.as_str());

    match apply_kill_switch_level(&state, level, &broker, trigger).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "received": true,
                "level": level,
                "dns_active": dns_block::is_block_active(),
            })),
        )
            .into_response(),
        Err(e) => {
            warn!("KillSwitch apply failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "received": false,
                    "level": level,
                    "error": e,
                    "dns_active": dns_block::is_block_active(),
                })),
            )
                .into_response()
        }
    }
}

pub async fn dismiss_kill_switch_handler(State(state): State<AppState>) -> Response {
    info!("Kill switch dismiss requested");
    let _ = dismiss_kill_switch_state(&state).await;
    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "dns_active": dns_block::is_block_active(),
        })),
    )
        .into_response()
}

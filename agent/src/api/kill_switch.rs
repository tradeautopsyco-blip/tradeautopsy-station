//! Station Slice B — kill switch fire + dismiss (L3 DNS in agent).

use crate::api::AppState;
use crate::dns_block;
use crate::kill_switch_audit::KillSwitchAuditAppend;
use crate::resolve_kill_switch_broker::resolve_kill_switch_broker;
use crate::AgentEvent;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tracing::{info, warn};

pub const L3_COUNTDOWN_SECS: u32 = 90;

#[derive(Debug, Deserialize)]
pub struct AuditTailQuery {
    pub tail: Option<u32>,
}

fn broker_from_body(body: &Value) -> Result<String, String> {
    let raw = body.get("broker").and_then(|v| v.as_str());
    resolve_kill_switch_broker(raw, std::env::var("AGENT_PROTECTIVE_BROKER_SLUG").ok().as_deref())
}

fn level_from_body(body: &Value) -> u64 {
    body.get("level").and_then(|v| v.as_u64()).unwrap_or(0)
}

fn hosts_for_audit(broker: &str) -> Vec<String> {
    dns_block::hosts_for_broker(broker)
        .iter()
        .map(|h| (*h).to_string())
        .collect()
}

fn append_audit_fire(
    state: &AppState,
    level: i32,
    broker: &str,
    trigger: Option<&str>,
) {
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: "fire".to_string(),
            level,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts: hosts_for_audit(broker),
        },
    ) {
        Ok(record) => info!(
            audit_id = record.id,
            event_type = %record.event_type,
            broker = %record.broker,
            "kill switch audit row appended"
        ),
        Err(e) => warn!("kill switch audit append (fire) failed: {e}"),
    }
}

fn append_audit_dismiss(state: &AppState, broker: &str, trigger: Option<&str>) {
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: "dismiss".to_string(),
            level: 3,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts: hosts_for_audit(broker),
        },
    ) {
        Ok(record) => info!(
            audit_id = record.id,
            event_type = %record.event_type,
            "kill switch dismiss audit row appended"
        ),
        Err(e) => warn!("kill switch audit append (dismiss) failed: {e}"),
    }
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
        if let Ok(mut last) = state.last_l3_broker.lock() {
            *last = Some(broker.clone());
        }
        let broker_for_block = broker.clone();
        let block_result = tokio::task::spawn_blocking(move || {
            dns_block::enable_block_with_watcher(&broker_for_block)
        })
            .await
            .map_err(|e| format!("hosts block task: {e}"))?;
        block_result?;
        append_audit_fire(state, level as i32, &broker, trigger);
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
    let broker = state
        .last_l3_broker
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let dns = tokio::task::spawn_blocking(dns_block::disable_block_and_watcher).await;
    match dns {
        Ok(Ok(())) => info!("KillSwitch DNS block removed"),
        Ok(Err(e)) => warn!("KillSwitch DNS unblock failed: {e}"),
        Err(e) => warn!("KillSwitch DNS unblock join error: {e}"),
    }
    append_audit_dismiss(state, &broker, None);
    if let Ok(mut last) = state.last_l3_broker.lock() {
        *last = None;
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
    let broker = match broker_from_body(&body) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "received": false,
                    "level": level,
                    "error": e,
                    "dns_active": dns_block::is_block_active(),
                })),
            )
                .into_response();
        }
    };
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

pub async fn kill_switch_audit_handler(
    State(state): State<AppState>,
    Query(query): Query<AuditTailQuery>,
) -> Response {
    let tail = query.tail.unwrap_or(10).clamp(1, 100);
    match state.kill_switch_audit.tail(tail) {
        Ok(rows) => {
            let entries: Vec<Value> = rows
                .into_iter()
                .map(|row| {
                    let verified = state.audit_signer.verify_record(&row);
                    json!({
                        "id": row.id,
                        "event_type": row.event_type,
                        "fired_at_ms": row.fired_at_ms,
                        "level": row.level,
                        "broker": row.broker,
                        "trigger": row.trigger,
                        "hosts_json": row.hosts_json,
                        "signature_b64": row.signature_b64,
                        "public_key_id": row.public_key_id,
                        "verified": verified,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({
                    "ok": true,
                    "tail": tail,
                    "entries": entries,
                })),
            )
                .into_response()
        }
        Err(e) => {
            warn!("kill switch audit tail failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "ok": false,
                    "error": e.to_string(),
                })),
            )
                .into_response()
        }
    }
}

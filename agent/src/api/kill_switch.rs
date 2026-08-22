//! Station Slice B — kill switch fire + dismiss (L3 DNS in agent).

use crate::api::AppState;
use crate::dns_block;
use crate::kill_policy::KillPolicy;
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

#[derive(Debug, Deserialize)]
pub struct AuditTailQuery {
    pub tail: Option<u32>,
}

fn loaded_policy(state: &AppState) -> KillPolicy {
    state
        .kill_policy
        .load_or_insert_defaults()
        .unwrap_or_default()
}

fn broker_from_body(body: &Value) -> Result<String, String> {
    let raw = body.get("broker").and_then(|v| v.as_str());
    resolve_kill_switch_broker(
        raw,
        std::env::var("AGENT_PROTECTIVE_BROKER_SLUG")
            .ok()
            .as_deref(),
    )
}

fn level_from_body(body: &Value) -> u64 {
    body.get("level").and_then(|v| v.as_u64()).unwrap_or(0)
}

/// HTTP / apply: omitted or 0 uses policy `default_level` (clamped to 1|2|3).
pub fn effective_level(requested: u64, policy: &KillPolicy) -> u64 {
    if requested == 0 {
        match policy.default_level {
            1 | 2 | 3 => policy.default_level as u64,
            _ => 3,
        }
    } else {
        requested
    }
}

/// R8: empty host map + website_block must refuse. `true` = write DNS.
pub fn plan_l3_dns(website_block: bool, hosts: &[&str]) -> Result<bool, String> {
    if !website_block {
        return Ok(false);
    }
    if hosts.is_empty() {
        return Err("no Kill DNS hosts for broker — refusing empty L3 block (R8)".to_string());
    }
    Ok(true)
}

fn hosts_for_audit(broker: &str) -> Vec<String> {
    dns_block::hosts_for_broker(broker)
        .iter()
        .map(|h| (*h).to_string())
        .collect()
}

fn remember_apply(state: &AppState, level: u64, broker: &str) {
    if let Ok(mut last) = state.last_l3_broker.lock() {
        *last = Some(broker.to_string());
    }
    if let Ok(mut last) = state.last_applied_level.lock() {
        *last = Some(level.min(u8::MAX as u64) as u8);
    }
}

fn append_audit_fire(
    state: &AppState,
    level: i32,
    broker: &str,
    trigger: Option<&str>,
    hosts: Vec<String>,
) {
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: "fire".to_string(),
            level,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts,
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
    let level = state
        .last_applied_level
        .lock()
        .ok()
        .and_then(|g| *g)
        .unwrap_or(0) as i32;
    let policy = loaded_policy(state);
    let hosts = if level >= 3 && policy.website_block {
        hosts_for_audit(broker)
    } else {
        Vec::new()
    };
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: "dismiss".to_string(),
            level,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts,
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
    let policy = loaded_policy(state);
    let level = effective_level(level, &policy);
    let countdown = policy.countdown_secs;
    info!(
        "Kill switch apply level={level} broker={broker} trigger={}",
        trigger.unwrap_or("—")
    );

    if level <= 1 {
        remember_apply(state, level, broker);
        // Q8: L1 is desk state only — do not set fog_active.
        append_audit_fire(state, level as i32, broker, trigger, Vec::new());
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L1".to_string()),
            countdown_secs: None,
            requires_ack: false,
        });
        return Ok(());
    }

    if level == 2 {
        remember_apply(state, level, broker);
        append_audit_fire(state, level as i32, broker, trigger, Vec::new());
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L2".to_string()),
            countdown_secs: Some(countdown),
            requires_ack: false,
        });
        return Ok(());
    }

    if level >= 3 {
        let hosts = dns_block::hosts_for_broker(broker);
        let write_dns = plan_l3_dns(policy.website_block, hosts)?;
        remember_apply(state, level, broker);
        if write_dns {
            let broker_for_block = broker.to_string();
            let block_result = tokio::task::spawn_blocking(move || {
                dns_block::enable_block_with_watcher(&broker_for_block)
            })
            .await
            .map_err(|e| format!("hosts block task: {e}"))?;
            block_result?;
        }
        let audit_hosts = if write_dns {
            hosts_for_audit(broker)
        } else {
            Vec::new()
        };
        append_audit_fire(state, level as i32, broker, trigger, audit_hosts);
        // Leftover L3 readers still look at fog_active.
        state.fog_active.store(true, Ordering::SeqCst);
        state.event_bus.publish(AgentEvent::KillSwitchState {
            active: true,
            level: Some("L3".to_string()),
            countdown_secs: Some(countdown),
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
    if let Ok(mut last) = state.last_applied_level.lock() {
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
    let policy = loaded_policy(&state);
    let level = effective_level(level_from_body(&body), &policy);
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
                    "fog_active": state.fog_active.load(Ordering::SeqCst),
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
                "fog_active": state.fog_active.load(Ordering::SeqCst),
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
                    "fog_active": state.fog_active.load(Ordering::SeqCst),
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
            "fog_active": state.fog_active.load(Ordering::SeqCst),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kill_policy::L3_COUNTDOWN_SECS;

    #[test]
    fn default_policy_is_l3_90s_block_on() {
        let p = KillPolicy::default();
        assert_eq!(p.default_level, 3);
        assert_eq!(p.countdown_secs, L3_COUNTDOWN_SECS);
        assert!(p.website_block);
    }

    #[test]
    fn omitted_or_zero_level_uses_policy_default() {
        let p = KillPolicy {
            default_level: 1,
            countdown_secs: 90,
            website_block: true,
        };
        assert_eq!(effective_level(0, &p), 1);
        assert_eq!(effective_level(2, &p), 2);
        assert_eq!(effective_level(3, &p), 3);
    }

    #[test]
    fn r8_empty_host_map_refuses_when_website_block_on() {
        assert!(plan_l3_dns(true, &[]).is_err());
        assert_eq!(plan_l3_dns(false, &[]).unwrap(), false);
        assert_eq!(plan_l3_dns(true, &["kite.zerodha.com"]).unwrap(), true);
    }
}

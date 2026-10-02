//! Station Slice B — kill switch fire + dismiss (L3 DNS in agent).

use crate::api::AppState;
use crate::dns_block;
use crate::kill_latch::{KillLatchLoad, KillLatchSnapshot};
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

fn unix_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn expires_at_ms(countdown_secs: Option<u32>) -> Option<i64> {
    countdown_secs.map(|c| unix_now_ms().saturating_add(i64::from(c).saturating_mul(1000)))
}

fn kill_state(
    active: bool,
    level: Option<&str>,
    countdown_secs: Option<u32>,
    requires_ack: bool,
) -> AgentEvent {
    AgentEvent::KillSwitchState {
        active,
        level: level.map(|s| s.to_string()),
        countdown_secs,
        expires_at_ms: expires_at_ms(countdown_secs),
        requires_ack,
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillApplyDecision {
    Apply { level: u64 },
    Ignore { armed: u64, requested: u64 },
}

/// Escalate-only (PLAN Kill Q4). Same-level re-fire applies. Softer fire is ignored.
pub fn resolve_kill_apply(armed: Option<u64>, requested: u64) -> KillApplyDecision {
    match armed {
        None => KillApplyDecision::Apply { level: requested },
        Some(current) if requested >= current => KillApplyDecision::Apply { level: requested },
        Some(current) => KillApplyDecision::Ignore {
            armed: current,
            requested,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillClearDecision {
    Dismiss,
    Ignore { armed: u64 },
}

/// `clear_fog` must not unlock L3. HTTP I’m Calm still dismisses.
pub fn resolve_clear_fog(armed: Option<u64>) -> KillClearDecision {
    match armed {
        Some(level) if level >= 3 => KillClearDecision::Ignore { armed: level },
        _ => KillClearDecision::Dismiss,
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

fn last_applied_level(state: &AppState) -> Option<u64> {
    last_applied_level_from_latch(state).or_else(|| {
        state
            .last_applied_level
            .lock()
            .ok()
            .and_then(|g| *g)
            .map(|v| v as u64)
    })
}

/// RAM mirrors for legacy readers — single source of truth is the latch sqlite row.
pub fn sync_derived_kill_state(state: &AppState) {
    let latch = effective_latch_load(state);
    match latch {
        EffectiveLatch::Inactive => {
            if let Ok(mut last) = state.last_l3_broker.lock() {
                *last = None;
            }
            if let Ok(mut last) = state.last_applied_level.lock() {
                *last = None;
            }
            state.fog_active.store(false, Ordering::SeqCst);
        }
        EffectiveLatch::Active(snap) => {
            if let Ok(mut last) = state.last_applied_level.lock() {
                *last = Some(snap.level.min(255) as u8);
            }
            if snap.level >= 3 {
                if let Ok(mut last) = state.last_l3_broker.lock() {
                    *last = Some(snap.broker.clone());
                }
                state.fog_active.store(true, Ordering::SeqCst);
            } else {
                if let Ok(mut last) = state.last_l3_broker.lock() {
                    *last = None;
                }
                state.fog_active.store(false, Ordering::SeqCst);
            }
        }
    }
}

enum EffectiveLatch {
    Inactive,
    Active(KillLatchSnapshot),
}

fn effective_latch_load(state: &AppState) -> EffectiveLatch {
    if state.kill_latch_fail_closed.load(Ordering::SeqCst) {
        return EffectiveLatch::Active(KillLatchSnapshot {
            active: true,
            level: 3,
            broker: state
                .last_l3_broker
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            armed_at_ms: 0,
            expires_at_ms: None,
            requires_ack: true,
        });
    }
    match state.kill_latch.load() {
        Ok(KillLatchLoad::Inactive) => EffectiveLatch::Inactive,
        Ok(KillLatchLoad::Active(s)) => EffectiveLatch::Active(s),
        Err(e) => {
            warn!("kill latch read failed (fail closed): {e}");
            state.kill_latch_fail_closed.store(true, Ordering::SeqCst);
            EffectiveLatch::Active(KillLatchSnapshot {
                active: true,
                level: 3,
                broker: state
                    .last_l3_broker
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                armed_at_ms: 0,
                expires_at_ms: None,
                requires_ack: true,
            })
        }
    }
}

pub fn latch_public_snapshot(state: &AppState) -> (bool, Option<KillLatchSnapshot>) {
    match effective_latch_load(state) {
        EffectiveLatch::Inactive => (false, None),
        EffectiveLatch::Active(s) => (true, Some(s)),
    }
}

fn write_latch_for_apply(
    state: &AppState,
    level: u64,
    broker: &str,
    policy: &KillPolicy,
) -> Result<(), String> {
    let now = unix_now_ms();
    let expires = if level >= 2 {
        Some(now.saturating_add(i64::from(policy.countdown_secs).saturating_mul(1000)))
    } else {
        None
    };
    let snap = KillLatchSnapshot {
        active: true,
        level: level.min(i32::MAX as u64) as i32,
        broker: broker.to_string(),
        armed_at_ms: now,
        expires_at_ms: expires,
        requires_ack: level >= 3,
    };
    state
        .kill_latch
        .arm(&snap)
        .map_err(|e| format!("kill latch arm: {e}"))?;
    sync_derived_kill_state(state);
    Ok(())
}

fn clear_latch_after_hosts_clean(state: &AppState) -> Result<(), String> {
    state
        .kill_latch
        .clear()
        .map_err(|e| format!("kill latch clear: {e}"))?;
    sync_derived_kill_state(state);
    Ok(())
}

fn last_applied_level_from_latch(state: &AppState) -> Option<u64> {
    match effective_latch_load(state) {
        EffectiveLatch::Inactive => None,
        EffectiveLatch::Active(s) => Some(s.level.max(0) as u64),
    }
}

fn append_audit_event(
    state: &AppState,
    event_type: &str,
    level: i32,
    broker: &str,
    trigger: Option<&str>,
    hosts: Vec<String>,
) {
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: event_type.to_string(),
            level,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts,
        },
    ) {
        Ok(record) => info!(
            audit_id = record.id,
            event_type = %record.event_type,
            "kill switch audit row appended"
        ),
        Err(e) => warn!("kill switch audit append ({event_type}) failed: {e}"),
    }
}

fn append_audit_ignored(state: &AppState, requested: i32, broker: &str, trigger: Option<&str>) {
    match state.kill_switch_audit.append_signed(
        state.audit_signer.as_ref(),
        KillSwitchAuditAppend {
            event_type: "ignored".to_string(),
            level: requested,
            broker: broker.to_string(),
            trigger: trigger.map(str::to_string),
            hosts: Vec::new(),
        },
    ) {
        Ok(record) => info!(
            audit_id = record.id,
            event_type = %record.event_type,
            "kill switch ignored audit row appended"
        ),
        Err(e) => warn!("kill switch audit append (ignored) failed: {e}"),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillApplyOutcome {
    Applied { level: u64 },
    Ignored { armed: u64, requested: u64 },
}

/// Shared enforcement path for HTTP handler + brain `fog_of_war` commands (#190).
pub async fn apply_kill_switch_level(
    state: &AppState,
    level: u64,
    broker: &str,
    trigger: Option<&str>,
) -> Result<KillApplyOutcome, String> {
    let policy = loaded_policy(state);
    let level = effective_level(level, &policy);
    match resolve_kill_apply(last_applied_level(state), level) {
        KillApplyDecision::Ignore { armed, requested } => {
            append_audit_ignored(state, requested as i32, broker, trigger);
            return Ok(KillApplyOutcome::Ignored { armed, requested });
        }
        KillApplyDecision::Apply { level } => {
            return apply_kill_switch_teeth(state, policy, level, broker, trigger).await;
        }
    }
}

async fn apply_kill_switch_teeth(
    state: &AppState,
    policy: KillPolicy,
    level: u64,
    broker: &str,
    trigger: Option<&str>,
) -> Result<KillApplyOutcome, String> {
    let countdown = policy.countdown_secs;
    info!(
        "Kill switch apply level={level} broker={broker} trigger={}",
        trigger.unwrap_or("—")
    );

    write_latch_for_apply(state, level, broker, &policy)?;

    if level <= 1 {
        // Q8: L1 is desk state only — do not set fog_active.
        append_audit_fire(state, level as i32, broker, trigger, Vec::new());
        state.event_bus.publish(kill_state(
            true,
            Some("L1"),
            None,
            false,
        ));
        return Ok(KillApplyOutcome::Applied { level });
    }

    if level == 2 {
        append_audit_fire(state, level as i32, broker, trigger, Vec::new());
        state.event_bus.publish(kill_state(
            true,
            Some("L2"),
            Some(countdown),
            false,
        ));
        return Ok(KillApplyOutcome::Applied { level });
    }

    if level >= 3 {
        let hosts = dns_block::hosts_for_broker(broker);
        let write_dns = plan_l3_dns(policy.website_block, hosts)?;
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
        sync_derived_kill_state(state);
        state.event_bus.publish(kill_state(
            true,
            Some("L3"),
            Some(countdown),
            true,
        ));
    }

    Ok(KillApplyOutcome::Applied { level })
}

/// Command-poll `clear_fog`. L3 stays locked; HTTP I’m Calm still dismisses.
pub async fn apply_clear_fog(state: &AppState) -> Result<(), String> {
    match resolve_clear_fog(last_applied_level(state)) {
        KillClearDecision::Ignore { armed } => {
            let broker = state
                .last_l3_broker
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| "unknown".to_string());
            append_audit_ignored(state, armed as i32, &broker, Some("clear_fog"));
            Ok(())
        }
        KillClearDecision::Dismiss => dismiss_kill_switch_state(state).await,
    }
}

pub async fn dismiss_kill_switch_state(state: &AppState) -> Result<(), String> {
    let broker = match effective_latch_load(state) {
        EffectiveLatch::Active(s) => s.broker,
        EffectiveLatch::Inactive => state
            .last_l3_broker
            .lock()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let dns = tokio::task::spawn_blocking(dns_block::disable_block_and_watcher).await;
    let hosts_clean = match dns {
        Ok(Ok(())) => {
            info!("KillSwitch DNS block removed");
            true
        }
        Ok(Err(e)) => {
            warn!("KillSwitch DNS unblock failed: {e}");
            false
        }
        Err(e) => {
            warn!("KillSwitch DNS unblock join error: {e}");
            false
        }
    };
    if !hosts_clean && dns_block::is_block_active() {
        return Err("hosts block still active — latch not cleared".to_string());
    }
    append_audit_dismiss(state, &broker, None);
    clear_latch_after_hosts_clean(state)?;
    state.event_bus.publish(kill_state(false, None, None, false));
    Ok(())
}

/// Compare durable latch intent to `/etc/hosts` (or test override) before serving HTTP.
pub async fn reconcile_kill_state(state: &AppState) -> Result<(), String> {
    let policy = loaded_policy(state);
    let dns_active = dns_block::is_block_active();
    let latch = effective_latch_load(state);

    match latch {
        EffectiveLatch::Active(snap) => {
            if snap
                .expires_at_ms
                .is_some_and(|exp| unix_now_ms() > exp)
            {
                info!("kill latch expired — auto dismiss");
                return dismiss_kill_switch_state(state).await;
            }
            if snap.level >= 3 && policy.website_block {
                let hosts = dns_block::hosts_for_broker(&snap.broker);
                if hosts.is_empty() {
                    return Err(format!(
                        "latched broker {} has no Kill DNS hosts (R8)",
                        snap.broker
                    ));
                }
                if dns_active {
                    let broker = snap.broker.clone();
                    tokio::task::spawn_blocking(move || dns_block::reconcile_armed_blocked(&broker))
                        .await
                        .map_err(|e| format!("reconcile armed+blocked join: {e}"))??;
                } else {
                    let broker = snap.broker.clone();
                    let block_result = tokio::task::spawn_blocking(move || {
                        dns_block::enable_block_with_watcher(&broker)
                    })
                    .await
                    .map_err(|e| format!("reconcile reapply join: {e}"))?;
                    block_result?;
                    append_audit_event(
                        state,
                        "reapply",
                        snap.level,
                        &snap.broker,
                        Some("boot_reconcile"),
                        hosts_for_audit(&snap.broker),
                    );
                }
            }
            sync_derived_kill_state(state);
            Ok(())
        }
        EffectiveLatch::Inactive => {
            if dns_active {
                let dns = tokio::task::spawn_blocking(dns_block::disable_block_and_watcher)
                    .await
                    .map_err(|e| format!("stale hosts cleanup join: {e}"))?;
                dns?;
                append_audit_event(
                    state,
                    "cleanup",
                    0,
                    "unknown",
                    Some("boot_reconcile"),
                    Vec::new(),
                );
            }
            sync_derived_kill_state(state);
            Ok(())
        }
    }
}

pub fn spawn_kill_latch_expiry_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.tick().await;
        loop {
            interval.tick().await;
            let expired = match effective_latch_load(&state) {
                EffectiveLatch::Active(s) => s
                    .expires_at_ms
                    .is_some_and(|exp| unix_now_ms() > exp),
                EffectiveLatch::Inactive => false,
            };
            if expired {
                let _ = dismiss_kill_switch_state(&state).await;
            }
        }
    });
}

pub async fn kill_switch_state_handler(State(state): State<AppState>) -> Response {
    let dns_active = dns_block::is_block_active();
    let (active, snap) = latch_public_snapshot(&state);
    let body = if let Some(s) = snap {
        json!({
            "active": active,
            "level": s.level,
            "broker": s.broker,
            "armed_at_ms": s.armed_at_ms,
            "expires_at_ms": s.expires_at_ms,
            "requires_ack": s.requires_ack,
            "dns_active": dns_active,
        })
    } else {
        json!({
            "active": false,
            "level": null,
            "broker": null,
            "armed_at_ms": null,
            "expires_at_ms": null,
            "requires_ack": false,
            "dns_active": dns_active,
        })
    };
    (StatusCode::OK, Json(body)).into_response()
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
        Ok(KillApplyOutcome::Ignored { armed, requested }) => (
            StatusCode::OK,
            Json(json!({
                "received": true,
                "ignored": true,
                "level": requested,
                "armed_level": armed,
                "dns_active": dns_block::is_block_active(),
                "fog_active": state.fog_active.load(Ordering::SeqCst),
            })),
        )
            .into_response(),
        Ok(KillApplyOutcome::Applied { level }) => {
            let countdown = if level >= 2 {
                Some(policy.countdown_secs)
            } else {
                None
            };
            (
            StatusCode::OK,
            Json(json!({
                "received": true,
                "ignored": false,
                "level": level,
                "countdown_secs": countdown,
                "expires_at_ms": expires_at_ms(countdown),
                "dns_active": dns_block::is_block_active(),
                "fog_active": state.fog_active.load(Ordering::SeqCst),
            })),
        )
            .into_response()
        }
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
    let store = state.kill_switch_audit.clone();
    let signer = state.audit_signer.clone();
    let result = tokio::task::spawn_blocking(move || {
        store.tail(tail).map(|rows| {
            rows.into_iter()
                .map(|row| {
                    let verified = signer.verify_record_cached(&row);
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
                .collect::<Vec<Value>>()
        })
    })
    .await;
    match result {
        Ok(Ok(entries)) => (
            StatusCode::OK,
            Json(json!({
                "ok": true,
                "tail": tail,
                "entries": entries,
            })),
        )
            .into_response(),
        Ok(Err(e)) => {
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
        Err(e) => {
            warn!("kill switch audit task join failed: {e}");
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

    #[test]
    fn escalate_only_unarmed_applies_requested() {
        assert_eq!(
            resolve_kill_apply(None, 1),
            KillApplyDecision::Apply { level: 1 }
        );
    }

    #[test]
    fn escalate_only_l1_to_l3_applies() {
        assert_eq!(
            resolve_kill_apply(Some(1), 3),
            KillApplyDecision::Apply { level: 3 }
        );
    }

    #[test]
    fn escalate_only_l3_then_l1_is_ignored() {
        assert_eq!(
            resolve_kill_apply(Some(3), 1),
            KillApplyDecision::Ignore {
                armed: 3,
                requested: 1
            }
        );
    }

    #[test]
    fn escalate_only_l3_then_l2_is_ignored() {
        assert_eq!(
            resolve_kill_apply(Some(3), 2),
            KillApplyDecision::Ignore {
                armed: 3,
                requested: 2
            }
        );
    }

    #[test]
    fn escalate_only_l3_re_fire_applies() {
        assert_eq!(
            resolve_kill_apply(Some(3), 3),
            KillApplyDecision::Apply { level: 3 }
        );
    }

    #[test]
    fn clear_fog_ignored_while_l3_armed() {
        assert_eq!(
            resolve_clear_fog(Some(3)),
            KillClearDecision::Ignore { armed: 3 }
        );
    }

    #[test]
    fn clear_fog_allowed_when_unarmed_or_below_l3() {
        assert_eq!(resolve_clear_fog(None), KillClearDecision::Dismiss);
        assert_eq!(resolve_clear_fog(Some(1)), KillClearDecision::Dismiss);
        assert_eq!(resolve_clear_fog(Some(2)), KillClearDecision::Dismiss);
    }
}

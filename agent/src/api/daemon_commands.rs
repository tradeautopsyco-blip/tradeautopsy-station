//! Brain `daemon_commands` poll → L1 fog / L2 overlay / L3 DNS (#190).

use super::capture::forward_daemon_json_with_optional_429_retry;
use super::kill_switch::{apply_clear_fog, apply_kill_switch_level};
use super::AppState;
use crate::resolve_kill_switch_broker::resolve_kill_switch_broker;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonCommandKind {
    FogOfWar,
    ClearFog,
}

/// Maps hosted `command_type` strings to executable kill-switch commands.
pub fn parse_daemon_command_type(command_type: &str) -> Option<DaemonCommandKind> {
    match command_type {
        "fog_of_war" => Some(DaemonCommandKind::FogOfWar),
        "clear_fog" => Some(DaemonCommandKind::ClearFog),
        _ => None,
    }
}

pub fn level_from_payload(payload: &Value) -> u64 {
    payload.get("level").and_then(|v| v.as_u64()).unwrap_or(1)
}

pub fn broker_from_payload(payload: &Value) -> Result<String, String> {
    let raw = payload.get("broker").and_then(|v| v.as_str());
    resolve_kill_switch_broker(
        raw,
        std::env::var("AGENT_PROTECTIVE_BROKER_SLUG")
            .ok()
            .as_deref(),
    )
}

pub fn trigger_from_payload(payload: &Value) -> Option<String> {
    payload
        .get("trigger")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub async fn execute_daemon_command(
    state: &AppState,
    user_id: &str,
    command_type: &str,
    payload: &Value,
) -> Result<(), String> {
    let kind = parse_daemon_command_type(command_type)
        .ok_or_else(|| format!("unsupported command type: {command_type}"))?;
    match kind {
        DaemonCommandKind::FogOfWar => {
            let level = level_from_payload(payload);
            let broker = broker_from_payload(payload)?;
            let trigger = trigger_from_payload(payload);
            apply_kill_switch_level(state, level, &broker, trigger.as_deref()).await?;
            if level >= 3 {
                ingest_kill_switch_triggered(state, user_id, level, trigger.as_deref()).await;
            }
            Ok(())
        }
        DaemonCommandKind::ClearFog => {
            apply_clear_fog(state).await?;
            Ok(())
        }
    }
}

async fn ingest_kill_switch_triggered(
    state: &AppState,
    user_id: &str,
    level: u64,
    trigger: Option<&str>,
) {
    let body = json!({
        "events": [{
            "signal_type": "kill_switch_triggered",
            "value": {
                "level": level,
                "trigger": trigger.unwrap_or("daemon_command"),
            },
        }],
    });
    match forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::POST,
        "/api/daemon/events",
        None,
        Some(&body),
        false,
    )
    .await
    {
        Ok((st, _)) if st.is_success() => {
            info!("kill_switch_triggered ingested for level {level}");
        }
        Ok((st, text)) => {
            warn!("kill_switch_triggered ingest failed status={st} body={text}");
        }
        Err(e) => warn!("kill_switch_triggered ingest error: {e}"),
    }
}

pub fn spawn_daemon_command_poll_loop(
    state: AppState,
    user_id: String,
    poll_interval_ms: u64,
    fog_active: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        let interval = Duration::from_millis(poll_interval_ms.max(200));
        let mut tick = tokio::time::interval(interval);
        tick.tick().await;
        loop {
            tick.tick().await;
            match poll_and_execute(&state, &user_id, &fog_active).await {
                Ok(n) if n > 0 => info!("daemon_commands executed count={n}"),
                Ok(_) => {}
                Err(e) => warn!("daemon_commands poll error: {e}"),
            }
        }
    });
}

async fn poll_and_execute(
    state: &AppState,
    user_id: &str,
    fog_active: &Arc<AtomicBool>,
) -> Result<usize, String> {
    let (status, text) = forward_daemon_json_with_optional_429_retry(
        &state.upstream,
        reqwest::Method::GET,
        "/api/daemon/command",
        None,
        None,
        false,
    )
    .await?;
    if !status.is_success() {
        return Err(format!("command poll status {status}: {text}"));
    }
    let parsed: Value = serde_json::from_str(&text).map_err(|e| format!("json: {e}"))?;
    let commands = parsed
        .get("commands")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut executed = 0usize;
    for cmd in commands {
        let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = cmd.get("payload").cloned().unwrap_or(json!({}));
        if parse_daemon_command_type(cmd_type).is_none() {
            continue;
        }
        if execute_daemon_command(state, user_id, cmd_type, &payload)
            .await
            .is_ok()
        {
            executed += 1;
            if cmd_type == "clear_fog" {
                fog_active.store(false, Ordering::SeqCst);
            }
        }
    }
    Ok(executed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fog_and_clear_commands() {
        assert_eq!(
            parse_daemon_command_type("fog_of_war"),
            Some(DaemonCommandKind::FogOfWar)
        );
        assert_eq!(
            parse_daemon_command_type("clear_fog"),
            Some(DaemonCommandKind::ClearFog)
        );
        assert_eq!(parse_daemon_command_type("show_overlay"), None);
    }

    #[test]
    fn level_defaults_to_one() {
        assert_eq!(level_from_payload(&json!({})), 1);
        assert_eq!(level_from_payload(&json!({ "level": 3 })), 3);
    }
}

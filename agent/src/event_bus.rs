use serde_json::json;
use tokio::sync::broadcast;

/// Internal SSE source events — serialized in `api/sse.rs` envelopes.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    AgentHealth {
        uptime_secs: u64,
    },
    ToolbarShow {
        trigger: String,
        detection_id: String,
        trade_id: String,
        symbol: String,
        side: String,
        qty: f64,
        price: f64,
        filled_at: String,
        broker: String,
    },
    BrokerSyncState {
        payload: serde_json::Value,
    },
    /// Per-venue egress posture. One event carries every slot's own posture so a
    /// banned venue never blanks a live one.
    VenueEgressState {
        payload: serde_json::Value,
    },
    /// Mirrors hosted intelligence `kill_switch_state` flat payload inside the SSE envelope.
    KillSwitchState {
        active: bool,
        level: Option<String>,
        countdown_secs: Option<u32>,
        expires_at_ms: Option<i64>,
        requires_ack: bool,
    },
    AuthState {
        authenticated: bool,
    },
    /// Behavioral session label for the toolbar (maps to intelligence `session_state` / `state_changed`).
    SessionState {
        behavioral_label: String,
    },
}

impl AgentEvent {
    pub fn sse_type_and_payload_json(&self) -> (&'static str, serde_json::Value) {
        match self {
            AgentEvent::AgentHealth { .. } => unreachable!("SSE maps AgentHealth specially"),
            AgentEvent::ToolbarShow {
                trigger,
                detection_id,
                trade_id,
                symbol,
                side,
                qty,
                price,
                filled_at,
                broker,
            } => (
                "toolbar_show",
                json!({
                    "trigger": trigger,
                    "detection_id": detection_id,
                    "trade_id": trade_id,
                    "symbol": symbol,
                    "side": side,
                    "qty": qty,
                    "price": price,
                    "filled_at": filled_at,
                    "broker": broker,
                }),
            ),
            AgentEvent::BrokerSyncState { payload } => ("broker_sync_state", payload.clone()),
            AgentEvent::VenueEgressState { payload } => ("venue_egress_state", payload.clone()),
            AgentEvent::KillSwitchState {
                active,
                level,
                countdown_secs,
                expires_at_ms,
                requires_ack,
            } => (
                "kill_switch_state",
                json!({
                    "active": active,
                    "level": level.as_ref().map(|s| json!(s)).unwrap_or(serde_json::Value::Null),
                    "countdown_secs": countdown_secs
                        .map(|n| json!(n))
                        .unwrap_or(serde_json::Value::Null),
                    "expires_at_ms": expires_at_ms
                        .map(|n| json!(n))
                        .unwrap_or(serde_json::Value::Null),
                    "requires_ack": *requires_ack,
                }),
            ),
            AgentEvent::AuthState { authenticated } => {
                ("auth_state", json!({ "authenticated": authenticated }))
            }
            AgentEvent::SessionState { behavioral_label } => {
                ("session_state", json!({ "state": behavioral_label }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentEvent;

    #[test]
    fn kill_switch_sse_payload_has_expected_keys() {
        let ev = AgentEvent::KillSwitchState {
            active: false,
            level: None,
            countdown_secs: None,
            expires_at_ms: None,
            requires_ack: false,
        };
        let (t, p) = ev.sse_type_and_payload_json();
        assert_eq!(t, "kill_switch_state");
        assert_eq!(p["active"], serde_json::json!(false));
        assert!(p["level"].is_null());
        assert!(p["countdown_secs"].is_null());
        assert!(p["expires_at_ms"].is_null());
        assert_eq!(p["requires_ack"], serde_json::json!(false));
    }

    #[test]
    fn kill_switch_sse_payload_includes_expires_at_ms() {
        let ev = AgentEvent::KillSwitchState {
            active: true,
            level: Some("L3".to_string()),
            countdown_secs: Some(90),
            expires_at_ms: Some(1_700_000_090_000),
            requires_ack: true,
        };
        let (_, p) = ev.sse_type_and_payload_json();
        assert_eq!(p["countdown_secs"], serde_json::json!(90));
        assert_eq!(p["expires_at_ms"], serde_json::json!(1_700_000_090_000i64));
    }
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AgentEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: AgentEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.tx.subscribe()
    }
}

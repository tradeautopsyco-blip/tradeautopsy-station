use crate::api::AppState;
use crate::event_bus::AgentEvent;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use chrono::SecondsFormat;
use serde_json::json;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

pub async fn handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_bus.subscribe();
    let runtime = state.runtime.clone();
    let signer = state.sse_signer.clone();
    let metrics = state.metrics.clone();
    let stream = BroadcastStream::new(rx).map(move |item| match item {
        Ok(event) => {
            let (event_type, payload) = match &event {
                AgentEvent::AgentHealth { uptime_secs } => (
                    "agent_health",
                    json!({
                        "uptime_secs": uptime_secs,
                        "boot_id": runtime.boot_id(),
                        "version": runtime.version(),
                        "build": runtime.build(),
                    }),
                ),
                AgentEvent::ToolbarShow { .. }
                | AgentEvent::BrokerSyncState { .. }
                | AgentEvent::VenueEgressState { .. }
                | AgentEvent::KillSwitchState { .. }
                | AgentEvent::AuthState { .. }
                | AgentEvent::SessionState { .. } => event.sse_type_and_payload_json(),
            };
            let event_id = ulid::Ulid::new().to_string();
            let seq = runtime.next_seq();
            let ts = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            let sig_b64 = signer.sign_event(&event_id, event_type, &payload);
            metrics.sse_published_for_type(event_type);
            let envelope = json!({
                "event_id": event_id,
                "type": event_type,
                "payload": payload,
                "ts": ts,
                "seq": seq,
                "sig": sig_b64,
            });
            let data = serde_json::to_string(&envelope).unwrap_or_default();
            Ok(Event::default().id(event_id).event(event_type).data(data))
        }
        Err(_) => Ok(Event::default().comment("lag")),
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

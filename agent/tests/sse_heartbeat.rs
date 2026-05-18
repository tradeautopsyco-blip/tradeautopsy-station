//! SSE delivers typed heartbeat events for bar connection state (issue #57 + wire v1 #58).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use futures::StreamExt;
use std::time::Duration;

fn extract_sse_data_json(chunk: &str) -> Option<serde_json::Value> {
    for line in chunk.lines() {
        if let Some(raw) = line.strip_prefix("data:") {
            let candidate = raw.trim();
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(candidate) {
                return Some(json);
            }
        }
    }
    None
}

#[tokio::test]
async fn sse_stream_includes_agent_health_event() {
    const PORT: u16 = 19_338;
    let path = "/api/daemon/events/stream";
    std::env::set_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS", "150");

    let handle = spawn_test_agent(PORT);

    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = format!("http://127.0.0.1:{PORT}{path}");
    let req = apply_wire_v1(
        client().get(&url),
        "GET",
        path,
        b"",
        WireHeaderOverrides::default(),
    );

    let resp = req
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("sse request should reach agent");

    assert_eq!(resp.status(), 200);
    let ct = resp.headers().get(reqwest::header::CONTENT_TYPE);
    assert!(
        ct.and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.starts_with("text/event-stream")),
        "content-type should be text/event-stream, got {ct:?}"
    );

    let mut stream = resp.bytes_stream();
    let mut saw_health = false;
    let mut saw_envelope = false;
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                let text = String::from_utf8_lossy(&chunk);
                if text.contains("agent_health") {
                    saw_health = true;
                }

                if let Some(event) = extract_sse_data_json(&text) {
                    let valid = event["type"] == "agent_health"
                        && event["event_id"].is_string()
                        && event["sig"].is_string()
                        && event["seq"].is_u64()
                        && event["payload"]["uptime_secs"].is_u64()
                        && event["payload"]["boot_id"].is_string();
                    if valid {
                        saw_envelope = true;
                        break;
                    }
                }
            }
        }
    }

    assert!(
        saw_health,
        "expected SSE body to mention agent_health within deadline"
    );
    assert!(
        saw_envelope,
        "expected structured agent_health SSE envelope"
    );

    handle.abort();
    std::env::remove_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS");
}

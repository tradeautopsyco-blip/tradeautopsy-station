//! SSE `id:` field supports Last-Event-ID replay contract (design §4.4 wire basis).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use futures::StreamExt;
use std::time::Duration;

fn looks_like_ulid(s: &str) -> bool {
    let t = s.trim();
    t.len() == 26 && t.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Returns true if buffer contains an SSE `id:` field with a ULID-shaped value.
fn buffer_has_event_id_line(buf: &str) -> bool {
    for line in buf.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("id:") {
            if looks_like_ulid(rest.trim()) {
                return true;
            }
        }
    }
    false
}

#[tokio::test]
async fn sse_includes_id_line_with_ulid_before_payload_events() {
    const PORT: u16 = 19_340;
    let path = "/api/daemon/events/stream";
    std::env::set_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS", "100");

    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

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
        .expect("sse");

    assert_eq!(resp.status(), 200);
    let mut stream = resp.bytes_stream();
    let mut acc = String::new();
    let deadline = tokio::time::sleep(Duration::from_secs(6));
    tokio::pin!(deadline);

    let mut ok = false;
    loop {
        tokio::select! {
            _ = &mut deadline => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                acc.push_str(&String::from_utf8_lossy(&chunk));
                if buffer_has_event_id_line(&acc) && acc.contains("agent_health") {
                    ok = true;
                    break;
                }
            }
        }
    }

    assert!(
        ok,
        "expected SSE with id: <ulid> and agent_health in body, got: {:?}",
        acc.chars().take(500).collect::<String>()
    );

    handle.abort();
    std::env::remove_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS");
}

//! Broadcast fan-out: multiple bar subscribers each see heartbeats (issue #57 + wire v1 #58).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use futures::StreamExt;
use std::time::Duration;

async fn stream_sees_agent_health(url: String, path: &'static str) -> bool {
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
        .expect("sse request");
    if resp.status() != 200 {
        return false;
    }
    let mut stream = resp.bytes_stream();
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            _ = &mut deadline => return false,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { return false };
                if String::from_utf8_lossy(&chunk).contains("agent_health") {
                    return true;
                }
            }
        }
    }
}

#[tokio::test]
async fn two_sse_clients_each_receive_agent_health() {
    const PORT: u16 = 19_339;
    let path = "/api/daemon/events/stream";
    std::env::set_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS", "120");

    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(350)).await;

    let base = format!("http://127.0.0.1:{PORT}{path}");
    let a = tokio::spawn(stream_sees_agent_health(base.clone(), path));
    let b = tokio::spawn(stream_sees_agent_health(base, path));

    let (ra, rb) = tokio::join!(a, b);
    assert!(ra.expect("task a"));
    assert!(rb.expect("task b"));

    handle.abort();
    std::env::remove_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS");
}

//! Phase 9 (#66) — SSE envelopes carry Ed25519 signatures verifiable with pinned agent pubkey (design §4.4).

mod common;

use common::{apply_wire_v1, client, spawn_test_agent, WireHeaderOverrides};
use futures::StreamExt;
use serde_json::Value;
use std::time::Duration;
use tradeautopsy_agent::{verify_sse_event_signature, SseSigningPubKey};

fn extract_sse_data_json(chunk: &str) -> Option<Value> {
    for line in chunk.lines() {
        if let Some(raw) = line.strip_prefix("data:") {
            let candidate = raw.trim();
            if let Ok(json) = serde_json::from_str::<Value>(candidate) {
                return Some(json);
            }
        }
    }
    None
}

#[tokio::test]
async fn health_exposes_sse_signing_pubkey_and_sse_events_verify() {
    const PORT: u16 = 19_448;
    let path_health = "/api/daemon/health";
    let path_sse = "/api/daemon/events/stream";
    std::env::set_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS", "120");

    let handle = spawn_test_agent(PORT);
    tokio::time::sleep(Duration::from_millis(280)).await;

    let health_url = format!("http://127.0.0.1:{PORT}{path_health}");
    let health_resp = apply_wire_v1(
        client().get(&health_url),
        "GET",
        path_health,
        b"",
        WireHeaderOverrides::default(),
    )
    .send()
    .await
    .expect("health");
    assert_eq!(health_resp.status(), 200);
    let health_json: Value = health_resp.json().await.expect("health json");
    let pk_b64 = health_json["sse_signing_pubkey_b64"]
        .as_str()
        .expect("sse_signing_pubkey_b64");
    let pubkey = SseSigningPubKey::from_standard_b64(pk_b64).expect("decode pubkey");

    let sse_url = format!("http://127.0.0.1:{PORT}{path_sse}");
    let sse_req = apply_wire_v1(
        client().get(&sse_url),
        "GET",
        path_sse,
        b"",
        WireHeaderOverrides::default(),
    );

    let resp = sse_req
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("sse");
    assert_eq!(resp.status(), 200);

    let mut stream = resp.bytes_stream();
    let deadline = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(deadline);
    let mut verified = false;

    loop {
        tokio::select! {
            _ = &mut deadline => break,
            next = stream.next() => {
                let Some(Ok(chunk)) = next else { break };
                let text = String::from_utf8_lossy(&chunk);
                if let Some(ev) = extract_sse_data_json(&text) {
                    if ev["type"] == "agent_health" && ev["sig"].is_string() {
                        let sig_b64 = ev["sig"].as_str().unwrap();
                        assert!(
                            verify_sse_event_signature(
                                &pubkey,
                                ev["event_id"].as_str().unwrap(),
                                "agent_health",
                                &ev["payload"],
                                sig_b64,
                            ),
                            "Ed25519 verify failed for envelope"
                        );
                        verified = true;
                        break;
                    }
                }
            }
        }
    }

    assert!(verified, "expected signed agent_health SSE within deadline");

    handle.abort();
    std::env::remove_var("TRADEAUTOPY_AGENT_HEARTBEAT_MS");
}

#[test]
fn verify_sse_event_signature_rejects_tampered_payload() {
    let seed = [7u8; 32];
    let signer = tradeautopsy_agent::SseSigner::from_seed(seed);
    let pubkey = signer.public_key_for_pinning();

    let event_id = "01HZYDTSJJTGPAKFYZVNPMEBNG";
    let payload = serde_json::json!({"uptime_secs": 1});
    let sig_b64 = signer.sign_event(event_id, "agent_health", &payload);

    let mut bad = payload.clone();
    bad["uptime_secs"] = serde_json::json!(2);

    assert!(
        !verify_sse_event_signature(&pubkey, event_id, "agent_health", &bad, &sig_b64),
        "tampered payload must not verify"
    );
}

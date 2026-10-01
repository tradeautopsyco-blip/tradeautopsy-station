//! Wave 3 — kill latch invariants (plans/kill-switch-lifetime-spec.md §7).

#[cfg(target_os = "macos")]
mod macos_tests {
    mod common;

    use common::{
        apply_wire_v1, client, spawn_test_agent_with_options, wait_ready, TestAgentOptions,
        WireHeaderOverrides, TEST_SECRET,
    };
    use serde_json::json;
    use std::time::Duration;
    use tradeautopsy_agent::{
        kill_latch::{KillLatchLoad, KillLatchSnapshot, KillLatchStore},
        BLOCK_MARKER, KillPolicy,
    };

    fn seed_hosts(label: &str) -> std::path::PathBuf {
        let hosts_path =
            std::env::temp_dir().join(format!("rta-latch-{label}-{}.hosts", uuid::Uuid::new_v4()));
        std::fs::write(&hosts_path, "127.0.0.1 localhost\n").expect("seed hosts");
        std::env::set_var(
            "TRADEAUTOPSY_HOSTS_FILE",
            hosts_path.to_string_lossy().to_string(),
        );
        hosts_path
    }

    fn cleanup_hosts(hosts_path: &std::path::Path) {
        std::env::remove_var("TRADEAUTOPSY_HOSTS_FILE");
        let _ = std::fs::remove_file(hosts_path);
    }

    async fn post_kill(port: u16, broker: &str, level: u64) {
        let path = "/api/daemon/kill-switch";
        let body = json!({ "broker": broker, "level": level });
        let payload = serde_json::to_vec(&body).expect("json");
        let url = format!("http://127.0.0.1:{port}{path}");
        apply_wire_v1(
            client()
                .post(&url)
                .header("content-type", "application/json"),
            "POST",
            path,
            &payload,
            WireHeaderOverrides::default(),
        )
        .body(payload)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("fire")
        .error_for_status()
        .expect("fire ok");
    }

    async fn get_state(port: u16) -> serde_json::Value {
        let path = "/api/daemon/kill-switch/state";
        let url = format!("http://127.0.0.1:{port}{path}");
        apply_wire_v1(
            client().get(&url),
            "GET",
            path,
            b"",
            WireHeaderOverrides::default(),
        )
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("state")
        .json()
        .await
        .expect("state json")
    }

    async fn dismiss(port: u16) {
        let path = "/api/daemon/dismiss-kill-switch";
        let url = format!("http://127.0.0.1:{port}{path}");
        apply_wire_v1(
            client()
                .post(&url)
                .header("content-type", "application/json"),
            "POST",
            path,
            b"{}",
            WireHeaderOverrides::default(),
        )
        .body("{}")
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .expect("dismiss")
        .error_for_status()
        .expect("dismiss ok");
    }

    fn pick_port() -> u16 {
        19200 + (uuid::Uuid::new_v4().as_u128() as u16 % 400)
    }

    #[tokio::test]
    async fn boot_reconciles_latch_to_hosts() {
        let port = pick_port();
        let hosts = seed_hosts("reconcile-four");
        let cfg_latch = tradeautopsy_agent::AgentConfig::test_on_port(port, TEST_SECRET)
            .kill_latch_db_path
            .clone();

        // armed + blocked: seed latch + hosts lines, boot re-arms watcher only.
        {
            let store = KillLatchStore::open(&cfg_latch).expect("latch open");
            store
                .arm(&KillLatchSnapshot {
                    active: true,
                    level: 3,
                    broker: "binance_us".to_string(),
                    armed_at_ms: 1,
                    expires_at_ms: None,
                    requires_ack: true,
                })
                .expect("arm");
            tradeautopsy_agent::dns_block::apply_hosts_block("binance_us").expect("hosts");
        }
        let mut opts = TestAgentOptions::default();
        opts.kill_policy = KillPolicy {
            default_level: 3,
            countdown_secs: 90,
            website_block: true,
        };
        let h = spawn_test_agent_with_options(port, opts);
        wait_ready(port).await;
        let state = get_state(port).await;
        assert_eq!(state["active"], true);
        assert_eq!(state["dns_active"], true);
        h.abort();
        let _ = h.await;
        tradeautopsy_agent::dns_block::disable_block().ok();
        KillLatchStore::open(&cfg_latch).expect("latch").clear().ok();
        std::fs::write(&hosts, "127.0.0.1 localhost\n").ok();

        // armed + clean: re-apply block on boot.
        {
            let store = KillLatchStore::open(&cfg_latch).expect("latch open");
            store
                .arm(&KillLatchSnapshot {
                    active: true,
                    level: 3,
                    broker: "binance_us".to_string(),
                    armed_at_ms: 1,
                    expires_at_ms: None,
                    requires_ack: true,
                })
                .expect("arm");
        }
        let h2 = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        let content = std::fs::read_to_string(&hosts).expect("read hosts");
        assert!(content.contains(BLOCK_MARKER));
        dismiss(port).await;
        h2.abort();
        let _ = h2.await;

        // clear + blocked: stale hosts cleaned.
        tradeautopsy_agent::dns_block::apply_hosts_block("binance_us").expect("stale hosts");
        let h3 = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        let after = std::fs::read_to_string(&hosts).expect("read");
        assert!(!after.contains(BLOCK_MARKER));
        h3.abort();
        let _ = h3.await;

        cleanup_hosts(&hosts);
        let _ = std::fs::remove_file(cfg_latch);
    }

    #[tokio::test]
    async fn boot_rearms_watcher_when_armed() {
        let port = pick_port();
        let hosts = seed_hosts("rearm-watcher");
        let mut opts = TestAgentOptions::default();
        let cfg_latch = tradeautopsy_agent::AgentConfig::test_on_port(port, TEST_SECRET)
            .kill_latch_db_path
            .clone();
        KillLatchStore::open(&cfg_latch)
            .expect("latch")
            .arm(&KillLatchSnapshot {
                active: true,
                level: 3,
                broker: "binance_us".to_string(),
                armed_at_ms: 1,
                expires_at_ms: None,
                requires_ack: true,
            })
            .expect("arm");
        tradeautopsy_agent::dns_block::apply_hosts_block("binance_us").expect("seed block");
        let before = std::fs::read_to_string(&hosts).expect("read");
        let h = spawn_test_agent_with_options(port, opts);
        wait_ready(port).await;
        let after_boot = std::fs::read_to_string(&hosts).expect("read after boot");
        assert_eq!(before.lines().count(), after_boot.lines().count());
        std::fs::write(&hosts, "127.0.0.1 localhost\n").expect("tamper");
        tokio::time::sleep(Duration::from_millis(800)).await;
        let restored = std::fs::read_to_string(&hosts).expect("restored");
        assert!(restored.contains(BLOCK_MARKER));
        h.abort();
        let _ = h.await;
        cleanup_hosts(&hosts);
    }

    #[tokio::test]
    async fn tamper_after_restart_is_reverted() {
        let port = pick_port();
        let hosts = seed_hosts("tamper-restart");
        let h = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        post_kill(port, "binance_us", 3).await;
        h.abort();
        let _ = h.await;

        let h2 = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        std::fs::write(&hosts, "127.0.0.1 localhost\n").expect("tamper");
        tokio::time::sleep(Duration::from_millis(800)).await;
        let restored = std::fs::read_to_string(&hosts).expect("restored");
        assert!(restored.contains(BLOCK_MARKER));
        dismiss(port).await;
        h2.abort();
        let _ = h2.await;
        cleanup_hosts(&hosts);
    }

    #[tokio::test]
    async fn expired_latch_self_dismisses() {
        let port = pick_port();
        let hosts = seed_hosts("expired");
        let cfg_latch = tradeautopsy_agent::AgentConfig::test_on_port(port, TEST_SECRET)
            .kill_latch_db_path
            .clone();
        KillLatchStore::open(&cfg_latch)
            .expect("latch")
            .arm(&KillLatchSnapshot {
                active: true,
                level: 3,
                broker: "binance_us".to_string(),
                armed_at_ms: 1,
                expires_at_ms: Some(1),
                requires_ack: true,
            })
            .expect("arm");
        tradeautopsy_agent::dns_block::apply_hosts_block("binance_us").expect("block");
        let h = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        let state = get_state(port).await;
        assert_eq!(state["active"], false);
        let content = std::fs::read_to_string(&hosts).expect("read");
        assert!(!content.contains(BLOCK_MARKER));
        h.abort();
        let _ = h.await;
        cleanup_hosts(&hosts);
    }

    #[tokio::test]
    async fn dismiss_clears_latch_only_after_hosts_clean() {
        let port = pick_port();
        let hosts = seed_hosts("dismiss-order");
        let h = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        post_kill(port, "binance_us", 3).await;
        let cfg_latch = tradeautopsy_agent::AgentConfig::test_on_port(port, TEST_SECRET)
            .kill_latch_db_path
            .clone();
        dismiss(port).await;
        let load = KillLatchStore::open(&cfg_latch).expect("latch").load().expect("load");
        assert_eq!(load, KillLatchLoad::Inactive);
        h.abort();
        let _ = h.await;
        cleanup_hosts(&hosts);
    }

    #[tokio::test]
    async fn latch_survives_sigterm() {
        let port = pick_port();
        let hosts = seed_hosts("sigterm");
        let cfg_latch = tradeautopsy_agent::AgentConfig::test_on_port(port, TEST_SECRET)
            .kill_latch_db_path
            .clone();
        let h = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        post_kill(port, "binance_us", 3).await;
        h.abort();
        let _ = h.await;
        let load = KillLatchStore::open(&cfg_latch).expect("latch").load().expect("load");
        assert!(
            matches!(load, KillLatchLoad::Active(_)),
            "latch must survive abrupt agent exit"
        );
        let content = std::fs::read_to_string(&hosts).expect("hosts");
        assert!(content.contains(BLOCK_MARKER));
        tradeautopsy_agent::dns_block::disable_block().ok();
        KillLatchStore::open(&cfg_latch).expect("latch").clear().ok();
        cleanup_hosts(&hosts);
    }

    #[tokio::test]
    async fn kill_switch_state_requires_wire_v1() {
        let port = pick_port();
        let hosts = seed_hosts("state-wire");
        let h = spawn_test_agent_with_options(port, TestAgentOptions::default());
        wait_ready(port).await;
        let url = format!("http://127.0.0.1:{port}/api/daemon/kill-switch/state");
        let resp = client()
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .expect("get");
        assert_eq!(resp.status(), 401);
        let signed = get_state(port).await;
        assert!(signed.get("dns_active").is_some());
        h.abort();
        let _ = h.await;
        cleanup_hosts(&hosts);
    }
}

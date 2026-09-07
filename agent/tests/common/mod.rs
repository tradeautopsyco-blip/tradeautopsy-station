//! Shared wire v1 signing for integration tests (issue #58).

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::SecondsFormat;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tradeautopsy_agent::{
    AgentConfig, BrokerAdapter, BrokerCredentialVault, BrokerSyncConfig, CredentialBlob,
    KillPolicy, MemoryBrokerCredentialVault, MemoryStationTokenStore, WireVerifier,
    WIRE_PROTO_VERSION,
};

pub const TEST_SECRET: &str = "integration-test-daemon-secret-min-32b";
pub const TEST_USER_ID: &str = "ac35ef44-6366-40d6-89d4-95530e8e3dbf";

pub struct TestAgentOptions {
    pub broker_adapter: Option<Arc<dyn BrokerAdapter>>,
    pub recent_trades_db_path: Option<PathBuf>,
    pub initial_backfill_days: i64,
    pub broker_initial_delay_ms: u64,
    pub broker_base_poll_ms: u64,
    pub toolbar_coalesce_ms: u64,
    pub failure_escalate_after: u32,
    pub broker_backoff_tick_1_ms: u64,
    pub broker_backoff_tick_2_ms: u64,
    pub failures_until_open: u32,
    pub fresh_secs: u64,
    pub stale_secs: u64,
    /// Phase 9 — Prometheus loopback (`127.0.0.1`). `None` = disabled (default).
    pub metrics_port: Option<u16>,
    /// Phase 9 — hosted daemon base URL override for proxy integration tests.
    pub upstream_base_url_override: Option<String>,
    /// Slice B — daemon command poll user (#190).
    pub daemon_poll_user_id: Option<String>,
    pub command_poll_ms: u64,
    /// Issues #13/#14 — inject adapter for runtime start/stop integration tests.
    pub runtime_poll_adapter: Option<Arc<dyn BrokerAdapter>>,
    pub start_key_log: Option<Arc<std::sync::Mutex<Vec<String>>>>,
    pub station_token_store: Option<Arc<dyn tradeautopsy_agent::StationTokenStore>>,
    /// B2 — shared host vault so tests seed credentials without wire secrets.
    pub credential_vault: Option<Arc<dyn BrokerCredentialVault>>,
    /// Slice 3 — plant a LiveBook snapshot before the first agent GET (no hydrate-on-sign-in).
    pub live_book_snapshot: Option<serde_json::Value>,
    /// Fact-plane boot — injectable online cadence (prod 20_000).
    pub fact_online_interval_ms: Option<u64>,
    /// Fact-plane boot — injectable clock so tests can pass the 15s coalesce without sleeping.
    pub fact_clock_ms: Option<Arc<std::sync::atomic::AtomicI64>>,
    /// Boot seed for the sqlite KillPolicy store (countdown / website_block / default_level).
    pub kill_policy: KillPolicy,
    /// Slice F — plant cash CSV + quote JSON (no live Kotak session).
    pub plant_kotak_s1k_fixtures: bool,
    /// Plant NFO LTP JSON into TickBook `kotak-nse-nfo` (no live session). Cash-only stays cash-only.
    pub plant_kotak_nfo_quote: bool,
    /// Plant lock-header FO CSV into the named NFO store (no live session).
    pub plant_kotak_nfo_contracts: bool,
    /// NFO order book — plant bounded snapshot into DepthBook `kotak-nse-nfo`.
    pub plant_kotak_nfo_depth: bool,
    /// NFO session OI — plant `quote_type=oi` slice into `nfo_oi_session`.
    pub plant_kotak_nfo_oi_session: bool,
    /// S2 — plant committed Binance klines JSON (no live Binance).
    pub plant_binance_s2_history: bool,
    /// Spot funds — plant AccountBook funds slot (no live Binance private GET).
    pub plant_binance_spot_funds: bool,
    /// Options last — plant committed eapi ticker JSON (no live eapi).
    pub plant_binance_options_quote: bool,
    /// Options chain/OI — plant committed exchangeInfo + OI JSON (no live eapi).
    pub plant_binance_options_chain: bool,
    /// Venue-published greeks — plant the committed `/eapi/v1/mark` JSON (no live eapi).
    pub plant_binance_options_mark: bool,
    /// Same, but the observed no-bid row (`"bidIV":"-1.0"`).
    pub plant_binance_options_mark_no_bid: bool,
    /// Options order book — plant the committed `/eapi/v1/depth` JSON (no live eapi).
    pub plant_binance_options_depth: bool,
    /// Options tradebook — plant the committed `/eapi/v1/userTrades` JSON (no live eapi).
    pub plant_binance_options_fills: bool,
    /// Spot private reads — point `BinanceComSpotClient` at wiremock (not `api.binance.com`).
    pub binance_spot_base_url: Option<String>,
}

impl Default for TestAgentOptions {
    fn default() -> Self {
        Self {
            broker_adapter: None,
            recent_trades_db_path: None,
            initial_backfill_days: 90,
            broker_initial_delay_ms: 0,
            broker_base_poll_ms: 250,
            toolbar_coalesce_ms: 50,
            failure_escalate_after: 3,
            broker_backoff_tick_1_ms: 15_000,
            broker_backoff_tick_2_ms: 30_000,
            failures_until_open: 5,
            fresh_secs: 6,
            stale_secs: 30,
            metrics_port: None,
            upstream_base_url_override: None,
            daemon_poll_user_id: None,
            command_poll_ms: 500,
            runtime_poll_adapter: None,
            start_key_log: None,
            station_token_store: None,
            credential_vault: None,
            live_book_snapshot: None,
            fact_online_interval_ms: None,
            fact_clock_ms: None,
            kill_policy: KillPolicy::default(),
            plant_kotak_s1k_fixtures: false,
            plant_kotak_nfo_quote: false,
            plant_kotak_nfo_contracts: false,
            plant_kotak_nfo_depth: false,
            plant_kotak_nfo_oi_session: false,
            plant_binance_s2_history: false,
            plant_binance_spot_funds: false,
            plant_binance_options_quote: false,
            plant_binance_options_chain: false,
            plant_binance_options_mark: false,
            plant_binance_options_mark_no_bid: false,
            plant_binance_options_depth: false,
            plant_binance_options_fills: false,
            binance_spot_base_url: None,
        }
    }
}

fn apply_broker_options(cfg: &mut AgentConfig, opts: &TestAgentOptions) {
    cfg.broker_adapter = opts.broker_adapter.clone();
    if let Some(p) = &opts.recent_trades_db_path {
        cfg.recent_trades_db_path = p.clone();
    }
    cfg.broker_sync = BrokerSyncConfig {
        initial_backfill_days: opts.initial_backfill_days,
        initial_startup_delay: Duration::from_millis(opts.broker_initial_delay_ms),
        base_poll_interval: Duration::from_millis(opts.broker_base_poll_ms),
        coalesce_window: Duration::from_millis(opts.toolbar_coalesce_ms),
        failure_escalate_after: opts.failure_escalate_after,
        backoff_tick_1: Duration::from_millis(opts.broker_backoff_tick_1_ms),
        backoff_tick_2: Duration::from_millis(opts.broker_backoff_tick_2_ms),
        failures_until_open: opts.failures_until_open,
        fresh_secs: opts.fresh_secs,
        stale_secs: opts.stale_secs,
        rate_limit_default_backoff_ms: 60_000,
    };
    cfg.metrics_port = opts.metrics_port;
    if let Some(base) = &opts.upstream_base_url_override {
        cfg.upstream.base_url = base.trim_end_matches('/').to_string();
    }
    cfg.daemon_poll_user_id = opts.daemon_poll_user_id.clone();
    cfg.test_runtime_adapter = opts.runtime_poll_adapter.clone();
    cfg.test_start_key_log = opts.start_key_log.clone();
    cfg.station_token_store = opts.station_token_store.clone();
    cfg.station_token_store_explicit = opts.station_token_store.is_some();
    if let Some(vault) = &opts.credential_vault {
        cfg.broker_credential_vault = Some(vault.clone());
    }
    cfg.live_book_snapshot = opts.live_book_snapshot.clone();
    if let Some(ms) = opts.fact_online_interval_ms {
        cfg.fact_online_interval_ms = ms;
    }
    cfg.fact_clock_ms = opts.fact_clock_ms.clone();
    cfg.kill_policy = opts.kill_policy.clone();
    cfg.plant_kotak_s1k_fixtures = opts.plant_kotak_s1k_fixtures;
    cfg.plant_kotak_nfo_quote = opts.plant_kotak_nfo_quote;
    cfg.plant_kotak_nfo_contracts = opts.plant_kotak_nfo_contracts;
    cfg.plant_kotak_nfo_depth = opts.plant_kotak_nfo_depth;
    cfg.plant_kotak_nfo_oi_session = opts.plant_kotak_nfo_oi_session;
    cfg.plant_binance_s2_history = opts.plant_binance_s2_history;
    cfg.plant_binance_spot_funds = opts.plant_binance_spot_funds;
    cfg.plant_binance_options_quote = opts.plant_binance_options_quote;
    cfg.plant_binance_options_chain = opts.plant_binance_options_chain;
    cfg.plant_binance_options_mark = opts.plant_binance_options_mark;
    cfg.plant_binance_options_mark_no_bid = opts.plant_binance_options_mark_no_bid;
    cfg.plant_binance_options_depth = opts.plant_binance_options_depth;
    cfg.plant_binance_options_fills = opts.plant_binance_options_fills;
    cfg.binance_spot_base_url = opts.binance_spot_base_url.clone();
}

fn remove_sqlite_files(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}

pub fn spawn_test_agent_with_options(
    port: u16,
    opts: TestAgentOptions,
) -> tokio::task::JoinHandle<()> {
    // A8 IV — agent→Console identity is Bearer; tests use env bootstrap token.
    if std::env::var("STATION_ACCESS_TOKEN").is_err() {
        std::env::set_var("STATION_ACCESS_TOKEN", "station.test.jwt");
    }
    let mut cfg = AgentConfig::test_on_port(port, TEST_SECRET.to_string());
    apply_broker_options(&mut cfg, &opts);
    if cfg.station_token_store.is_none() {
        // Tests must not block on developer Keychain / live-state hydrate.
        cfg.station_token_store = Some(Arc::new(MemoryStationTokenStore::default()));
    }
    if opts.recent_trades_db_path.is_none() {
        remove_sqlite_files(&cfg.recent_trades_db_path);
    }
    remove_sqlite_files(&cfg.kill_switch_audit_db_path);
    remove_sqlite_files(&cfg.fact_outbox_db_path);
    remove_sqlite_files(&cfg.history_db_path);
    tokio::spawn(async move {
        if let Err(e) = tradeautopsy_agent::run_agent(cfg).await {
            eprintln!("agent boot failed: {e:#}");
            panic!("agent should bind: {e:#}");
        }
    })
}

#[allow(dead_code)]
pub fn spawn_test_agent(port: u16) -> tokio::task::JoinHandle<()> {
    spawn_test_agent_with_options(port, TestAgentOptions::default())
}

#[allow(dead_code)]
pub async fn wait_ready(port: u16) {
    let path = "/api/daemon/health";
    let url = format!("http://127.0.0.1:{port}{path}");
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(80)).await;
        let Ok(resp) = apply_wire_v1(
            client().get(&url),
            "GET",
            path,
            b"",
            WireHeaderOverrides::default(),
        )
        .timeout(Duration::from_millis(400))
        .send()
        .await
        else {
            continue;
        };
        if resp.status().is_success() {
            return;
        }
    }
    panic!("agent on port {port} did not become ready");
}

/// Apply wire v1 headers for a canonical request. `path` must match `Uri::path` (e.g. `/api/daemon/health`).
pub fn apply_wire_v1(
    builder: reqwest::RequestBuilder,
    method: &str,
    path: &str,
    body: &[u8],
    overrides: WireHeaderOverrides<'_>,
) -> reqwest::RequestBuilder {
    let verifier = WireVerifier::new(TEST_SECRET);
    let request_id = overrides
        .request_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| ulid::Ulid::new().to_string());
    let ts = overrides
        .timestamp
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true));
    let nonce_bytes: [u8; 16] = overrides
        .nonce_16
        .unwrap_or_else(|| *uuid::Uuid::new_v4().as_bytes());
    let nonce_b64 = B64.encode(nonce_bytes);
    let sig = verifier.compute_signature(method, path, &ts, &request_id, body);
    let sig_b64 = overrides
        .signature_b64
        .map(|s| s.to_string())
        .unwrap_or_else(|| B64.encode(sig));

    let proto = overrides.proto_version.unwrap_or(WIRE_PROTO_VERSION);

    // Loopback wire: secret + HMAC = machine integrity; x-user-id = wire hint only (A8 IV).
    builder
        .header("x-proto-version", proto)
        .header("x-daemon-secret", TEST_SECRET)
        .header("x-user-id", TEST_USER_ID)
        .header("x-request-id", request_id.as_str())
        .header("x-timestamp", ts.as_str())
        .header("x-nonce", nonce_b64)
        .header("x-signature", sig_b64)
}

#[derive(Default)]
pub struct WireHeaderOverrides<'a> {
    pub proto_version: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub timestamp: Option<String>,
    pub nonce_16: Option<[u8; 16]>,
    /// Replace computed HMAC (for negative tests).
    pub signature_b64: Option<&'a str>,
}

pub fn client() -> reqwest::Client {
    reqwest::Client::new()
}

pub const TEST_BROKER_CONNECTION_ID: &str = "00000000-0000-4000-8000-000000000001";

/// Shared memory vault + seed HMAC credentials for B2 identity-only Start tests.
pub fn seeded_hmac_vault(broker_slug: &str, api_key: &str) -> Arc<MemoryBrokerCredentialVault> {
    let vault = Arc::new(MemoryBrokerCredentialVault::new());
    vault
        .save(
            "prod",
            broker_slug,
            TEST_BROKER_CONNECTION_ID,
            &CredentialBlob::hmac(api_key, "test-secret"),
        )
        .expect("seed vault");
    vault
}

pub fn identity_start_body(broker_slug: &str) -> serde_json::Value {
    serde_json::json!({
        "brokerSlug": broker_slug,
        "brokerConnectionId": TEST_BROKER_CONNECTION_ID,
        "environment": "prod",
        "assetClass": "crypto"
    })
}

mod api;
mod bar_fill_ingress;
/// Native Binance.com spot modules — **reference only** (ADR 0001 / B5).
/// Live Start uses `build_runtime_adapter` → `WasmBrokerAdapter`, never these.
mod binance_com_spot_adapter;
mod binance_com_spot_client;
mod binance_com_validation;
mod broker;
mod broker_data_class;
mod broker_sync;
mod broker_sync_control;
mod broker_validation;
mod broker_behavioral;
mod broker_redaction;
mod device_login;
mod dns_block;
mod event_bus;
mod instruments;
mod kill_switch_audit;
mod metrics;
mod outbox;
mod exchange_info;
mod recent_trades;
mod round_trip_engine;
mod resolve_kill_switch_broker;
mod sse_signing;
mod station_tokens;
mod today;
mod ubi;
mod wire;

pub use station_tokens::{
    bearer_authorization, KeyringStationTokenStore, MemoryStationTokenStore, StationTokenStore,
    StationTokens,
};
pub use device_login::{
    begin_device_login, complete_device_login, prove_station_session, DeviceLoginPending,
    DeviceLoginPublic, StationSessionIdentity,
};
pub use bar_fill_ingress::{BarBrokerFillIngressConfig, BarFillIngestSource};
pub use binance_com_spot_adapter::BinanceComSpotBrokerAdapter;
pub use binance_com_spot_client::{
    BinanceComAccountInfo, BinanceComBalance, BinanceComMyTrade, BinanceComSpotClient,
    BinanceComSpotError, DEFAULT_BASE_URL as BINANCE_COM_SPOT_BASE_URL,
};
pub use binance_com_validation::{
    FakeBinanceComValidationAdapter, LiveBinanceComValidationAdapter,
    DEFAULT_BASE_URL as BINANCE_COM_VALIDATION_BASE_URL,
};
pub use broker::{
    BrokerAdapter, BrokerError, BrokerFill, ConfigurableDataClassAdapter, CountingPollAdapter,
    DataClassPollRound, SeqMockBrokerAdapter,
};
pub use ubi::{
    calc_profile, catalog_v1, classify_response, component_candidate_paths, component_crate_dir,
    component_file_name, component_path_for_slug, compliance_profile, decode_credential_blob,
    descriptor_for_slug, effective_host, fill_event_to_broker_fill, host_allowed, prepare_request,
    redact_response_headers, run_fetch_fills, AdapterOrigin, AuthScheme, BrokerAvailability,
    BrokerCredentialVault, BrokerDescriptor, BrokerHttpFixture, BrokerHttpMode,
    BrokerHttpTransport, CalcProfile, ComplianceProfile, CredentialBlob, FillCursor,
    FillEvent as UbiFillEvent, HostCredentialBlob, KeyringBrokerCredentialVault,
    MemoryBrokerCredentialVault, PreparedHttpRequest, RecordingTransport,
    ReqwestBrokerHttpTransport, TransportResponse, UbiHostConfig, UbiHostError, UbiHostState,
    WasmBrokerAdapter, ALLOWED_BROKER_HOSTS, BROKER_CREDENTIAL_KEYCHAIN_SERVICE,
    COMPONENT_DIR_ENV, FORBIDDEN_COMPONENT_HEADERS, RESPONSE_HEADER_ALLOWLIST,
    KOTAK_SESSION_KEYCHAIN_SERVICE, keychain_service_for,
};
pub use broker_behavioral::{BrokerBehavioralRecorder, BrokerConnectionIdentityFields};
pub use broker_data_class::{
    BrokerBalancesSnapshot, BrokerDataClass, BrokerDataClassCompleteness, BrokerHolding,
    BrokerOpenOrder, BrokerOpenOrdersSnapshot, DataClassFreshness,
};
pub use broker_redaction::RedactionBoundary;
pub use broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
pub use broker_sync_control::{
    build_runtime_adapter, build_wasm_runtime_adapter, default_credential_vault,
    memory_credential_vault, uses_wasm_component, BrokerRuntimeCardStatus, BrokerSyncController,
    BrokerSyncStartRequest,
};
pub use broker_validation::{
    BrokerValidationAdapter, FakeBinanceUSValidationAdapter, LiveBinanceUSValidationAdapter,
    PermissionPosture, ValidationFailure, ValidationResult,
};
pub use dns_block::{hosts_for_broker, BLOCK_MARKER};
pub use event_bus::{AgentEvent, EventBus};
pub use instruments::InstrumentStore;
pub use kill_switch_audit::{
    canonical_audit_message, verify_audit_signature, KillSwitchAuditAppend, KillSwitchAuditRecord,
    KillSwitchAuditSigner, KillSwitchAuditStore,
};
pub use metrics::AgentMetrics;
pub use outbox::{
    queued_response_json, CaptureOutbox, DeadLetterStatusItem, OutboxConfig, OutboxCounts,
    OutboxStatusSnapshot, ProcessNowResult,
};
pub use exchange_info::{
    is_usd_pegged_stablecoin, is_usd_quoted_symbol, live_com_filters_ready, resolve_symbol_assets,
    ExchangeInfoSymbolCache, SymbolAssets, SymbolFilters,
};
pub use recent_trades::RecentTradesStore;
pub use round_trip_engine::{
    aggregate_known_pnl, is_aggregate_eligible, FillTimeFeePriceLookup, PairAssetFeeLookup,
    ReconstructResult, RoundTrip, RoundTripEngine, StablecoinAndBaseAssetFeeLookup, UnhandledFee,
};
pub use today::{
    TodayDegradedReason, TodayHeroPayload, TodayPayload, TodayService, TodayStore,
    open_inventory_from_fills, OpenInventoryRow,
};
pub use resolve_kill_switch_broker::resolve_kill_switch_broker;
pub use sse_signing::{verify_sse_event_signature, SseSigner, SseSigningPubKey};
pub use wire::{WireVerifier, WIRE_PROTO_VERSION};

pub use api::daemon_commands::{parse_daemon_command_type, DaemonCommandKind};

use chrono::Utc;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct UpstreamConfig {
    pub base_url: String,
    /// Loopback / legacy machine secret — NOT Console user identity (A8).
    pub daemon_secret: String,
}

impl UpstreamConfig {
    pub fn from_env(daemon_secret: &str) -> Self {
        let base_url = std::env::var("TRADEAUTOPSY_SERVER_BASE_URL")
            .or_else(|_| std::env::var("TRADEAUTOPSY_API_URL"))
            .unwrap_or_else(|_| "https://localhost:3000".to_string());
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            daemon_secret: daemon_secret.to_string(),
        }
    }

    /// Console brain URL must be HTTPS (A8). Local wiremock may use http when STATION_ACCESS_TOKEN is set.
    pub fn require_https_base(&self) -> anyhow::Result<()> {
        if self.base_url.starts_with("https://") {
            return Ok(());
        }
        if self.is_loopback_http_bootstrap() {
            return Ok(());
        }
        anyhow::bail!(
            "TRADEAUTOPSY_SERVER_BASE_URL must be https:// (got {})",
            self.base_url
        )
    }

    /// True when Console is a loopback origin (local `npm run dev`).
    /// Used only to trust mkcert / skip rustls webpki roots — never a production identity rail.
    fn is_loopback_console(&self) -> bool {
        let u = self.base_url.to_ascii_lowercase();
        let after_scheme = u.split("://").nth(1).unwrap_or("");
        let hostport = after_scheme.split('/').next().unwrap_or("");
        let host = if let Some(rest) = hostport.strip_prefix('[') {
            rest.split(']').next().unwrap_or("")
        } else {
            hostport.split(':').next().unwrap_or("")
        };
        matches!(host, "localhost" | "127.0.0.1" | "::1")
    }

    /// True only for the loopback-http test/bootstrap escape hatch (wiremock, integration
    /// tests). T1 hardening: `STATION_ACCESS_TOKEN` must never be treated as a production
    /// identity rail — it is only honored against a loopback base URL, never a real
    /// (https) Console, so a stray env var can't substitute for the Keychain Bearer.
    fn is_loopback_http_bootstrap(&self) -> bool {
        let testing = std::env::var("STATION_ACCESS_TOKEN").is_ok();
        let loopback_http = self.base_url.starts_with("http://127.0.0.1")
            || self.base_url.starts_with("http://localhost")
            || self.base_url.starts_with("http://[::1]");
        testing && loopback_http
    }
}

#[derive(Clone)]
pub struct UpstreamClient {
    pub config: UpstreamConfig,
    pub http: reqwest::Client,
}

impl UpstreamClient {
    pub fn new(config: UpstreamConfig) -> anyhow::Result<Self> {
        // TLS verification must always be on for a real (https) Console.
        // Loopback Console uses mkcert; reqwest rustls-webpki does not trust that CA.
        let accept_invalid_certs =
            config.is_loopback_http_bootstrap() || config.is_loopback_console();
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()?;
        Ok(Self { config, http })
    }

    /// Brain identity: Station Caller Bearer from Keychain (A8). Never x-user-id.
    pub fn brain_authorization_header(&self) -> anyhow::Result<String> {
        self.config.require_https_base()?;
        if let Some(tokens) = KeyringStationTokenStore.load()? {
            return Ok(bearer_authorization(&tokens));
        }
        // Test / bootstrap only — never a production identity rail (T1). Gated to the
        // same loopback-http escape hatch as `require_https_base` so this can never
        // apply against a real (https) Console, even if the env var is stray-set.
        if self.config.is_loopback_http_bootstrap() {
            if let Ok(token) = std::env::var("STATION_ACCESS_TOKEN") {
                if !token.trim().is_empty() {
                    return Ok(format!("Bearer {}", token.trim()));
                }
            }
        }
        anyhow::bail!("No Station Caller tokens in Keychain — complete device login")
    }

    /// Attach Authorization Bearer for Console APIs. Does not send daemon identity headers.
    pub fn authorize_brain(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let auth = self.brain_authorization_header()?;
        Ok(builder.header(reqwest::header::AUTHORIZATION, auth))
    }
}

/// Configuration for [`run_agent`], including mandatory daemon shared secret for wire v1.
pub struct AgentConfig {
    /// Loopback HTTP port (env **`AGENT_PORT`**, default **9137**). Must not collide with TradeAutopsy Tauri **`DAEMON_PORT`** on the same machine.
    pub port: u16,
    pub daemon_secret: String,
    pub heartbeat_ms: u64,
    /// Loopback Prometheus `/metrics` port (`127.0.0.1`). `None` disables (tests).
    pub metrics_port: Option<u16>,
    pub upstream: UpstreamConfig,
    pub outbox: OutboxConfig,
    pub recent_trades_db_path: PathBuf,
    pub today_db_path: PathBuf,
    pub instruments_db_path: PathBuf,
    pub kill_switch_audit_db_path: PathBuf,
    pub broker_adapter: Option<Arc<dyn BrokerAdapter>>,
    pub broker_sync: BrokerSyncConfig,
    pub bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
    /// Integration tests — inject runtime start adapter (#13/#14).
    pub test_runtime_adapter: Option<Arc<dyn BrokerAdapter>>,
    pub test_start_key_log: Option<Arc<std::sync::Mutex<Vec<String>>>>,
    /// Host credential vault (Keychain in prod; memory in tests).
    pub broker_credential_vault: Option<Arc<dyn crate::ubi::BrokerCredentialVault>>,
    /// User id for `GET /api/daemon/command` poll (#190). `None` disables poll loop.
    pub daemon_poll_user_id: Option<String>,
    /// Override Station token store (tests use memory; prod uses Keychain).
    pub station_token_store: Option<Arc<dyn StationTokenStore>>,
}

impl AgentConfig {
    /// Loads env per ship defaults; Phase 6 adds `AGENT_RECENT_TRADES_DB_PATH`.
    pub fn from_env() -> anyhow::Result<Self> {
        let port: u16 = std::env::var("AGENT_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(9137);
        let daemon_secret = std::env::var("AGENT_DAEMON_SECRET").map_err(|_| {
            anyhow::anyhow!("AGENT_DAEMON_SECRET is required for tradeautopsy-agent")
        })?;
        let heartbeat_ms: u64 = std::env::var("TRADEAUTOPY_AGENT_HEARTBEAT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000);
        let upstream = UpstreamConfig::from_env(&daemon_secret);
        let outbox = OutboxConfig::from_env();
        let recent_trades_db_path = std::env::var("AGENT_RECENT_TRADES_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-recent-trades.db");
                p
            });
        let today_db_path = std::env::var("AGENT_TODAY_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-today.db");
                p
            });
        let instruments_db_path = std::env::var("AGENT_INSTRUMENTS_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("instruments.db");
                p
            });
        let kill_switch_audit_db_path = KillSwitchAuditStore::db_path_from_env_or_default();
        let metrics_port: Option<u16> = match std::env::var("AGENT_METRICS_PORT").ok() {
            Some(s) => match s.parse::<u16>() {
                Ok(0) => None,
                Ok(p) => Some(p),
                Err(_) => Some(9138),
            },
            None => Some(9138),
        };
        let bar_fill_ingress = BarBrokerFillIngressConfig::from_env();
        let daemon_poll_user_id = std::env::var("AGENT_DAEMON_USER_ID")
            .ok()
            .or_else(|| std::env::var("AGENT_BAR_INGEST_USER_ID").ok())
            .filter(|s| uuid::Uuid::parse_str(s).is_ok());
        Ok(Self {
            port,
            daemon_secret,
            heartbeat_ms,
            metrics_port,
            upstream,
            outbox,
            recent_trades_db_path,
            today_db_path,
            instruments_db_path,
            kill_switch_audit_db_path,
            broker_adapter: None,
            broker_sync: BrokerSyncConfig::default(),
            bar_fill_ingress,
            test_runtime_adapter: None,
            test_start_key_log: None,
            broker_credential_vault: None,
            daemon_poll_user_id,
            station_token_store: None,
        })
    }

    /// Deterministic local config for integration tests (issue #57 / #58 wire harness).
    pub fn test_on_port(port: u16, daemon_secret: impl Into<String>) -> Self {
        let daemon_secret = daemon_secret.into();
        let mut recent_trades_db_path = std::env::temp_dir();
        recent_trades_db_path.push(format!("rta-recent-{port}.db"));
        let mut today_db_path = std::env::temp_dir();
        today_db_path.push(format!("rta-today-{port}.db"));
        let mut instruments_db_path = std::env::temp_dir();
        instruments_db_path.push(format!("rta-instruments-{port}.db"));
        let mut kill_switch_audit_db_path = std::env::temp_dir();
        kill_switch_audit_db_path.push(format!("rta-kill-switch-audit-{port}.db"));
        Self {
            port,
            daemon_secret: daemon_secret.clone(),
            heartbeat_ms: std::env::var("TRADEAUTOPY_AGENT_HEARTBEAT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5000),
            metrics_port: None,
            upstream: UpstreamConfig::from_env(&daemon_secret),
            outbox: OutboxConfig::from_env(),
            recent_trades_db_path,
            today_db_path,
            instruments_db_path,
            kill_switch_audit_db_path,
            broker_adapter: None,
            broker_sync: BrokerSyncConfig::default(),
            bar_fill_ingress: None,
            test_runtime_adapter: None,
            test_start_key_log: None,
            broker_credential_vault: Some(memory_credential_vault()),
            daemon_poll_user_id: None,
            station_token_store: None,
        }
    }
}

#[cfg(test)]
mod upstream_config_bridge_harden_tests {
    use super::UpstreamConfig;
    use serial_test::serial;

    fn cfg(base_url: &str) -> UpstreamConfig {
        UpstreamConfig {
            base_url: base_url.to_string(),
            daemon_secret: "unit-test-secret".to_string(),
        }
    }

    #[test]
    #[serial]
    fn https_base_never_uses_bootstrap_escape_hatch() {
        std::env::set_var("STATION_ACCESS_TOKEN", "unit-test-token");
        let c = cfg("https://tradeautopsy.in");
        assert!(c.require_https_base().is_ok());
        assert!(
            !c.is_loopback_http_bootstrap(),
            "a real https Console base must never be treated as the STATION_ACCESS_TOKEN bootstrap escape hatch"
        );
        std::env::remove_var("STATION_ACCESS_TOKEN");
    }

    #[test]
    fn loopback_https_console_is_local_mkcert_origin_not_production() {
        let local = cfg("https://localhost:3000");
        assert!(local.is_loopback_console());
        assert!(cfg("https://127.0.0.1:3000").is_loopback_console());
        assert!(!cfg("https://tradeautopsy.in").is_loopback_console());
    }

    #[test]
    #[serial]
    fn loopback_http_requires_station_access_token_to_bootstrap() {
        std::env::remove_var("STATION_ACCESS_TOKEN");
        let c = cfg("http://127.0.0.1:9999");
        assert!(c.require_https_base().is_err(), "no bootstrap token set");
        assert!(!c.is_loopback_http_bootstrap());

        std::env::set_var("STATION_ACCESS_TOKEN", "unit-test-token");
        assert!(c.require_https_base().is_ok());
        assert!(c.is_loopback_http_bootstrap());
        std::env::remove_var("STATION_ACCESS_TOKEN");
    }

    #[test]
    #[serial]
    fn non_loopback_http_base_is_never_a_bootstrap_target() {
        std::env::set_var("STATION_ACCESS_TOKEN", "unit-test-token");
        let c = cfg("http://example.com");
        assert!(
            !c.is_loopback_http_bootstrap(),
            "STATION_ACCESS_TOKEN must not bootstrap a non-loopback http base"
        );
        assert!(c.require_https_base().is_err());
        std::env::remove_var("STATION_ACCESS_TOKEN");
    }
}

pub struct AgentRuntime {
    boot_id: String,
    build: String,
    version: String,
    started_at: Instant,
    seq: AtomicU64,
}

impl AgentRuntime {
    fn new() -> Self {
        Self {
            boot_id: ulid::Ulid::new().to_string(),
            build: format!("tradeautopsy-agent/{}", env!("CARGO_PKG_VERSION")),
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: Instant::now(),
            seq: AtomicU64::new(0),
        }
    }

    pub fn boot_id(&self) -> &str {
        &self.boot_id
    }

    pub fn build(&self) -> &str {
        &self.build
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Headless agent: loopback HTTP + SSE; wire v1 verification on bar routes (issue #58).
pub async fn run_agent(config: AgentConfig) -> anyhow::Result<()> {
    let event_bus = EventBus::new(2048);
    let runtime = Arc::new(AgentRuntime::new());
    let metrics = Arc::new(AgentMetrics::default());
    if let Some(mp) = config.metrics_port {
        metrics::spawn_metrics_server(metrics.clone(), mp);
    }
    let sse_signer = Arc::new(SseSigner::from_env_or_generate());
    let wire = Arc::new(WireVerifier::new(config.daemon_secret));
    let upstream = Arc::new(UpstreamClient::new(config.upstream)?);
    let outbox = Arc::new(CaptureOutbox::new(config.outbox, upstream.clone())?);
    let recent_trades = RecentTradesStore::open(&config.recent_trades_db_path)?;
    let today_store = TodayStore::open(&config.today_db_path)?;
    let kill_switch_audit = KillSwitchAuditStore::open(&config.kill_switch_audit_db_path)?;
    let audit_signer = Arc::new(KillSwitchAuditSigner::from_env_or_generate());
    let instruments = Arc::new(InstrumentStore::new(
        config
            .instruments_db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("instruments db path is not valid UTF-8"))?,
    )?);
    let inst_clone = instruments.clone();
    tokio::spawn(async move {
        match inst_clone.needs_refresh() {
            Ok(true) => match inst_clone.refresh_from_zerodha().await {
                Ok(n) => tracing::info!("instruments refreshed: {n} rows"),
                Err(e) => tracing::warn!("instrument refresh failed: {e}"),
            },
            Ok(false) => tracing::debug!("instruments cache still fresh"),
            Err(e) => tracing::warn!("instrument refresh check failed: {e}"),
        }
    });

    let broker_status = Arc::new(std::sync::Mutex::new(BrokerRuntimeState::default()));

    let initial_since = recent_trades
        .newest_filled_at()
        .unwrap_or(None)
        .or_else(|| {
            let days = config.broker_sync.initial_backfill_days.max(0);
            Some(Utc::now() - chrono::Duration::days(days))
        });
    let since = Arc::new(RwLock::new(initial_since));

    let today_service_slot = Arc::new(Mutex::new(None));
    let credential_vault = config
        .broker_credential_vault
        .clone()
        .unwrap_or_else(default_credential_vault);
    let broker_sync_control = Arc::new(BrokerSyncController::new(
        broker_status.clone(),
        recent_trades.clone(),
        since.clone(),
        config.broker_sync.clone(),
        event_bus.clone(),
        Some(upstream.clone()),
        config.bar_fill_ingress.clone(),
        config.test_runtime_adapter.clone(),
        config.test_start_key_log.clone(),
        today_service_slot.clone(),
        credential_vault,
    ));

    let today_service = Arc::new(TodayService::new(
        recent_trades.clone(),
        today_store,
        broker_status.clone(),
        broker_sync_control.clone(),
        config.broker_sync.clone(),
        ExchangeInfoSymbolCache::empty(), // I-S4: empty ≠ filters ready; live COM degrades until loaded.
    ));
    *today_service_slot.lock().expect("today service slot") = Some(today_service.clone());

    if let Some(adapter) = config.broker_adapter.clone() {
        broker_sync_control
            .start_with_adapter(adapter)
            .expect("boot-time broker adapter should start");
    }

    let fog_active = Arc::new(AtomicBool::new(false));

    let state = api::AppState {
        event_bus: event_bus.clone(),
        runtime: runtime.clone(),
        wire,
        sse_signer,
        metrics: metrics.clone(),
        metrics_listen_port: config.metrics_port,
        outbox: outbox.clone(),
        upstream,
        recent_trades,
        instruments,
        broker_status: broker_status.clone(),
        broker_sync_control: broker_sync_control.clone(),
        broker_limits: config.broker_sync.clone(),
        fog_active: fog_active.clone(),
        kill_switch_audit,
        audit_signer,
        last_l3_broker: Arc::new(std::sync::Mutex::new(None)),
        today_service: today_service.clone(),
        device_login_pending: Arc::new(std::sync::Mutex::new(None)),
        station_token_store: config
            .station_token_store
            .unwrap_or_else(|| Arc::new(KeyringStationTokenStore)),
    };
    let router = api::router(state.clone());

    metrics.set_uptime_secs(runtime.uptime_secs());

    let heartbeat_ms = config.heartbeat_ms;
    let bus = event_bus.clone();
    let heartbeat_metrics = metrics.clone();
    let mut background = tokio::task::JoinSet::new();
    background.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(heartbeat_ms));
        interval.tick().await;
        loop {
            interval.tick().await;
            let uptime = runtime.uptime_secs();
            heartbeat_metrics.set_uptime_secs(uptime);
            bus.publish(AgentEvent::AgentHealth {
                uptime_secs: uptime,
            });
        }
    });

    for _ in 0..outbox.worker_count() {
        let worker = outbox.clone();
        background.spawn(async move {
            worker.run_worker_loop().await;
        });
    }

    if let Some(user_id) = config.daemon_poll_user_id.clone() {
        let poll_ms = std::env::var("AGENT_COMMAND_POLL_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10_000);
        tracing::info!(
            user_id = %user_id,
            poll_ms,
            "daemon_commands poll loop starting"
        );
        api::daemon_commands::spawn_daemon_command_poll_loop(
            state.clone(),
            user_id,
            poll_ms,
            fog_active,
        );
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!(%addr, "tradeautopsy-agent listening");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                anyhow::anyhow!(
                    "tradeautopsy-agent cannot bind {addr}: {e} (address already in use). \
                     TradeAutopsy (Tauri) may already own this port as DAEMON_PORT (default 9137). \
                     Remediation: quit the desktop app, or set AGENT_PORT to a free port (e.g. `AGENT_PORT=9140`). \
                     Swift Notch defaults to 9137 unless the host app passes another port at launch."
                )
            } else {
                e.into()
            }
        })?;
    let serve_result = axum::serve(listener, router).await;
    background.abort_all();
    serve_result?;
    Ok(())
}

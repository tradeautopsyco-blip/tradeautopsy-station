mod api;
mod bar_fill_ingress;
mod broker;
mod broker_data_class;
mod broker_sync;
mod broker_sync_control;
mod broker_validation;
mod broker_behavioral;
mod broker_redaction;
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
mod today;
mod wire;

pub use bar_fill_ingress::{BarBrokerFillIngressConfig, BarFillIngestSource};
pub use broker::{
    BrokerAdapter, BrokerError, BrokerFill, ConfigurableDataClassAdapter, CountingPollAdapter,
    DataClassPollRound, SeqMockBrokerAdapter,
};
pub use broker_behavioral::{BrokerBehavioralRecorder, BrokerConnectionIdentityFields};
pub use broker_data_class::{
    BrokerBalancesSnapshot, BrokerDataClass, BrokerDataClassCompleteness, BrokerHolding,
    BrokerOpenOrder, BrokerOpenOrdersSnapshot, DataClassFreshness,
};
pub use broker_redaction::RedactionBoundary;
pub use broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
pub use broker_sync_control::{BrokerRuntimeCardStatus, BrokerSyncController, BrokerSyncStartRequest};
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
    is_usd_pegged_stablecoin, is_usd_quoted_symbol, resolve_symbol_assets, ExchangeInfoSymbolCache,
    SymbolAssets,
};
pub use recent_trades::RecentTradesStore;
pub use round_trip_engine::{
    aggregate_known_pnl, is_aggregate_eligible, FillTimeFeePriceLookup, PairAssetFeeLookup,
    ReconstructResult, RoundTrip, RoundTripEngine, StablecoinAndBaseAssetFeeLookup, UnhandledFee,
};
pub use today::{
    TodayDegradedReason, TodayHeroPayload, TodayPayload, TodayService, TodayStore,
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
    pub daemon_secret: String,
}

impl UpstreamConfig {
    pub fn from_env(daemon_secret: &str) -> Self {
        let base_url = std::env::var("TRADEAUTOPSY_SERVER_BASE_URL")
            .unwrap_or_else(|_| "https://tradeautopsy.in".to_string());
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            daemon_secret: daemon_secret.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct UpstreamClient {
    pub config: UpstreamConfig,
    pub http: reqwest::Client,
}

impl UpstreamClient {
    pub fn new(config: UpstreamConfig) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()?;
        Ok(Self { config, http })
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
    /// User id for `GET /api/daemon/command` poll (#190). `None` disables poll loop.
    pub daemon_poll_user_id: Option<String>,
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
            daemon_poll_user_id,
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
            daemon_poll_user_id: None,
        }
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
    ));

    let today_service = Arc::new(TodayService::new(
        recent_trades.clone(),
        today_store,
        broker_status.clone(),
        broker_sync_control.clone(),
        config.broker_sync.clone(),
        ExchangeInfoSymbolCache::empty(),
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

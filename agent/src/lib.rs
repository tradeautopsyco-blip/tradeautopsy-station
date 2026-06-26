mod api;
mod bar_fill_ingress;
mod broker;
mod broker_sync;
mod dns_block;
mod event_bus;
mod instruments;
mod metrics;
mod outbox;
mod recent_trades;
mod sse_signing;
mod wire;

pub use bar_fill_ingress::{BarBrokerFillIngressConfig, BarFillIngestSource};
pub use broker::{BrokerAdapter, BrokerError, BrokerFill, SeqMockBrokerAdapter};
pub use broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
pub use dns_block::{hosts_for_broker, BLOCK_MARKER};
pub use event_bus::{AgentEvent, EventBus};
pub use metrics::AgentMetrics;
pub use outbox::{
    queued_response_json, CaptureOutbox, DeadLetterStatusItem, OutboxConfig, OutboxCounts,
    OutboxStatusSnapshot, ProcessNowResult,
};
pub use instruments::InstrumentStore;
pub use recent_trades::RecentTradesStore;
pub use sse_signing::{verify_sse_event_signature, SseSigner, SseSigningPubKey};
pub use wire::{WireVerifier, WIRE_PROTO_VERSION};

pub use api::daemon_commands::{
    parse_daemon_command_type, DaemonCommandKind,
};

use crate::broker_sync::spawn_broker_stack;
use chrono::{DateTime, Utc};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
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
    pub instruments_db_path: PathBuf,
    pub broker_adapter: Option<Arc<dyn BrokerAdapter>>,
    pub broker_sync: BrokerSyncConfig,
    pub bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
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
        let instruments_db_path = std::env::var("AGENT_INSTRUMENTS_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("instruments.db");
                p
            });
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
            instruments_db_path,
            broker_adapter: None,
            broker_sync: BrokerSyncConfig::default(),
            bar_fill_ingress,
            daemon_poll_user_id,
        })
    }

    /// Deterministic local config for integration tests (issue #57 / #58 wire harness).
    pub fn test_on_port(port: u16, daemon_secret: impl Into<String>) -> Self {
        let daemon_secret = daemon_secret.into();
        let mut recent_trades_db_path = std::env::temp_dir();
        recent_trades_db_path.push(format!("rta-recent-{port}.db"));
        let mut instruments_db_path = std::env::temp_dir();
        instruments_db_path.push(format!("rta-instruments-{port}.db"));
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
            instruments_db_path,
            broker_adapter: None,
            broker_sync: BrokerSyncConfig::default(),
            bar_fill_ingress: None,
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

    let since = Arc::new(RwLock::new(Option::<DateTime<Utc>>::None));

    if let Some(adapter) = config.broker_adapter.clone() {
        spawn_broker_stack(
            adapter,
            recent_trades.clone(),
            since,
            config.broker_sync.clone(),
            event_bus.clone(),
            broker_status.clone(),
            Some(upstream.clone()),
            config.bar_fill_ingress.clone(),
        );
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
        broker_limits: config.broker_sync.clone(),
        fog_active: fog_active.clone(),
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

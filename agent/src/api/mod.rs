use crate::{
    broker_sync::{BrokerRuntimeState, BrokerSyncConfig},
    broker_sync_control::BrokerSyncController,
    data::{BrokerConnectionRuntime, DepthBook, HistoryBook, Registry, SourceManifest, TickBook},
    event_bus::EventBus,
    exchange_info::ExchangeInfoSymbolCache,
    instruments::InstrumentStore,
    kill_policy::KillPolicyStore,
    kill_switch_audit::{KillSwitchAuditSigner, KillSwitchAuditStore},
    kotak_scrip_master::KotakScripMaster,
    live_book::LiveBook,
    metrics::AgentMetrics,
    recent_trades::RecentTradesStore,
    sse_signing::SseSigner,
    wire, AgentRuntime, CaptureOutbox, UpstreamClient,
};
use axum::{
    middleware,
    routing::{get, patch, post},
    Router,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod bar;
mod broker_credentials;
mod broker_sync;
mod broker_sync_state;
mod capture;
pub mod daemon_commands;
pub(crate) mod desk;
mod health;
mod instruments;
mod kill_switch;
pub use kill_switch::{
    effective_level, plan_l3_dns, resolve_clear_fog, resolve_kill_apply, KillApplyDecision,
    KillClearDecision,
};
mod glance;
mod history;
mod kotak_session;
mod manifest;
mod outbox_status;
mod phase8;
mod positions;
mod quote;
mod recent_trades;
mod sse;
mod station_auth;
mod today;

#[derive(Clone)]
pub struct AppState {
    pub event_bus: EventBus,
    pub runtime: Arc<AgentRuntime>,
    pub wire: Arc<wire::WireVerifier>,
    pub sse_signer: Arc<SseSigner>,
    pub metrics: Arc<AgentMetrics>,
    /// Loopback Prometheus port (`127.0.0.1`), if enabled — surfaced on `/health` for bar QA UI.
    pub metrics_listen_port: Option<u16>,
    pub outbox: Arc<CaptureOutbox>,
    pub upstream: Arc<UpstreamClient>,
    pub recent_trades: RecentTradesStore,
    pub instruments: Arc<InstrumentStore>,
    pub broker_status: Arc<std::sync::Mutex<BrokerRuntimeState>>,
    pub broker_sync_control: Arc<BrokerSyncController>,
    pub broker_limits: BrokerSyncConfig,
    /// L1 fog-of-war armed (#190). L1 apply must not set this (T5 / Q8).
    pub fog_active: Arc<AtomicBool>,
    pub kill_switch_audit: KillSwitchAuditStore,
    pub audit_signer: Arc<KillSwitchAuditSigner>,
    pub last_l3_broker: Arc<std::sync::Mutex<Option<String>>>,
    /// Last fire level (1|2|3+) so dismiss audit is not hardcoded to 3.
    pub last_applied_level: Arc<std::sync::Mutex<Option<u8>>>,
    /// One sqlite row next to audit. Apply always loads this store.
    pub kill_policy: KillPolicyStore,
    pub today_service: Arc<crate::today::TodayService>,
    /// In-flight WorkOS device grant (`device_code` never leaves this process).
    pub device_login_pending: Arc<std::sync::Mutex<Option<crate::DeviceLoginPending>>>,
    /// Station Caller tokens (Keychain in prod; memory in tests when injected).
    pub station_token_store: Arc<dyn crate::StationTokenStore>,
    /// In-memory live-read book. After one snapshot, GET live-state serves this.
    /// BAR hosted live-state — not quote TickBook.
    pub live_book: Arc<LiveBook>,
    /// S1 quote registry (family + id + physics). Fail-closed at boot.
    pub quote_registry: Arc<Registry>,
    /// Market Plane last-price book. Not LiveBook, not broker account REST.
    pub tickbook: Arc<Mutex<TickBook>>,
    /// REST depth bounded snapshots. Not TickBook, not an ordered replica.
    pub depthbook: Arc<Mutex<DepthBook>>,
    /// Licensed historical_series. Not TickBook, not Yahoo.
    pub historybook: Arc<Mutex<HistoryBook>>,
    pub quote_freshness: Duration,
    pub s1_desk_symbol: Option<String>,
    /// First-party S0 manifests. Fail-closed at boot.
    pub source_manifests: Arc<Vec<SourceManifest>>,
    /// Persistent connected-broker runtimes. Handles only — never raw secrets.
    pub broker_connections: Arc<Mutex<HashMap<String, BrokerConnectionRuntime>>>,
    /// Local COM instrument master from unsigned `GET /api/v3/exchangeInfo`.
    pub instrument_master: Arc<Mutex<ExchangeInfoSymbolCache>>,
    /// Kotak cash scrip master. Separate from `instrument_master` (do not blend).
    pub kotak_scrip_master: Arc<Mutex<KotakScripMaster>>,
    /// Instruments that already have a public `@trade` stream task.
    pub quote_streams: Arc<Mutex<HashSet<String>>>,
    /// Instrument+interval keys already kicked for public klines → HistoryBook.
    pub klines_inflight: Arc<Mutex<HashSet<String>>>,
    /// Start-time env + connection_id for Kotak PrivateRead quotes (handles only).
    pub kotak_session_locator: crate::kotak_rest_quotes::SessionLocator,
    /// In-flight Kotak REST quote GETs (not book.subscribe — REST must stay open).
    pub kotak_quote_inflight: Arc<Mutex<HashSet<String>>>,
    /// In-flight Kotak REST depth GETs (`quote_type=depth` only).
    pub kotak_depth_inflight: Arc<Mutex<HashSet<String>>>,
    /// Shared catalog fetch status (not an ObtainStatus variant).
    pub instrument_master_status: Arc<Mutex<crate::data::InstrumentMasterStatus>>,
    /// Stop / re-Start cancels in-flight catalog retries.
    pub instrument_master_cancel: Arc<AtomicBool>,
    pub instrument_master_cache_dir: std::path::PathBuf,
    /// Last quote GET that passed connected-broker id validation. Quote pill keys off this.
    pub selected_quote_instrument: Arc<Mutex<Option<String>>>,
    /// Per-instrument quote fetch class (`quotes_http` / `session` / `quotes_unusable`) — never URLs or bodies.
    pub quote_fetch_error: Arc<Mutex<HashMap<String, String>>>,
}

pub fn router(state: AppState) -> Router {
    let state_for_layer = state.clone();
    let protected = Router::new()
        .route("/api/daemon/health", get(health::handler))
        .route("/api/daemon/events/stream", get(sse::handler))
        .route(
            "/api/daemon/toolbar/recent-trades",
            get(recent_trades::handler),
        )
        .route(
            "/api/daemon/broker/sync-state",
            get(broker_sync_state::handler),
        )
        .route(
            "/api/daemon/broker/sync/start",
            post(broker_sync::start_handler),
        )
        .route(
            "/api/daemon/broker/sync/stop",
            post(broker_sync::stop_handler),
        )
        .route(
            "/api/daemon/broker/sync/retry",
            post(broker_sync::retry_handler),
        )
        .route(
            "/api/daemon/broker/kotak/session/mint",
            post(kotak_session::mint_handler),
        )
        .route(
            "/api/daemon/broker/credentials/clear",
            post(broker_credentials::clear_handler),
        )
        .route(
            "/api/daemon/broker/credentials/present",
            post(broker_credentials::present_handler),
        )
        .route("/api/daemon/today", get(today::handler))
        .route("/api/daemon/positions", get(positions::handler))
        .route(
            "/api/daemon/journal/toolbar-capture/accept",
            post(capture::accept_handler),
        )
        .route(
            "/api/daemon/journal/toolbar-capture/outbox/status",
            get(outbox_status::handler),
        )
        .route(
            "/api/daemon/screenshot/presign",
            post(capture::screenshot_presign_handler),
        )
        .route(
            "/api/daemon/journal/toolbar-capture/pending/:id",
            get(capture::pending_get_handler).patch(capture::pending_patch_handler),
        )
        .route("/api/daemon/bar/live-state", get(bar::live_state_handler))
        .route("/api/daemon/bar/declare", post(bar::declare_handler))
        .route("/api/daemon/bar/stop-me", post(bar::stop_me_handler))
        .route(
            "/api/daemon/bar/stop-me/clear",
            post(bar::stop_me_clear_handler),
        )
        .route(
            "/api/daemon/bar/cancel-declaration",
            post(bar::cancel_declaration_handler),
        )
        .route("/api/daemon/bar/protective", post(bar::protective_handler))
        .route(
            "/api/daemon/bar/live-interference",
            post(bar::live_interference_handler),
        )
        .route(
            "/api/daemon/bar/swing-check-in",
            post(bar::swing_check_in_handler),
        )
        .route(
            "/api/daemon/bar/post-trade-debrief",
            patch(bar::post_trade_debrief_handler),
        )
        .route(
            "/api/daemon/bar/profile/loss-limits",
            get(bar::loss_limits_get_handler).post(bar::loss_limits_post_handler),
        )
        .route(
            "/api/daemon/kill-switch",
            post(kill_switch::kill_switch_handler),
        )
        .route(
            "/api/daemon/dismiss-kill-switch",
            post(kill_switch::dismiss_kill_switch_handler),
        )
        .route(
            "/api/daemon/kill-switch/ack",
            post(phase8::kill_switch_ack_handler),
        )
        .route(
            "/api/daemon/kill-switch/audit",
            get(kill_switch::kill_switch_audit_handler),
        )
        .route("/api/daemon/auth/begin", post(phase8::auth_begin_handler))
        .route("/api/daemon/auth/finish", post(phase8::auth_finish_handler))
        .route(
            "/api/daemon/auth/station/begin",
            post(station_auth::station_auth_begin_handler),
        )
        .route(
            "/api/daemon/auth/station/complete",
            post(station_auth::station_auth_complete_handler),
        )
        .route(
            "/api/daemon/auth/station/session",
            get(station_auth::station_auth_session_handler),
        )
        .route(
            "/api/daemon/auth/station/sign-out",
            post(station_auth::station_auth_sign_out_handler),
        )
        .route("/instruments/search", get(instruments::search_instruments))
        .route("/instruments/ltp", get(instruments::get_ltp))
        .route_layer(middleware::from_fn_with_state(
            state_for_layer,
            wire::verify_middleware,
        ));

    // Loopback extract so the founder can curl last price without HMAC.
    // Bind remains 127.0.0.1. TickBook ≠ LiveBook. No ingestSignal.
    Router::new()
        .route("/api/station/quote", get(quote::handler))
        .route("/api/station/history", get(history::handler))
        .route("/api/station/chain", get(glance::chain_handler))
        .route("/api/station/oi", get(glance::oi_handler))
        .route("/api/station/manifest", get(manifest::manifest_handler))
        .route("/api/station/obtain", get(manifest::obtain_handler))
        .merge(protected)
        .with_state(state)
}

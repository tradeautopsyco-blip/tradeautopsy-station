use crate::{
    broker_sync::{BrokerRuntimeState, BrokerSyncConfig},
    broker_sync_control::BrokerSyncController,
    event_bus::EventBus,
    instruments::InstrumentStore,
    kill_switch_audit::{KillSwitchAuditSigner, KillSwitchAuditStore},
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
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

mod bar;
mod broker_sync;
mod broker_sync_state;
mod capture;
pub mod daemon_commands;
mod health;
mod instruments;
mod kill_switch;
mod outbox_status;
mod phase8;
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
    /// L1 fog-of-war armed (#190).
    pub fog_active: Arc<AtomicBool>,
    pub kill_switch_audit: KillSwitchAuditStore,
    pub audit_signer: Arc<KillSwitchAuditSigner>,
    pub last_l3_broker: Arc<std::sync::Mutex<Option<String>>>,
    pub today_service: Arc<crate::today::TodayService>,
    /// In-flight WorkOS device grant (`device_code` never leaves this process).
    pub device_login_pending: Arc<std::sync::Mutex<Option<crate::DeviceLoginPending>>>,
    /// Station Caller tokens (Keychain in prod; memory in tests when injected).
    pub station_token_store: Arc<dyn crate::StationTokenStore>,
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
        .route("/api/daemon/today", get(today::handler))
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
            patch(capture::pending_patch_handler),
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
        .route_layer(middleware::from_fn_with_state(
            state_for_layer,
            wire::verify_middleware,
        ));

    Router::new()
        .route("/instruments/search", get(instruments::search_instruments))
        .route("/instruments/ltp", get(instruments::get_ltp))
        .merge(protected)
        .with_state(state)
}

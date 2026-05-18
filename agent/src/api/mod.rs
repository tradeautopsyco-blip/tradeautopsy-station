use crate::{
    broker_sync::{BrokerRuntimeState, BrokerSyncConfig},
    event_bus::EventBus,
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
use std::sync::Arc;

mod bar;
mod broker_sync_state;
mod capture;
mod health;
mod outbox_status;
mod phase8;
mod recent_trades;
mod sse;

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
    pub broker_status: Arc<std::sync::Mutex<BrokerRuntimeState>>,
    pub broker_limits: BrokerSyncConfig,
}

pub fn router(state: AppState) -> Router {
    let state_for_layer = state.clone();
    Router::new()
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
        .route(
            "/api/daemon/bar/live-state",
            get(bar::live_state_handler),
        )
        .route("/api/daemon/bar/declare", post(bar::declare_handler))
        .route("/api/daemon/bar/stop-me", post(bar::stop_me_handler))
        .route(
            "/api/daemon/bar/protective",
            post(bar::protective_handler),
        )
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
            "/api/daemon/kill-switch/ack",
            post(phase8::kill_switch_ack_handler),
        )
        .route("/api/daemon/auth/begin", post(phase8::auth_begin_handler))
        .route("/api/daemon/auth/finish", post(phase8::auth_finish_handler))
        .route_layer(middleware::from_fn_with_state(
            state_for_layer,
            wire::verify_middleware,
        ))
        .with_state(state)
}

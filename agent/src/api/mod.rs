use crate::{
    broker_sync::{BrokerRuntimeState, BrokerSyncConfig},
    broker_sync_control::BrokerSyncController,
    data::{
        AccountBook, BrokerConnectionRuntime, DepthBook, HistoryBook, MarketBind, Registry,
        SourceManifest, TickBook,
    },
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
    routing::{get, patch, post, put},
    Router,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod bar;
mod journal_n2;
mod morning_brief;
mod broker_credentials;
mod broker_sync;
mod broker_sync_state;
mod capture;
pub mod daemon_commands;
pub(crate) mod desk;
mod quote_selection;
pub(crate) use quote_selection::QuoteSelections;
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
mod dhan_session;
mod groww_session;
mod fyers_session;
mod upstox_session;
mod zerodha_session;
mod manifest;
mod outbox_status;
mod phase8;
mod positions;
mod quote;
mod recent_trades;
mod sse;
mod station_auth;
mod sync_hint;
mod today;
mod vendor_bindings;

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
    pub account_book: Arc<Mutex<AccountBook>>,
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
    pub journal_n2: crate::journal_n2::JournalN2Store,
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
    /// Forming bars seeded from HistoryBook; ticked from `@trade` / cash ltq.
    pub candle_builders: Arc<Mutex<crate::data::CandleBuilders>>,
    pub quote_freshness: Duration,
    pub s1_desk_symbol: Option<String>,
    /// Dev default for a Binance options ticker. `None` until that desk is subscribed.
    pub s1_options_symbol: Option<String>,
    /// First-party S0 manifests. Fail-closed at boot.
    pub source_manifests: Arc<Vec<SourceManifest>>,
    /// Persistent connected-broker runtimes. Handles only — never raw secrets.
    pub broker_connections: Arc<Mutex<HashMap<String, BrokerConnectionRuntime>>>,
    /// Local COM instrument master from unsigned `GET /api/v3/exchangeInfo`.
    pub instrument_master: Arc<Mutex<ExchangeInfoSymbolCache>>,
    /// Kotak cash scrip master. Separate from `instrument_master` (do not blend).
    pub kotak_scrip_master: Arc<Mutex<KotakScripMaster>>,
    /// Named NFO scrip master. Separate from cash; do not blend FO rows.
    pub kotak_nfo_scrip_master: Arc<Mutex<crate::kotak_nfo_scrip::KotakNfoScripMaster>>,
    pub kotak_cds_scrip_master: Arc<Mutex<crate::kotak_nfo_scrip::KotakNfoScripMaster>>,
    pub kotak_mcx_scrip_master: Arc<Mutex<crate::kotak_nfo_scrip::KotakNfoScripMaster>>,
    /// `optionSymbols` from eapi exchangeInfo. Separate from spot `instrument_master`.
    pub options_option_symbols: Arc<Mutex<Vec<crate::data::OptionsSymbolRow>>>,
    /// USDM `GET /fapi/v1/exchangeInfo` filters. Third identity — never spot `instrument_master`.
    pub usdm_exchange_info: Arc<Mutex<crate::data::UsdmExchangeInfoCache>>,
    /// Coin-M `GET /dapi/v1/exchangeInfo` filters. Fourth identity — never USDM, never spot.
    pub coinm_exchange_info: Arc<Mutex<crate::data::CoinmExchangeInfoCache>>,
    /// Planted OI rows for CI. Live fetch fills this path when `eapi_public_fetch`.
    pub options_oi_rows: Arc<Mutex<Vec<crate::data::OptionsOiRow>>>,
    /// NFO open interest, keyed `nse_fo|{token}`, read off the same
    /// `quote_type=all` body that feeds TickBook last. Never master `dOpenInterest `.
    pub nfo_open_interest: crate::kotak_rest_quotes::NfoOpenInterestSlot,
    /// NFO session OI band from `quote_type=oi`, keyed `nse_fo|{token}`.
    /// Supplementary to `nfo_open_interest` — never overwrites `open_int`.
    pub nfo_oi_session: crate::kotak_rest_quotes::NfoOiSessionSlot,
    /// One cached `/eapi/v1/mark` row, keyed by its own mixed-case symbol. Not a vec:
    /// mark is per contract, and a different contract is a miss, never a repaint.
    pub options_mark: Arc<Mutex<Option<crate::data::CachedMark>>>,
    /// One cached `/eapi/v1/index` row, keyed by catalog `underlying` (`BTCUSDT`).
    /// Empty / fail stays unavailable — never S=0, never spot last.
    pub options_index: Arc<Mutex<Option<crate::data::CachedIndex>>>,
    /// Prod dials eapi for last/chain/OI/mark/depth/klines/index. Tests stay fixture-only.
    pub eapi_public_fetch: bool,
    /// Options quote keys `{book}\0{symbol}`. Spot trade is `com_trade`, not this set.
    pub quote_streams: Arc<Mutex<HashSet<String>>>,
    /// One bound COM `@trade` id. Loops park on `None`.
    pub com_trade: MarketBind,
    /// One bound COM `@depth` id. Loops park on `None`.
    pub com_depth: MarketBind,
    /// One bound COM klines id (one-shot REST, not a WS loop).
    pub com_klines: MarketBind,
    /// Instrument+interval keys already kicked for public klines → HistoryBook.
    pub klines_inflight: Arc<Mutex<HashSet<String>>>,
    /// Start-time env + connection_id for Kotak PrivateRead quotes (handles only).
    pub kotak_session_locator: crate::kotak_rest_quotes::SessionLocator,
    /// In-flight Kotak REST quote GETs (not book.subscribe — REST must stay open).
    pub kotak_quote_inflight: Arc<Mutex<HashSet<String>>>,
    /// In-flight COM `ticker/price` GETs, so two loopback quotes on one id share
    /// a call. Separate from `klines_inflight` (that keys on instrument+interval).
    pub com_ticker_inflight: Arc<Mutex<HashSet<String>>>,
    /// In-flight Kotak REST depth GETs (`quote_type=depth` only).
    pub kotak_depth_inflight: Arc<Mutex<HashSet<String>>>,
    /// Shared catalog fetch status (not an ObtainStatus variant).
    pub instrument_master_status: Arc<Mutex<crate::data::InstrumentMasterStatus>>,
    /// Stop / re-Start cancels in-flight catalog retries.
    pub instrument_master_cancel: Arc<AtomicBool>,
    pub instrument_master_cache_dir: std::path::PathBuf,
    /// Per-book selected instruments. Quote obtain reads `selected_quote_for(book_id)`.
    pub quote_selections: Arc<Mutex<QuoteSelections>>,
    /// Per-instrument quote fetch class (`quotes_http` / `session` / `quotes_unusable`) — never URLs or bodies.
    pub quote_fetch_error: Arc<Mutex<HashMap<String, String>>>,
    /// Test seam: wiremock base for spot USER_DATA. Prod is always `None`.
    pub binance_spot_base_url: Option<String>,
    /// Test seam: wiremock base for USDM USER_DATA. Prod is always `None`.
    pub binance_usdm_base_url: Option<String>,
    /// Test seam: wiremock base for Coin-M USER_DATA. Prod is always `None`.
    pub binance_coinm_base_url: Option<String>,
    /// Test seam: wiremock base for options eapi USER_DATA. Prod is always `None`.
    pub binance_eapi_base_url: Option<String>,
    /// Per-book lossy force-order observation. Never TickBook, never Kill/PnL.
    pub force_order_book: Arc<Mutex<crate::data::ForceOrderBook>>,
    /// USDM realized from income `REALIZED_PNL`. Not spot WAC. Not force-order.
    /// Written by `ensure_usdm_realized_income`; not an obtain operation this slice.
    pub usdm_realized: Arc<Mutex<Option<crate::UsdmRealizedSlot>>>,
    /// Coin-M income REALIZED_PNL slot — parallel identity to `usdm_realized`.
    pub coinm_realized: Arc<Mutex<Option<crate::coinm_realized_pnl::CoinmRealizedSlot>>>,
    /// M1 cited-PnL share-up (A8 Bearer).
    pub fact_outbox: Arc<crate::fact_outbox::FactOutbox>,
    /// Test seam: wiremock base for Kotak private reads. Prod is always `None`.
    pub kotak_private_base_url: Option<String>,
    /// Test seam: wiremock base for AMFI NAV GET. Prod is always `None` (official host).
    pub amfi_nav_base_url: Option<String>,
    /// Test seam: override AMFI fence host. Prod is always `None`.
    pub amfi_nav_host: Option<String>,
    /// S7 declared-gap fixture. Live quota is shared and decremented on vendor history success.
    pub gap_vendor: Arc<Mutex<crate::data::GapVendorConfig>>,
    /// AMFI labs vendor. Public file — Enable still gates Health + obtain.
    pub amfi_enabled: Arc<Mutex<bool>>,
}

pub fn router(state: AppState) -> Router {
    let state_for_layer = state.clone();
    let protected = Router::new()
        .route("/api/daemon/health", get(health::handler))
        .route(
            "/api/daemon/vendor-bindings",
            put(vendor_bindings::put_handler),
        )
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
            "/api/daemon/broker/zerodha/connect/begin",
            post(zerodha_session::connect_begin_handler),
        )
        .route(
            "/api/daemon/broker/upstox/begin",
            post(upstox_session::connect_begin_handler),
        )
        .route(
            "/api/daemon/broker/fyers/begin",
            post(fyers_session::connect_begin_handler),
        )
        .route(
            "/api/daemon/broker/dhan/connect/begin",
            post(dhan_session::connect_begin_handler),
        )
        .route(
            "/api/daemon/broker/dhan/callback",
            get(dhan_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/groww/connect",
            post(groww_session::connect_handler),
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
        .route("/api/daemon/morning-brief", get(morning_brief::handler))
        .route(
            "/api/daemon/journal/condition-fire",
            post(journal_n2::condition_fire_handler),
        )
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
        .route(
            "/api/daemon/bar/declarations",
            get(bar::declarations_list_handler),
        )
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
            "/api/daemon/bar/test/fill-matched",
            post(bar::test_fill_matched_handler),
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
        .route(
            "/api/daemon/broker/zerodha/callback",
            get(zerodha_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/upstox/callback",
            get(upstox_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/fyers/callback",
            get(fyers_session::callback_handler),
        )
        .route("/api/station/quote", get(quote::handler))
        .route("/api/station/history", get(history::handler))
        .route("/api/station/chain", get(glance::chain_handler))
        .route("/api/station/oi", get(glance::oi_handler))
        .route("/api/station/depth", get(glance::depth_handler))
        .route("/api/station/greeks", get(glance::greeks_handler))
        .route("/api/station/index", get(glance::index_handler))
        .route("/api/station/manifest", get(manifest::manifest_handler))
        .route("/api/station/obtain", get(manifest::obtain_handler))
        .route("/api/station/sync-hint", get(sync_hint::handler))
        .merge(protected)
        .with_state(state)
}

/// Browser OAuth redirects only — served on loopback HTTPS (port 9140 by default).
pub fn oauth_callback_router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/daemon/broker/zerodha/callback",
            get(zerodha_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/dhan/callback",
            get(dhan_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/upstox/callback",
            get(upstox_session::callback_handler),
        )
        .route(
            "/api/daemon/broker/fyers/callback",
            get(fyers_session::callback_handler),
        )
        .with_state(state)
}

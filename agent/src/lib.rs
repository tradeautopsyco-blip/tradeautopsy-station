mod api;
mod bar_fill_ingress;
mod binance_com_coinm_client;
/// Native Binance.com spot modules — **reference only** (ADR 0001 / B5).
/// Live Start uses `build_runtime_adapter` → `WasmBrokerAdapter`, never these.
mod binance_com_options_client;
mod binance_com_spot_adapter;
mod binance_com_spot_client;
mod binance_com_usdm_client;
mod binance_com_validation;
mod broker;
mod broker_behavioral;
mod broker_data_class;
mod broker_redaction;
mod broker_sync;
mod broker_sync_control;
mod broker_validation;
mod coinm_realized_pnl;
mod data;
mod device_login;
mod dns_block;
mod egress;
mod event_bus;
mod exchange_info;
mod fact_outbox;
mod instruments;
mod inr_cash_wac;
mod kill_policy;
mod kill_switch_audit;
mod kotak_nfo_scrip;
mod kotak_rest_history;
mod kotak_rest_quotes;
mod kotak_scrip_master;
mod live_book;
mod metrics;
mod outbox;
mod recent_trades;
mod resolve_kill_switch_broker;
mod round_trip_engine;
mod share_cited_pnl;
mod sse_signing;
mod station_tokens;
mod today;
mod ubi;
mod usdm_realized_pnl;
mod wire;

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
pub use broker_behavioral::{BrokerBehavioralRecorder, BrokerConnectionIdentityFields};
pub use broker_data_class::{
    BrokerBalancesSnapshot, BrokerDataClass, BrokerDataClassCompleteness, BrokerHolding,
    BrokerOpenOrder, BrokerOpenOrdersSnapshot, DataClassFreshness,
};
pub use broker_redaction::RedactionBoundary;
pub use broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
pub use broker_sync_control::{
    build_runtime_adapter, build_wasm_runtime_adapter, build_wasm_runtime_adapter_for_book,
    default_credential_vault, memory_credential_vault, uses_wasm_component,
    BrokerRuntimeCardStatus, BrokerSyncController, BrokerSyncStartRequest,
};
pub use broker_validation::{
    BrokerValidationAdapter, FakeBinanceUSValidationAdapter, LiveBinanceUSValidationAdapter,
    PermissionPosture, ValidationFailure, ValidationResult,
};
pub use data::{
    apply_quote, authorize_book_call, authorize_book_fence, authorize_host_call,
    authorize_inferred_call, binance_com_quote_descriptor, capital_may_light, extract_chain,
    extract_contracts, extract_contracts_from_rows, extract_depth, extract_greeks, extract_history,
    extract_index, extract_licensed_history, extract_margin_estimate, extract_open_interest,
    extract_quote, extract_quote_for, extract_quote_for_book, extract_resample,
    fixture_quote_descriptor, infer_capability, inherit, is_kotak_fo_scrip_csv_path, is_mutation,
    kotak_cash_limits_jdata_body, kotak_neo_nfo_manifest, kotak_neo_quote_descriptor,
    kotak_neo_s1k_manifest, kotak_nfo_limits_jdata_body, normalize_quote_instrument, obtain,
    quote_tick_from_binance_json, quote_tick_from_kotak_json, quote_tick_from_options_ticker_json,
    resolve_desk_instrument, ApplyError, AuthMode, ContractRow, DepthBook, DepthEnvelope,
    DepthStatus, GlanceEnvelope, GlanceStatus, HistoryBook, HistoryEnvelope, HistoryStatus,
    HonestyStatus, HostRefuse, InputHonesty, InstrumentMasterPhase, InstrumentMasterStatus,
    ObtainEnvelope, ObtainStatus, Physics, ProvenanceLine, QuoteEnvelope, QuoteStatus, QuoteTick,
    Registry, TickBook, Transport, BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID,
    BINANCE_COM_SPOT_BOOK_ID, KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID, R0_ALLOWED_HOSTS,
};
pub use device_login::{
    begin_device_login, complete_device_login, prove_station_session, DeviceLoginPending,
    DeviceLoginPublic, StationSessionIdentity,
};
pub use dns_block::{arm_venue_ban, clear_venue_ban, hosts_for_broker, BlockReason, BLOCK_MARKER};

/// Test seams for the multi-owner hosts file. Integration tests drive the
/// kill-switch side of the registry through these; production code calls the
/// kill switch's own paths.
pub fn dns_apply_hosts_block_for_tests(broker: &str) -> Result<(), String> {
    dns_block::apply_hosts_block(broker)
}

pub fn dns_disable_block_for_tests() -> Result<(), String> {
    dns_block::disable_block()
}

pub fn dns_is_block_active_for_tests() -> bool {
    dns_block::is_block_active()
}
pub use egress::{
    Decision as EgressDecision, EgressCall, EgressError, EgressRequest, EgressResponse,
    EgressTransport, Lane, Outcome as EgressOutcome, RefuseKind, RefuseReason, VenueEgress,
    VenuePosture,
};
pub use event_bus::{AgentEvent, EventBus};
pub use exchange_info::{
    is_usd_pegged_stablecoin, is_usd_quoted_symbol, live_com_filters_ready, resolve_symbol_assets,
    ExchangeInfoSymbolCache, SymbolAssets, SymbolFilters,
};
pub use fact_outbox::{EnqueueOutcome, Fact, FactOutbox, FactRow};
pub use instruments::{zerodha_instruments_enabled, InstrumentStore};
pub use kill_policy::{KillPolicy, KillPolicyStore};
pub use kill_switch_audit::{
    canonical_audit_message, verify_audit_signature, KillSwitchAuditAppend, KillSwitchAuditRecord,
    KillSwitchAuditSigner, KillSwitchAuditStore,
};
pub use metrics::AgentMetrics;
pub use outbox::{
    queued_response_json, CaptureOutbox, DeadLetterStatusItem, OutboxConfig, OutboxCounts,
    OutboxStatusSnapshot, ProcessNowResult,
};
pub use recent_trades::RecentTradesStore;
pub use resolve_kill_switch_broker::resolve_kill_switch_broker;
pub use inr_cash_wac::{
    aggregate_known_pnl_inr, is_aggregate_eligible as is_inr_cash_aggregate_eligible,
    is_inr_cash_fill, InrCashReconstructResult, InrCashRoundTrip, InrCashWacEngine, BOOK_ID as INR_CASH_BOOK_ID,
    CALC_PROFILE_ID as INR_CASH_CALC_PROFILE_ID, OWNER_PATH as INR_CASH_WAC_OWNER_PATH,
};
pub use round_trip_engine::{
    aggregate_known_pnl, is_aggregate_eligible, FillTimeFeePriceLookup, PairAssetFeeLookup,
    ReconstructResult, RoundTrip, RoundTripEngine, StablecoinAndBaseAssetFeeLookup, UnhandledFee,
};
pub use sse_signing::{verify_sse_event_signature, SseSigner, SseSigningPubKey};
pub use station_tokens::{
    bearer_authorization, KeyringStationTokenStore, MemoryStationTokenStore, StationTokenStore,
    StationTokens,
};
pub use today::{
    open_inventory_from_fills, OpenInventoryRow, TodayDegradedReason, TodayHeroPayload,
    TodayPayload, TodayService, TodayStore,
};
pub use ubi::{
    calc_profile, catalog_v1, classify_response, compliance_profile, component_candidate_paths,
    component_crate_dir, component_file_name, component_path_for_slug, decode_credential_blob,
    descriptor_for_slug, effective_host, fill_event_to_broker_fill, host_allowed,
    keychain_service_for, prepare_kotak_file_paths_get, prepare_request, prepare_unsigned_request,
    redact_response_headers, run_describe, run_fetch_fills, run_obtain, AdapterOrigin, AuthScheme,
    BrokerAvailability, BrokerCredentialVault, BrokerDescriptor, BrokerHttpFixture, BrokerHttpMode,
    BrokerHttpTransport, CalcProfile, ComplianceProfile, CredentialBlob, FillCursor,
    FillEvent as UbiFillEvent, HostCredentialBlob, KeyringBrokerCredentialVault,
    MemoryBrokerCredentialVault, PreparedHttpRequest, RecordingTransport,
    ReqwestBrokerHttpTransport, TransportResponse, UbiHostConfig, UbiHostError, UbiHostState,
    WasmBrokerAdapter, ALLOWED_BROKER_HOSTS, BROKER_CREDENTIAL_KEYCHAIN_SERVICE, COMPONENT_DIR_ENV,
    FORBIDDEN_COMPONENT_HEADERS, KOTAK_SESSION_KEYCHAIN_SERVICE, RESPONSE_HEADER_ALLOWLIST,
};
pub use usdm_realized_pnl::{
    income_realized_from_json, realized_pnl_usd, usdm_income_call, UsdmIncomeCall, UsdmIncomeRow,
    UsdmRealizedSlot, OWNER_PATH as USDM_REALIZED_PNL_OWNER_PATH,
};
pub use wire::{WireVerifier, WIRE_PROTO_VERSION};

pub use api::daemon_commands::{parse_daemon_command_type, DaemonCommandKind};
pub use api::{
    effective_level, plan_l3_dns, resolve_clear_fog, resolve_kill_apply, KillApplyDecision,
    KillClearDecision,
};

use chrono::Utc;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
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
        // Loopback bootstrap first so tests / wiremock never block on Keychain.
        // Still gated to loopback-http + STATION_ACCESS_TOKEN (T1) — never a
        // production identity rail against a real https Console.
        if self.config.is_loopback_http_bootstrap() {
            if let Ok(token) = std::env::var("STATION_ACCESS_TOKEN") {
                if !token.trim().is_empty() {
                    return Ok(format!("Bearer {}", token.trim()));
                }
            }
        }
        if let Some(tokens) = KeyringStationTokenStore.load()? {
            return Ok(bearer_authorization(&tokens));
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
    /// Licensed Binance HistoryBook coverage (`AGENT_HISTORY_DB_PATH`). Not TickBook.
    pub history_db_path: PathBuf,
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
    /// True when [`station_token_store`] was set by the caller (not the test harness default).
    /// An explicit empty store blocks loopback `STATION_ACCESS_TOKEN` bootstrap.
    pub station_token_store_explicit: bool,
    /// Optional planted LiveBook snapshot (tests). Prod starts empty until snapshot-once.
    pub live_book_snapshot: Option<serde_json::Value>,
    /// Sibling SQLite for typed facts — never the capture outbox.
    pub fact_outbox_db_path: PathBuf,
    /// Online cadence while JWT is valid. Prod **20_000** (PRD 5.1).
    pub fact_online_interval_ms: u64,
    /// Injectable clock for tests (unix ms). `None` uses wall clock.
    pub fact_clock_ms: Option<Arc<AtomicI64>>,
    /// Boot seed written into [`KillPolicyStore`]. Apply always loads the store.
    pub kill_policy: KillPolicy,
    /// S1 desk: public Binance last-price stream (`AGENT_S1_DESK_SYMBOL`). `None` = no WS.
    pub s1_desk_symbol: Option<String>,
    /// Optional options contract (`AGENT_S1_OPTIONS_SYMBOL`). Unset = no default dial.
    pub s1_options_symbol: Option<String>,
    /// Quote extract freshness window (`AGENT_S1_FRESHNESS_MS`, default 2000).
    pub quote_freshness: Duration,
    /// Slice F CI: plant cash CSV + quote JSON into TickBook / scrip master. No live session.
    pub plant_kotak_s1k_fixtures: bool,
    /// CI: plant NFO LTP JSON into TickBook `{kotak-nse-nfo}`. Cash-only tests stay cash-only.
    pub plant_kotak_nfo_quote: bool,
    /// CI: plant lock-header FO CSV into the named NFO store. Not cash KotakScripMaster.
    pub plant_kotak_nfo_contracts: bool,
    /// NFO order book CI: plant a bounded snapshot into DepthBook `{kotak-nse-nfo}`.
    /// Live capture may carry zero levels when the market is shut; CI uses
    /// representative non-zero levels with the observed key shape.
    pub plant_kotak_nfo_depth: bool,
    /// NFO session OI CI: plant `quote_type=oi` slice into `nfo_oi_session`.
    /// Supplementary to `open_int` on the quote fixture — never overwrites it.
    pub plant_kotak_nfo_oi_session: bool,
    /// S2 CI: plant committed klines JSON into HistoryBook. No live Binance.
    pub plant_binance_s2_history: bool,
    /// Spot funds CI: plant AccountBook funds slot. No live Binance private GET.
    pub plant_binance_spot_funds: bool,
    /// Options last CI: plant committed eapi ticker JSON into TickBook. No live eapi.
    pub plant_binance_options_quote: bool,
    /// USDM last CI: plant committed fapi ticker JSON into TickBook `binance-com-usdm`. No live fapi.
    pub plant_binance_usdm_quote: bool,
    /// USDM listing CI: plant committed fapi exchangeInfo JSON. No live fapi.
    pub plant_binance_usdm_exchange_info: bool,
    /// Coin-M last CI: plant committed dapi ticker JSON into TickBook `binance-com-coinm`. No live dapi.
    pub plant_binance_coinm_quote: bool,
    /// Coin-M listing CI: plant committed dapi exchangeInfo JSON. No live dapi.
    pub plant_binance_coinm_exchange_info: bool,
    /// Options chain/OI CI: plant committed exchangeInfo + OI JSON. No live eapi.
    pub plant_binance_options_chain: bool,
    /// Venue-published greeks CI: plant the committed `/eapi/v1/mark` JSON. No live eapi.
    pub plant_binance_options_mark: bool,
    /// Same, but the observed no-bid row (`"bidIV":"-1.0"`). Its own flag because a
    /// `CachedMark` holds exactly one contract — the two rows cannot share a plant.
    pub plant_binance_options_mark_no_bid: bool,
    /// Options order book CI: plant the committed `/eapi/v1/depth` JSON into the
    /// DepthBook's `binance-com-options` slot. No live eapi.
    pub plant_binance_options_depth: bool,
    /// Options session series CI: plant committed eapi klines JSON into HistoryBook.
    /// No live eapi. Mixed-case dated contract — never spot `/api/v3/klines`.
    pub plant_binance_options_history: bool,
    /// Options tradebook CI: plant the committed `/eapi/v1/userTrades` JSON into
    /// AccountBook `binance-com-options`. No live eapi private GET.
    pub plant_binance_options_fills: bool,
    /// Options index S CI: plant committed `/eapi/v1/index` JSON. No live eapi.
    pub plant_binance_options_index: bool,
    /// COM depth gap CI: stamp spot DepthBook Unusable (no live WS).
    pub plant_binance_spot_depth_unusable: bool,
    /// Prod may GET eapi ticker / exchangeInfo / openInterest / mark / depth / klines / index.
    /// Tests stay false.
    pub eapi_public_fetch: bool,
    /// Test seam: non-venue base URL for spot private reads (wiremock). `None` = `api.binance.com`.
    pub binance_spot_base_url: Option<String>,
    /// Test seam: non-venue base URL for USDM private reads (wiremock). `None` = `fapi.binance.com`.
    pub binance_usdm_base_url: Option<String>,
    /// Test seam: non-venue base URL for Coin-M private reads (wiremock). `None` = `dapi.binance.com`.
    pub binance_coinm_base_url: Option<String>,
    /// Test seam: non-venue base URL for options eapi private reads (wiremock). `None` = `eapi.binance.com`.
    pub binance_eapi_base_url: Option<String>,
    /// Test seam: non-venue base URL for Kotak private reads (wiremock). `None` = session `baseUrl`.
    pub kotak_private_base_url: Option<String>,
    /// Test seam: non-venue base URL for AMFI NAV (wiremock). `None` = `www.amfiindia.com`.
    pub amfi_nav_base_url: Option<String>,
    /// Test seam: override the AMFI host fence. `None` = `www.amfiindia.com`.
    pub amfi_nav_host: Option<String>,
    /// S7 CI: enable fixture `licensed_history` as a declared Kotak history gap.
    pub gap_vendor_enabled: bool,
    pub gap_vendor_key: Option<String>,
    pub gap_vendor_history_budget: u32,
    pub kotak_quote_budget: u32,
    pub plant_licensed_history_gap: bool,
    /// AMFI labs vendor starts enabled (public file). PUT can dark it.
    pub amfi_enabled: bool,
    /// Disk cache for exchangeInfo JSON / Kotak cash CSVs (`AGENT_INSTRUMENT_MASTER_CACHE_DIR`).
    pub instrument_master_cache_dir: PathBuf,
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
        let history_db_path = std::env::var("AGENT_HISTORY_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-history.db");
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
        let fact_outbox_db_path = std::env::var("AGENT_FACT_OUTBOX_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = std::env::temp_dir();
                p.push("tradeautopsy-agent-fact-outbox.db");
                p
            });
        Ok(Self {
            port,
            daemon_secret,
            heartbeat_ms,
            metrics_port,
            upstream,
            outbox,
            recent_trades_db_path,
            history_db_path,
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
            station_token_store_explicit: false,
            live_book_snapshot: None,
            fact_outbox_db_path,
            fact_online_interval_ms: std::env::var("AGENT_FACT_ONLINE_INTERVAL_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20_000),
            fact_clock_ms: None,
            kill_policy: KillPolicy::default(),
            s1_desk_symbol: std::env::var("AGENT_S1_DESK_SYMBOL")
                .ok()
                .map(|s| crate::data::normalize_quote_instrument(&s))
                .filter(|s| !s.is_empty()),
            s1_options_symbol: std::env::var("AGENT_S1_OPTIONS_SYMBOL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            quote_freshness: std::env::var("AGENT_S1_FRESHNESS_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Duration::from_millis)
                .unwrap_or_else(|| Duration::from_millis(2000)),
            plant_kotak_s1k_fixtures: false,
            plant_kotak_nfo_quote: false,
            plant_kotak_nfo_contracts: false,
            plant_kotak_nfo_depth: false,
            plant_kotak_nfo_oi_session: false,
            plant_binance_s2_history: false,
            plant_binance_spot_funds: false,
            plant_binance_options_quote: false,
            plant_binance_usdm_quote: false,
            plant_binance_usdm_exchange_info: false,
            plant_binance_coinm_quote: false,
            plant_binance_coinm_exchange_info: false,
            plant_binance_options_chain: false,
            plant_binance_options_mark: false,
            plant_binance_options_mark_no_bid: false,
            plant_binance_options_depth: false,
            plant_binance_options_history: false,
            plant_binance_options_fills: false,
            plant_binance_options_index: false,
            plant_binance_spot_depth_unusable: false,
            eapi_public_fetch: true,
            binance_spot_base_url: None,
            binance_usdm_base_url: None,
            binance_coinm_base_url: None,
            binance_eapi_base_url: None,
            kotak_private_base_url: None,
            amfi_nav_base_url: None,
            amfi_nav_host: None,
            gap_vendor_enabled: false,
            gap_vendor_key: None,
            gap_vendor_history_budget: 0,
            kotak_quote_budget: 1_000,
            plant_licensed_history_gap: false,
            amfi_enabled: true,
            instrument_master_cache_dir: instrument_master_cache_dir_from_env(),
        })
    }

    /// Deterministic local config for integration tests (issue #57 / #58 wire harness).
    pub fn test_on_port(port: u16, daemon_secret: impl Into<String>) -> Self {
        let daemon_secret = daemon_secret.into();
        let mut recent_trades_db_path = std::env::temp_dir();
        recent_trades_db_path.push(format!("rta-recent-{port}.db"));
        let mut history_db_path = std::env::temp_dir();
        history_db_path.push(format!("rta-history-{port}.db"));
        let mut today_db_path = std::env::temp_dir();
        today_db_path.push(format!("rta-today-{port}.db"));
        let mut instruments_db_path = std::env::temp_dir();
        instruments_db_path.push(format!("rta-instruments-{port}.db"));
        let mut kill_switch_audit_db_path = std::env::temp_dir();
        kill_switch_audit_db_path.push(format!("rta-kill-switch-audit-{port}.db"));
        let mut fact_outbox_db_path = std::env::temp_dir();
        fact_outbox_db_path.push(format!("rta-fact-outbox-{port}.db"));
        let mut instrument_master_cache_dir = std::env::temp_dir();
        instrument_master_cache_dir.push(format!("rta-instrument-master-{port}"));
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
            history_db_path,
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
            station_token_store_explicit: false,
            live_book_snapshot: None,
            fact_outbox_db_path,
            fact_online_interval_ms: 20_000,
            fact_clock_ms: None,
            kill_policy: KillPolicy::default(),
            s1_desk_symbol: None,
            s1_options_symbol: None,
            quote_freshness: Duration::from_millis(2000),
            plant_kotak_s1k_fixtures: false,
            plant_kotak_nfo_quote: false,
            plant_kotak_nfo_contracts: false,
            plant_kotak_nfo_depth: false,
            plant_kotak_nfo_oi_session: false,
            plant_binance_s2_history: false,
            plant_binance_spot_funds: false,
            plant_binance_options_quote: false,
            plant_binance_usdm_quote: false,
            plant_binance_usdm_exchange_info: false,
            plant_binance_coinm_quote: false,
            plant_binance_coinm_exchange_info: false,
            plant_binance_options_chain: false,
            plant_binance_options_mark: false,
            plant_binance_options_mark_no_bid: false,
            plant_binance_options_depth: false,
            plant_binance_options_history: false,
            plant_binance_options_fills: false,
            plant_binance_options_index: false,
            plant_binance_spot_depth_unusable: false,
            eapi_public_fetch: false,
            binance_spot_base_url: None,
            binance_usdm_base_url: None,
            binance_coinm_base_url: None,
            binance_eapi_base_url: None,
            kotak_private_base_url: None,
            amfi_nav_base_url: None,
            amfi_nav_host: None,
            gap_vendor_enabled: false,
            gap_vendor_key: None,
            gap_vendor_history_budget: 0,
            kotak_quote_budget: 1_000,
            plant_licensed_history_gap: false,
            amfi_enabled: true,
            instrument_master_cache_dir,
        }
    }
}

fn instrument_master_cache_dir_from_env() -> PathBuf {
    if let Some(dir) = std::env::var_os("AGENT_INSTRUMENT_MASTER_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = PathBuf::from(home);
        p.push("Library/Application Support/tradeautopsy/instrument-master");
        return p;
    }
    let mut p = std::env::temp_dir();
    p.push("tradeautopsy-instrument-master");
    p
}

#[cfg(test)]
mod options_depth_plant_tests {
    use super::*;

    /// The committed `/eapi/v1/depth` fixture must land in the **options** slot,
    /// mixed-case, with its `lastUpdateId` and no invented order count.
    #[test]
    fn planting_the_depth_fixture_lands_in_the_options_slot_only() {
        let book: Arc<Mutex<crate::data::DepthBook>> =
            Arc::new(Mutex::new(crate::data::DepthBook::new()));
        plant_binance_options_depth(&book);
        let guard = book.lock().expect("depthbook mutex poisoned");
        let row = guard
            .get("binance-com-options", "BTC-200730-9000-C")
            .expect("the fixture must plant one options ladder");
        assert_eq!(row.book_id, "binance-com-options");
        assert_eq!(row.adapter_id, "binance_com");
        assert_eq!(row.sequence, Some(361));
        assert_eq!(row.bids[0].price, "1000.000");
        assert!(row.bids.iter().all(|level| level.orders.is_none()));
        // Not reachable as spot depth, by either id.
        assert!(guard.get("binance-com-spot", "BTC-200730-9000-C").is_none());
        assert!(guard.get("binance-com-spot", "btcusdt").is_none());
    }
}

#[cfg(test)]
mod options_mark_plant_tests {
    use super::*;

    /// The committed `/eapi/v1/mark` fixture must survive the plant verbatim —
    /// same mixed-case symbol, same published delta string, no reparse.
    #[test]
    fn planting_the_mark_fixture_round_trips_the_published_delta() {
        let store: Arc<Mutex<Option<crate::data::CachedMark>>> = Arc::new(Mutex::new(None));
        plant_binance_options_mark(&store);
        let cached = store
            .lock()
            .expect("options mark mutex poisoned")
            .clone()
            .expect("the fixture must plant one row");
        assert_eq!(cached.symbol, "BTC-200730-9000-C");
        assert_eq!(cached.row.delta, "0.55937056");
        assert_eq!(cached.row.vega, "978.58874732");
        // Station stamps its own fetch time; the venue publishes none.
        assert!(!cached.as_of.trim().is_empty());
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
    git_sha: String,
    started_at: Instant,
    seq: AtomicU64,
}

impl AgentRuntime {
    fn new() -> Self {
        let git_sha = option_env!("GIT_SHA").unwrap_or("unknown").to_string();
        Self {
            boot_id: ulid::Ulid::new().to_string(),
            build: format!(
                "tradeautopsy-agent/{} ({git_sha})",
                env!("CARGO_PKG_VERSION")
            ),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha,
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

    pub fn git_sha(&self) -> &str {
        &self.git_sha
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
/// Slice F: load committed cash CSV + quote JSON. Does not mint a Kotak session.
fn plant_kotak_s1k_fixtures(
    registry: &crate::data::Registry,
    tickbook: &Arc<Mutex<crate::data::TickBook>>,
    depthbook: &Arc<Mutex<crate::data::DepthBook>>,
    master: &Arc<Mutex<crate::kotak_scrip_master::KotakScripMaster>>,
    broker_status: &Arc<Mutex<crate::broker_sync::BrokerRuntimeState>>,
    instrument_master_status: &Arc<Mutex<crate::data::InstrumentMasterStatus>>,
) {
    let csv = include_str!("../fixtures/kotak/nse_cm_cash.csv");
    match crate::kotak_scrip_master::KotakScripMaster::from_csv_bytes(csv.as_bytes(), None) {
        Ok(loaded) => {
            let n = loaded.len();
            *master.lock().expect("kotak scrip master mutex poisoned") = loaded;
            instrument_master_status
                .lock()
                .expect("instrument master status poisoned")
                .mark_loaded(crate::data::KOTAK_NEO_ADAPTER_ID, n);
        }
        Err(err) => tracing::warn!(error = %err, "s1k fixture: cash CSV plant failed"),
    }
    let json = include_str!("../fixtures/kotak/quotes_neosymbol.json");
    if let Some(tick) = crate::data::quote_tick_from_kotak_json(json, Utc::now()) {
        let mut book = tickbook.lock().expect("tickbook mutex poisoned");
        if let Err(err) = crate::data::apply_quote(registry, &mut book, tick) {
            tracing::warn!(error = %err, "s1k fixture: quote plant refused");
        }
    }
    let depth_json = include_str!("../fixtures/kotak/quotes_neosymbol_depth.json");
    if let Some(snapshot) = crate::data::depth_snapshot_from_kotak_json(depth_json, Utc::now()) {
        depthbook
            .lock()
            .expect("depthbook mutex poisoned")
            .upsert(snapshot);
    }
    // Slug only — not broker_connected. Search uses the Kotak master; no live poll.
    broker_status
        .lock()
        .expect("broker_status mutex poisoned")
        .active_broker_slug = Some(crate::data::KOTAK_NEO_ADAPTER_ID.to_string());
}

/// Headless NFO last: committed quotes JSON into the named book. No live session.
fn plant_kotak_nfo_quote(
    registry: &crate::data::Registry,
    tickbook: &Arc<Mutex<crate::data::TickBook>>,
    nfo_open_interest: &crate::kotak_rest_quotes::NfoOpenInterestSlot,
    broker_status: &Arc<Mutex<crate::broker_sync::BrokerRuntimeState>>,
) {
    let json = include_str!("../fixtures/kotak/quotes_neosymbol_nfo.json");
    if let Some(tick) = crate::data::quote_tick_from_kotak_json_for_book(
        json,
        Utc::now(),
        crate::data::KOTAK_NSE_NFO_BOOK_ID,
    ) {
        let mut book = tickbook.lock().expect("tickbook mutex poisoned");
        if let Err(err) = crate::data::apply_quote(registry, &mut book, tick) {
            tracing::warn!(error = %err, "nfo fixture: quote plant refused");
        }
    }
    // OI rides the same fixture body, exactly as it rides the same live body.
    crate::kotak_rest_quotes::apply_nfo_open_interest_body(nfo_open_interest, json);
    broker_status
        .lock()
        .expect("broker_status mutex poisoned")
        .active_broker_slug = Some(crate::data::KOTAK_NEO_ADAPTER_ID.to_string());
}

/// Headless NFO master: lock-header CSV into the named store. Not cash. No live session.
fn plant_kotak_nfo_contracts(
    master: &Arc<Mutex<crate::kotak_nfo_scrip::KotakNfoScripMaster>>,
    broker_status: &Arc<Mutex<crate::broker_sync::BrokerRuntimeState>>,
) {
    let csv = include_str!("../fixtures/kotak/nse_fo_header.csv");
    match crate::kotak_nfo_scrip::KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()) {
        Ok(loaded) => {
            let _ = crate::kotak_nfo_scrip::install_master_if_nonempty(master, loaded);
        }
        Err(err) => tracing::warn!(error = %err, "nfo fixture: FO CSV plant failed"),
    }
    broker_status
        .lock()
        .expect("broker_status mutex poisoned")
        .active_broker_slug = Some(crate::data::KOTAK_NEO_ADAPTER_ID.to_string());
}

/// Headless options last: committed eapi ticker JSON into `binance-com-options`. No live eapi.
fn plant_binance_options_quote(
    registry: &crate::data::Registry,
    tickbook: &Arc<Mutex<crate::data::TickBook>>,
) {
    let json = include_str!("../fixtures/binance/options_ticker.json");
    if let Some(tick) = crate::data::quote_tick_from_options_ticker_json(json, Utc::now()) {
        let mut book = tickbook.lock().expect("tickbook mutex poisoned");
        if let Err(err) = crate::data::apply_quote(registry, &mut book, tick) {
            tracing::warn!(error = %err, "options fixture: quote plant refused");
        }
    }
}

/// Headless USDM last: committed fapi ticker JSON into `binance-com-usdm`. No live fapi.
fn plant_binance_usdm_quote(
    registry: &crate::data::Registry,
    tickbook: &Arc<Mutex<crate::data::TickBook>>,
) {
    let json = include_str!("../fixtures/binance/usdm_ticker.json");
    if let Some(tick) = crate::data::quote_tick_from_usdm_ticker_json(json, Utc::now()) {
        let mut book = tickbook.lock().expect("tickbook mutex poisoned");
        if let Err(err) = crate::data::apply_quote(registry, &mut book, tick) {
            tracing::warn!(error = %err, "usdm fixture: quote plant refused");
        }
    }
}

fn plant_binance_usdm_exchange_info(
    cache: &Arc<std::sync::Mutex<crate::data::UsdmExchangeInfoCache>>,
) {
    let json = include_str!("../fixtures/binance/usdm_exchange_info.json");
    *cache.lock().expect("usdm exchange info mutex poisoned") =
        crate::data::UsdmExchangeInfoCache::from_exchange_info_json(json);
}

/// Headless Coin-M last: committed dapi ticker JSON into `binance-com-coinm`. No live dapi.
fn plant_binance_coinm_quote(
    registry: &crate::data::Registry,
    tickbook: &Arc<Mutex<crate::data::TickBook>>,
) {
    let json = include_str!("../fixtures/binance/coinm_ticker.json");
    if let Some(tick) = crate::data::quote_tick_from_coinm_ticker_json(json, Utc::now()) {
        let mut book = tickbook.lock().expect("tickbook mutex poisoned");
        if let Err(err) = crate::data::apply_quote(registry, &mut book, tick) {
            tracing::warn!(error = %err, "coinm fixture: quote plant refused");
        }
    }
}

fn plant_binance_coinm_exchange_info(
    cache: &Arc<std::sync::Mutex<crate::data::CoinmExchangeInfoCache>>,
) {
    let json = include_str!("../fixtures/binance/coinm_exchange_info.json");
    *cache.lock().expect("coinm exchange info mutex poisoned") =
        crate::data::CoinmExchangeInfoCache::from_exchange_info_json(json);
}

fn plant_binance_options_chain(
    symbols: &Arc<Mutex<Vec<crate::data::OptionsSymbolRow>>>,
    oi: &Arc<Mutex<Vec<crate::data::OptionsOiRow>>>,
) {
    let info = include_str!("../fixtures/binance/options_exchange_info.json");
    *symbols
        .lock()
        .expect("options option symbols mutex poisoned") =
        crate::data::option_symbols_from_exchange_info_json(info);
    let oi_json = include_str!("../fixtures/binance/options_open_interest.json");
    *oi.lock().expect("options oi mutex poisoned") = crate::data::oi_rows_from_json(oi_json);
}

/// Headless venue-published greeks: the committed `/eapi/v1/mark` fixture into the
/// one-row store. No live eapi. Keyed by the row's own symbol, same as a real fetch.
fn plant_binance_options_mark(mark: &Arc<Mutex<Option<crate::data::CachedMark>>>) {
    plant_options_mark_json(mark, include_str!("../fixtures/binance/options_mark.json"));
}

/// The observed no-bid row (`"bidIV":"-1.0"`). Separate fixture, separate flag: a
/// `CachedMark` holds one contract, so this row cannot ride the official example.
fn plant_binance_options_mark_no_bid(mark: &Arc<Mutex<Option<crate::data::CachedMark>>>) {
    plant_options_mark_json(
        mark,
        include_str!("../fixtures/binance/options_mark_no_bid.json"),
    );
}

/// Headless index S: committed `/eapi/v1/index` fixture keyed by catalog
/// `underlying=BTCUSDT` (the exchangeInfo plant's `optionSymbols.underlying`).
/// No live eapi. Do not freeze a live price as a golden — this string is CI-only.
fn plant_binance_options_index(index: &Arc<Mutex<Option<crate::data::CachedIndex>>>) {
    let json = include_str!("../fixtures/binance/options_index.json");
    let Some(mut row) = crate::data::index_price_from_json_for_underlying(json, "BTCUSDT") else {
        return;
    };
    row.as_of = Utc::now().to_rfc3339();
    *index.lock().expect("options index mutex poisoned") = Some(row);
}

/// Headless NFO depth: bounded snapshot into the named book. The committed
/// live body may carry zero levels when the market is shut; CI plants
/// representative non-zero levels using the observed key shape.
fn plant_kotak_nfo_depth(depthbook: &Arc<Mutex<crate::data::DepthBook>>) {
    let json = r#"[{
        "exchange": "nse_fo",
        "exchange_token": "56526",
        "depth": {
            "buy": [{"price": "10.50", "quantity": "120", "orders": "3"}],
            "sell": [{"price": "11.00", "quantity": "90", "orders": "4"}]
        }
    }]"#;
    let mut book = depthbook.lock().expect("depthbook mutex poisoned");
    if crate::kotak_rest_quotes::apply_kotak_depth_body(&mut book, json, Utc::now()) == 0 {
        tracing::warn!("nfo fixture: depth plant refused, no usable ladder");
    }
}

/// Headless NFO session OI: `quote_type=oi` slice into the named slot.
fn plant_kotak_nfo_oi_session(nfo_oi_session: &crate::kotak_rest_quotes::NfoOiSessionSlot) {
    let json = include_str!("../fixtures/kotak/quotes_neosymbol_nfo_oi.json");
    if crate::kotak_rest_quotes::apply_nfo_oi_session_body(nfo_oi_session, json) == 0 {
        tracing::warn!("nfo fixture: oi session plant refused");
    }
}

/// Headless options order book: the committed `/eapi/v1/depth` fixture into the
/// **options** slot of the DepthBook. No live eapi. The symbol is supplied by
/// Station (the body carries none), and stays mixed-case.
fn plant_binance_options_depth(depthbook: &Arc<Mutex<crate::data::DepthBook>>) {
    let json = include_str!("../fixtures/binance/options_depth.json");
    let Some(snapshot) =
        crate::data::depth_snapshot_from_eapi_json(json, "BTC-200730-9000-C", Utc::now())
    else {
        tracing::warn!("options fixture: depth plant refused, no usable ladder");
        return;
    };
    depthbook
        .lock()
        .expect("depthbook mutex poisoned")
        .upsert(snapshot);
}

/// COM gap path: placeholder Unusable row so glance is unusable, not unavailable.
fn plant_binance_spot_depth_unusable(depthbook: &Arc<Mutex<crate::data::DepthBook>>) {
    depthbook
        .lock()
        .expect("depthbook mutex poisoned")
        .invalidate("btcusdt", crate::data::BINANCE_COM_SPOT_BOOK_ID);
}

fn plant_options_mark_json(mark: &Arc<Mutex<Option<crate::data::CachedMark>>>, json: &str) {
    let Some(row) = crate::data::mark_rows_from_json(json).into_iter().next() else {
        tracing::warn!("options fixture: mark plant refused, no complete row");
        return;
    };
    *mark.lock().expect("options mark mutex poisoned") = Some(crate::data::CachedMark {
        symbol: row.symbol.clone(),
        row,
        as_of: Utc::now().to_rfc3339(),
    });
}

fn plant_licensed_history_gap(
    historybook: &Arc<Mutex<crate::data::HistoryBook>>,
    builders: &Arc<Mutex<crate::data::CandleBuilders>>,
) {
    let series = crate::data::HistorySeries {
        instrument_id: "nse_cm|2885".into(),
        adapter_id: crate::data::LICENSED_HISTORY_ADAPTER_ID.into(),
        interval: crate::data::DEFAULT_HISTORY_INTERVAL.into(),
        candles: vec![crate::data::HistoryCandle {
            open_time_ms: 1_700_000_000_000,
            open: "1400.00".into(),
            high: "1402.00".into(),
            low: "1398.00".into(),
            close: "1401.00".into(),
            volume: "10".into(),
            close_time_ms: 1_700_000_060_000,
        }],
        transport: crate::data::Transport::Fixture,
    };
    crate::data::apply_history_series_and_seed(
        &mut historybook.lock().expect("historybook mutex poisoned"),
        &mut builders.lock().expect("candle builders mutex poisoned"),
        series,
    );
}

fn plant_binance_s2_history(
    historybook: &Arc<Mutex<crate::data::HistoryBook>>,
    builders: &Arc<Mutex<crate::data::CandleBuilders>>,
) {
    let json = include_str!("../fixtures/binance/klines.json");
    let series = crate::data::series_from_klines_json(
        json,
        "BTCUSDT",
        crate::data::DEFAULT_HISTORY_INTERVAL,
        crate::data::Transport::Fixture,
    )
    .expect("committed klines fixture must parse");
    crate::data::apply_history_series_and_seed(
        &mut historybook.lock().expect("historybook mutex poisoned"),
        &mut builders.lock().expect("candle builders mutex poisoned"),
        series,
    );
}

fn plant_binance_options_history(
    historybook: &Arc<Mutex<crate::data::HistoryBook>>,
    builders: &Arc<Mutex<crate::data::CandleBuilders>>,
) {
    let json = include_str!("../fixtures/binance/options_klines.json");
    let series = crate::data::series_from_eapi_klines_json(
        json,
        "BTC-200730-9000-C",
        crate::data::DEFAULT_OPTIONS_HISTORY_INTERVAL,
        crate::data::Transport::Fixture,
    )
    .expect("committed eapi klines fixture must parse");
    crate::data::apply_history_series_and_seed(
        &mut historybook.lock().expect("historybook mutex poisoned"),
        &mut builders.lock().expect("candle builders mutex poisoned"),
        series,
    );
}

fn plant_binance_spot_funds(account_book: &Arc<Mutex<crate::data::AccountBook>>) {
    use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding};
    account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_funds(
            crate::data::BINANCE_COM_SPOT_BOOK_ID,
            BrokerBalancesSnapshot {
                holdings: vec![BrokerHolding {
                    asset: "BTC".into(),
                    free: 0.01,
                    locked: 0.0,
                }],
                unrealized_pnl: None,
            },
            "/api/v3/account",
            1_700_000_000_000,
        );
}

fn plant_binance_options_fills(account_book: &Arc<Mutex<crate::data::AccountBook>>) {
    let json = include_str!("../fixtures/binance/options_user_trades.json");
    let trades = crate::binance_com_options_client::parse_user_trades(json)
        .expect("committed options userTrades fixture must parse");
    let fills: Vec<_> = trades
        .iter()
        .map(crate::binance_com_options_client::user_trade_to_broker_fill)
        .collect();
    account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_fills(
            crate::data::BINANCE_COM_OPTIONS_BOOK_ID,
            fills,
            "/eapi/v1/userTrades",
            1_700_000_000_000,
        );
}

pub async fn run_agent(config: AgentConfig) -> anyhow::Result<()> {
    let event_bus = EventBus::new(2048);
    // Per-venue egress posture reaches Notch over the same SSE stream as broker
    // sync. Every slot is reported on every change, so a banned venue never
    // blanks a live one.
    egress::shared_engine().attach_event_bus(Arc::new(event_bus.clone()));
    // Ring 3: a venue IP ban is mirrored into /etc/hosts for the life of the ban.
    egress::shared_engine().enable_hosts_backstop();
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
    let historybook = Arc::new(std::sync::Mutex::new(crate::data::HistoryBook::open(
        &config.history_db_path,
    )?));
    let candle_builders = Arc::new(std::sync::Mutex::new(crate::data::CandleBuilders::new()));
    let today_store = TodayStore::open(&config.today_db_path)?;
    let kill_switch_audit = KillSwitchAuditStore::open(&config.kill_switch_audit_db_path)?;
    let kill_policy = KillPolicyStore::open(&config.kill_switch_audit_db_path)?;
    kill_policy.load_or_insert_defaults()?;
    kill_policy.save(&config.kill_policy)?;
    let audit_signer = Arc::new(KillSwitchAuditSigner::from_env_or_generate());
    let instruments = Arc::new(InstrumentStore::new(
        config
            .instruments_db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("instruments db path is not valid UTF-8"))?,
    )?);
    if crate::zerodha_instruments_enabled() {
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
    }

    let broker_status = Arc::new(std::sync::Mutex::new(BrokerRuntimeState::default()));
    let account_book = Arc::new(Mutex::new(crate::data::AccountBook::new()));

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
    let kotak_nfo_scrip_master = Arc::new(std::sync::Mutex::new(
        crate::kotak_nfo_scrip::KotakNfoScripMaster::empty(),
    ));
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
        account_book.clone(),
        kotak_nfo_scrip_master.clone(),
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

    let live_book = Arc::new(crate::live_book::LiveBook::new());
    if let Some(planted) = config.live_book_snapshot {
        live_book.hydrate(planted);
    }
    {
        let book = live_book.clone();
        let mut rx = event_bus.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(crate::event_bus::AgentEvent::ToolbarShow {
                        symbol,
                        side,
                        qty,
                        price,
                        broker,
                        filled_at,
                        ..
                    }) => {
                        book.apply(crate::live_book::LiveBookEvent::Fill {
                            symbol,
                            side,
                            qty,
                            price: Some(price),
                            broker: Some(broker),
                            filled_at_iso: Some(filled_at),
                        });
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    let quote_registry = Arc::new(
        crate::data::Registry::load(&[
            crate::data::binance_com_quote_descriptor(),
            crate::data::kotak_neo_quote_descriptor(),
            crate::data::fixture_quote_descriptor(),
        ])
        .map_err(|rejects| anyhow::anyhow!("quote registry refused: {rejects:?}"))?,
    );
    let source_manifests = Arc::new(
        crate::data::load_first_party_manifests()
            .map_err(|rejects| anyhow::anyhow!("S1 manifests refused: {rejects:?}"))?,
    );
    let broker_connections = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let quote_streams = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let com_trade = crate::data::MarketBind::new();
    let com_depth = crate::data::MarketBind::new();
    let com_klines = crate::data::MarketBind::new();
    let klines_inflight = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let instrument_master = Arc::new(std::sync::Mutex::new(ExchangeInfoSymbolCache::empty()));
    let kotak_scrip_master = Arc::new(std::sync::Mutex::new(
        crate::kotak_scrip_master::KotakScripMaster::empty(),
    ));
    let kotak_session_locator = Arc::new(std::sync::Mutex::new(None));
    let kotak_quote_inflight = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let com_ticker_inflight = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let kotak_depth_inflight = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let instrument_master_status = Arc::new(std::sync::Mutex::new(
        crate::data::InstrumentMasterStatus::default(),
    ));
    let instrument_master_cancel = Arc::new(AtomicBool::new(false));
    let instrument_master_cache_dir = config.instrument_master_cache_dir.clone();
    let tickbook = Arc::new(std::sync::Mutex::new(crate::data::TickBook::new()));
    let depthbook = Arc::new(std::sync::Mutex::new(crate::data::DepthBook::new()));
    let nfo_open_interest: crate::kotak_rest_quotes::NfoOpenInterestSlot =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let nfo_oi_session: crate::kotak_rest_quotes::NfoOiSessionSlot =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    if config.plant_kotak_s1k_fixtures {
        plant_kotak_s1k_fixtures(
            quote_registry.as_ref(),
            &tickbook,
            &depthbook,
            &kotak_scrip_master,
            &broker_status,
            &instrument_master_status,
        );
    }
    if config.plant_kotak_nfo_quote {
        plant_kotak_nfo_quote(
            quote_registry.as_ref(),
            &tickbook,
            &nfo_open_interest,
            &broker_status,
        );
    }
    if config.plant_kotak_nfo_contracts {
        plant_kotak_nfo_contracts(&kotak_nfo_scrip_master, &broker_status);
    }
    if config.plant_kotak_nfo_depth {
        plant_kotak_nfo_depth(&depthbook);
    }
    if config.plant_kotak_nfo_oi_session {
        plant_kotak_nfo_oi_session(&nfo_oi_session);
    }
    if config.plant_binance_s2_history {
        plant_binance_s2_history(&historybook, &candle_builders);
    }
    if config.plant_binance_options_history {
        plant_binance_options_history(&historybook, &candle_builders);
    }
    if config.plant_licensed_history_gap {
        plant_licensed_history_gap(&historybook, &candle_builders);
    }
    if config.plant_binance_spot_funds {
        plant_binance_spot_funds(&account_book);
    }
    if config.plant_binance_options_fills {
        plant_binance_options_fills(&account_book);
    }
    if config.plant_binance_options_quote {
        plant_binance_options_quote(quote_registry.as_ref(), &tickbook);
    }
    if config.plant_binance_usdm_quote {
        plant_binance_usdm_quote(quote_registry.as_ref(), &tickbook);
    }
    let options_option_symbols = Arc::new(std::sync::Mutex::new(Vec::new()));
    let usdm_exchange_info = Arc::new(std::sync::Mutex::new(
        crate::data::UsdmExchangeInfoCache::empty(),
    ));
    if config.plant_binance_usdm_exchange_info {
        plant_binance_usdm_exchange_info(&usdm_exchange_info);
    }
    let coinm_exchange_info = Arc::new(std::sync::Mutex::new(
        crate::data::CoinmExchangeInfoCache::empty(),
    ));
    if config.plant_binance_coinm_exchange_info {
        plant_binance_coinm_exchange_info(&coinm_exchange_info);
    }
    if config.plant_binance_coinm_quote {
        plant_binance_coinm_quote(quote_registry.as_ref(), &tickbook);
    }
    let options_oi_rows = Arc::new(std::sync::Mutex::new(Vec::new()));
    if config.plant_binance_options_chain {
        plant_binance_options_chain(&options_option_symbols, &options_oi_rows);
    }
    let options_mark = Arc::new(std::sync::Mutex::new(None));
    if config.plant_binance_options_mark {
        plant_binance_options_mark(&options_mark);
    }
    if config.plant_binance_options_mark_no_bid {
        plant_binance_options_mark_no_bid(&options_mark);
    }
    let options_index = Arc::new(std::sync::Mutex::new(None));
    if config.plant_binance_options_index {
        plant_binance_options_index(&options_index);
    }
    if config.plant_binance_options_depth {
        plant_binance_options_depth(&depthbook);
    }
    if config.plant_binance_spot_depth_unusable {
        plant_binance_spot_depth_unusable(&depthbook);
    }

    let injected_station_tokens = config.station_token_store.is_some();
    let station_token_store_explicit = config.station_token_store_explicit;
    let station_token_store = config
        .station_token_store
        .unwrap_or_else(|| Arc::new(KeyringStationTokenStore));
    let station_tokens_in_store = station_token_store.load().ok().flatten().is_some();
    let loopback_bootstrap_jwt = upstream.config.is_loopback_http_bootstrap()
        && std::env::var("STATION_ACCESS_TOKEN")
            .ok()
            .is_some_and(|t| !t.trim().is_empty());
    let mut fact_outbox =
        crate::fact_outbox::FactOutbox::open(&config.fact_outbox_db_path, upstream.clone())?;
    if station_tokens_in_store || (injected_station_tokens && station_token_store_explicit) {
        fact_outbox = fact_outbox.with_token_store(station_token_store.clone());
    }
    if let Some(clock) = config.fact_clock_ms.clone() {
        fact_outbox = fact_outbox.with_clock(clock);
    }
    let fact_outbox = Arc::new(fact_outbox);
    today_service.attach_fact_outbox(fact_outbox.clone());

    let jwt_loadable = if injected_station_tokens {
        station_tokens_in_store || (!station_token_store_explicit && loopback_bootstrap_jwt)
    } else if loopback_bootstrap_jwt {
        true
    } else {
        tokio::task::spawn_blocking(|| KeyringStationTokenStore.load().ok().flatten().is_some())
            .await
            .unwrap_or(false)
    };
    if jwt_loadable && live_book.snapshot().is_none() {
        let url = format!("{}/api/bar/v1/live-state", upstream.config.base_url);
        if let Ok(req) = upstream.authorize_brain(upstream.http.get(&url)) {
            if let Ok(resp) = req.send().await {
                if resp.status().is_success() {
                    if let Ok(value) = resp.json::<serde_json::Value>().await {
                        live_book.hydrate(value);
                    }
                }
            }
        }
    }

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
        account_book: account_book.clone(),
        broker_sync_control: broker_sync_control.clone(),
        broker_limits: config.broker_sync.clone(),
        fog_active: fog_active.clone(),
        kill_switch_audit,
        audit_signer,
        last_l3_broker: Arc::new(std::sync::Mutex::new(None)),
        last_applied_level: Arc::new(std::sync::Mutex::new(None)),
        kill_policy,
        today_service: today_service.clone(),
        device_login_pending: Arc::new(std::sync::Mutex::new(None)),
        station_token_store,
        live_book,
        quote_registry,
        tickbook,
        depthbook,
        historybook,
        candle_builders,
        quote_freshness: config.quote_freshness,
        s1_desk_symbol: config.s1_desk_symbol.clone(),
        s1_options_symbol: config.s1_options_symbol.clone(),
        source_manifests,
        broker_connections,
        instrument_master,
        kotak_scrip_master,
        kotak_nfo_scrip_master,
        options_option_symbols,
        usdm_exchange_info,
        coinm_exchange_info,
        options_oi_rows,
        nfo_open_interest,
        nfo_oi_session,
        options_mark,
        options_index,
        eapi_public_fetch: config.eapi_public_fetch,
        quote_streams,
        com_trade,
        com_depth,
        com_klines,
        klines_inflight,
        kotak_session_locator,
        kotak_quote_inflight,
        com_ticker_inflight,
        kotak_depth_inflight,
        instrument_master_status,
        instrument_master_cancel,
        instrument_master_cache_dir,
        quote_selections: Arc::new(std::sync::Mutex::new(crate::api::QuoteSelections::default())),
        quote_fetch_error: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        binance_spot_base_url: config.binance_spot_base_url.clone(),
        binance_usdm_base_url: config.binance_usdm_base_url.clone(),
        binance_coinm_base_url: config.binance_coinm_base_url.clone(),
        binance_eapi_base_url: config.binance_eapi_base_url.clone(),
        force_order_book: Arc::new(Mutex::new(crate::data::ForceOrderBook::default())),
        usdm_realized: Arc::new(Mutex::new(None)),
        kotak_private_base_url: config.kotak_private_base_url.clone(),
        amfi_nav_base_url: config.amfi_nav_base_url.clone(),
        amfi_nav_host: config.amfi_nav_host.clone(),
        gap_vendor: Arc::new(Mutex::new(crate::data::GapVendorConfig {
            enabled: config.gap_vendor_enabled,
            key: config.gap_vendor_key.clone(),
            history_budget: config.gap_vendor_history_budget,
            quote_budget: config.kotak_quote_budget,
        })),
        amfi_enabled: Arc::new(Mutex::new(config.amfi_enabled)),
    };
    crate::data::spawn_binance_com_trade_loop(
        state.quote_registry.clone(),
        state.tickbook.clone(),
        state.candle_builders.clone(),
        state.com_trade.subscribe(),
    );
    crate::data::spawn_binance_com_depth_loop(state.depthbook.clone(), state.com_depth.subscribe());
    if let Some(symbol) = state.s1_desk_symbol.clone() {
        tracing::info!(
            instrument = %symbol,
            "s1 desk: public last-price stream (TickBook, not LiveBook); env is a dev default"
        );
        state.bind_spot_market(&symbol);
        crate::api::desk::try_load_binance_cache(
            &state.instrument_master_cache_dir,
            &state.instrument_master,
            &state.instrument_master_status,
        );
        crate::api::desk::spawn_exchange_info_refresh(
            state.instrument_master.clone(),
            state.instrument_master_status.clone(),
            state.instrument_master_cancel.clone(),
            state.instrument_master_cache_dir.clone(),
            state.broker_connections.clone(),
            false,
        );
    } else {
        tracing::info!(
            "s1 desk: no runtime subscription yet — TickBook empty until Start or quote resolve"
        );
    }
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

    let fact_worker = fact_outbox.clone();
    let fact_online_interval_ms = config.fact_online_interval_ms.max(1);
    tokio::spawn(async move {
        async fn publish_online(outbox: &Arc<crate::fact_outbox::FactOutbox>) {
            let worker = outbox.clone();
            let enqueued = tokio::task::spawn_blocking(move || {
                worker.enqueue(crate::fact_outbox::Fact::StationOnline)
            })
            .await;
            if matches!(
                enqueued,
                Ok(Ok(crate::fact_outbox::EnqueueOutcome::Enqueued { .. }))
            ) {
                let _ = outbox.drain().await;
            }
        }
        publish_online(&fact_worker).await;
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(fact_online_interval_ms));
        interval.tick().await;
        loop {
            interval.tick().await;
            publish_online(&fact_worker).await;
        }
    });

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
    tracing::info!(%addr, "tradeautopsy-agent listening");
    let serve_result = axum::serve(listener, router).await;
    background.abort_all();
    serve_result?;
    Ok(())
}

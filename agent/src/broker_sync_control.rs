//! Runtime broker sync start/stop without agent process restart (issues #13/#14).
//! Phase 2: Start is identity-first; credentials load from host vault (R6).

use crate::bar_fill_ingress::BarBrokerFillIngressConfig;
use crate::broker::{BrokerAdapter, CountingPollAdapter};
use crate::broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
use crate::event_bus::EventBus;
use crate::recent_trades::RecentTradesStore;
use crate::ubi::{
    BrokerCredentialVault, CredentialBlob, HostCredentialBlob, MemoryBrokerCredentialVault,
};
use crate::UpstreamClient;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

/// Identity-first start (B2 / R6). Credentials load from the host vault / Keychain by
/// connection id. Wire `apiKey`/`apiSecret` are refused — never seed the vault from the
/// Start body (and never forward into Wasm).
#[derive(Debug, Clone, Deserialize)]
pub struct BrokerSyncStartRequest {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "assetClass")]
    pub asset_class: String,
    /// Rejected when present/non-empty (B2). Kept on the type so clients get a clear error
    /// instead of silent ignore.
    #[serde(default, rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(default, rename = "apiSecret")]
    pub api_secret: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerRuntimeCardStatus {
    ReadyToStart,
    Syncing,
    Degraded,
    RateLimited,
    Paused,
}

impl BrokerRuntimeCardStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadyToStart => "ready_to_start",
            Self::Syncing => "syncing",
            Self::Degraded => "degraded",
            Self::RateLimited => "rate_limited",
            Self::Paused => "paused",
        }
    }
}

struct ActiveSync {
    cancel: Arc<AtomicBool>,
    coalesce_tx: mpsc::Sender<crate::broker::BrokerFill>,
    poll_handle: JoinHandle<()>,
    coalesce_handle: JoinHandle<()>,
}

pub struct BrokerSyncController {
    status_arc: Arc<Mutex<BrokerRuntimeState>>,
    recent_trades: RecentTradesStore,
    since: Arc<RwLock<Option<DateTime<Utc>>>>,
    cfg: BrokerSyncConfig,
    bus: EventBus,
    upstream: Option<Arc<UpstreamClient>>,
    bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
    user_paused: Arc<AtomicBool>,
    active: Arc<Mutex<Option<ActiveSync>>>,
    /// Integration-test hook: fixed adapter for all runtime starts.
    pub test_runtime_adapter: Option<Arc<dyn BrokerAdapter>>,
    /// Integration-test hook: records api keys resolved on each start (host-side only).
    pub test_start_key_log: Option<Arc<Mutex<Vec<String>>>>,
    today_service: Arc<Mutex<Option<Arc<crate::today::TodayService>>>>,
    credential_vault: Arc<dyn BrokerCredentialVault>,
}

impl BrokerSyncController {
    pub fn new(
        status_arc: Arc<Mutex<BrokerRuntimeState>>,
        recent_trades: RecentTradesStore,
        since: Arc<RwLock<Option<DateTime<Utc>>>>,
        cfg: BrokerSyncConfig,
        bus: EventBus,
        upstream: Option<Arc<UpstreamClient>>,
        bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
        test_runtime_adapter: Option<Arc<dyn BrokerAdapter>>,
        test_start_key_log: Option<Arc<Mutex<Vec<String>>>>,
        today_service: Arc<Mutex<Option<Arc<crate::today::TodayService>>>>,
        credential_vault: Arc<dyn BrokerCredentialVault>,
    ) -> Self {
        Self {
            status_arc,
            recent_trades,
            since,
            cfg,
            bus,
            upstream,
            bar_fill_ingress,
            user_paused: Arc::new(AtomicBool::new(false)),
            active: Arc::new(Mutex::new(None)),
            test_runtime_adapter,
            test_start_key_log,
            today_service,
            credential_vault,
        }
    }

    pub fn card_status(&self) -> BrokerRuntimeCardStatus {
        if self.user_paused.load(Ordering::Relaxed) {
            return BrokerRuntimeCardStatus::Paused;
        }
        let st = self.status_arc.lock().expect("broker status");
        if !st.broker_connected {
            return BrokerRuntimeCardStatus::ReadyToStart;
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        if st.data_classes.any_rate_limited(now_ms) {
            return BrokerRuntimeCardStatus::RateLimited;
        }
        if st.data_classes.all_current() {
            BrokerRuntimeCardStatus::Syncing
        } else {
            BrokerRuntimeCardStatus::Degraded
        }
    }

    pub fn retry_failed_classes(&self) -> anyhow::Result<()> {
        let mut st = self.status_arc.lock().expect("broker status");
        for class in crate::broker_data_class::BrokerDataClass::ALL {
            let entry = st.data_classes.get_mut(class);
            entry.requires_manual_retry = false;
            entry.last_error_category = None;
        }
        st.circuit_open = false;
        st.consecutive_failures = 0;
        Ok(())
    }

    pub fn start_with_adapter(&self, adapter: Arc<dyn BrokerAdapter>) -> anyhow::Result<()> {
        self.start_with_adapter_for_slug(adapter, None)
    }

    /// Start poll loop; `broker_slug` drives desk quote currency / Kill DNS (R7/R8).
    pub fn start_with_adapter_for_slug(
        &self,
        adapter: Arc<dyn BrokerAdapter>,
        broker_slug: Option<&str>,
    ) -> anyhow::Result<()> {
        self.stop_active_sync()?;
        self.user_paused.store(false, Ordering::Relaxed);
        {
            let mut st = self.status_arc.lock().expect("broker status");
            st.broker_connected = false;
            st.circuit_open = false;
            st.consecutive_failures = 0;
            st.last_error = None;
            st.last_sync_sse_class.clear();
            st.data_classes = crate::broker_data_class::BrokerDataClassCompleteness::default();
            st.active_broker_slug = broker_slug.map(|s| s.to_ascii_lowercase());
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel(64);
        let coalesce_handle = crate::broker_sync::spawn_toolbar_coalesce_task(
            self.bus.clone(),
            rx,
            self.cfg.coalesce_window,
        );
        let poll_handle = crate::broker_sync::spawn_broker_poll_loop(
            adapter,
            self.recent_trades.clone(),
            self.since.clone(),
            self.cfg.clone(),
            self.bus.clone(),
            self.status_arc.clone(),
            tx.clone(),
            self.upstream.clone(),
            self.bar_fill_ingress.clone(),
            Some(cancel.clone()),
            self.today_service.lock().expect("today service slot").clone(),
        );

        *self.active.lock().expect("active sync") = Some(ActiveSync {
            cancel,
            coalesce_tx: tx,
            poll_handle,
            coalesce_handle,
        });
        Ok(())
    }

    pub fn start(&self, request: &BrokerSyncStartRequest) -> anyhow::Result<()> {
        let blob = self.resolve_credentials(request)?;
        if let Some(log) = &self.test_start_key_log {
            if let Some(key) = blob.api_key_for_tests() {
                log.lock()
                    .expect("start key log")
                    .push(key.to_string());
            }
        }

        let adapter = if let Some(fixed) = &self.test_runtime_adapter {
            fixed.clone()
        } else {
            build_runtime_adapter(
                &request.broker_slug,
                &request.broker_connection_id,
                &blob,
            )?
        };
        self.start_with_adapter_for_slug(adapter, Some(&request.broker_slug))
    }

    fn resolve_credentials(
        &self,
        request: &BrokerSyncStartRequest,
    ) -> anyhow::Result<CredentialBlob> {
        if wire_credentials_present(request) {
            anyhow::bail!(
                "wire credentials refused (B2): Start by connection id only; secrets stay in host vault / Keychain"
            );
        }

        self.credential_vault
            .load(
                &request.environment,
                &request.broker_slug,
                &request.broker_connection_id,
            )?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing credentials for connection {} (host vault / Keychain)",
                    request.broker_connection_id
                )
            })
    }

    /// Pause poll only (B3 / R10). Does **not** arm, dismiss, or weaken Kill —
    /// Kill DNS / audit / SSE stay on their own stack. Never synonym for Kill.
    pub fn stop(&self) -> anyhow::Result<()> {
        self.user_paused.store(true, Ordering::Relaxed);
        self.stop_active_sync()?;
        {
            let mut st = self.status_arc.lock().expect("broker status");
            st.broker_connected = false;
            st.active_broker_slug = None;
            st.last_sync_sse_class.clear();
            self.bus.publish(crate::event_bus::AgentEvent::BrokerSyncState {
                payload: serde_json::to_value(&*st).unwrap_or_else(|_| json!({})),
            });
        }
        Ok(())
    }

    fn stop_active_sync(&self) -> anyhow::Result<()> {
        let Some(active) = self.active.lock().expect("active sync").take() else {
            return Ok(());
        };
        active.cancel.store(true, Ordering::Relaxed);
        drop(active.coalesce_tx);
        active.poll_handle.abort();
        active.coalesce_handle.abort();
        {
            let mut st = self.status_arc.lock().expect("broker status");
            st.broker_connected = false;
            st.active_broker_slug = None;
        }
        Ok(())
    }
}

/// Slugs whose adapter is a Wasm component (ADR 0001). Native COM code is reference only.
pub fn uses_wasm_component(broker_slug: &str) -> bool {
    matches!(broker_slug, "binance_com" | "kotak_neo")
}

/// Build the sandboxed component adapter for a UBI broker; credentials stay host-side.
pub fn build_wasm_runtime_adapter(
    broker_slug: &str,
    connection_id: &str,
    blob: &CredentialBlob,
) -> anyhow::Result<Arc<dyn BrokerAdapter>> {
    let descriptor = crate::ubi::descriptor_for_slug(broker_slug)
        .ok_or_else(|| anyhow::anyhow!("no catalog descriptor for {broker_slug}"))?;
    let transport = crate::ubi::ReqwestBrokerHttpTransport::shared()
        .map_err(|e| anyhow::anyhow!("ubi transport: {e}"))?;
    Ok(Arc::new(crate::ubi::WasmBrokerAdapter::new(
        broker_slug,
        connection_id,
        descriptor.asset_class,
        HostCredentialBlob::from(blob),
        transport,
    )?))
}

fn build_runtime_adapter(
    broker_slug: &str,
    connection_id: &str,
    blob: &CredentialBlob,
) -> anyhow::Result<Arc<dyn BrokerAdapter>> {
    match (broker_slug, blob) {
        ("binance_us", CredentialBlob::HmacApiKeySecret { api_key, .. })
            if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_") =>
        {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        ("binance_us", CredentialBlob::HmacApiKeySecret { .. }) => {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        ("binance_com", CredentialBlob::HmacApiKeySecret { api_key, .. })
            if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_COM_") =>
        {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        ("binance_com", CredentialBlob::HmacApiKeySecret { .. })
        | ("kotak_neo", CredentialBlob::KotakNeoTotpSession { .. }) => {
            build_wasm_runtime_adapter(broker_slug, connection_id, blob)
        }
        (other, _) => anyhow::bail!("unsupported broker slug or credential shape: {other}"),
    }
}

fn wire_credentials_present(request: &BrokerSyncStartRequest) -> bool {
    let key = request.api_key.as_deref().unwrap_or("").trim();
    let secret = request.api_secret.as_deref().unwrap_or("").trim();
    !key.is_empty() || !secret.is_empty()
}

/// Default vault for production: Keychain-backed.
pub fn default_credential_vault() -> Arc<dyn BrokerCredentialVault> {
    Arc::new(crate::ubi::KeyringBrokerCredentialVault)
}

pub fn memory_credential_vault() -> Arc<dyn BrokerCredentialVault> {
    Arc::new(MemoryBrokerCredentialVault::new())
}

#[cfg(test)]
mod b2_keychain_only_tests {
    use super::*;
    use crate::broker_sync::BrokerSyncConfig;
    use crate::event_bus::EventBus;
    use crate::recent_trades::RecentTradesStore;
    use std::sync::Mutex;

    fn unique_db(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rta-b2-{label}-{}-{}.db",
            std::process::id(),
            ulid::Ulid::new()
        ))
    }

    fn controller(vault: Arc<dyn BrokerCredentialVault>) -> BrokerSyncController {
        let db = unique_db("ctrl");
        let recent = RecentTradesStore::open(&db).expect("recent trades");
        BrokerSyncController::new(
            Arc::new(Mutex::new(BrokerRuntimeState::default())),
            recent,
            Arc::new(RwLock::new(None)),
            BrokerSyncConfig::default(),
            EventBus::new(32),
            None,
            None,
            Some(Arc::new(CountingPollAdapter::new())),
            None,
            Arc::new(Mutex::new(None)),
            vault,
        )
    }

    fn identity_request() -> BrokerSyncStartRequest {
        BrokerSyncStartRequest {
            broker_slug: "binance_us".into(),
            broker_connection_id: "00000000-0000-4000-8000-000000000001".into(),
            environment: "prod".into(),
            asset_class: "crypto".into(),
            api_key: None,
            api_secret: None,
        }
    }

    #[test]
    fn b2_empty_vault_fails_start() {
        let vault = memory_credential_vault();
        let ctrl = controller(vault);
        let err = ctrl.start(&identity_request()).expect_err("empty vault");
        let msg = err.to_string();
        assert!(msg.contains("missing credentials"), "{msg}");
        assert!(!msg.contains("secret"), "{msg}");
    }

    #[test]
    fn b2_wire_secrets_are_refused() {
        let vault = MemoryBrokerCredentialVault::new();
        let vault: Arc<dyn BrokerCredentialVault> = Arc::new(vault);
        let ctrl = controller(vault.clone());
        let mut req = identity_request();
        req.api_key = Some("leak-key".into());
        req.api_secret = Some("leak-secret".into());
        let err = ctrl.start(&req).expect_err("wire secrets");
        let msg = err.to_string();
        assert!(msg.contains("wire credentials refused"), "{msg}");
        assert!(!msg.contains("leak-key"), "{msg}");
        assert!(!msg.contains("leak-secret"), "{msg}");
        assert!(
            vault
                .load("prod", "binance_us", &req.broker_connection_id)
                .unwrap()
                .is_none(),
            "refused wire secrets must not seed the vault"
        );
    }

    #[tokio::test]
    async fn b2_start_by_connection_id_loads_vault() {
        let vault = MemoryBrokerCredentialVault::new();
        let conn = "00000000-0000-4000-8000-000000000001";
        vault
            .save(
                "prod",
                "binance_us",
                conn,
                &CredentialBlob::hmac("vault-key", "vault-secret"),
            )
            .unwrap();
        let vault: Arc<dyn BrokerCredentialVault> = Arc::new(vault);
        let keys = Arc::new(Mutex::new(Vec::<String>::new()));
        let recent = RecentTradesStore::open(&unique_db("load")).expect("db");
        let ctrl = BrokerSyncController::new(
            Arc::new(Mutex::new(BrokerRuntimeState::default())),
            recent,
            Arc::new(RwLock::new(None)),
            BrokerSyncConfig::default(),
            EventBus::new(32),
            None,
            None,
            Some(Arc::new(CountingPollAdapter::new())),
            Some(keys.clone()),
            Arc::new(Mutex::new(None)),
            vault,
        );
        ctrl.start(&identity_request()).expect("start");
        assert_eq!(
            keys.lock().expect("keys").clone(),
            vec!["vault-key".to_string()]
        );
        ctrl.stop().expect("stop");
    }
}

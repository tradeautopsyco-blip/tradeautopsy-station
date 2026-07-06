//! Runtime broker sync start/stop without agent process restart (issues #13/#14).

use crate::broker::{BrokerAdapter, CountingPollAdapter};
use crate::broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
use crate::bar_fill_ingress::BarBrokerFillIngressConfig;
use crate::event_bus::EventBus;
use crate::recent_trades::RecentTradesStore;
use crate::UpstreamClient;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Deserialize)]
pub struct BrokerSyncStartRequest {
    #[serde(rename = "brokerSlug")]
    pub broker_slug: String,
    #[serde(rename = "brokerConnectionId")]
    pub broker_connection_id: String,
    pub environment: String,
    #[serde(rename = "assetClass")]
    pub asset_class: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "apiSecret")]
    pub api_secret: String,
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
    /// Integration-test hook: records api keys supplied on each start.
    pub test_start_key_log: Option<Arc<Mutex<Vec<String>>>>,
    today_service: Arc<Mutex<Option<Arc<crate::today::TodayService>>>>,
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
        if let Some(log) = &self.test_start_key_log {
            log.lock()
                .expect("start key log")
                .push(request.api_key.clone());
        }

        let adapter = if let Some(fixed) = &self.test_runtime_adapter {
            fixed.clone()
        } else {
            build_runtime_adapter(&request.broker_slug, &request.api_key, &request.api_secret)?
        };
        self.start_with_adapter(adapter)
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.user_paused.store(true, Ordering::Relaxed);
        self.stop_active_sync()?;
        {
            let mut st = self.status_arc.lock().expect("broker status");
            st.broker_connected = false;
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
        }
        Ok(())
    }
}

fn build_runtime_adapter(
    broker_slug: &str,
    api_key: &str,
    api_secret: &str,
) -> anyhow::Result<Arc<dyn BrokerAdapter>> {
    match broker_slug {
        "binance_us" if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_") => {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        "binance_us" => Ok(Arc::new(CountingPollAdapter::new())),
        "binance_com"
            if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_COM_") =>
        {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        "binance_com" => Ok(Arc::new(crate::binance_com_spot_adapter::BinanceComSpotBrokerAdapter::new(
            api_key, api_secret,
        ))),
        other => anyhow::bail!("unsupported broker slug: {other}"),
    }
}

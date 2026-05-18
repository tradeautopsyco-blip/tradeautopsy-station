//! Broker poll worker — breaker cadence + `toolbar_show` coalesce (design §8.4).

use crate::bar_fill_ingress::{
    post_bar_broker_fill_ingress, BarBrokerFillIngressConfig, BarFillIngestSource,
};
use crate::broker::{BrokerAdapter, BrokerError};
use crate::event_bus::{AgentEvent, EventBus};
use crate::recent_trades::RecentTradesStore;
use crate::UpstreamClient;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::warn;

use crate::broker::BrokerFill;

#[derive(Clone, Debug)]
pub struct BrokerSyncConfig {
    /// Delay before the first emission + poll (lets the bar subscribe to SSE in tests/integration).
    pub initial_startup_delay: Duration,
    pub base_poll_interval: Duration,
    pub coalesce_window: Duration,
    pub failure_escalate_after: u32,
    pub backoff_tick_1: Duration,
    pub backoff_tick_2: Duration,
    pub failures_until_open: u32,
    pub fresh_secs: u64,
    pub stale_secs: u64,
}

impl Default for BrokerSyncConfig {
    fn default() -> Self {
        Self {
            initial_startup_delay: Duration::ZERO,
            base_poll_interval: Duration::from_secs(3),
            coalesce_window: Duration::from_millis(50),
            failure_escalate_after: 3,
            backoff_tick_1: Duration::from_secs(15),
            backoff_tick_2: Duration::from_secs(30),
            failures_until_open: 5,
            fresh_secs: 6,
            stale_secs: 30,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct BrokerRuntimeState {
    pub broker_connected: bool,
    pub circuit_open: bool,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub last_success_at_ms: Option<i64>,
    pub last_poll_at_ms: Option<i64>,
    pub backend_broker_label: Option<String>,
    #[serde(skip)]
    pub last_sync_sse_class: String,
}

impl Default for BrokerRuntimeState {
    fn default() -> Self {
        Self {
            broker_connected: false,
            circuit_open: false,
            consecutive_failures: 0,
            last_error: None,
            last_success_at_ms: None,
            last_poll_at_ms: None,
            backend_broker_label: None,
            last_sync_sse_class: String::new(),
        }
    }
}

impl BrokerRuntimeState {
    pub fn sync_state_literal(&self, fresh_secs: u64, stale_secs: u64) -> &'static str {
        if !self.broker_connected {
            return "not_connected";
        }
        if self.circuit_open {
            return "disconnected";
        }
        let Some(last_ok) = self.last_success_at_ms else {
            if self.last_error.is_some() {
                return "disconnected";
            }
            /* Broker session up, first successful poll not yet observed. */
            return "stale";
        };

        let now_ms = Utc::now().timestamp_millis();
        let age_secs = ((now_ms - last_ok).max(0) / 1000) as u64;

        if age_secs < fresh_secs {
            "synced"
        } else if age_secs <= stale_secs && self.last_error.is_none() {
            "stale"
        } else if self.last_error.is_some() {
            "disconnected"
        } else {
            "stale"
        }
    }
}

fn broker_err_as_str(e: &BrokerError) -> String {
    match e {
        BrokerError::Http(msg) => msg.clone(),
    }
}

fn emit_sync_transition(
    bus: &EventBus,
    st: &mut BrokerRuntimeState,
    class: &'static str,
    extras: Value,
) {
    if st.last_sync_sse_class == class {
        return;
    }
    st.last_sync_sse_class = class.to_string();
    let mut body = serde_json::to_value(st).unwrap_or_else(|_| json!({}));
    if let Value::Object(ref mut m) = body {
        m.insert("class".into(), Value::String(class.to_string()));
        if let Value::Object(e) = extras {
            for (k, v) in e {
                m.insert(k, v);
            }
        }
    }
    bus.publish(AgentEvent::BrokerSyncState { payload: body });
}

fn toolbar_coalesce_task(bus: EventBus, mut rx: mpsc::Receiver<BrokerFill>, window: Duration) {
    tokio::spawn(async move {
        loop {
            let Some(first) = rx.recv().await else {
                break;
            };

            let mut latest = first;
            let deadline = tokio::time::Instant::now() + window;
            loop {
                let dur = deadline.saturating_duration_since(tokio::time::Instant::now());
                if dur.is_zero() {
                    break;
                }
                match tokio::time::timeout(dur, rx.recv()).await {
                    Ok(Some(m)) => latest = m,
                    Ok(None) => break,
                    Err(_) => break,
                }
            }

            let detection_id = ulid::Ulid::new().to_string();
            let filled_at = latest
                .filled_at
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            bus.publish(AgentEvent::ToolbarShow {
                trigger: "trade_detected".to_string(),
                detection_id,
                trade_id: latest.trade_id,
                symbol: latest.symbol,
                side: latest.side,
                qty: latest.qty,
                price: latest.price,
                filled_at,
                broker: latest.broker,
            });
        }
    });
}

fn poll_interval(consec_failures: u32, circuit_open: bool, cfg: &BrokerSyncConfig) -> Duration {
    if circuit_open {
        return Duration::from_secs(60 * 60);
    }
    match consec_failures {
        0 => cfg.base_poll_interval,
        n if n < cfg.failure_escalate_after => cfg.base_poll_interval,
        n if n == cfg.failure_escalate_after => cfg.backoff_tick_1,
        n if n < cfg.failures_until_open => cfg.backoff_tick_2,
        _ => cfg.backoff_tick_2.max(cfg.base_poll_interval),
    }
}

fn failure_sse_class(consec: u32, cfg: &BrokerSyncConfig, circuit_open: bool) -> &'static str {
    if circuit_open {
        return "circuit_open";
    }
    if consec >= cfg.failures_until_open {
        return "circuit_open";
    }
    if consec >= cfg.failure_escalate_after {
        "broker_backoff"
    } else if consec > 0 {
        "disconnected"
    } else {
        "synced"
    }
}

pub fn spawn_broker_poll_loop(
    adapter: Arc<dyn BrokerAdapter>,
    store: RecentTradesStore,
    since: Arc<RwLock<Option<DateTime<Utc>>>>,
    cfg: BrokerSyncConfig,
    bus: EventBus,
    status_arc: Arc<std::sync::Mutex<BrokerRuntimeState>>,
    coalesce_tx: mpsc::Sender<BrokerFill>,
    upstream: Option<Arc<UpstreamClient>>,
    bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let broker_label = adapter.name().to_owned();

        {
            let mut st = status_arc.lock().expect("mux");
            st.broker_connected = true;
            st.backend_broker_label = Some(broker_label.clone());
        }

        tokio::time::sleep(cfg.initial_startup_delay).await;

        {
            let mut st = status_arc.lock().expect("mux");
            emit_sync_transition(&bus, &mut st, "polling", json!({ "broker": broker_label }));
        }

        let mut first_poll_after_boot = true;
        loop {
            let sleep_dur = if first_poll_after_boot {
                first_poll_after_boot = false;
                Duration::ZERO
            } else {
                let st = status_arc.lock().expect("mux");
                poll_interval(st.consecutive_failures, st.circuit_open, &cfg)
            };

            tokio::time::sleep(sleep_dur).await;

            let circuit_open = {
                let st = status_arc.lock().expect("mux");
                st.circuit_open
            };

            if circuit_open {
                continue;
            }

            let since_opt = (*since.read().await).clone();
            match adapter.poll_fills(since_opt).await {
                Ok(fills) => {
                    let new_fills = match store.merge_poll(&fills) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!(error = ?e, "merge_poll failed");
                            vec![]
                        }
                    };

                    if let Some(max_t) = fills.iter().map(|f| f.filled_at).max() {
                        let mut w = since.write().await;
                        *w = Some(match *w {
                            Some(cur) => cur.max(max_t),
                            None => max_t,
                        });
                    }

                    let ok_ms = Utc::now().timestamp_millis();

                    let to_coalesce: Vec<BrokerFill> = new_fills.clone();
                    {
                        let mut st = status_arc.lock().expect("mux");
                        st.consecutive_failures = 0;
                        st.last_error = None;
                        st.circuit_open = false;
                        st.last_success_at_ms = Some(ok_ms);
                        st.last_poll_at_ms = Some(ok_ms);
                        emit_sync_transition(&bus, &mut st, "synced", json!({}));
                    }

                    if let (Some(up), Some(ref ingest_cfg)) = (&upstream, &bar_fill_ingress) {
                        for nf in &new_fills {
                            post_bar_broker_fill_ingress(
                                up.as_ref(),
                                ingest_cfg,
                                nf,
                                BarFillIngestSource::Reconciliation,
                            )
                            .await;
                        }
                    }

                    for nf in to_coalesce {
                        let _ = coalesce_tx.send(nf).await;
                    }
                }
                Err(e) => {
                    let msg = broker_err_as_str(&e);

                    {
                        let mut st = status_arc.lock().expect("mux");
                        let n = st.consecutive_failures.saturating_add(1);
                        st.consecutive_failures = n;
                        st.last_error = Some(msg.clone());
                        let poll_ms = Utc::now().timestamp_millis();
                        st.last_poll_at_ms = Some(poll_ms);

                        if n >= cfg.failures_until_open {
                            st.circuit_open = true;
                        }

                        let class = failure_sse_class(n, &cfg, st.circuit_open);
                        emit_sync_transition(&bus, &mut st, class, json!({ "error": msg }));
                    }
                }
            }
        }
    })
}

pub fn spawn_broker_stack(
    adapter: Arc<dyn BrokerAdapter>,
    store: RecentTradesStore,
    since: Arc<RwLock<Option<DateTime<Utc>>>>,
    cfg: BrokerSyncConfig,
    bus: EventBus,
    status_arc: Arc<std::sync::Mutex<BrokerRuntimeState>>,
    upstream: Option<Arc<UpstreamClient>>,
    bar_fill_ingress: Option<BarBrokerFillIngressConfig>,
) -> tokio::task::JoinHandle<()> {
    let (tx, rx) = mpsc::channel(64);
    toolbar_coalesce_task(bus.clone(), rx, cfg.coalesce_window);
    spawn_broker_poll_loop(
        adapter,
        store,
        since,
        cfg,
        bus,
        status_arc,
        tx,
        upstream,
        bar_fill_ingress,
    )
}

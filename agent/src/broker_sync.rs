//! Broker poll worker — breaker cadence + `toolbar_show` coalesce (design §8.4).

use crate::bar_fill_ingress::{
    post_bar_broker_fill_ingress, BarBrokerFillIngressConfig, BarFillIngestSource,
};
use crate::broker::{BrokerAdapter, BrokerError, BrokerFill};
use crate::broker_data_class::BrokerDataClass;
use crate::event_bus::{AgentEvent, EventBus};
use crate::recent_trades::RecentTradesStore;
use crate::UpstreamClient;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::warn;

pub use crate::broker_data_class::{
    BrokerBalancesSnapshot, BrokerDataClassCompleteness, BrokerHolding, BrokerOpenOrder,
    BrokerOpenOrdersSnapshot, DataClassFreshness,
};

#[derive(Clone, Debug)]
pub struct BrokerSyncConfig {
    pub initial_backfill_days: i64,
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
    pub rate_limit_default_backoff_ms: i64,
}

impl Default for BrokerSyncConfig {
    fn default() -> Self {
        Self {
            initial_backfill_days: 90,
            initial_startup_delay: Duration::ZERO,
            base_poll_interval: Duration::from_secs(3),
            coalesce_window: Duration::from_millis(50),
            failure_escalate_after: 3,
            backoff_tick_1: Duration::from_secs(15),
            backoff_tick_2: Duration::from_secs(30),
            failures_until_open: 5,
            fresh_secs: 6,
            stale_secs: 30,
            rate_limit_default_backoff_ms: 60_000,
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
    /// Catalog slug of the active sync (`binance_com` / `kotak_neo`) — desk honesty (R7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_broker_slug: Option<String>,
    pub data_classes: BrokerDataClassCompleteness,
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
            active_broker_slug: None,
            data_classes: BrokerDataClassCompleteness::default(),
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
        BrokerError::RateLimited { retry_after_ms } => {
            if let Some(ms) = retry_after_ms {
                format!("rate_limited:retry_after_ms={ms}")
            } else {
                "rate_limited".to_string()
            }
        }
    }
}

fn mark_class_success(
    completeness: &mut BrokerDataClassCompleteness,
    class: BrokerDataClass,
    ok_ms: i64,
) {
    let entry = completeness.get_mut(class);
    entry.current = true;
    entry.last_success_at_ms = Some(ok_ms);
    entry.last_error_category = None;
    entry.rate_limited_until_ms = None;
    entry.requires_manual_retry = false;
}

fn mark_class_failure(
    completeness: &mut BrokerDataClassCompleteness,
    class: BrokerDataClass,
    err: &BrokerError,
    cfg: &BrokerSyncConfig,
    class_failures: &mut u32,
) {
    let entry = completeness.get_mut(class);
    entry.current = false;
    entry.last_error_category = Some(err.category().to_string());

    match err {
        BrokerError::RateLimited { retry_after_ms } => {
            let until = retry_after_ms.unwrap_or_else(|| {
                Utc::now().timestamp_millis() + cfg.rate_limit_default_backoff_ms
            });
            entry.rate_limited_until_ms = Some(until);
        }
        BrokerError::Http(_) => {
            *class_failures = class_failures.saturating_add(1);
            if *class_failures >= cfg.failures_until_open {
                entry.requires_manual_retry = true;
            }
        }
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

pub fn spawn_toolbar_coalesce_task(
    bus: EventBus,
    mut rx: mpsc::Receiver<BrokerFill>,
    window: Duration,
) -> tokio::task::JoinHandle<()> {
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
    })
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
    cancel: Option<Arc<AtomicBool>>,
    today_service: Option<Arc<crate::today::TodayService>>,
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
        let mut class_failures: std::collections::HashMap<BrokerDataClass, u32> =
            std::collections::HashMap::new();
        loop {
            if cancel
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                break;
            }

            let sleep_dur = if first_poll_after_boot {
                first_poll_after_boot = false;
                Duration::ZERO
            } else {
                let st = status_arc.lock().expect("mux");
                let now_ms = Utc::now().timestamp_millis();
                if st.data_classes.any_rate_limited(now_ms) {
                    if let Some(until) = st.data_classes.earliest_rate_limit_retry_ms(now_ms) {
                        let wait_ms = (until - now_ms).max(0) as u64;
                        Duration::from_millis(wait_ms.min(60_000))
                    } else {
                        poll_interval(st.consecutive_failures, st.circuit_open, &cfg)
                    }
                } else {
                    poll_interval(st.consecutive_failures, st.circuit_open, &cfg)
                }
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
            let fills_result = adapter.poll_fills(since_opt).await;
            let balances_result = adapter.poll_balances_holdings().await;
            let open_orders_result = adapter.poll_open_orders().await;

            let ok_ms = Utc::now().timestamp_millis();
            let mut any_class_error = false;
            let mut last_err_msg: Option<String> = None;
            let mut new_fills: Vec<BrokerFill> = Vec::new();

            if let Ok(fills) = &fills_result {
                new_fills = match store.merge_poll(fills) {
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
            }

            {
                let mut st = status_arc.lock().expect("mux");
                st.last_poll_at_ms = Some(ok_ms);

                if fills_result.is_ok() {
                    mark_class_success(&mut st.data_classes, BrokerDataClass::FillsTradeHistory, ok_ms);
                    class_failures.insert(BrokerDataClass::FillsTradeHistory, 0);
                } else if let Err(e) = &fills_result {
                    any_class_error = true;
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::FillsTradeHistory)
                        .or_insert(0);
                    mark_class_failure(&mut st.data_classes, BrokerDataClass::FillsTradeHistory, e, &cfg, count);
                }

                if balances_result.is_ok() {
                    mark_class_success(&mut st.data_classes, BrokerDataClass::BalancesHoldings, ok_ms);
                    class_failures.insert(BrokerDataClass::BalancesHoldings, 0);
                } else if let Err(e) = &balances_result {
                    any_class_error = true;
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::BalancesHoldings)
                        .or_insert(0);
                    mark_class_failure(&mut st.data_classes, BrokerDataClass::BalancesHoldings, e, &cfg, count);
                }

                if open_orders_result.is_ok() {
                    mark_class_success(&mut st.data_classes, BrokerDataClass::OpenOrders, ok_ms);
                    class_failures.insert(BrokerDataClass::OpenOrders, 0);
                } else if let Err(e) = &open_orders_result {
                    any_class_error = true;
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::OpenOrders)
                        .or_insert(0);
                    mark_class_failure(&mut st.data_classes, BrokerDataClass::OpenOrders, e, &cfg, count);
                }

                if any_class_error {
                    let n = st.consecutive_failures.saturating_add(1);
                    st.consecutive_failures = n;
                    st.last_error = last_err_msg.clone();
                    if n >= cfg.failures_until_open {
                        st.circuit_open = true;
                    }
                    let class = failure_sse_class(n, &cfg, st.circuit_open);
                    let failing = st.data_classes.failing_class_labels();
                    emit_sync_transition(
                        &bus,
                        &mut st,
                        class,
                        json!({
                            "error": last_err_msg,
                            "failingDataClasses": failing,
                        }),
                    );
                } else {
                    st.consecutive_failures = 0;
                    st.last_error = None;
                    st.circuit_open = false;
                    st.last_success_at_ms = Some(ok_ms);
                    let sync_class = if st.data_classes.all_current() {
                        "synced"
                    } else {
                        "partial_sync"
                    };
                    emit_sync_transition(&bus, &mut st, sync_class, json!({}));
                }
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

            if !new_fills.is_empty() {
                if let Some(ref today) = today_service {
                    if let Err(e) = today.refresh_from_fills() {
                        warn!(error = ?e, "today refresh after fill ingest failed");
                    }
                }
            }

            for nf in new_fills {
                let _ = coalesce_tx.send(nf).await;
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
    let _coalesce = spawn_toolbar_coalesce_task(bus.clone(), rx, cfg.coalesce_window);
    let poll = spawn_broker_poll_loop(
        adapter,
        store,
        since,
        cfg,
        bus,
        status_arc,
        tx,
        upstream,
        bar_fill_ingress,
        None,
        None,
    );
    poll
}

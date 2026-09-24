//! Broker poll worker — breaker cadence + `toolbar_show` coalesce (design §8.4).

use crate::bar_fill_ingress::{
    post_bar_broker_fill_ingress, BarBrokerFillIngressConfig, BarFillIngestSource,
};
use crate::broker::{slug_to_slot, BrokerAdapter, BrokerError, BrokerFill};
use crate::broker_data_class::BrokerDataClass;
use crate::data::{
    fills_provenance_path, merge_poll_book_id, split_fills_by_book, stamp_nfo_fills, AccountBook,
};
use crate::event_bus::{AgentEvent, EventBus};
use crate::kotak_nfo_scrip::KotakNfoScripMaster;
use crate::instruments::{stamp_zerodha_kite_nfo_fills, InstrumentStore};
use crate::recent_trades::RecentTradesStore;
use crate::UpstreamClient;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::warn;

pub use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerDataClassCompleteness};

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
    /// Last successful balances poll. Obtain(funds) only — never TickBook.
    #[serde(skip)]
    pub last_balances: Option<BrokerBalancesSnapshot>,
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
            last_balances: None,
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
        // Dead TOTP/session is Stop-class. A live session whose fills poll
        // failed (quiet-day `Not_Ok`, funds leftover) is stale/degraded — Notch
        // maps `disconnected` to "Not connected" and that was lying.
        if self
            .last_error
            .as_deref()
            .is_some_and(|e| e.contains("session_expired"))
        {
            return "disconnected";
        }
        let Some(last_ok) = self.last_success_at_ms else {
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
        BrokerError::VenueStopped { until_ms } => {
            if let Some(ms) = until_ms {
                format!("venue_stopped:until_ms={ms}")
            } else {
                "venue_stopped".to_string()
            }
        }
    }
}

fn is_venue_stop(err: &BrokerError) -> bool {
    matches!(err, BrokerError::VenueStopped { .. })
}

fn engine_until_ms(slot: Option<&str>) -> Option<i64> {
    slot.and_then(|s| crate::egress::shared_engine().posture_for(s))
        .and_then(|p| p.until_ms)
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
    slot: Option<&str>,
) {
    let entry = completeness.get_mut(class);
    entry.current = false;
    entry.last_error_category = Some(err.category().to_string());
    let from_engine = engine_until_ms(slot);

    match err {
        BrokerError::RateLimited { retry_after_ms } => {
            let until = from_engine.unwrap_or_else(|| {
                retry_after_ms.unwrap_or_else(|| {
                    Utc::now().timestamp_millis() + cfg.rate_limit_default_backoff_ms
                })
            });
            entry.rate_limited_until_ms = Some(until);
        }
        BrokerError::VenueStopped { until_ms } => {
            entry.rate_limited_until_ms = from_engine.or(*until_ms);
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

/// How long the poll loop should wait (and skip `poll_fills`) for a banned/backoff
/// slot. Cap at 60s so a 5-minute ban cannot hang a test. `None` means poll.
fn venue_stop_sleep(posture: &crate::egress::VenuePosture, now_ms: i64) -> Option<Duration> {
    if posture.posture != "banned" && posture.posture != "backoff" {
        return None;
    }
    let remaining_ms = (posture.until_ms.unwrap_or(now_ms) - now_ms).max(0) as u64;
    Some(Duration::from_millis(remaining_ms.min(60_000)))
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
    account_book: Arc<std::sync::Mutex<AccountBook>>,
    nfo_master: Arc<Mutex<KotakNfoScripMaster>>,
    instruments: Option<Arc<InstrumentStore>>,
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

            if let Some(slot) = slug_to_slot(adapter.name()) {
                let engine = crate::egress::shared_engine();
                if let Some(p) = engine.posture_for(slot) {
                    if let Some(wait) = venue_stop_sleep(&p, engine.now_ms()) {
                        tokio::time::sleep(wait).await;
                        continue;
                    }
                }
            }

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
            let slot = slug_to_slot(adapter.name());
            let adapter_name = slug_to_slot(adapter.name()).unwrap_or(adapter.name());

            if let Ok(fills) = &fills_result {
                let path = fills_provenance_path(adapter_name);
                let split = if adapter_name == "kotak_neo" {
                    let master = nfo_master
                        .lock()
                        .expect("kotak nfo scrip master mutex poisoned");
                    let mut split =
                        split_fills_by_book(adapter_name, fills.clone(), Some(&*master));
                    if let Some(nfo_rows) = split.get_mut(crate::data::KOTAK_NSE_NFO_BOOK_ID) {
                        stamp_nfo_fills(nfo_rows, &*master);
                    }
                    split
                } else if adapter_name == "zerodha_kite" {
                    let mut split = split_fills_by_book(adapter_name, fills.clone(), None);
                    if let Some(store) = instruments.as_ref() {
                        if let Some(nfo_rows) = split.get_mut(crate::data::ZERODHA_NSE_NFO_BOOK_ID) {
                            stamp_zerodha_kite_nfo_fills(nfo_rows, store);
                        }
                    }
                    split
                } else {
                    split_fills_by_book(adapter_name, fills.clone(), None)
                };
                {
                    let mut book = account_book.lock().expect("account_book mutex poisoned");
                    match adapter_name {
                        "binance_com" => {
                            let rows = split
                                .get(crate::data::BINANCE_COM_SPOT_BOOK_ID)
                                .cloned()
                                .unwrap_or_default();
                            book.replace_fills(
                                crate::data::BINANCE_COM_SPOT_BOOK_ID,
                                rows,
                                path,
                                ok_ms,
                            );
                        }
                        "kotak_neo" => {
                            for book_id in [
                                crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID,
                                crate::data::KOTAK_NSE_NFO_BOOK_ID,
                                crate::data::KOTAK_NSE_CDS_BOOK_ID,
                                crate::data::KOTAK_MCX_FUTURE_BOOK_ID,
                            ] {
                                let rows = split.get(book_id).cloned().unwrap_or_default();
                                book.replace_fills(book_id, rows, path, ok_ms);
                            }
                        }
                        "zerodha_kite" => {
                            for book_id in [
                                crate::data::ZERODHA_NSE_BSE_CASH_BOOK_ID,
                                crate::data::ZERODHA_NSE_NFO_BOOK_ID,
                            ] {
                                let rows = split.get(book_id).cloned().unwrap_or_default();
                                book.replace_fills(book_id, rows, path, ok_ms);
                            }
                        }
                        "upstox" => {
                            for book_id in [
                                crate::data::UPSTOX_NSE_BSE_CASH_BOOK_ID,
                                crate::data::UPSTOX_NSE_NFO_BOOK_ID,
                            ] {
                                let rows = split.get(book_id).cloned().unwrap_or_default();
                                book.replace_fills(book_id, rows, path, ok_ms);
                            }
                        }
                        "fyers" => {
                            for book_id in [
                                crate::data::FYERS_NSE_BSE_CASH_BOOK_ID,
                                crate::data::FYERS_NSE_NFO_BOOK_ID,
                            ] {
                                let rows = split.get(book_id).cloned().unwrap_or_default();
                                book.replace_fills(book_id, rows, path, ok_ms);
                            }
                        }
                        _ => {
                            for (book_id, book_fills) in &split {
                                book.replace_fills(book_id, book_fills.clone(), path, ok_ms);
                            }
                        }
                    }
                }
                let merge_book = merge_poll_book_id(adapter_name);
                let merge_fills = split.get(merge_book).cloned().unwrap_or_default();
                new_fills = match store.merge_poll(&merge_fills) {
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
                    mark_class_success(
                        &mut st.data_classes,
                        BrokerDataClass::FillsTradeHistory,
                        ok_ms,
                    );
                    class_failures.insert(BrokerDataClass::FillsTradeHistory, 0);
                } else if let Err(e) = &fills_result {
                    if !is_venue_stop(e) {
                        any_class_error = true;
                    }
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::FillsTradeHistory)
                        .or_insert(0);
                    mark_class_failure(
                        &mut st.data_classes,
                        BrokerDataClass::FillsTradeHistory,
                        e,
                        &cfg,
                        count,
                        slot,
                    );
                }

                if balances_result.is_ok() {
                    mark_class_success(
                        &mut st.data_classes,
                        BrokerDataClass::BalancesHoldings,
                        ok_ms,
                    );
                    class_failures.insert(BrokerDataClass::BalancesHoldings, 0);
                    st.last_balances = balances_result.as_ref().ok().cloned();
                    if adapter_name == "binance_com" {
                        if let Some(balances) = balances_result.as_ref().ok() {
                            let mut book =
                                account_book.lock().expect("account_book mutex poisoned");
                            book.replace_funds(
                                crate::data::BINANCE_COM_SPOT_BOOK_ID,
                                balances.clone(),
                                "/api/v3/account",
                                ok_ms,
                            );
                        }
                    }
                } else if let Err(e) = &balances_result {
                    if !is_venue_stop(e) {
                        any_class_error = true;
                    }
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::BalancesHoldings)
                        .or_insert(0);
                    mark_class_failure(
                        &mut st.data_classes,
                        BrokerDataClass::BalancesHoldings,
                        e,
                        &cfg,
                        count,
                        slot,
                    );
                }

                if open_orders_result.is_ok() {
                    mark_class_success(&mut st.data_classes, BrokerDataClass::OpenOrders, ok_ms);
                    class_failures.insert(BrokerDataClass::OpenOrders, 0);
                    if adapter_name == "binance_com" {
                        if let Some(orders) = open_orders_result.as_ref().ok() {
                            let mut book =
                                account_book.lock().expect("account_book mutex poisoned");
                            book.replace_orders(
                                crate::data::BINANCE_COM_SPOT_BOOK_ID,
                                orders.clone(),
                                "/api/v3/openOrders",
                                ok_ms,
                            );
                        }
                    }
                } else if let Err(e) = &open_orders_result {
                    if !is_venue_stop(e) {
                        any_class_error = true;
                    }
                    last_err_msg = Some(broker_err_as_str(e));
                    let count = class_failures
                        .entry(BrokerDataClass::OpenOrders)
                        .or_insert(0);
                    mark_class_failure(
                        &mut st.data_classes,
                        BrokerDataClass::OpenOrders,
                        e,
                        &cfg,
                        count,
                        slot,
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::BrokerError;
    use crate::broker_data_class::{
        BrokerDataClass, BrokerDataClassCompleteness, BrokerOpenOrdersSnapshot,
    };
    use crate::event_bus::AgentEvent;
    use async_trait::async_trait;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;

    struct AlwaysVenueStoppedAdapter;

    #[async_trait]
    impl BrokerAdapter for AlwaysVenueStoppedAdapter {
        fn name(&self) -> &'static str {
            "always_venue_stopped"
        }

        async fn poll_fills(
            &self,
            _since: Option<DateTime<Utc>>,
        ) -> Result<Vec<BrokerFill>, BrokerError> {
            Err(BrokerError::VenueStopped { until_ms: None })
        }
    }

    struct AlwaysHttpFailAdapter;

    #[async_trait]
    impl BrokerAdapter for AlwaysHttpFailAdapter {
        fn name(&self) -> &'static str {
            "always_http_fail"
        }

        async fn poll_fills(
            &self,
            _since: Option<DateTime<Utc>>,
        ) -> Result<Vec<BrokerFill>, BrokerError> {
            Err(BrokerError::Http("unit-fail".into()))
        }

        async fn poll_balances_holdings(&self) -> Result<BrokerBalancesSnapshot, BrokerError> {
            Err(BrokerError::Http("unit-fail".into()))
        }

        async fn poll_open_orders(&self) -> Result<BrokerOpenOrdersSnapshot, BrokerError> {
            Err(BrokerError::Http("unit-fail".into()))
        }
    }

    fn fail_cfg() -> BrokerSyncConfig {
        BrokerSyncConfig {
            initial_startup_delay: Duration::ZERO,
            base_poll_interval: Duration::from_millis(15),
            coalesce_window: Duration::from_millis(10),
            failure_escalate_after: 3,
            backoff_tick_1: Duration::from_millis(20),
            backoff_tick_2: Duration::from_millis(20),
            failures_until_open: 5,
            ..BrokerSyncConfig::default()
        }
    }

    fn temp_store() -> RecentTradesStore {
        let path = std::env::temp_dir().join(format!(
            "rta-venue-stop-{}-{}.db",
            std::process::id(),
            ulid::Ulid::new()
        ));
        RecentTradesStore::open(&path).expect("recent trades store")
    }

    #[test]
    fn mark_class_failure_venue_stopped_does_not_open_circuit_after_five() {
        let kill_before = crate::dns_block::is_block_active();
        let mut completeness = BrokerDataClassCompleteness::default();
        let cfg = BrokerSyncConfig::default();
        let mut class_failures = 0u32;
        let err = BrokerError::VenueStopped { until_ms: None };
        for _ in 0..5 {
            mark_class_failure(
                &mut completeness,
                BrokerDataClass::FillsTradeHistory,
                &err,
                &cfg,
                &mut class_failures,
                None,
            );
        }
        assert_eq!(class_failures, 0);
        assert!(
            !completeness.fills_trade_history.requires_manual_retry,
            "VenueStopped must not open the class circuit"
        );
        assert_eq!(crate::dns_block::is_block_active(), kill_before);
    }

    #[test]
    fn live_session_trade_book_not_ok_is_stale_not_disconnected() {
        // Founder symptom 2026-09-18: Kotak session live (quotes/holdings) but
        // Notch "Not connected" because fills poll got `stat: Not_Ok` and
        // last_success_at_ms stayed None.
        let st = BrokerRuntimeState {
            broker_connected: true,
            last_success_at_ms: None,
            last_error: Some("adapter: kotak_neo trade_book_not_ok (Not_Ok)".into()),
            ..BrokerRuntimeState::default()
        };
        assert_eq!(st.sync_state_literal(15, 60), "stale");
    }

    #[test]
    fn session_expired_error_is_disconnected_even_if_start_flag_still_true() {
        let st = BrokerRuntimeState {
            broker_connected: true,
            last_success_at_ms: None,
            last_error: Some("adapter: kotak_neo session_expired".into()),
            ..BrokerRuntimeState::default()
        };
        assert_eq!(st.sync_state_literal(15, 60), "disconnected");
    }

    #[test]
    fn stop_without_session_is_not_connected() {
        let st = BrokerRuntimeState::default();
        assert_eq!(st.sync_state_literal(15, 60), "not_connected");
    }

    #[test]
    fn circuit_open_is_disconnected() {
        let st = BrokerRuntimeState {
            broker_connected: true,
            circuit_open: true,
            last_error: Some("adapter: kotak_neo trade_book_not_ok (Not_Ok)".into()),
            ..BrokerRuntimeState::default()
        };
        assert_eq!(st.sync_state_literal(15, 60), "disconnected");
    }

    #[test]
    fn mark_class_failure_http_still_opens_after_five() {
        let mut completeness = BrokerDataClassCompleteness::default();
        let cfg = BrokerSyncConfig::default();
        let mut class_failures = 0u32;
        let err = BrokerError::Http("unit-fail".into());
        for _ in 0..5 {
            mark_class_failure(
                &mut completeness,
                BrokerDataClass::FillsTradeHistory,
                &err,
                &cfg,
                &mut class_failures,
                None,
            );
        }
        assert_eq!(class_failures, 5);
        assert!(completeness.fills_trade_history.requires_manual_retry);
    }

    async fn run_polls(
        adapter: Arc<dyn BrokerAdapter>,
        cfg: BrokerSyncConfig,
    ) -> (BrokerRuntimeState, Vec<String>) {
        let store = temp_store();
        let since = Arc::new(RwLock::new(None));
        let bus = EventBus::new(64);
        let mut rx = bus.subscribe();
        let status_arc = Arc::new(Mutex::new(BrokerRuntimeState::default()));
        let account_book = Arc::new(Mutex::new(AccountBook::new()));
        let (tx, mut coalesce_rx) = mpsc::channel(8);
        let cancel = Arc::new(AtomicBool::new(false));
        let handle = spawn_broker_poll_loop(
            adapter,
            store,
            since,
            cfg,
            bus,
            status_arc.clone(),
            tx,
            None,
            None,
            Some(cancel.clone()),
            None,
            account_book,
            Arc::new(Mutex::new(KotakNfoScripMaster::empty())),
            None,
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
        cancel.store(true, Ordering::Relaxed);
        handle.abort();
        while coalesce_rx.try_recv().is_ok() {}

        let mut classes = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if let AgentEvent::BrokerSyncState { payload } = ev {
                if let Some(c) = payload.get("class").and_then(|v| v.as_str()) {
                    classes.push(c.to_string());
                }
            }
        }
        let snap = status_arc.lock().expect("status").clone();
        (snap, classes)
    }

    #[tokio::test]
    async fn mark_class_failure_five_venue_stopped_keeps_poll_circuit_closed() {
        let kill_before = crate::dns_block::is_block_active();
        let (snap, classes) = run_polls(Arc::new(AlwaysVenueStoppedAdapter), fail_cfg()).await;
        assert!(
            !snap.circuit_open,
            "five VenueStopped must not open the circuit, consecutive={}",
            snap.consecutive_failures
        );
        assert!(
            !classes.iter().any(|c| c == "disconnected"),
            "VenueStopped must not emit disconnected SSE, got {classes:?}"
        );
        assert_eq!(crate::dns_block::is_block_active(), kill_before);
    }

    #[tokio::test]
    async fn mark_class_failure_http_poll_still_opens_after_five() {
        let (snap, _) = run_polls(Arc::new(AlwaysHttpFailAdapter), fail_cfg()).await;
        assert!(
            snap.circuit_open,
            "ordinary Http must still open after 5, consecutive={}",
            snap.consecutive_failures
        );
    }

    fn posture(posture: &'static str, until_ms: Option<i64>) -> crate::egress::VenuePosture {
        crate::egress::VenuePosture {
            venue: "binance_com".into(),
            posture,
            until_ms,
            meters: vec![],
        }
    }

    #[test]
    fn venue_stop_sleep_banned_waits_remaining_capped() {
        let now = 1_000_000;
        assert_eq!(
            venue_stop_sleep(&posture("banned", Some(now + 10_000)), now),
            Some(Duration::from_millis(10_000))
        );
        assert_eq!(
            venue_stop_sleep(&posture("banned", Some(now + 300_000)), now),
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn venue_stop_sleep_backoff_waits_remaining() {
        let now = 1_000_000;
        assert_eq!(
            venue_stop_sleep(&posture("backoff", Some(now + 4_000)), now),
            Some(Duration::from_millis(4_000))
        );
    }

    #[test]
    fn venue_stop_sleep_live_polls_now() {
        let now = 1_000_000;
        assert_eq!(venue_stop_sleep(&posture("live", None), now), None);
        assert_eq!(
            venue_stop_sleep(&posture("live", Some(now + 10_000)), now),
            None
        );
    }
}

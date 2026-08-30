//! One bound instrument per COM capability. Replace, never insert.

use crate::egress::ban::reconnect_backoff_ms;
use crate::egress::{EgressError, VenuePosture};
use std::future::Future;
use std::time::Duration;
use tokio::sync::watch;

const COM_WS_RECONNECT_BASE_MS: i64 = 2_000;
const COM_WS_RECONNECT_CAP_MS: i64 = 120_000;

/// Single-slot bind. `bind(B)` after `bind(A)` leaves B only — not a set of A+B.
#[derive(Clone)]
pub struct MarketBind {
    tx: watch::Sender<Option<String>>,
}

impl MarketBind {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(None);
        Self { tx }
    }

    /// REPLACE never insert. `send_replace` so it works with zero receivers.
    pub fn bind(&self, id: Option<String>) {
        let _ = self.tx.send_replace(id);
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<String>> {
        self.tx.subscribe()
    }

    pub fn current(&self) -> Option<String> {
        self.tx.borrow().clone()
    }
}

impl Default for MarketBind {
    fn default() -> Self {
        Self::new()
    }
}

/// How long a COM WS loop should sleep (and skip `connect_async`) for a
/// banned/backoff slot. Cap at 60s like `venue_stop_sleep`. `None` means live.
pub fn com_ws_stop_sleep(posture: &VenuePosture, now_ms: i64) -> Option<Duration> {
    if posture.posture != "banned" && posture.posture != "backoff" {
        return None;
    }
    let remaining_ms = (posture.until_ms.unwrap_or(now_ms) - now_ms).max(0) as u64;
    Some(Duration::from_millis(remaining_ms.min(60_000)))
}

/// Same numbers as the depth reconnect ladder. Venue `until_ms` outranks backoff.
pub(crate) fn com_ws_reconnect_wait_ms(
    err: Option<&anyhow::Error>,
    attempt: u32,
    now_ms: i64,
) -> i64 {
    if let Some(until) = err.and_then(|e| {
        e.chain()
            .find_map(|cause| cause.downcast_ref::<EgressError>())
            .and_then(|e| e.until_ms())
    }) {
        let wait = until.saturating_sub(now_ms);
        if wait > 0 {
            return wait;
        }
    }
    reconnect_backoff_ms(attempt, COM_WS_RECONNECT_BASE_MS, COM_WS_RECONNECT_CAP_MS)
}

/// Park on `None` (no `connect_async`). Freeze-stop before `run_one`. A later
/// `bind` cancels the in-flight `run_one` by abandoning it.
pub async fn run_bound_com_ws_loop<F, Fut>(mut rx: watch::Receiver<Option<String>>, mut run_one: F)
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<(), anyhow::Error>>,
{
    let mut attempt = 0u32;
    loop {
        let id = loop {
            {
                let current = rx.borrow_and_update().clone();
                if let Some(id) = current.filter(|s| !s.is_empty()) {
                    break id;
                }
            }
            if rx.changed().await.is_err() {
                return;
            }
        };

        if let Some(p) = crate::egress::shared_engine().posture_for("binance_com") {
            if let Some(wait) = com_ws_stop_sleep(&p, crate::egress::shared_engine().now_ms()) {
                tokio::time::sleep(wait).await;
                continue;
            }
        }

        tokio::select! {
            changed = rx.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            result = run_one(id) => {
                let err = match result {
                    Ok(()) => {
                        attempt = 0;
                        None
                    }
                    Err(err) => {
                        attempt = attempt.saturating_add(1);
                        Some(err)
                    }
                };
                let wait = com_ws_reconnect_wait_ms(
                    err.as_ref(),
                    attempt,
                    crate::egress::shared_engine().now_ms(),
                );
                tokio::select! {
                    changed = rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(wait.max(0) as u64)) => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    fn posture(posture: &'static str, until_ms: Option<i64>) -> VenuePosture {
        VenuePosture {
            venue: "binance_com".into(),
            posture,
            until_ms,
            meters: vec![],
        }
    }

    #[test]
    fn bind_a_then_b_current_and_receiver_are_b_only() {
        let bind = MarketBind::new();
        bind.bind(Some("A".into()));
        bind.bind(Some("B".into()));
        assert_eq!(bind.current().as_deref(), Some("B"));
        let rx = bind.subscribe();
        assert_eq!(rx.borrow().as_deref(), Some("B"));
        assert_ne!(bind.current().as_deref(), Some("A"));
    }

    #[test]
    fn bind_none_parks_current_empty() {
        let bind = MarketBind::new();
        bind.bind(Some("btcusdt".into()));
        bind.bind(None);
        assert!(bind.current().is_none());
    }

    #[test]
    fn com_ws_stop_sleep_banned_waits_remaining_capped() {
        let now = 1_000_000;
        assert_eq!(
            com_ws_stop_sleep(&posture("banned", Some(now + 10_000)), now),
            Some(Duration::from_millis(10_000))
        );
        assert_eq!(
            com_ws_stop_sleep(&posture("banned", Some(now + 300_000)), now),
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn com_ws_stop_sleep_backoff_waits_remaining() {
        let now = 1_000_000;
        assert_eq!(
            com_ws_stop_sleep(&posture("backoff", Some(now + 4_000)), now),
            Some(Duration::from_millis(4_000))
        );
    }

    #[test]
    fn com_ws_stop_sleep_live_is_none() {
        let now = 1_000_000;
        assert_eq!(com_ws_stop_sleep(&posture("live", None), now), None);
        assert_eq!(
            com_ws_stop_sleep(&posture("live", Some(now + 10_000)), now),
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bind_replace_cancels_the_running_connection() {
        let bind = MarketBind::new();
        let a_cancelled = Arc::new(AtomicBool::new(false));
        let a_started = Arc::new(tokio::sync::Notify::new());
        let rx = bind.subscribe();
        let a_cancelled_task = a_cancelled.clone();
        let a_started_task = a_started.clone();

        let handle = tokio::spawn(async move {
            run_bound_com_ws_loop(rx, move |id| {
                let a_cancelled = a_cancelled_task.clone();
                let a_started = a_started_task.clone();
                async move {
                    struct Guard(Arc<AtomicBool>);
                    impl Drop for Guard {
                        fn drop(&mut self) {
                            self.0.store(true, Ordering::SeqCst);
                        }
                    }
                    let _guard = if id == "A" {
                        a_started.notify_one();
                        Some(Guard(a_cancelled))
                    } else {
                        None
                    };
                    std::future::pending::<()>().await;
                    #[allow(unreachable_code)]
                    Ok(())
                }
            })
            .await;
        });

        bind.bind(Some("A".into()));
        tokio::time::timeout(Duration::from_secs(2), a_started.notified())
            .await
            .expect("run_one(A) should start");
        bind.bind(Some("B".into()));
        tokio::time::timeout(Duration::from_secs(2), async {
            while !a_cancelled.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("bind B must cancel run_one(A)");
        assert!(a_cancelled.load(Ordering::SeqCst));
        handle.abort();
    }
}

//! Phase 9 (#66) — loopback Prometheus text exposition (design §11.2).

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct AgentMetrics {
    agent_uptime_seconds: AtomicU64,
    sse_agent_health: AtomicU64,
    sse_toolbar_show: AtomicU64,
    sse_broker_sync_state: AtomicU64,
    sse_kill_switch_state: AtomicU64,
    sse_auth_state: AtomicU64,
    sse_session_state: AtomicU64,
    hmac_verify_failures_total: AtomicU64,
}

impl AgentMetrics {
    pub fn set_uptime_secs(&self, secs: u64) {
        self.agent_uptime_seconds.store(secs, Ordering::Relaxed);
    }

    pub fn sse_published_for_type(&self, event_type: &str) {
        match event_type {
            "agent_health" => {
                self.sse_agent_health.fetch_add(1, Ordering::Relaxed);
            }
            "toolbar_show" => {
                self.sse_toolbar_show.fetch_add(1, Ordering::Relaxed);
            }
            "broker_sync_state" => {
                self.sse_broker_sync_state.fetch_add(1, Ordering::Relaxed);
            }
            "kill_switch_state" => {
                self.sse_kill_switch_state.fetch_add(1, Ordering::Relaxed);
            }
            "auth_state" => {
                self.sse_auth_state.fetch_add(1, Ordering::Relaxed);
            }
            "session_state" => {
                self.sse_session_state.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub fn record_hmac_verify_failure(&self) {
        self.hmac_verify_failures_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn render_prometheus(&self) -> String {
        format!(
            "# HELP agent_uptime_seconds Agent process uptime from heartbeat ticker\n\
             # TYPE agent_uptime_seconds gauge\n\
             agent_uptime_seconds {}\n\
             # HELP sse_events_published_total SSE events emitted by type\n\
             # TYPE sse_events_published_total counter\n\
             sse_events_published_total{{type=\"agent_health\"}} {}\n\
             sse_events_published_total{{type=\"toolbar_show\"}} {}\n\
             sse_events_published_total{{type=\"broker_sync_state\"}} {}\n\
             sse_events_published_total{{type=\"kill_switch_state\"}} {}\n\
             sse_events_published_total{{type=\"auth_state\"}} {}\n\
             sse_events_published_total{{type=\"session_state\"}} {}\n\
             # HELP hmac_verify_failures_total Wire HMAC verification failures\n\
             # TYPE hmac_verify_failures_total counter\n\
             hmac_verify_failures_total {}\n",
            self.agent_uptime_seconds.load(Ordering::Relaxed),
            self.sse_agent_health.load(Ordering::Relaxed),
            self.sse_toolbar_show.load(Ordering::Relaxed),
            self.sse_broker_sync_state.load(Ordering::Relaxed),
            self.sse_kill_switch_state.load(Ordering::Relaxed),
            self.sse_auth_state.load(Ordering::Relaxed),
            self.sse_session_state.load(Ordering::Relaxed),
            self.hmac_verify_failures_total.load(Ordering::Relaxed),
        )
    }

    pub fn snapshot_json(&self) -> serde_json::Value {
        serde_json::json!({
            "agent_uptime_seconds": self.agent_uptime_seconds.load(Ordering::Relaxed),
            "sse_events_published": {
                "agent_health": self.sse_agent_health.load(Ordering::Relaxed),
                "toolbar_show": self.sse_toolbar_show.load(Ordering::Relaxed),
                "broker_sync_state": self.sse_broker_sync_state.load(Ordering::Relaxed),
                "kill_switch_state": self.sse_kill_switch_state.load(Ordering::Relaxed),
                "auth_state": self.sse_auth_state.load(Ordering::Relaxed),
                "session_state": self.sse_session_state.load(Ordering::Relaxed),
            },
            "hmac_verify_failures_total": self.hmac_verify_failures_total.load(Ordering::Relaxed),
        })
    }
}

async fn metrics_handler(metrics: Arc<AgentMetrics>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        metrics.render_prometheus(),
    )
}

pub fn spawn_metrics_server(metrics: Arc<AgentMetrics>, port: u16) {
    if port == 0 {
        return;
    }
    tokio::spawn(async move {
        let m = metrics.clone();
        let app = Router::new().route(
            "/metrics",
            get(move || {
                let m = m.clone();
                async move { metrics_handler(m).await }
            }),
        );
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let Ok(listener) = tokio::net::TcpListener::bind(addr).await else {
            tracing::warn!(%port, "metrics bind failed — observability port unavailable");
            return;
        };
        tracing::info!(%addr, "tradeautopsy-agent metrics listening");
        let _ = axum::serve(listener, app).await;
    });
}

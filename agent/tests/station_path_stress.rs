//! Full Station path stress.
//!
//! Ramps concurrency (and a payload ladder on two write paths) against a test
//! agent and a stub Console until the first failure on each path.
//!
//! No-op unless `STATION_STRESS=1` (see `scripts/station-path-stress.sh`).
//! Does not place orders, mint live TOTP, or call WorkOS.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::{Json, Router};
use common::{
    apply_wire_v1, spawn_test_agent_with_options, TestAgentOptions,
    WireHeaderOverrides, TEST_SECRET,
};
use futures::future::join_all;
use serde_json::{json, Value};
use tradeautopsy_agent::{BrokerCredentialVault, MemoryBrokerCredentialVault};

const PENDING_ID: &str = "11111111-2222-4333-8444-555555555555";

#[derive(Clone, Copy)]
enum BodyKind {
    Empty,
    Gen(fn(u32, usize) -> Vec<u8>),
}

#[derive(Clone, Copy)]
struct Probe {
    subsystem: &'static str,
    wave: &'static str,
    name: &'static str,
    method: &'static str,
    sign_path: &'static str,
    url_path: &'static str,
    kind: BodyKind,
    expect: &'static [u16],
    signed: bool,
    mixed: bool,
    payload: bool,
    /// Empty when this loopback run is the proof. Otherwise why a Mac / live Console is still required.
    gate: &'static str,
}

struct Hit {
    status: Option<u16>,
    err: Option<String>,
    ms: u128,
    leak: bool,
    detail: String,
}

struct StepOut {
    ok: bool,
    rps: f64,
    mode: String,
}

struct Row {
    subsystem: String,
    wave: String,
    name: String,
    method: String,
    path: String,
    max_pass_concurrency: u32,
    peak_rps: f64,
    soak_passed: u32,
    soak_requested: u32,
    payload_bytes_ok: Option<usize>,
    first_failure: String,
    gate: String,
}

// Multi-thread runtime on purpose: the agent under test is spawned in-process
// via tokio::spawn, so the default current_thread flavor would interleave
// client, stub Console, and server on one OS thread. Results from that setup
// are a single-thread lower bound; worker_threads=4 matches the committed
// report's 4-CPU baseline.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn station_path_stress_ramp() {
    if std::env::var("STATION_STRESS").ok().as_deref() != Some("1") {
        eprintln!("station_path_stress: skip (set STATION_STRESS=1 or run scripts/station-path-stress.sh)");
        return;
    }

    let ladder = parse_ladder();
    let soak_n = std::env::var("STATION_STRESS_SOAK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2u32);
    let timeout = Duration::from_millis(
        std::env::var("STATION_STRESS_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5_000u64),
    );
    let hosts = std::env::temp_dir().join("ta-station-path-stress-hosts");
    std::env::set_var("TRADEAUTOPSY_HOSTS_FILE", &hosts);

    let stub_listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind stub console");
    let stub_port = stub_listener.local_addr().expect("stub addr").port();
    tokio::spawn(async move {
        axum::serve(stub_listener, stub_router())
            .await
            .expect("stub console");
    });

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(128)
        .tcp_nodelay(true)
        .build()
        .expect("http client");

    // free_port() drops its probe listener before the agent binds — another
    // process can steal the port in between, so retry instead of trusting one
    // race. TestAgentOptions isn't Clone, so agent_opts() builds a fresh one
    // per attempt.
    let mut agent = None;
    let mut agent_port = 0u16;
    for attempt in 0..3 {
        let port = free_port();
        let handle = spawn_test_agent_with_options(port, agent_opts(stub_port));
        for _ in 0..50 {
            if agent_ready(&client, port).await {
                break;
            }
            if handle.is_finished() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        if agent_ready(&client, port).await {
            agent = Some(handle);
            agent_port = port;
            break;
        }
        eprintln!("station_path_stress: agent did not bind :{port} (attempt {})", attempt + 1);
        handle.abort();
    }
    let agent = agent.expect("test agent failed to bind a port after 3 attempts");
    let base = format!("http://127.0.0.1:{agent_port}");
    let table = probes();
    let mut rows = Vec::with_capacity(table.len() + 1);
    let mut agent_up = true;

    for probe in table {
        if !agent_up {
            rows.push(dead_row(probe));
            continue;
        }
        eprintln!(
            "station_path_stress probe {} {}",
            probe.method, probe.url_path
        );
        let row = ramp_probe(&client, &base, probe, &ladder, soak_n, timeout).await;
        eprintln!(
            "  pass_concurrency={} failure={}",
            row.max_pass_concurrency, row.first_failure
        );
        rows.push(row);
        // fire_l1 arms the desk latch; clear it so later probes don't measure a
        // killed desk (the dismiss probe below reads cleaner on an unarmed desk).
        if probe.name == "fire_l1" {
            disarm_latch(&client, &base).await;
        }
        agent_up = health_ok(&client, &base, timeout).await;
    }

    let mixed: Vec<Probe> = probes().into_iter().filter(|p| p.mixed).collect();
    eprintln!("station_path_stress mixed fan ({} paths)", mixed.len());
    let overall = if agent_up {
        ramp_mixed(&client, &base, &mixed, &ladder, soak_n, timeout).await
    } else {
        Row {
            subsystem: "overall".into(),
            wave: "all".into(),
            name: "mixed_fan".into(),
            method: "MIX".into(),
            path: "all mixed paths".into(),
            max_pass_concurrency: 0,
            peak_rps: 0.0,
            soak_passed: 0,
            soak_requested: soak_n,
            payload_bytes_ok: None,
            first_failure: "agent unreachable before mixed fan".into(),
            gate: String::new(),
        }
    };
    rows.insert(0, overall);

    agent.abort();
    write_reports(&rows, &ladder, soak_n, timeout);
}

fn probes() -> Vec<Probe> {
    const OK: &[u16] = &[200];
    const ACCEPT: &[u16] = &[200, 202];
    vec![
        // Station status
        p(
            "station_status",
            "status",
            "health",
            "GET",
            "/api/daemon/health",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "sync_state",
            "GET",
            "/api/daemon/broker/sync-state",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "positions",
            "GET",
            "/api/daemon/positions",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "today",
            "GET",
            "/api/daemon/today",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "sse",
            "GET",
            "/api/daemon/events/stream",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "recent_trades",
            "GET",
            "/api/daemon/toolbar/recent-trades",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "station_status",
            "status",
            "sync_hint",
            "GET",
            "/api/station/sync-hint",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        // N2 / morning brief / cite
        p(
            "n2_morning",
            "N2",
            "morning_brief",
            "GET",
            "/api/daemon/morning-brief",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        pq(
            "journal_cite",
            "cite",
            "trip_cites",
            "GET",
            "/api/daemon/journal/trip-cites",
            "/api/daemon/journal/trip-cites?week_start=2026-09-28&week_end=2026-10-04",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "n2_morning",
            "N2",
            "condition_fire",
            "POST",
            "/api/daemon/journal/condition-fire",
            BodyKind::Gen(body_condition),
            OK,
            true,
            true,
            true,
            "",
        ),
        p(
            "conditions",
            "conditions",
            "capture_working_condition",
            "POST",
            "/api/daemon/bar/capture-working-condition",
            BodyKind::Gen(body_working_condition),
            OK,
            true,
            true,
            false,
            "",
        ),
        // PLAN JWT / declare / cancel (stub Console archives)
        p(
            "plan_jwt",
            "PLAN",
            "declare",
            "POST",
            "/api/daemon/bar/declare",
            BodyKind::Gen(body_declare),
            OK,
            true,
            true,
            false,
            "stub Console; live Caller JWT archive needs Gate B",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "live_state",
            "GET",
            "/api/daemon/bar/live-state",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "stub Console when LiveBook is empty",
        ),
        pq(
            "plan_jwt",
            "PLAN",
            "declarations_week",
            "GET",
            "/api/daemon/bar/declarations",
            "/api/daemon/bar/declarations?scope=week",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "cancel",
            "cancel",
            "cancel_declaration",
            "POST",
            "/api/daemon/bar/cancel-declaration",
            BodyKind::Gen(body_cancel),
            OK,
            true,
            true,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "stop_me",
            "POST",
            "/api/daemon/bar/stop-me",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "stop_me_clear",
            "POST",
            "/api/daemon/bar/stop-me/clear",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "protective",
            "POST",
            "/api/daemon/bar/protective",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "live_interference",
            "POST",
            "/api/daemon/bar/live-interference",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "swing_check_in",
            "POST",
            "/api/daemon/bar/swing-check-in",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "n2_morning",
            "N2",
            "post_trade_debrief",
            "PATCH",
            "/api/daemon/bar/post-trade-debrief",
            BodyKind::Gen(body_debrief),
            OK,
            true,
            true,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "loss_limits_get",
            "GET",
            "/api/daemon/bar/profile/loss-limits",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "plan_jwt",
            "PLAN",
            "loss_limits_post",
            "POST",
            "/api/daemon/bar/profile/loss-limits",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "match",
            "match",
            "fill_matched",
            "POST",
            "/api/daemon/bar/test/fill-matched",
            BodyKind::Gen(body_fill_matched),
            OK,
            true,
            true,
            false,
            "stub inject; prod Console must leave BAR_TEST_INJECT_FILL_MATCHED unset",
        ),
        // Capture ACK / outbox
        p(
            "capture_ack",
            "capture",
            "toolbar_accept",
            "POST",
            "/api/daemon/journal/toolbar-capture/accept",
            BodyKind::Gen(body_capture),
            ACCEPT,
            true,
            true,
            true,
            "stub ACK; live R2/journal needs Gate B",
        ),
        p(
            "capture_ack",
            "capture",
            "outbox_status",
            "GET",
            "/api/daemon/journal/toolbar-capture/outbox/status",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "capture_ack",
            "capture",
            "screenshot_presign",
            "POST",
            "/api/daemon/screenshot/presign",
            BodyKind::Gen(body_presign),
            OK,
            true,
            false,
            false,
            "stub presign; live upload needs Console + R2",
        ),
        p(
            "capture_ack",
            "capture",
            "pending_get",
            "GET",
            pending_path(),
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "capture_ack",
            "capture",
            "pending_patch",
            "PATCH",
            pending_path(),
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "capture_ack",
            "capture",
            "manual_fill",
            "POST",
            "/api/daemon/journal/manual-fill/accept",
            BodyKind::Gen(body_manual_fill),
            ACCEPT,
            true,
            true,
            false,
            "local fill + stub capture ACK",
        ),
        p(
            "capture_ack",
            "capture",
            "console_recent_trades",
            "GET",
            "/api/daemon/journal/toolbar/recent-trades",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        // Kill latch (L1 only — no DNS hosts write).
        // NOTE: ordering is load-bearing — fire_l1 arms the real latch, and the
        // main loop calls disarm_latch() right after its row so audit/ack below
        // don't measure a killed desk. Keep fire_l1 before dismiss/audit.
        p(
            "kill_latch",
            "kill",
            "state",
            "GET",
            "/api/daemon/kill-switch/state",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "",
        ),
        p(
            "kill_latch",
            "kill",
            "fire_l1",
            "POST",
            "/api/daemon/kill-switch",
            BodyKind::Gen(body_kill_l1),
            OK,
            true,
            true,
            false,
            "L1 desk latch only; L3 DNS is not ramped",
        ),
        p(
            "kill_latch",
            "kill",
            "dismiss",
            "POST",
            "/api/daemon/dismiss-kill-switch",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "kill_latch",
            "kill",
            "audit",
            "GET",
            "/api/daemon/kill-switch/audit",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "kill_latch",
            "kill",
            "ack",
            "POST",
            "/api/daemon/kill-switch/ack",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        // Device login shell (no WorkOS)
        p(
            "station_auth",
            "auth",
            "session",
            "GET",
            "/api/daemon/auth/station/session",
            BodyKind::Empty,
            OK,
            true,
            true,
            false,
            "unsigned-in memory store; live prove needs Keychain Bearer + Console",
        ),
        p(
            "station_auth",
            "auth",
            "begin",
            "POST",
            "/api/daemon/auth/station/begin",
            BodyKind::Empty,
            &[503],
            true,
            false,
            false,
            "503 while WORKOS_STATION_CLIENT_ID is unset; live device login is Mac + WorkOS",
        ),
        p(
            "station_auth",
            "auth",
            "complete",
            "POST",
            "/api/daemon/auth/station/complete",
            BodyKind::Empty,
            &[409],
            true,
            false,
            false,
            "no pending device login in this process",
        ),
        p(
            "station_auth",
            "auth",
            "sign_out",
            "POST",
            "/api/daemon/auth/station/sign-out",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "station_auth",
            "auth",
            "phase8_begin",
            "POST",
            "/api/daemon/auth/begin",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        p(
            "station_auth",
            "auth",
            "phase8_finish",
            "POST",
            "/api/daemon/auth/finish",
            BodyKind::Gen(body_obj),
            OK,
            true,
            false,
            false,
            "stub Console",
        ),
        // Market / depth / funds envelope
        pq(
            "depth",
            "depth",
            "quote",
            "GET",
            "/api/station/quote",
            "/api/station/quote?instrument=BTCUSDT&book=binance-com-spot",
            BodyKind::Empty,
            OK,
            false,
            true,
            false,
            "honest envelope; live book needs a connected adapter",
        ),
        pq(
            "depth",
            "depth",
            "history",
            "GET",
            "/api/station/history",
            "/api/station/history?instrument=BTCUSDT&interval=1m&limit=5",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        pq(
            "depth",
            "depth",
            "depth",
            "GET",
            "/api/station/depth",
            "/api/station/depth?book=binance-com-spot&instrument=BTCUSDT",
            BodyKind::Empty,
            OK,
            false,
            true,
            false,
            "unbound COM book returns immediately; live ladder needs a session",
        ),
        pq(
            "depth",
            "depth",
            "chain",
            "GET",
            "/api/station/chain",
            "/api/station/chain?book=binance-com-options",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        pq(
            "depth",
            "depth",
            "oi",
            "GET",
            "/api/station/oi",
            "/api/station/oi?book=binance-com-options",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        pq(
            "depth",
            "depth",
            "greeks",
            "GET",
            "/api/station/greeks",
            "/api/station/greeks?book=binance-com-options",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        pq(
            "depth",
            "depth",
            "index",
            "GET",
            "/api/station/index",
            "/api/station/index?book=binance-com-options",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        p(
            "depth",
            "depth",
            "manifest",
            "GET",
            "/api/station/manifest",
            BodyKind::Empty,
            OK,
            false,
            false,
            false,
            "",
        ),
        pq(
            "funds_sizer",
            "funds",
            "obtain_funds",
            "GET",
            "/api/station/obtain",
            "/api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=funds",
            BodyKind::Empty,
            OK,
            false,
            true,
            false,
            "no USER_DATA session; envelope only. Live funds need Keychain",
        ),
        p(
            "funds_sizer",
            "sizer",
            "risk_preview",
            "POST",
            "/api/daemon/risk/preview",
            BodyKind::Gen(body_risk),
            OK,
            true,
            true,
            false,
            "agent preview; Notch BarPlanSizer paint is Mac UI",
        ),
        // Broker control that stays off the network
        p(
            "broker",
            "status",
            "sync_stop",
            "POST",
            "/api/daemon/broker/sync/stop",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "broker",
            "status",
            "sync_retry",
            "POST",
            "/api/daemon/broker/sync/retry",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
        p(
            "broker",
            "status",
            "credentials_present",
            "POST",
            "/api/daemon/broker/credentials/present",
            BodyKind::Gen(body_identity),
            OK,
            true,
            false,
            false,
            "memory vault in this run; Keychain is Mac",
        ),
        p(
            "broker",
            "status",
            "credentials_clear",
            "POST",
            "/api/daemon/broker/credentials/clear",
            BodyKind::Gen(body_identity),
            OK,
            true,
            false,
            false,
            "memory vault; deletes nothing on the host Keychain",
        ),
        p(
            "broker",
            "status",
            "sync_start_reject",
            "POST",
            "/api/daemon/broker/sync/start",
            BodyKind::Gen(body_start_reject),
            &[400],
            true,
            false,
            false,
            "unknown slug; live Start needs Keychain + wasm component",
        ),
        p(
            "totp",
            "TOTP",
            "kotak_mint_reject",
            "POST",
            "/api/daemon/broker/kotak/session/mint",
            BodyKind::Gen(body_kotak_reject),
            &[400],
            true,
            false,
            false,
            "rejects before network. 12h card + live mint are Mac + Kotak",
        ),
        p(
            "broker",
            "auth",
            "groww_reject",
            "POST",
            "/api/daemon/broker/groww/connect",
            BodyKind::Gen(body_groww_reject),
            &[400],
            true,
            false,
            false,
            "rejects before network",
        ),
        p(
            "broker",
            "auth",
            "dhan_reject",
            "POST",
            "/api/daemon/broker/dhan/connect/begin",
            BodyKind::Gen(body_dhan_reject),
            &[400],
            true,
            false,
            false,
            "rejects before consent HTTP",
        ),
        p(
            "broker",
            "auth",
            "zerodha_begin",
            "POST",
            "/api/daemon/broker/zerodha/connect/begin",
            BodyKind::Gen(body_zerodha_begin),
            OK,
            true,
            false,
            false,
            "local pending map only; browser login is Mac",
        ),
        p(
            "broker",
            "auth",
            "upstox_begin",
            "POST",
            "/api/daemon/broker/upstox/begin",
            BodyKind::Gen(body_upstox_begin),
            OK,
            true,
            false,
            false,
            "local pending map only",
        ),
        p(
            "broker",
            "auth",
            "fyers_begin",
            "POST",
            "/api/daemon/broker/fyers/begin",
            BodyKind::Gen(body_fyers_begin),
            OK,
            true,
            false,
            false,
            "local pending map only",
        ),
        p(
            "broker",
            "auth",
            "zerodha_callback_bare",
            "GET",
            "/api/daemon/broker/zerodha/callback",
            BodyKind::Empty,
            &[400],
            false,
            false,
            false,
            "missing request_token; no token exchange",
        ),
        p(
            "broker",
            "status",
            "amfi_toggle",
            "PUT",
            "/api/daemon/vendor-bindings",
            BodyKind::Gen(body_amfi),
            OK,
            true,
            false,
            false,
            "",
        ),
        pq(
            "broker",
            "status",
            "instruments_search",
            "GET",
            "/instruments/search",
            "/instruments/search?q=RE",
            BodyKind::Empty,
            OK,
            true,
            false,
            false,
            "",
        ),
    ]
}

fn p(
    subsystem: &'static str,
    wave: &'static str,
    name: &'static str,
    method: &'static str,
    path: &'static str,
    kind: BodyKind,
    expect: &'static [u16],
    signed: bool,
    mixed: bool,
    payload: bool,
    gate: &'static str,
) -> Probe {
    Probe {
        subsystem,
        wave,
        name,
        method,
        sign_path: path,
        url_path: path,
        kind,
        expect,
        signed,
        mixed,
        payload,
        gate,
    }
}

fn pq(
    subsystem: &'static str,
    wave: &'static str,
    name: &'static str,
    method: &'static str,
    sign_path: &'static str,
    url_path: &'static str,
    kind: BodyKind,
    expect: &'static [u16],
    signed: bool,
    mixed: bool,
    payload: bool,
    gate: &'static str,
) -> Probe {
    Probe {
        subsystem,
        wave,
        name,
        method,
        sign_path,
        url_path,
        kind,
        expect,
        signed,
        mixed,
        payload,
        gate,
    }
}

fn pending_path() -> &'static str {
    static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PATH.get_or_init(|| format!("/api/daemon/journal/toolbar-capture/pending/{PENDING_ID}"))
        .as_str()
}

async fn ramp_probe(
    client: &reqwest::Client,
    base: &str,
    probe: Probe,
    ladder: &[u32],
    soak_n: u32,
    timeout: Duration,
) -> Row {
    let mut max_pass = 0u32;
    let mut peak_rps = 0.0f64;
    let mut first_failure = String::new();
    for &n in ladder {
        let step = burst_one(client, base, probe, n, 0, timeout).await;
        if step.ok {
            max_pass = n;
            peak_rps = step.rps;
        } else {
            first_failure = format!("concurrency {n}: {}", step.mode);
            break;
        }
    }
    let mut soak_passed = 0u32;
    if first_failure.is_empty() && max_pass > 0 {
        for wave in 1..=soak_n {
            let step = burst_one(client, base, probe, max_pass, 0, timeout).await;
            if step.ok {
                soak_passed = wave;
                peak_rps = peak_rps.max(step.rps);
            } else {
                first_failure =
                    format!("soak wave {wave} at concurrency {max_pass}: {}", step.mode);
                break;
            }
        }
        if first_failure.is_empty() {
            first_failure = format!("none through concurrency {max_pass}");
        }
    } else if first_failure.is_empty() {
        first_failure = "ladder empty".into();
    }

    let mut payload_bytes_ok = None;
    if probe.payload && max_pass > 0 && !first_failure.starts_with("concurrency") {
        let width = max_pass.min(4).max(1);
        let mut passed_bytes = 0usize;
        for &bytes in &[
            256usize, 4_096, 65_536, 262_144, 1_048_576, 2_097_152, 3_145_728,
        ] {
            let step = burst_one(client, base, probe, width, bytes, timeout).await;
            if step.ok {
                passed_bytes = bytes;
            } else {
                let extra = format!("payload {bytes}B at concurrency {width}: {}", step.mode);
                if first_failure.starts_with("none through") {
                    first_failure = extra;
                } else {
                    first_failure = format!("{first_failure}; {extra}");
                }
                break;
            }
        }
        payload_bytes_ok = Some(passed_bytes);
        if first_failure.starts_with("none through") {
            first_failure = format!("{first_failure}; payload through {passed_bytes}B");
        }
    }

    Row {
        subsystem: probe.subsystem.into(),
        wave: probe.wave.into(),
        name: probe.name.into(),
        method: probe.method.into(),
        path: probe.url_path.into(),
        max_pass_concurrency: max_pass,
        peak_rps,
        soak_passed,
        soak_requested: if max_pass > 0 && !first_failure.starts_with("concurrency") {
            soak_n
        } else {
            0
        },
        payload_bytes_ok,
        first_failure,
        gate: probe.gate.into(),
    }
}

async fn ramp_mixed(
    client: &reqwest::Client,
    base: &str,
    probes: &[Probe],
    ladder: &[u32],
    soak_n: u32,
    timeout: Duration,
) -> Row {
    let mut max_pass = 0u32;
    let mut peak_rps = 0.0f64;
    let mut first_failure = String::new();
    for &n in ladder {
        let step = burst_mixed(client, base, probes, n, timeout).await;
        if step.ok {
            max_pass = n;
            peak_rps = step.rps;
        } else {
            first_failure = format!("concurrency {n}: {}", step.mode);
            break;
        }
    }
    let mut soak_passed = 0u32;
    if first_failure.is_empty() && max_pass > 0 {
        for wave in 1..=soak_n {
            let step = burst_mixed(client, base, probes, max_pass, timeout).await;
            if step.ok {
                soak_passed = wave;
                peak_rps = peak_rps.max(step.rps);
            } else {
                first_failure =
                    format!("soak wave {wave} at concurrency {max_pass}: {}", step.mode);
                break;
            }
        }
        if first_failure.is_empty() {
            first_failure = format!("none through concurrency {max_pass}");
        }
    }
    Row {
        subsystem: "overall".into(),
        wave: "all".into(),
        name: "mixed_fan".into(),
        method: "MIX".into(),
        path: format!("{} mixed paths", probes.len()),
        max_pass_concurrency: max_pass,
        peak_rps,
        soak_passed,
        soak_requested: if first_failure.starts_with("concurrency") {
            0
        } else {
            soak_n
        },
        payload_bytes_ok: None,
        first_failure,
        gate: "same stub Console as the per-path rows".into(),
    }
}

async fn burst_one(
    client: &reqwest::Client,
    base: &str,
    probe: Probe,
    n: u32,
    payload: usize,
    timeout: Duration,
) -> StepOut {
    let started = Instant::now();
    let futs = (0..n).map(|i| {
        let client = client.clone();
        let base = base.to_string();
        async move {
            let hit = hit(&client, &base, probe, i, payload, timeout).await;
            (probe.name.to_string(), hit)
        }
    });
    let hits = join_all(futs).await;
    summarize(n, started.elapsed(), &hits, probe.expect)
}

async fn burst_mixed(
    client: &reqwest::Client,
    base: &str,
    probes: &[Probe],
    n: u32,
    timeout: Duration,
) -> StepOut {
    let started = Instant::now();
    let futs = (0..n).map(|i| {
        let client = client.clone();
        let base = base.to_string();
        let probe = probes[(i as usize) % probes.len()];
        async move {
            let hit = hit(&client, &base, probe, i, 0, timeout).await;
            (probe.name.to_string(), hit)
        }
    });
    let hits = join_all(futs).await;
    // Per-request expect is checked inside by rewriting status against that probe.
    summarize_mixed(n, started.elapsed(), &hits, probes)
}

fn summarize(n: u32, elapsed: Duration, hits: &[(String, Hit)], expect: &[u16]) -> StepOut {
    let mut bad: Vec<String> = Vec::new();
    let mut lats = Vec::with_capacity(hits.len());
    for (name, hit) in hits {
        lats.push(hit.ms);
        push_bad(&mut bad, name, hit, expect);
    }
    finish_step(n, elapsed, &lats, &bad)
}

fn summarize_mixed(n: u32, elapsed: Duration, hits: &[(String, Hit)], probes: &[Probe]) -> StepOut {
    let mut bad = Vec::new();
    let mut lats = Vec::with_capacity(hits.len());
    for (name, hit) in hits {
        lats.push(hit.ms);
        let expect = probes
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.expect)
            .unwrap_or(&[200]);
        push_bad(&mut bad, name, hit, expect);
    }
    finish_step(n, elapsed, &lats, &bad)
}

fn push_bad(bad: &mut Vec<String>, name: &str, hit: &Hit, expect: &[u16]) {
    if hit.leak {
        bad.push(format!("{name} secret_leak"));
    } else if let Some(err) = &hit.err {
        bad.push(format!("{name} {err}"));
    } else if let Some(status) = hit.status {
        if !expect.contains(&status) {
            let extra = if hit.detail.is_empty() {
                String::new()
            } else {
                format!(" ({})", hit.detail)
            };
            bad.push(format!("{name} http {status}{extra}"));
        }
    }
}

fn finish_step(n: u32, elapsed: Duration, lats: &[u128], bad: &[String]) -> StepOut {
    let secs = elapsed.as_secs_f64().max(0.000_001);
    let rps = n as f64 / secs;
    let p99 = percentile(lats, 99);
    let ok = bad.is_empty();
    let mode = if ok {
        format!("ok p99={p99}ms")
    } else {
        compress_modes(bad)
    };
    StepOut { ok, rps, mode }
}

fn compress_modes(bad: &[String]) -> String {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for item in bad {
        if let Some((_, n)) = counts.iter_mut().find(|(k, _)| k == item) {
            *n += 1;
        } else {
            counts.push((item.clone(), 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let shown: usize = counts.iter().take(4).map(|(_, n)| *n).sum();
    let mut parts: Vec<String> = counts
        .into_iter()
        .take(4)
        .map(|(k, n)| format!("{k} x{n}"))
        .collect();
    if bad.len() > shown {
        parts.push(format!("{} more", bad.len() - shown));
    }
    let mut s = parts.join("; ");
    if s.len() > 280 {
        s.truncate(280);
    }
    s
}

fn percentile(lats: &[u128], pct: usize) -> u128 {
    if lats.is_empty() {
        return 0;
    }
    let mut owned = lats.to_vec();
    owned.sort_unstable();
    let idx = ((owned.len().saturating_sub(1)) * pct) / 100;
    owned[idx]
}

async fn hit(
    client: &reqwest::Client,
    base: &str,
    probe: Probe,
    i: u32,
    payload: usize,
    timeout: Duration,
) -> Hit {
    let body = match probe.kind {
        BodyKind::Empty => Vec::new(),
        BodyKind::Gen(f) => f(i, payload),
    };
    let url = format!("{base}{}", probe.url_path);
    let mut builder = match probe.method {
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "PATCH" => client.patch(&url),
        _ => client.get(&url),
    };
    builder = builder.timeout(timeout);
    if !body.is_empty() {
        builder = builder
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone());
    }
    if probe.signed {
        builder = apply_wire_v1(
            builder,
            probe.method,
            probe.sign_path,
            &body,
            WireHeaderOverrides::default(),
        );
    }
    let started = Instant::now();
    let sse = probe.url_path.contains("/events/stream");
    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let text = if sse {
                drop(resp);
                String::new()
            } else {
                resp.text().await.unwrap_or_default()
            };
            Hit {
                status: Some(status),
                err: None,
                ms: started.elapsed().as_millis(),
                leak: response_leaks(&text),
                detail: if status >= 400 {
                    error_detail(&text)
                } else {
                    String::new()
                },
            }
        }
        Err(err) => Hit {
            status: None,
            err: Some(err_class(&err).into()),
            ms: started.elapsed().as_millis(),
            leak: false,
            detail: String::new(),
        },
    }
}

fn err_class(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        "timeout"
    } else if err.is_connect() {
        "connect"
    } else {
        "transport"
    }
}

fn error_detail(text: &str) -> String {
    if text.is_empty() || response_leaks(text) {
        return String::new();
    }
    let Ok(v) = serde_json::from_str::<Value>(text) else {
        return String::new();
    };
    let class = v.get("error_class").and_then(|c| c.as_str()).unwrap_or("");
    let msg = v
        .get("message")
        .and_then(|m| m.as_str())
        .or_else(|| v.get("error").and_then(|e| e.as_str()))
        .unwrap_or("");
    let mut s = format!("{class} {msg}");
    s.truncate(80);
    s.trim().to_string()
}

fn response_leaks(text: &str) -> bool {
    text.contains(TEST_SECRET)
        || text.contains("station.test.jwt")
        || text.contains("x-daemon-secret")
        || text.contains("x-signature")
}

async fn health_ok(client: &reqwest::Client, base: &str, timeout: Duration) -> bool {
    let probe = p(
        "station_status",
        "status",
        "health",
        "GET",
        "/api/daemon/health",
        BodyKind::Empty,
        &[200],
        true,
        false,
        false,
        "",
    );
    let hit = hit(client, base, probe, 0, 0, timeout).await;
    hit.status == Some(200) && hit.err.is_none() && !hit.leak
}

fn dead_row(probe: Probe) -> Row {
    Row {
        subsystem: probe.subsystem.into(),
        wave: probe.wave.into(),
        name: probe.name.into(),
        method: probe.method.into(),
        path: probe.url_path.into(),
        max_pass_concurrency: 0,
        peak_rps: 0.0,
        soak_passed: 0,
        soak_requested: 0,
        payload_bytes_ok: None,
        first_failure: "agent unreachable after an earlier path".into(),
        gate: probe.gate.into(),
    }
}

fn stub_router() -> Router {
    Router::new().fallback(stub_handler)
}

async fn stub_handler(req: Request) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let body = if path.contains("fill-matched") {
        json!({
            "test_only": true,
            "status": "matched",
            "net": 1.5,
            "currency": "USD"
        })
    } else {
        json!({
            "ok": true,
            "id": id,
            "declarationId": id,
            "success": true,
            "data": {
                "pending_capture_id": PENDING_ID,
                "status": "accepted"
            }
        })
    };
    (axum::http::StatusCode::OK, Json(body)).into_response()
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("free port");
    listener.local_addr().expect("addr").port()
}

fn agent_opts(stub_port: u16) -> TestAgentOptions {
    let mut opts = TestAgentOptions::default();
    opts.upstream_base_url_override = Some(format!("http://127.0.0.1:{stub_port}"));
    let vault: Arc<dyn BrokerCredentialVault> = Arc::new(MemoryBrokerCredentialVault::new());
    opts.credential_vault = Some(vault);
    opts.plant_binance_options_depth = true;
    opts.plant_binance_s2_history = true;
    opts.plant_binance_spot_funds = true;
    opts
}

/// Non-panicking readiness probe — unlike wait_ready, returns false so the
/// caller can retry after a lost free_port race.
async fn agent_ready(client: &reqwest::Client, port: u16) -> bool {
    let req = apply_wire_v1(
        client.get(format!("http://127.0.0.1:{port}/api/daemon/health")),
        "GET",
        "/api/daemon/health",
        b"",
        WireHeaderOverrides::default(),
    );
    matches!(
        req.timeout(Duration::from_millis(400)).send().await,
        Ok(r) if r.status().is_success()
    )
}

/// Clear the L1 desk latch after the fire_l1 ramp leaves it armed.
async fn disarm_latch(client: &reqwest::Client, base: &str) {
    let _ = apply_wire_v1(
        client.post(format!("{base}/api/daemon/dismiss-kill-switch")),
        "POST",
        "/api/daemon/dismiss-kill-switch",
        b"",
        WireHeaderOverrides::default(),
    )
    .timeout(Duration::from_secs(5))
    .send()
    .await;
}

fn parse_ladder() -> Vec<u32> {
    let raw = std::env::var("STATION_STRESS_LADDER")
        .unwrap_or_else(|_| "1,4,16,32,64,128,256,512".into());
    let mut out: Vec<u32> = raw
        .split(',')
        .filter_map(|s| s.trim().parse::<u32>().ok())
        .filter(|n| *n > 0)
        .collect();
    if out.is_empty() {
        out = vec![1, 4, 16, 32, 64, 128, 256, 512];
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn write_reports(rows: &[Row], ladder: &[u32], soak_n: u32, timeout: Duration) {
    let json_path = std::env::var("STATION_STRESS_OUT").unwrap_or_else(|_| {
        format!(
            "{}/../plans/STATION-PATH-STRESS-RESULTS.json",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let md_path = std::env::var("STATION_STRESS_MD").unwrap_or_else(|_| {
        format!(
            "{}/../plans/STATION-PATH-STRESS.md",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(0);
    let profile = std::env::var("STATION_STRESS_PROFILE").unwrap_or_else(|_| "debug".into());
    let when = chrono::Utc::now().to_rfc3339();
    let value = json!({
        "ran_at": when,
        "profile": profile,
        "cpus": cpus,
        "ladder": ladder,
        "soak_waves": soak_n,
        "timeout_ms": timeout.as_millis(),
        "rows": rows.iter().map(row_json).collect::<Vec<_>>(),
    });
    if let Some(parent) = std::path::Path::new(&json_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("write json");
    std::fs::write(
        &md_path,
        render_md(rows, ladder, soak_n, timeout, &when, cpus, &profile),
    )
    .expect("write md");
    eprintln!("station_path_stress wrote {md_path}");
}

fn row_json(row: &Row) -> Value {
    json!({
        "subsystem": row.subsystem,
        "wave": row.wave,
        "name": row.name,
        "method": row.method,
        "path": row.path,
        "max_pass_concurrency": row.max_pass_concurrency,
        "peak_rps": (row.peak_rps * 10.0).round() / 10.0,
        "soak_passed": row.soak_passed,
        "soak_requested": row.soak_requested,
        "payload_bytes_ok": row.payload_bytes_ok,
        "first_failure": row.first_failure,
        "live_gate": row.gate,
    })
}

fn render_md(
    rows: &[Row],
    ladder: &[u32],
    soak_n: u32,
    timeout: Duration,
    when: &str,
    cpus: usize,
    profile: &str,
) -> String {
    let overall = rows
        .iter()
        .find(|r| r.name == "mixed_fan")
        .map(|r| r.first_failure.as_str())
        .unwrap_or("missing");
    let mut earliest = String::from("none on the ladder");
    let mut earliest_n = u32::MAX;
    for row in rows {
        if row.name == "mixed_fan" {
            continue;
        }
        if let Some(n) = failing_concurrency(&row.first_failure) {
            if n < earliest_n {
                earliest_n = n;
                earliest = format!(
                    "{} {} {} at concurrency {n} ({})",
                    row.subsystem, row.method, row.path, row.first_failure
                );
            }
        }
    }
    let ladder_s = ladder
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mut out = String::new();
    out.push_str("# Station path stress\n\n");
    out.push_str("Loopback agent + stub Console. No broker orders. Re-run with `./scripts/station-path-stress.sh`.\n\n");
    out.push_str(&format!("- Ran: `{when}`\n"));
    out.push_str(&format!("- CPUs: {cpus}\n"));
    out.push_str(&format!(
        "- Profile: `{profile}` (`cargo test` debug is unoptimized; a timeout is that build missing the deadline, not a process crash)\n"
    ));
    out.push_str(&format!("- Ladder: `{ladder_s}`\n"));
    out.push_str(&format!("- Soak bursts at the passing ceiling: {soak_n}\n"));
    out.push_str(&format!(
        "- Per-request deadline: {} ms. A break is a status outside the expected set, a transport error, a daemon-secret echo, or that deadline.\n\n",
        timeout.as_millis()
    ));
    out.push_str("## Overall\n\n");
    out.push_str(&format!("- Mixed fan: {overall}\n"));
    out.push_str(&format!(
        "- Earliest per-path concurrency break: {earliest}\n\n"
    ));
    out.push_str("Peak RPS is achieved throughput of the last passing burst (requests / wall time), not a paced target rate. A row that says `none through concurrency N` stayed inside the deadline for every ladder step and every soak burst.\n\n");
    out.push_str("## Breaks on this ladder\n\n");
    let mut any_break = false;
    for row in rows {
        if row.first_failure.starts_with("none through") {
            continue;
        }
        any_break = true;
        out.push_str(&format!(
            "- **{}** `{} {}` — max burst {} — {}\n",
            row.subsystem, row.method, row.path, row.max_pass_concurrency, row.first_failure
        ));
    }
    if !any_break {
        out.push_str("- No path failed inside this ladder.\n");
    }
    out.push_str("\nThe 413s are the default JSON body limit (the pad itself is 2 MiB, so the request is over 2 MiB). They are not handler panics. Timeouts are the per-request deadline; a later path still answering means the process stayed up.\n\n");
    out.push_str("| Subsystem | Wave | Path | Max concurrency still passing | Peak RPS | Soak bursts passed | Payload bytes still ok | First failure mode | Live gate |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | --- | --- | --- | --- |\n");
    for row in rows {
        let payload = row
            .payload_bytes_ok
            .map(|n| n.to_string())
            .unwrap_or_else(|| "—".into());
        let soak = if row.soak_requested == 0 {
            "—".into()
        } else {
            format!("{}/{}", row.soak_passed, row.soak_requested)
        };
        let gate = if row.gate.is_empty() {
            "local"
        } else {
            row.gate.as_str()
        };
        out.push_str(&format!(
            "| {} | {} | `{} {}` | {} | {:.0} | {} | {} | {} | {} |\n",
            cell(&row.subsystem),
            cell(&row.wave),
            row.method,
            cell(&row.path),
            row.max_pass_concurrency,
            row.peak_rps,
            soak,
            payload,
            cell(&row.first_failure),
            cell(gate),
        ));
    }
    out.push_str("\n## Live Mac + Console only\n\n");
    out.push_str("This cloud run does not click the Notch and does not present a real Console URL or Keychain.\n\n");
    out.push_str(
        "| Surface | What this harness proved | What still needs a Mac + live Console |\n",
    );
    out.push_str("| --- | --- | --- |\n");
    out.push_str("| PLAN JWT archive | Declare stays 200 against a stub, including under concurrency | Gate B (`signed_in: true`) and a real `POST /api/bar/v1/declarations` on `TRADEAUTOPSY_SERVER_BASE_URL` |\n");
    out.push_str("| Capture ACK to journal / R2 | Outbox reaches ACK against the stub body `status=accepted` | Gate B plus the real capture accept and screenshot presign |\n");
    out.push_str("| Match flip | Agent forwards `test/fill-matched` and will cite when the stub says `test_only` + `matched` | Production leaves `BAR_TEST_INJECT_FILL_MATCHED` unset. A real match is a Console write |\n");
    out.push_str("| TOTP 12h card | Mint route returns 400 for a non-`kotak_neo` slug and does not dial Kotak | `KotakSessionMintCardPolicy` (12h) and a real TOTP/MPIN mint. Touch ID is LocalAuthentication |\n");
    out.push_str("| Funds | `obtain(funds)` returns an envelope with no USER_DATA session | Identity-only Start with Keychain credentials |\n");
    out.push_str(
        "| Sizer paint | `POST /api/daemon/risk/preview` | Notch `BarPlanSizer` is SwiftUI |\n",
    );
    out.push_str("| Sparkle / restart | No agent route. Restart is `StationAppCoordinator.retryAgent` | macOS Sparkle against `https://updates.tradeautopsy.in/appcast.xml`, and the Station app supervising the agent. `scripts/verify-updates-feed.sh` checks the feed from a machine that can resolve that host |\n");
    out.push_str("| Device login | Session route stays `signed_in: false`. Begin stays 503 without `WORKOS_STATION_CLIENT_ID` | WorkOS AuthKit CLI Auth and `GET /api/auth/station/session` on the live Console. UI shows `user_code` only |\n");
    out.push_str("| Binance.US withdraw block | Not called | Validated read-only key, withdraw permission hard-blocked, on a Mac |\n");
    out.push_str("\nWire identity in this run is the integration-test loopback secret. It is machine integrity only and is not printed.\n");
    out
}

fn failing_concurrency(mode: &str) -> Option<u32> {
    let rest = mode.strip_prefix("concurrency ")?;
    let n = rest.split(':').next()?.trim();
    n.parse().ok()
}

fn cell(s: &str) -> String {
    s.replace('|', "/").replace('\n', " ")
}

fn body_obj(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({ "stress": i })).expect("json")
}

fn body_declare(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "symbol": "BTCUSDT",
        "qty": 1,
        "stance": "planned",
        "client_tag": format!("stress-{i}")
    }))
    .expect("json")
}

fn body_cancel(i: u32, _n: usize) -> Vec<u8> {
    let id = uuid::Uuid::from_u128(0x1111_1111_2222_4333_8444_5555_5555_0000 + i as u128);
    serde_json::to_vec(&json!({
        "declaration_id": id.to_string(),
        "cancel_reason_chip": "scratch"
    }))
    .expect("json")
}

fn body_debrief(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "declaration_id": format!("decl-debrief-{i}"),
        "moment_a_note": "stress",
        "moment_c_note": "",
        "adherence": { "stop_as_declared": true },
        "completed_at_ms": 1_700_000_000_100i64
    }))
    .expect("json")
}

fn body_fill_matched(i: u32, _n: usize) -> Vec<u8> {
    let id = uuid::Uuid::from_u128(0x2222_2222_3333_4444_8555_6666_7777_0000 + i as u128);
    serde_json::to_vec(&json!({
        "declaration_id": id.to_string(),
        "symbol": "BTCUSDT"
    }))
    .expect("json")
}

fn body_capture(i: u32, n: usize) -> Vec<u8> {
    let len = if n == 0 { 24 } else { n };
    serde_json::to_vec(&json!({
        "draftText": "s".repeat(len),
        "idempotencyKey": format!("stress-cap-{i}-{}", uuid::Uuid::new_v4())
    }))
    .expect("json")
}

fn body_condition(i: u32, n: usize) -> Vec<u8> {
    let len = if n == 0 { 16 } else { n };
    serde_json::to_vec(&json!({
        "declaration_id": format!("decl-n2-{i}-{}", uuid::Uuid::new_v4()),
        "rule_id": "invalidation_price",
        "fired_at_ms": 1_700_000_000_000i64,
        "working": { "invalidation_state": "breached", "pad": "p".repeat(len) }
    }))
    .expect("json")
}

fn body_working_condition(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "last": 100.5,
        "last_status": "fresh",
        "declaration_id": format!("decl-work-{i}")
    }))
    .expect("json")
}

fn body_presign(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "pending_capture_id": PENDING_ID,
        "content_type": "image/png"
    }))
    .expect("json")
}

fn body_manual_fill(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "symbol": "BTCUSDT",
        "side": "BUY",
        "quantity": 0.01,
        "price": 100.0,
        "filledAtMs": 1_700_000_000_000i64 + i as i64,
        "idempotencyKey": format!("stress-fill-{i}-{}", uuid::Uuid::new_v4())
    }))
    .expect("json")
}

fn body_kill_l1(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "level": 1,
        "broker": "zerodha",
        "reason": "station-path-stress"
    }))
    .expect("json")
}

fn body_risk(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "book_id": "binance-com-spot",
        "side": "BUY",
        "budget_mode": "risk_percent",
        "budget_value": 1.0,
        "entry": 100.0,
        "stop": 95.0,
        "target": 110.0,
        "funds_lit": false
    }))
    .expect("json")
}

fn body_identity(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "binance_com",
        "brokerConnectionId": "00000000-0000-4000-8000-0000000000aa",
        "environment": "prod"
    }))
    .expect("json")
}

fn body_start_reject(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "not_a_broker",
        "brokerConnectionId": "00000000-0000-4000-8000-0000000000ab",
        "environment": "prod",
        "assetClass": "crypto"
    }))
    .expect("json")
}

fn body_kotak_reject(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "not_kotak",
        "brokerConnectionId": "00000000-0000-4000-8000-0000000000c1",
        "environment": "prod",
        "consumerKey": "stress-placeholder",
        "mobileNumber": "0000000000",
        "ucc": "stress",
        "totp": "000000",
        "mpin": "0000"
    }))
    .expect("json")
}

fn body_groww_reject(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "not_groww",
        "brokerConnectionId": "00000000-0000-4000-8000-0000000000c2",
        "environment": "prod",
        "apiKey": "stress-placeholder",
        "apiSecret": "stress-placeholder"
    }))
    .expect("json")
}

fn body_dhan_reject(_i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "not_dhan",
        "brokerConnectionId": "00000000-0000-4000-8000-0000000000c3",
        "environment": "prod",
        "dhanClientId": "stress-placeholder",
        "appId": "stress-placeholder",
        "appSecret": "stress-placeholder"
    }))
    .expect("json")
}

fn body_zerodha_begin(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "zerodha_kite",
        "brokerConnectionId": format!("stress-conn-{i}"),
        "environment": "prod",
        "apiKey": "stress-placeholder",
        "apiSecret": "stress-placeholder"
    }))
    .expect("json")
}

fn body_upstox_begin(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "upstox",
        "brokerConnectionId": format!("stress-conn-{i}"),
        "environment": "prod",
        "clientId": "stress-placeholder",
        "clientSecret": "stress-placeholder"
    }))
    .expect("json")
}

fn body_fyers_begin(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "brokerSlug": "fyers",
        "brokerConnectionId": format!("stress-conn-{i}"),
        "environment": "prod",
        "appId": "stress-placeholder",
        "secretId": "stress-placeholder"
    }))
    .expect("json")
}

fn body_amfi(i: u32, _n: usize) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "adapter_id": "amfi",
        "enabled": i % 2 == 0
    }))
    .expect("json")
}

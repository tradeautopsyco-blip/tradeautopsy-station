//! Today session mirror — engine + signals + persistence orchestration.

use crate::broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
use crate::broker_sync_control::{BrokerRuntimeCardStatus, BrokerSyncController};
use crate::exchange_info::{live_com_filters_ready, ExchangeInfoSymbolCache};
use crate::recent_trades::RecentTradesStore;
use crate::round_trip_engine::{
    aggregate_known_pnl, is_aggregate_eligible, RoundTrip, RoundTripEngine,
};
use crate::today::signals::{analyze_signals, BehaviorSignal, SignalAnalysis};
use crate::today::store::{DailySnapshot, TodayStore};
use chrono::{Local, NaiveDate, Utc};
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TodayDegradedReason {
    SyncUnavailable,
    SyncStale,
    /// Live adapter cannot produce fills (CountingPoll / test stub). Not a quiet day.
    StubAdapter,
    /// I-S4: live COM started without LOT_SIZE / tickSize / notional filters loaded.
    ExchangeFiltersNotReady,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayHeroPayload {
    pub pnl_today_usd: Option<f64>,
    pub trades_today: Option<u32>,
    pub win_rate: Option<f64>,
    /// Eligible closed wins — share, don't refilter rows in Swift (Y3 / S2).
    pub wins_today: Option<u32>,
    pub losses_today: Option<u32>,
}

impl TodayHeroPayload {
    pub fn empty() -> Self {
        Self {
            pnl_today_usd: None,
            trades_today: None,
            win_rate: None,
            wins_today: None,
            losses_today: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySignalPayload {
    pub kind: String,
    pub severity: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayTradeRowPayload {
    pub closed_at: String,
    pub symbol: String,
    pub avg_entry: f64,
    pub avg_exit: f64,
    pub qty: f64,
    pub net_pnl_usd: Option<f64>,
    pub primary_flag: String,
    pub flag_severity: String,
    pub data_quality_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayPayload {
    pub local_date: String,
    pub performance_basis_not_tax: bool,
    pub degraded_reason: Option<TodayDegradedReason>,
    pub learning_baseline: bool,
    pub hero: TodayHeroPayload,
    pub top_signals: Vec<TodaySignalPayload>,
    pub trades: Vec<TodayTradeRowPayload>,
    pub open_position_count: u32,
    /// Active connection desk honesty (R7). Absent when no sync / unknown slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calc_profile_id: Option<String>,
}

pub struct TodayService {
    recent_trades: RecentTradesStore,
    store: TodayStore,
    engine: RoundTripEngine,
    broker_status: Arc<Mutex<BrokerRuntimeState>>,
    broker_sync_control: Arc<BrokerSyncController>,
    broker_limits: BrokerSyncConfig,
    exchange_cache: ExchangeInfoSymbolCache,
}

impl TodayService {
    pub fn new(
        recent_trades: RecentTradesStore,
        store: TodayStore,
        broker_status: Arc<Mutex<BrokerRuntimeState>>,
        broker_sync_control: Arc<BrokerSyncController>,
        broker_limits: BrokerSyncConfig,
        exchange_cache: ExchangeInfoSymbolCache,
    ) -> Self {
        Self {
            recent_trades,
            store,
            engine: RoundTripEngine::with_exchange_info(exchange_cache.clone(), true),
            broker_status,
            broker_sync_control,
            broker_limits,
            exchange_cache,
        }
    }

    pub fn refresh_from_fills(&self) -> anyhow::Result<()> {
        let fills = self.recent_trades.fetch_all_fills()?;
        let result = self.engine.reconstruct(fills);
        let today = local_today();
        let today_trips: Vec<RoundTrip> = result
            .round_trips
            .iter()
            .filter(|rt| local_date(rt.closed_at) == today)
            .cloned()
            .collect();
        let analysis = analyze_signals(&result.round_trips, &today_trips);
        let flagged: Vec<_> = result
            .round_trips
            .iter()
            .zip(analysis.trip_flags.iter())
            .map(|(rt, flag)| (rt.clone(), flag.clone()))
            .collect();
        self.store.replace_round_trips(&flagged)?;
        self.ensure_past_day_snapshots(&result.round_trips)?;
        self.store
            .upsert_daily_snapshot(&build_snapshot(today, &today_trips, &analysis))?;
        Ok(())
    }

    /// Leftover inventory from fills — same book Pulse / Today Open now consume (S2).
    pub fn open_inventory(&self) -> anyhow::Result<Vec<OpenInventoryRow>> {
        let fills = self.recent_trades.fetch_all_fills()?;
        Ok(open_inventory_from_fills(&fills))
    }

    pub fn build_payload(&self) -> anyhow::Result<TodayPayload> {
        let desk = self.active_desk_fields();
        if let Some(reason) = self.degraded_reason() {
            return Ok(degraded_payload(local_today(), reason, desk));
        }

        let fills = self.recent_trades.fetch_all_fills()?;
        let open_positions = count_open_positions(&fills);
        let result = self.engine.reconstruct(fills);
        let today = local_today();
        let today_trips: Vec<RoundTrip> = result
            .round_trips
            .iter()
            .filter(|rt| local_date(rt.closed_at) == today)
            .cloned()
            .collect();
        let analysis = analyze_signals(&result.round_trips, &today_trips);
        self.ensure_past_day_snapshots(&result.round_trips)?;

        let eligible: Vec<_> = today_trips
            .iter()
            .filter(|rt| is_aggregate_eligible(rt))
            .collect();
        let pnl = if eligible.is_empty() {
            None
        } else {
            Some(aggregate_known_pnl(&today_trips))
        };
        let trades_count = if today_trips.is_empty() {
            None
        } else {
            Some(eligible.len() as u32)
        };
        let wins = eligible
            .iter()
            .filter(|rt| rt.realized_pnl_usd.unwrap_or(0.0) > 0.0)
            .count() as u32;
        let losses = eligible
            .iter()
            .filter(|rt| rt.realized_pnl_usd.unwrap_or(0.0) < 0.0)
            .count() as u32;
        let win_rate = if eligible.is_empty() {
            None
        } else {
            Some(wins as f64 / eligible.len() as f64)
        };

        let trades = build_trade_rows(&today_trips, &result.round_trips, &analysis, 50);

        Ok(TodayPayload {
            local_date: today.format("%Y-%m-%d").to_string(),
            performance_basis_not_tax: true,
            degraded_reason: None,
            learning_baseline: analysis.learning_baseline,
            hero: TodayHeroPayload {
                pnl_today_usd: pnl,
                trades_today: trades_count,
                win_rate,
                wins_today: if eligible.is_empty() { None } else { Some(wins) },
                losses_today: if eligible.is_empty() {
                    None
                } else {
                    Some(losses)
                },
            },
            top_signals: analysis
                .top_signals
                .into_iter()
                .map(signal_to_payload)
                .collect(),
            trades,
            open_position_count: open_positions,
            broker_slug: desk.0,
            quote_currency: desk.1,
            calc_profile_id: desk.2,
        })
    }

    fn active_desk_fields(&self) -> (Option<String>, Option<String>, Option<String>) {
        let slug = self
            .broker_status
            .lock()
            .expect("broker status")
            .active_broker_slug
            .clone();
        match crate::ubi::desk_profile_for_slug(slug.as_deref()) {
            Some(p) => (
                Some(p.broker_slug),
                Some(p.quote_currency),
                Some(p.calc_profile_id),
            ),
            None => (slug, None, None),
        }
    }

    fn degraded_reason(&self) -> Option<TodayDegradedReason> {
        let card = self.broker_sync_control.card_status();
        if matches!(
            card,
            BrokerRuntimeCardStatus::Paused | BrokerRuntimeCardStatus::ReadyToStart
        ) {
            return Some(TodayDegradedReason::SyncUnavailable);
        }

        let snap = self.broker_status.lock().expect("broker status").clone();
        if !snap.broker_connected {
            return Some(TodayDegradedReason::SyncUnavailable);
        }

        if is_stub_fill_adapter(snap.backend_broker_label.as_deref()) {
            return Some(TodayDegradedReason::StubAdapter);
        }

        if !live_com_filters_ready(snap.active_broker_slug.as_deref(), &self.exchange_cache) {
            return Some(TodayDegradedReason::ExchangeFiltersNotReady);
        }

        let sync_state = snap.sync_state_literal(
            self.broker_limits.fresh_secs,
            self.broker_limits.stale_secs,
        );
        match sync_state {
            "stale" | "disconnected" | "not_connected" => Some(TodayDegradedReason::SyncStale),
            _ => None,
        }
    }

    fn ensure_past_day_snapshots(&self, all_trips: &[RoundTrip]) -> anyhow::Result<()> {
        let today = local_today();
        let mut past_dates: BTreeSet<NaiveDate> = BTreeSet::new();
        for rt in all_trips {
            let d = local_date(rt.closed_at);
            if d < today {
                past_dates.insert(d);
            }
        }
        for date in past_dates {
            let day_trips: Vec<_> = all_trips
                .iter()
                .filter(|rt| local_date(rt.closed_at) == date)
                .cloned()
                .collect();
            let analysis = analyze_signals(all_trips, &day_trips);
            self.store
                .upsert_daily_snapshot(&build_snapshot(date, &day_trips, &analysis))?;
        }
        Ok(())
    }
}

fn degraded_payload(
    local_date: NaiveDate,
    reason: TodayDegradedReason,
    desk: (Option<String>, Option<String>, Option<String>),
) -> TodayPayload {
    TodayPayload {
        local_date: local_date.format("%Y-%m-%d").to_string(),
        performance_basis_not_tax: true,
        degraded_reason: Some(reason),
        learning_baseline: true,
        hero: TodayHeroPayload::empty(),
        top_signals: vec![],
        trades: vec![],
        open_position_count: 0,
        broker_slug: desk.0,
        quote_currency: desk.1,
        calc_profile_id: desk.2,
    }
}

impl TodayPayload {
    pub fn unavailable(reason: TodayDegradedReason) -> Self {
        degraded_payload(local_today(), reason, (None, None, None))
    }
}

fn sort_round_trips_desc(trips: &mut [RoundTrip]) {
    trips.sort_by(|a, b| b.closed_at.cmp(&a.closed_at));
}

fn build_trade_rows(
    today_trips: &[RoundTrip],
    all_trips: &[RoundTrip],
    analysis: &SignalAnalysis,
    limit: usize,
) -> Vec<TodayTradeRowPayload> {
    let mut sorted: Vec<_> = today_trips.to_vec();
    sort_round_trips_desc(&mut sorted);
    sorted
        .into_iter()
        .take(limit)
        .filter_map(|rt| {
            let flag = all_trips
                .iter()
                .position(|t| t.closed_at == rt.closed_at && t.symbol == rt.symbol)
                .and_then(|i| analysis.trip_flags.get(i))?;
            Some(TodayTradeRowPayload {
                closed_at: rt.closed_at.to_rfc3339(),
                symbol: rt.symbol.clone(),
                avg_entry: rt.avg_entry_price,
                avg_exit: rt.avg_exit_price,
                qty: rt.qty,
                net_pnl_usd: rt.realized_pnl_usd,
                primary_flag: flag.label.clone(),
                flag_severity: flag.severity.as_str().to_string(),
                data_quality_flags: data_quality_flags(&rt),
            })
        })
        .collect()
}

fn build_snapshot(
    local_date: NaiveDate,
    today_trips: &[RoundTrip],
    analysis: &SignalAnalysis,
) -> DailySnapshot {
    let eligible: Vec<_> = today_trips
        .iter()
        .filter(|rt| is_aggregate_eligible(rt))
        .collect();
    let net_pnl = aggregate_known_pnl(today_trips);
    let wins = eligible
        .iter()
        .filter(|rt| rt.realized_pnl_usd.unwrap_or(0.0) > 0.0)
        .count() as u32;
    let losses = eligible.len() as u32 - wins;
    let unknown_basis_count = today_trips.iter().filter(|rt| rt.unknown_basis).count() as u32;

    DailySnapshot {
        local_date,
        round_trips_closed: today_trips.len() as u32,
        net_pnl_usd: net_pnl,
        wins,
        losses,
        discipline_index: 0.0,
        trades_count: eligible.len() as u32,
        learning_baseline: analysis.learning_baseline,
        unknown_basis_count,
    }
}

fn signal_to_payload(signal: BehaviorSignal) -> TodaySignalPayload {
    TodaySignalPayload {
        kind: signal_kind_str(signal.kind).to_string(),
        severity: signal.severity.as_str().to_string(),
        name: signal.kind.display_name().to_string(),
        description: signal.description,
    }
}

fn signal_kind_str(kind: crate::today::signals::SignalKind) -> &'static str {
    use crate::today::signals::SignalKind;
    match kind {
        SignalKind::LossChasing => "loss_chasing",
        SignalKind::Revenge => "revenge",
        SignalKind::Overtrading => "overtrading",
        SignalKind::PositionSizing => "position_sizing",
    }
}

fn local_today() -> NaiveDate {
    Local::now().date_naive()
}

fn local_date(ts: chrono::DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&Local).date_naive()
}

pub(crate) fn data_quality_flags(rt: &RoundTrip) -> Vec<String> {
    let mut flags = Vec::new();
    if rt.unknown_basis {
        flags.push("unknown_basis".to_string());
    }
    if rt.fee_unhandled {
        flags.push("fee_unhandled".to_string());
    }
    if rt.quote_not_usd {
        flags.push("quote_not_usd".to_string());
    }
    flags
}

#[derive(Debug, Clone)]
struct SideFill {
    symbol: String,
    side: String,
    qty: f64,
}

fn is_stub_fill_adapter(label: Option<&str>) -> bool {
    matches!(label, Some("counting_poll"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenInventoryRow {
    pub symbol: String,
    pub qty: f64,
    pub side: &'static str,
}

pub fn open_inventory_from_fills(fills: &[crate::broker::BrokerFill]) -> Vec<OpenInventoryRow> {
    use std::collections::BTreeMap;
    let mut qty_by_symbol: BTreeMap<String, f64> = BTreeMap::new();
    for fill in fills {
        let entry = qty_by_symbol.entry(fill.symbol.clone()).or_insert(0.0);
        if fill.side.eq_ignore_ascii_case("BUY") {
            *entry += fill.qty;
        } else if fill.side.eq_ignore_ascii_case("SELL") {
            *entry -= fill.qty;
        }
    }
    qty_by_symbol
        .into_iter()
        .filter(|(_, q)| q.abs() > 1e-12)
        .map(|(symbol, qty)| OpenInventoryRow {
            side: if qty > 0.0 { "LONG" } else { "SHORT" },
            symbol,
            qty: qty.abs(),
        })
        .collect()
}

fn count_open_positions(fills: &[crate::broker::BrokerFill]) -> u32 {
    open_inventory_from_fills(fills).len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn local_noon(date: NaiveDate) -> chrono::DateTime<Utc> {
        let local_dt = date.and_hms_opt(12, 0, 0).unwrap();
        Local.from_local_datetime(&local_dt)
            .single()
            .expect("local noon")
            .with_timezone(&Utc)
    }

    fn btc_trip(closed_at: chrono::DateTime<Utc>, pnl: f64) -> RoundTrip {
        RoundTrip {
            symbol: "BTCUSDT".into(),
            opened_at: closed_at - chrono::Duration::hours(1),
            closed_at,
            avg_entry_price: 60_000.0,
            avg_exit_price: 61_000.0,
            qty: 0.01,
            realized_pnl_usd: Some(pnl),
            fees_usd: Some(0.5),
            unknown_basis: false,
            fee_unhandled: false,
            quote_not_usd: false,
        }
    }

    #[test]
    fn build_trade_rows_sorted_most_recent_first() {
        let day = local_today();
        let t1 = btc_trip(local_noon(day) + chrono::Duration::hours(1), 1.0);
        let t2 = btc_trip(local_noon(day) + chrono::Duration::hours(3), 2.0);
        let t3 = btc_trip(local_noon(day) + chrono::Duration::hours(2), 3.0);
        let today_trips = vec![t1.clone(), t2.clone(), t3.clone()];
        let all = today_trips.clone();
        let analysis = analyze_signals(&all, &today_trips);
        let rows = build_trade_rows(&today_trips, &all, &analysis, 50);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].closed_at, t2.closed_at.to_rfc3339());
        assert_eq!(rows[1].closed_at, t3.closed_at.to_rfc3339());
        assert_eq!(rows[2].closed_at, t1.closed_at.to_rfc3339());
    }

    #[test]
    fn ensure_past_day_snapshots_writes_yesterday_without_today_fill() {
        let dir = std::env::temp_dir().join(format!(
            "rta-day-boundary-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("today.db");
        let _ = std::fs::remove_file(&db);
        let store = TodayStore::open(&db).expect("open today store");
        let today = local_today();
        let yesterday = today.pred_opt().expect("yesterday");
        let trip = btc_trip(local_noon(yesterday), 8.79);
        let service = TodayServiceHarness { store: store.clone() };
        service
            .ensure_past_day_snapshots(&[trip])
            .expect("finalize yesterday");
        let snap = store
            .fetch_daily_snapshot(yesterday)
            .expect("fetch")
            .expect("snapshot row");
        assert_eq!(snap.round_trips_closed, 1);
        assert!((snap.net_pnl_usd - 8.79).abs() < 0.01);
        assert!(store.fetch_daily_snapshot(today).expect("fetch today").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hero_metrics_empty_when_only_prior_day_trips_exist() {
        let today = local_today();
        let yesterday = today.pred_opt().expect("yesterday");
        let trip = btc_trip(local_noon(yesterday), 5.0);
        let today_trips: Vec<_> = [trip]
            .into_iter()
            .filter(|rt| local_date(rt.closed_at) == today)
            .collect();
        assert!(today_trips.is_empty());
        let eligible: Vec<_> = today_trips
            .iter()
            .filter(|rt| is_aggregate_eligible(rt))
            .collect();
        assert!(eligible.is_empty());
    }

    #[test]
    fn ensure_past_day_snapshots_writes_two_prior_days() {
        let dir = std::env::temp_dir().join(format!(
            "rta-day-boundary-two-{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("today.db");
        let _ = std::fs::remove_file(&db);
        let store = TodayStore::open(&db).expect("open today store");
        let today = local_today();
        let yesterday = today.pred_opt().expect("yesterday");
        let day_before = yesterday.pred_opt().expect("day before");
        let service = TodayServiceHarness { store: store.clone() };
        service
            .ensure_past_day_snapshots(&[
                btc_trip(local_noon(day_before), 1.0),
                btc_trip(local_noon(yesterday), 2.0),
            ])
            .expect("finalize past days");
        assert_eq!(
            store
                .fetch_daily_snapshot(day_before)
                .expect("fetch")
                .expect("row")
                .round_trips_closed,
            1
        );
        assert_eq!(
            store
                .fetch_daily_snapshot(yesterday)
                .expect("fetch")
                .expect("row")
                .round_trips_closed,
            1
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fill_near_local_midnight_assigned_to_correct_local_date() {
        // 23:30 UTC on Jan 15 → Jan 15 UTC, but Jan 16 in any east-of-UTC timezone.
        let ts = Utc.with_ymd_and_hms(2026, 1, 15, 23, 30, 0).unwrap();
        assert_eq!(local_date(ts), ts.with_timezone(&Local).date_naive());
        let offset_secs = Local::now().offset().local_minus_utc();
        if offset_secs > 0 {
            assert_ne!(
                local_date(ts),
                ts.date_naive(),
                "local_date must not use raw UTC day when offset is east of UTC"
            );
        }
    }

    #[test]
    fn data_quality_flags_reflects_all_active_honesty_flags() {
        let rt = RoundTrip {
            symbol: "ETHBTC".into(),
            opened_at: Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap(),
            closed_at: Utc.with_ymd_and_hms(2026, 7, 4, 12, 30, 0).unwrap(),
            avg_entry_price: 0.0,
            avg_exit_price: 0.05,
            qty: 1.0,
            realized_pnl_usd: None,
            fees_usd: None,
            unknown_basis: true,
            fee_unhandled: true,
            quote_not_usd: true,
        };
        let flags = data_quality_flags(&rt);
        assert_eq!(
            flags,
            vec![
                "unknown_basis".to_string(),
                "fee_unhandled".to_string(),
                "quote_not_usd".to_string(),
            ]
        );
        let analysis = crate::today::signals::analyze_signals(&[rt.clone()], &[rt]);
        let primary = analysis.trip_flags.first().expect("flag");
        assert_eq!(primary.label, "Unknown basis");
    }

    #[test]
    fn stub_adapter_reason_is_not_quiet_day() {
        assert_eq!(
            serde_json::to_string(&TodayDegradedReason::StubAdapter).unwrap(),
            "\"stub_adapter\""
        );
        assert!(is_stub_fill_adapter(Some("counting_poll")));
        assert!(!is_stub_fill_adapter(Some("binance_com_wasm")));
        assert!(!is_stub_fill_adapter(Some("seq_mock")));
        assert!(!is_stub_fill_adapter(None));
    }

    #[test]
    fn live_com_empty_cache_is_not_filters_ready() {
        // I-S4
        assert_eq!(
            serde_json::to_string(&TodayDegradedReason::ExchangeFiltersNotReady).unwrap(),
            "\"exchange_filters_not_ready\""
        );
        assert!(!crate::exchange_info::live_com_filters_ready(
            Some("binance_com"),
            &ExchangeInfoSymbolCache::empty()
        ));
    }

    #[test]
    fn open_inventory_omits_flat_symbols() {
        let fills = vec![
            crate::broker::BrokerFill {
                fill_id: "b1".into(),
                trade_id: "t-b1".into(),
                symbol: "BTCUSDT".into(),
                side: "BUY".into(),
                qty: 0.01,
                price: 60_000.0,
                filled_at: Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap(),
                broker: "binance_us".into(),
                fee_amount: Some(0.5),
                fee_asset: Some("USDT".into()),
                ..Default::default()
            },
            crate::broker::BrokerFill {
                fill_id: "s1".into(),
                trade_id: "t-s1".into(),
                symbol: "BTCUSDT".into(),
                side: "SELL".into(),
                qty: 0.01,
                price: 61_000.0,
                filled_at: Utc.with_ymd_and_hms(2026, 7, 4, 13, 0, 0).unwrap(),
                broker: "binance_us".into(),
                fee_amount: Some(0.5),
                fee_asset: Some("USDT".into()),
                ..Default::default()
            },
        ];
        assert!(open_inventory_from_fills(&fills).is_empty());
    }

    struct TodayServiceHarness {
        store: TodayStore,
    }

    impl TodayServiceHarness {
        fn ensure_past_day_snapshots(&self, all_trips: &[RoundTrip]) -> anyhow::Result<()> {
            let today = local_today();
            let mut past_dates: BTreeSet<NaiveDate> = BTreeSet::new();
            for rt in all_trips {
                let d = local_date(rt.closed_at);
                if d < today {
                    past_dates.insert(d);
                }
            }
            for date in past_dates {
                let day_trips: Vec<_> = all_trips
                    .iter()
                    .filter(|rt| local_date(rt.closed_at) == date)
                    .cloned()
                    .collect();
                let analysis = analyze_signals(all_trips, &day_trips);
                self.store
                    .upsert_daily_snapshot(&build_snapshot(date, &day_trips, &analysis))?;
            }
            Ok(())
        }
    }
}

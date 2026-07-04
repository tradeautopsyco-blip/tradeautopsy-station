//! Today session mirror — engine + signals + persistence orchestration.

use crate::broker_sync::{BrokerRuntimeState, BrokerSyncConfig};
use crate::broker_sync_control::{BrokerRuntimeCardStatus, BrokerSyncController};
use crate::exchange_info::ExchangeInfoSymbolCache;
use crate::recent_trades::RecentTradesStore;
use crate::round_trip_engine::{
    aggregate_known_pnl, is_aggregate_eligible, RoundTrip, RoundTripEngine,
};
use crate::today::signals::{analyze_signals, BehaviorSignal, SignalAnalysis};
use crate::today::store::{DailySnapshot, TodayStore};
use chrono::{Local, NaiveDate, Utc};
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TodayDegradedReason {
    SyncUnavailable,
    SyncStale,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayHeroPayload {
    pub pnl_today_usd: Option<f64>,
    pub trades_today: Option<u32>,
    pub win_rate: Option<f64>,
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
}

pub struct TodayService {
    recent_trades: RecentTradesStore,
    store: TodayStore,
    engine: RoundTripEngine,
    broker_status: Arc<Mutex<BrokerRuntimeState>>,
    broker_sync_control: Arc<BrokerSyncController>,
    broker_limits: BrokerSyncConfig,
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
            engine: RoundTripEngine::with_exchange_info(exchange_cache, true),
            broker_status,
            broker_sync_control,
            broker_limits,
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
        self.store
            .upsert_daily_snapshot(&build_snapshot(today, &today_trips, &analysis))?;
        Ok(())
    }

    pub fn build_payload(&self) -> anyhow::Result<TodayPayload> {
        if let Some(reason) = self.degraded_reason() {
            return Ok(degraded_payload(local_today(), reason));
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
            .count();
        let win_rate = if eligible.is_empty() {
            None
        } else {
            Some(wins as f64 / eligible.len() as f64)
        };

        let trades: Vec<TodayTradeRowPayload> = today_trips
            .iter()
            .take(50)
            .filter_map(|rt| {
                let flag = result
                    .round_trips
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
                    data_quality_flags: data_quality_flags(rt),
                })
            })
            .collect();

        Ok(TodayPayload {
            local_date: today.format("%Y-%m-%d").to_string(),
            performance_basis_not_tax: true,
            degraded_reason: None,
            learning_baseline: analysis.learning_baseline,
            hero: TodayHeroPayload {
                pnl_today_usd: pnl,
                trades_today: trades_count,
                win_rate,
            },
            top_signals: analysis
                .top_signals
                .into_iter()
                .map(signal_to_payload)
                .collect(),
            trades,
            open_position_count: open_positions,
        })
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

        let sync_state = snap.sync_state_literal(
            self.broker_limits.fresh_secs,
            self.broker_limits.stale_secs,
        );
        match sync_state {
            "stale" | "disconnected" | "not_connected" => Some(TodayDegradedReason::SyncStale),
            _ => None,
        }
    }
}

fn degraded_payload(local_date: NaiveDate, reason: TodayDegradedReason) -> TodayPayload {
    TodayPayload {
        local_date: local_date.format("%Y-%m-%d").to_string(),
        performance_basis_not_tax: true,
        degraded_reason: Some(reason),
        learning_baseline: true,
        hero: TodayHeroPayload {
            pnl_today_usd: None,
            trades_today: None,
            win_rate: None,
        },
        top_signals: vec![],
        trades: vec![],
        open_position_count: 0,
    }
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

fn count_open_positions(fills: &[crate::broker::BrokerFill]) -> u32 {
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
    qty_by_symbol.values().filter(|q| **q > 1e-12).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

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
}

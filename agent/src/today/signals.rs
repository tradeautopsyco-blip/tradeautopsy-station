//! Local behavior signals for Today v1 — no brain HTTP calls.

use crate::round_trip_engine::{is_aggregate_eligible, RoundTrip};
use chrono::{DateTime, Local, Utc};
use std::collections::BTreeMap;

const LEARNING_BASELINE_TRIP_THRESHOLD: usize = 10;
const LOSS_CHASE_GAP_MINS: i64 = 30;
const REVENGE_GAP_MINS: i64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlagSeverity {
    Clean,
    Watch,
    Firing,
}

impl FlagSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Watch => "watch",
            Self::Firing => "firing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    LossChasing,
    Revenge,
    Overtrading,
    PositionSizing,
}

impl SignalKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::LossChasing => "Loss chasing",
            Self::Revenge => "Revenge trading",
            Self::Overtrading => "Overtrading",
            Self::PositionSizing => "Position sizing",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BehaviorSignal {
    pub kind: SignalKind,
    pub severity: FlagSeverity,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TripBehaviorFlag {
    pub label: String,
    pub severity: FlagSeverity,
}

#[derive(Debug, Clone)]
pub struct SignalAnalysis {
    pub learning_baseline: bool,
    pub top_signals: Vec<BehaviorSignal>,
    pub trip_flags: Vec<TripBehaviorFlag>,
}

pub fn analyze_signals(all_trips: &[RoundTrip], today_trips: &[RoundTrip]) -> SignalAnalysis {
    let learning_baseline = all_trips.len() < LEARNING_BASELINE_TRIP_THRESHOLD;
    let baselines = compute_baselines(all_trips);
    let today_count = today_trips.len();

    let mut trip_flags = Vec::with_capacity(all_trips.len());
    for trip in all_trips {
        let today_idx = today_trips
            .iter()
            .position(|t| t.closed_at == trip.closed_at && t.symbol == trip.symbol)
            .unwrap_or(0);
        trip_flags.push(flag_for_trip(
            trip,
            today_idx,
            all_trips,
            today_trips,
            &baselines,
            learning_baseline,
        ));
    }

    let mut signals = vec![
        loss_chasing_signal(all_trips, today_trips, &baselines, learning_baseline),
        revenge_signal(all_trips, today_trips, &baselines, learning_baseline),
        overtrading_signal(today_count, &baselines, learning_baseline),
        position_sizing_signal(today_trips, &baselines, learning_baseline),
    ];
    signals.sort_by(|a, b| b.severity.cmp(&a.severity));
    signals.truncate(2);

    SignalAnalysis {
        learning_baseline,
        top_signals: signals,
        trip_flags,
    }
}

#[derive(Debug, Clone)]
struct Baselines {
    median_trades_per_day: f64,
    median_notional_usd: f64,
}

fn compute_baselines(trips: &[RoundTrip]) -> Baselines {
    if trips.is_empty() {
        return Baselines {
            median_trades_per_day: 2.0,
            median_notional_usd: 1_000.0,
        };
    }

    let mut per_day: BTreeMap<String, usize> = BTreeMap::new();
    let mut notionals = Vec::new();
    for trip in trips {
        let day = trip.closed_at.with_timezone(&Local).date_naive().to_string();
        *per_day.entry(day).or_default() += 1;
        notionals.push(trip.qty * trip.avg_entry_price);
    }
    let mut counts: Vec<f64> = per_day.values().map(|c| *c as f64).collect();
    counts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    notionals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Baselines {
        median_trades_per_day: median(&counts).unwrap_or(2.0),
        median_notional_usd: median(&notionals).unwrap_or(1_000.0),
    }
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

fn flag_for_trip(
    trip: &RoundTrip,
    today_idx: usize,
    all_trips: &[RoundTrip],
    today_trips: &[RoundTrip],
    baselines: &Baselines,
    learning: bool,
) -> TripBehaviorFlag {
    if trip.unknown_basis {
        return TripBehaviorFlag {
            label: "Unknown basis".to_string(),
            severity: FlagSeverity::Watch,
        };
    }
    if trip.fee_unhandled {
        return TripBehaviorFlag {
            label: "Fee unhandled".to_string(),
            severity: FlagSeverity::Watch,
        };
    }
    if trip.quote_not_usd {
        return TripBehaviorFlag {
            label: "Non-USD quote".to_string(),
            severity: FlagSeverity::Watch,
        };
    }

    let notional = trip.qty * trip.avg_entry_price;
    let mut severity = FlagSeverity::Clean;
    let mut label = "Clean".to_string();

    if let Some(prev) = previous_trip(all_trips, trip) {
        let gap_mins = minutes_between(prev.closed_at, trip.opened_at);
        let prev_loss = prev.realized_pnl_usd.unwrap_or(0.0) < 0.0;
        let size_up = notional > prev.qty * prev.avg_entry_price * 1.05;

        if prev_loss && gap_mins <= REVENGE_GAP_MINS {
            severity = FlagSeverity::Firing;
            label = "Revenge".to_string();
        } else if prev_loss && gap_mins <= LOSS_CHASE_GAP_MINS && size_up {
            severity = FlagSeverity::Firing;
            label = "Loss chasing".to_string();
        }
    }

    if severity == FlagSeverity::Clean
        && notional > baselines.median_notional_usd * if learning { 1.5 } else { 2.0 }
    {
        severity = FlagSeverity::Watch;
        label = "Oversized".to_string();
    }

    if severity == FlagSeverity::Clean && today_trips.len() as f64 > baselines.median_trades_per_day + 1.0
        && today_idx >= 2
    {
        severity = FlagSeverity::Watch;
        label = "Overtrade".to_string();
    }

    TripBehaviorFlag { label, severity }
}

fn loss_chasing_signal(
    all_trips: &[RoundTrip],
    today_trips: &[RoundTrip],
    baselines: &Baselines,
    learning: bool,
) -> BehaviorSignal {
    if learning {
        return BehaviorSignal {
            kind: SignalKind::LossChasing,
            severity: FlagSeverity::Watch,
            description:
                "Still learning your baseline — signals activate after ~10 round-trips."
                    .to_string(),
        };
    }

    let firing = today_trips.iter().any(|trip| {
        previous_trip(all_trips, trip).is_some_and(|prev| {
            let gap = minutes_between(prev.closed_at, trip.opened_at);
            prev.realized_pnl_usd.unwrap_or(0.0) < 0.0
                && gap <= LOSS_CHASE_GAP_MINS
                && trip.qty * trip.avg_entry_price
                    > prev.qty * prev.avg_entry_price * 1.05
        })
    });

    if firing {
        BehaviorSignal {
            kind: SignalKind::LossChasing,
            severity: FlagSeverity::Firing,
            description:
                "You opened round-trips soon after a loss with larger size than your baseline."
                    .to_string(),
        }
    } else if today_trips.len() as f64 > baselines.median_trades_per_day {
        BehaviorSignal {
            kind: SignalKind::LossChasing,
            severity: FlagSeverity::Watch,
            description: "No loss-chasing pattern detected today.".to_string(),
        }
    } else {
        BehaviorSignal {
            kind: SignalKind::LossChasing,
            severity: FlagSeverity::Clean,
            description: "No loss-chasing pattern detected today.".to_string(),
        }
    }
}

fn revenge_signal(
    all_trips: &[RoundTrip],
    today_trips: &[RoundTrip],
    _baselines: &Baselines,
    learning: bool,
) -> BehaviorSignal {
    if learning {
        return BehaviorSignal {
            kind: SignalKind::Revenge,
            severity: FlagSeverity::Watch,
            description:
                "Still learning your baseline — revenge signals need more history.".to_string(),
        };
    }

    let firing = today_trips.iter().any(|trip| {
        previous_trip(all_trips, trip).is_some_and(|prev| {
            minutes_between(prev.closed_at, trip.opened_at) <= REVENGE_GAP_MINS
                && prev.realized_pnl_usd.unwrap_or(0.0) < 0.0
        })
    });

    BehaviorSignal {
        kind: SignalKind::Revenge,
        severity: if firing {
            FlagSeverity::Firing
        } else {
            FlagSeverity::Clean
        },
        description: if firing {
            "Fast re-entry after a loss detected today.".to_string()
        } else {
            "No revenge-trading pattern detected today.".to_string()
        },
    }
}

fn overtrading_signal(count: usize, baselines: &Baselines, learning: bool) -> BehaviorSignal {
    if learning {
        return BehaviorSignal {
            kind: SignalKind::Overtrading,
            severity: FlagSeverity::Watch,
            description:
                "Still learning your baseline — trade-count signals need more history.".to_string(),
        };
    }

    let threshold = baselines.median_trades_per_day.ceil() as usize;
    let severity = if count > threshold.saturating_add(1) {
        FlagSeverity::Firing
    } else if count > threshold {
        FlagSeverity::Watch
    } else {
        FlagSeverity::Clean
    };

    BehaviorSignal {
        kind: SignalKind::Overtrading,
        severity,
        description: if count > threshold {
            format!(
                "{count} trades today vs your median {:.0} on active days.",
                baselines.median_trades_per_day
            )
        } else {
            format!(
                "{count} trades today — within your typical {:.0}/day baseline.",
                baselines.median_trades_per_day
            )
        },
    }
}

fn position_sizing_signal(
    today_trips: &[RoundTrip],
    baselines: &Baselines,
    learning: bool,
) -> BehaviorSignal {
    if learning {
        return BehaviorSignal {
            kind: SignalKind::PositionSizing,
            severity: FlagSeverity::Watch,
            description:
                "Still learning your size baseline — sizing signals need more history.".to_string(),
        };
    }

    let oversized = today_trips.iter().any(|trip| {
        is_aggregate_eligible(trip)
            && trip.qty * trip.avg_entry_price > baselines.median_notional_usd * 2.0
    });

    BehaviorSignal {
        kind: SignalKind::PositionSizing,
        severity: if oversized {
            FlagSeverity::Watch
        } else {
            FlagSeverity::Clean
        },
        description: if oversized {
            "At least one round-trip today was larger than your typical size.".to_string()
        } else {
            "Position sizes today are within your typical range.".to_string()
        },
    }
}

fn previous_trip<'a>(all_trips: &'a [RoundTrip], current: &RoundTrip) -> Option<&'a RoundTrip> {
    all_trips
        .iter()
        .filter(|t| t.closed_at < current.opened_at && t.symbol == current.symbol)
        .max_by_key(|t| t.closed_at)
}

fn minutes_between(from: DateTime<Utc>, to: DateTime<Utc>) -> i64 {
    to.signed_duration_since(from).num_minutes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn closed_trip(
        symbol: &str,
        opened: DateTime<Utc>,
        closed: DateTime<Utc>,
        entry: f64,
        qty: f64,
        exit: f64,
        pnl: f64,
    ) -> RoundTrip {
        RoundTrip {
            symbol: symbol.to_string(),
            opened_at: opened,
            closed_at: closed,
            avg_entry_price: entry,
            avg_exit_price: exit,
            qty,
            realized_pnl_usd: Some(pnl),
            fees_usd: Some(0.0),
            unknown_basis: false,
            fee_unhandled: false,
            quote_not_usd: false,
        }
    }

    #[test]
    fn loss_then_fast_reentry_with_escalating_size_fires_loss_chasing_and_revenge() {
        let historical: Vec<RoundTrip> = (1..=10)
            .map(|day| {
                let opened = Utc.with_ymd_and_hms(2026, 6, day, 10, 0, 0).unwrap();
                let closed = Utc.with_ymd_and_hms(2026, 6, day, 10, 30, 0).unwrap();
                closed_trip("BTCUSDT", opened, closed, 50_000.0, 0.01, 51_000.0, 10.0)
            })
            .collect();

        let loss_opened = Utc.with_ymd_and_hms(2026, 7, 4, 10, 0, 0).unwrap();
        let loss_closed = Utc.with_ymd_and_hms(2026, 7, 4, 10, 20, 0).unwrap();
        let loss_trip = closed_trip(
            "BTCUSDT",
            loss_opened,
            loss_closed,
            50_000.0,
            0.01,
            49_000.0,
            -10.0,
        );

        let chase_opened = Utc.with_ymd_and_hms(2026, 7, 4, 10, 22, 0).unwrap();
        let chase_closed = Utc.with_ymd_and_hms(2026, 7, 4, 10, 45, 0).unwrap();
        let chase_trip = closed_trip(
            "BTCUSDT",
            chase_opened,
            chase_closed,
            50_000.0,
            0.011,
            50_100.0,
            1.1,
        );

        let mut all_trips = historical;
        all_trips.push(loss_trip.clone());
        all_trips.push(chase_trip.clone());
        let today_trips = vec![loss_trip, chase_trip];

        let analysis = analyze_signals(&all_trips, &today_trips);
        assert!(
            !analysis.learning_baseline,
            "fixture must exceed learning threshold"
        );

        let loss_chasing = analysis
            .top_signals
            .iter()
            .find(|s| s.kind == SignalKind::LossChasing)
            .expect("loss chasing signal");
        assert_eq!(loss_chasing.severity, FlagSeverity::Firing);

        let revenge = analysis
            .top_signals
            .iter()
            .find(|s| s.kind == SignalKind::Revenge)
            .expect("revenge signal");
        assert_eq!(revenge.severity, FlagSeverity::Firing);

        let chase_flag = analysis.trip_flags.last().expect("chase row flag");
        assert_eq!(
            chase_flag.label, "Revenge",
            "trip primary flag prioritizes revenge over loss chasing"
        );
    }
}

//! One meter = one venue-published budget. Meters never share counters, even
//! inside a slot: `api.binance.com` weight and `eapi.binance.com` weight are two
//! ledgers behind one IP ban.

use std::collections::VecDeque;

use super::types::{Lane, Posture};

/// A budget only exists if a docs lock publishes numbers for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Budget {
    /// The lock names a limit and a window.
    Specified {
        limit: u32,
        window_ms: i64,
        /// Headroom for latency and clock drift. 85 => usable cap is 85% of limit.
        headroom_pct: u32,
        /// Reserve floor. 70 => market data refuses above 70% of limit; the rest
        /// is reachable only by `PrivateRead` / `Execution`.
        market_data_pct: u32,
    },
    /// The lock says NOT SPECIFIED. A meter in this state paces by concurrency
    /// and freezes on 429; it must never invent a ledger.
    NotSpecified,
}

impl Budget {
    pub fn window_ms(self) -> Option<i64> {
        match self {
            Budget::Specified { window_ms, .. } => Some(window_ms),
            Budget::NotSpecified => None,
        }
    }

    pub fn effective_cap(self) -> Option<u32> {
        match self {
            Budget::Specified {
                limit,
                headroom_pct,
                ..
            } => Some(limit.saturating_mul(headroom_pct) / 100),
            Budget::NotSpecified => None,
        }
    }

    pub fn market_data_cap(self) -> Option<u32> {
        match self {
            Budget::Specified {
                limit,
                market_data_pct,
                ..
            } => Some(limit.saturating_mul(market_data_pct) / 100),
            Budget::NotSpecified => None,
        }
    }

    pub fn cap_for(self, lane: Lane) -> Option<u32> {
        if lane.is_reserved() {
            self.effective_cap()
        } else {
            self.market_data_cap()
        }
    }
}

#[derive(Debug)]
pub struct Meter {
    pub id: &'static str,
    pub budget: Budget,
    pub max_concurrency: u32,
    /// `(charged_at_ms, cost)` inside the rolling window. Empty when the budget
    /// is NotSpecified — that meter has no ledger by construction.
    entries: VecDeque<(i64, u32)>,
    inflight: u32,
    posture: Posture,
}

impl Meter {
    pub fn new(id: &'static str, budget: Budget, max_concurrency: u32) -> Self {
        Self {
            id,
            budget,
            max_concurrency,
            entries: VecDeque::new(),
            inflight: 0,
            posture: Posture::Live,
        }
    }

    pub fn posture(&self, now_ms: i64) -> Posture {
        self.posture.resolved(now_ms)
    }

    pub fn inflight(&self) -> u32 {
        self.inflight
    }

    pub fn prune(&mut self, now_ms: i64) {
        let Some(window_ms) = self.budget.window_ms() else {
            self.entries.clear();
            return;
        };
        while let Some((at, _)) = self.entries.front() {
            if now_ms.saturating_sub(*at) >= window_ms {
                self.entries.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn used(&mut self, now_ms: i64) -> u32 {
        self.prune(now_ms);
        self.entries.iter().map(|(_, c)| *c).sum()
    }

    /// Earliest time enough weight rolls off for `cost` to fit under `cap`.
    /// Best effort — used only to tell the caller when to look again.
    pub fn relief_at_ms(&mut self, now_ms: i64, cost: u32, cap: u32) -> Option<i64> {
        let window_ms = self.budget.window_ms()?;
        self.prune(now_ms);
        let mut remaining = self.entries.iter().map(|(_, c)| *c).sum::<u32>();
        for (at, charged) in self.entries.iter() {
            if remaining.saturating_add(cost) <= cap {
                return Some(*at);
            }
            remaining = remaining.saturating_sub(*charged);
            if remaining.saturating_add(cost) <= cap {
                return Some(at.saturating_add(window_ms));
            }
        }
        Some(now_ms.saturating_add(window_ms))
    }

    /// Does `cost` fit for this lane right now? `None` = fits.
    pub fn over_budget_until(&mut self, now_ms: i64, lane: Lane, cost: u32) -> Option<Option<i64>> {
        let Some(cap) = self.budget.cap_for(lane) else {
            return None; // NotSpecified meters have no ledger to exceed.
        };
        let used = self.used(now_ms);
        if used.saturating_add(cost) <= cap {
            return None;
        }
        Some(self.relief_at_ms(now_ms, cost, cap))
    }

    pub fn at_concurrency_cap(&self) -> bool {
        self.inflight >= self.max_concurrency
    }

    /// Charge the forecast and take an in-flight slot.
    pub fn charge(&mut self, now_ms: i64, cost: u32) {
        if self.budget.window_ms().is_some() && cost > 0 {
            self.entries.push_back((now_ms, cost));
        }
        self.inflight = self.inflight.saturating_add(1);
    }

    /// Give back a forecast that was never spent, because the call never left the
    /// Mac. Removes the most recent matching charge rather than the oldest, so the
    /// window's age profile stays honest.
    pub fn refund(&mut self, cost: u32) {
        if cost == 0 || self.budget.window_ms().is_none() {
            return;
        }
        if let Some(pos) = self.entries.iter().rposition(|(_, c)| *c == cost) {
            self.entries.remove(pos);
        }
    }

    pub fn release(&mut self) {
        self.inflight = self.inflight.saturating_sub(1);
    }

    /// Correct the forecast against what the venue actually counted.
    ///
    /// The ledger is a forecast built from the endpoint weight table. Depth
    /// now uses the published limit tiers; other ranges still forecast at
    /// maximum. Without this correction an over-forecast would hold extra
    /// weight for a full minute after the venue header has already counted less.
    pub fn reconcile(&mut self, now_ms: i64, venue_used: u32) {
        if self.budget.window_ms().is_none() {
            return;
        }
        self.prune(now_ms);
        let local: u32 = self.entries.iter().map(|(_, c)| *c).sum();
        match venue_used.cmp(&local) {
            std::cmp::Ordering::Greater => {
                // We under-forecast. Top up; the synthetic entry ages out with
                // the window, which over-counts slightly and never under-counts.
                self.entries.push_back((now_ms, venue_used - local));
            }
            std::cmp::Ordering::Less => {
                // We over-forecast. Trim from the oldest, which is closest to
                // rolling off anyway.
                let mut excess = local - venue_used;
                while excess > 0 {
                    let Some((at, charged)) = self.entries.pop_front() else {
                        break;
                    };
                    if charged > excess {
                        self.entries.push_front((at, charged - excess));
                        break;
                    }
                    excess -= charged;
                }
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    pub fn freeze_until(&mut self, until_ms: i64) {
        let next = Posture::Backoff { until_ms };
        // A ban already in force outranks a backoff and must not be downgraded.
        if let Posture::Banned { until_ms: banned } = self.posture {
            if banned >= until_ms {
                return;
            }
        }
        self.posture = next;
    }

    pub fn clear_posture(&mut self) {
        self.posture = Posture::Live;
    }

    /// Mark the ledger spent for the rest of the window. Used the moment a 429
    /// lands so concurrent callers stop immediately rather than each discovering
    /// the limit for themselves.
    pub fn spend_window(&mut self, now_ms: i64) {
        let Budget::Specified { limit, .. } = self.budget else {
            return;
        };
        self.prune(now_ms);
        let local: u32 = self.entries.iter().map(|(_, c)| *c).sum();
        if limit > local {
            self.entries.push_back((now_ms, limit - local));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot_meter() -> Meter {
        Meter::new(
            "test",
            Budget::Specified {
                limit: 6000,
                window_ms: 60_000,
                headroom_pct: 85,
                market_data_pct: 70,
            },
            8,
        )
    }

    #[test]
    fn caps_come_from_the_lock_not_from_guesses() {
        let m = spot_meter();
        assert_eq!(m.budget.effective_cap(), Some(5100));
        assert_eq!(m.budget.market_data_cap(), Some(4200));
        assert_eq!(m.budget.cap_for(Lane::MarketData), Some(4200));
        assert_eq!(m.budget.cap_for(Lane::Execution), Some(5100));
    }

    #[test]
    fn reserve_floor_holds_execution_capacity_under_market_data_flood() {
        let mut m = spot_meter();
        // Flood market data right up to its floor.
        m.charge(0, 4200);
        assert!(m.over_budget_until(0, Lane::MarketData, 250).is_some());
        // Execution still has the reserve.
        assert!(m.over_budget_until(0, Lane::Execution, 250).is_none());
        assert!(m.over_budget_until(0, Lane::PrivateRead, 250).is_none());
    }

    #[test]
    fn execution_refuses_only_above_the_headroom_cap() {
        let mut m = spot_meter();
        m.charge(0, 5100);
        assert!(m.over_budget_until(0, Lane::Execution, 1).is_some());
    }

    #[test]
    fn window_rolls_off() {
        let mut m = spot_meter();
        m.charge(0, 4200);
        assert!(m.over_budget_until(0, Lane::MarketData, 250).is_some());
        assert_eq!(m.used(60_000), 0);
        assert!(m.over_budget_until(60_000, Lane::MarketData, 250).is_none());
    }

    #[test]
    fn reconcile_trims_an_over_forecast() {
        let mut m = spot_meter();
        m.charge(0, 250); // forecast the max for a ranged weight
        m.reconcile(10, 5); // venue actually charged 5
        assert_eq!(m.used(10), 5);
    }

    #[test]
    fn reconcile_tops_up_an_under_forecast() {
        let mut m = spot_meter();
        m.charge(0, 2);
        m.reconcile(10, 900);
        assert_eq!(m.used(10), 900);
    }

    #[test]
    fn not_specified_meter_never_invents_a_ledger() {
        let mut m = Meter::new("kotak", Budget::NotSpecified, 4);
        assert_eq!(m.budget.effective_cap(), None);
        m.charge(0, 999);
        assert_eq!(m.used(0), 0);
        assert!(m.over_budget_until(0, Lane::MarketData, 999).is_none());
        m.reconcile(0, 5_000);
        assert_eq!(m.used(0), 0);
    }

    #[test]
    fn not_specified_meter_still_paces_by_concurrency() {
        let mut m = Meter::new("kotak", Budget::NotSpecified, 2);
        m.charge(0, 0);
        assert!(!m.at_concurrency_cap());
        m.charge(0, 0);
        assert!(m.at_concurrency_cap());
        m.release();
        assert!(!m.at_concurrency_cap());
    }

    #[test]
    fn refund_returns_an_unsent_forecast() {
        let mut m = spot_meter();
        m.charge(0, 250);
        m.charge(0, 20);
        m.refund(250);
        assert_eq!(m.used(0), 20);
    }

    #[test]
    fn refund_is_a_no_op_without_a_ledger() {
        let mut m = Meter::new("kotak", Budget::NotSpecified, 4);
        m.charge(0, 0);
        m.refund(0);
        assert_eq!(m.used(0), 0);
    }

    #[test]
    fn spend_window_stops_concurrent_callers_at_once() {
        let mut m = spot_meter();
        m.charge(0, 10);
        m.spend_window(0);
        assert_eq!(m.used(0), 6000);
        assert!(m.over_budget_until(0, Lane::Execution, 1).is_some());
    }

    #[test]
    fn a_backoff_cannot_downgrade_a_longer_ban() {
        let mut m = spot_meter();
        m.posture = Posture::Banned { until_ms: 900_000 };
        m.freeze_until(1_000);
        assert_eq!(m.posture(0), Posture::Banned { until_ms: 900_000 });
    }
}

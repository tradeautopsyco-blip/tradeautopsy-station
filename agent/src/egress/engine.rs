//! VenueEgress — the one place a call leaves this Mac for a venue.
//!
//! Universal for every broker: nothing sends without `admit`, every response goes
//! through `record`, and the engine owns the clock. Not universal, and never
//! blended: budgets, ban semantics, and header names, which live per slot in
//! `policy.rs`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::ban;
use super::clock::{Clock, SystemClock};
use super::meter::Meter;
use super::policy::{self, SlotPolicy};
use super::types::{
    Decision, EgressRequest, Lane, Outcome, Permit, Posture, RefuseKind, RefuseReason,
};
use crate::data::authorize_book_call;
use crate::event_bus::{AgentEvent, EventBus};

struct Slot {
    policy: &'static SlotPolicy,
    /// True while Ring 3 has this venue's hosts sinkholed in `/etc/hosts`.
    ban_block_armed: bool,
    /// Slot-wide posture. A `Banned` here is the IP ban and covers every meter.
    posture: Posture,
    /// Drives `SlotPolicy::ban_escalation_ms` when a venue keeps banning us.
    consecutive_bans: u32,
    meters: HashMap<&'static str, Meter>,
}

impl Slot {
    fn new(policy: &'static SlotPolicy) -> Self {
        let meters = policy::METERS
            .iter()
            .filter(|m| m.slot_id == policy.slot_id)
            .map(|m| {
                (
                    m.meter_id,
                    Meter::new(m.meter_id, m.budget, m.max_concurrency),
                )
            })
            .collect();
        Self {
            policy,
            ban_block_armed: false,
            posture: Posture::Live,
            consecutive_bans: 0,
            meters,
        }
    }
}

/// A slot's public posture, for Notch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VenuePosture {
    pub venue: String,
    pub posture: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_ms: Option<i64>,
    pub meters: Vec<MeterPosture>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MeterPosture {
    pub meter: String,
    pub posture: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until_ms: Option<i64>,
    /// `None` renders as `not_specified` — a meter with no published budget never
    /// reports an invented one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<u32>,
    pub inflight: u32,
}

pub struct VenueEgress {
    clock: Arc<dyn Clock>,
    slots: Mutex<HashMap<&'static str, Slot>>,
    events: Mutex<Option<Arc<EventBus>>>,
    /// Ring 3: mirror an IP ban into `/etc/hosts` so nothing on this Mac can keep
    /// knocking. Off unless explicitly enabled, because it shells out to `sudo`
    /// and no test should touch the real hosts file.
    hosts_backstop: std::sync::atomic::AtomicBool,
}

impl VenueEgress {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let slots = policy::SLOTS
            .iter()
            .map(|p| (p.slot_id, Slot::new(p)))
            .collect();
        Self {
            clock,
            slots: Mutex::new(slots),
            events: Mutex::new(None),
            hosts_backstop: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Turn on the `/etc/hosts` backstop. Production only.
    pub fn enable_hosts_backstop(&self) {
        self.hosts_backstop
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn hosts_backstop_on(&self) -> bool {
        self.hosts_backstop
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn with_system_clock() -> Self {
        Self::new(Arc::new(SystemClock))
    }

    /// Notch subscribes here. Optional so the engine works in tests with no bus.
    pub fn attach_event_bus(&self, bus: Arc<EventBus>) {
        *self.events.lock().expect("egress events lock") = Some(bus);
    }

    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    /// May this call leave the Mac right now?
    ///
    /// Synchronous by design: the refuse-fast contract means admission is a
    /// lock-and-decide, never a wait. That is what lets one engine serve both the
    /// blocking Wasm transport and the async desk paths.
    pub fn admit(&self, req: &EgressRequest<'_>) -> Decision {
        let now = self.clock.now_ms();

        // 1. Book must be one this engine knows. An unknown book has no meter and
        //    therefore no budget it could be spending.
        let Some(meter_policy) = policy::route_book(req.book_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };

        // 2. The existing R0/R6 fence decides host, path, method, and credential
        //    shape. It is not re-implemented here — it is given a caller that
        //    cannot be skipped.
        let attach_private = req.lane.is_reserved();
        if authorize_book_call(req.book_id, req.host, req.method, req.path, attach_private).is_err()
        {
            return Decision::Refuse(RefuseReason::new(RefuseKind::HostBlocked, None));
        }

        if let Some(symbol) = crate::data::query_symbol(req.query) {
            if !crate::data::book_accepts_symbol(req.book_id, symbol) {
                return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
            }
        }

        let mut slots = self.slots.lock().expect("egress slots lock");
        let Some(slot) = slots.get_mut(meter_policy.slot_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };

        // 3. Slot-wide ban. One IP, one freeze: a COM 418 stops both COM meters
        //    and touches nothing in Kotak.
        slot.posture = slot.posture.resolved(now);
        if let Posture::Banned { until_ms } = slot.posture {
            return Decision::Refuse(RefuseReason::new(RefuseKind::Banned, Some(until_ms)));
        }
        // The ban expired on its own clock; lift Ring 3 with it. Releasing the
        // venue-ban reason leaves an armed kill switch untouched.
        if slot.ban_block_armed {
            slot.ban_block_armed = false;
            clear_hosts_backstop(slot.policy.slot_id);
        }

        let Some(meter) = slot.meters.get_mut(meter_policy.meter_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };

        // 4. This meter's own 429 backoff.
        if let Posture::Backoff { until_ms } = meter.posture(now) {
            return Decision::Refuse(RefuseReason::new(RefuseKind::Frozen, Some(until_ms)));
        }

        // 5. Concurrency. The only pacing a NotSpecified meter is allowed to have.
        if meter.at_concurrency_cap() {
            return Decision::Refuse(RefuseReason::new(RefuseKind::OverBudget, None));
        }

        // 6. Budget, against this lane's cap. Market data stops at the reserve
        //    floor so private reads keep their headroom.
        let cost = req
            .weight_hint
            .unwrap_or_else(|| policy::forecast_weight(meter_policy, req.path, req.query));
        if let Some(until) = meter.over_budget_until(now, req.lane, cost) {
            return Decision::Refuse(RefuseReason::new(RefuseKind::OverBudget, until));
        }

        meter.charge(now, cost);
        Decision::Admit(Permit {
            slot: meter_policy.slot_id,
            meter: meter_policy.meter_id,
            lane: req.lane,
            forecast_cost: cost,
            issued_at_ms: now,
            recorded: false,
        })
    }

    /// Admission for connect-time auth traffic — Binance `/sapi` credential
    /// checks, the Kotak TOTP login. These paths sit outside the R0 *data* fence
    /// on purpose, so `authorize_book_call` would refuse them. They still reach
    /// the venue's IP, so a connect-retry storm can ban that IP exactly the way
    /// the depth loop did, and they must still respect a ban already in force.
    ///
    /// Narrower than `admit` in the one way that matters: the host must belong to
    /// a known venue. It never widens what a *data* call may reach.
    pub fn admit_auth(&self, host: &str, lane: Lane, weight_hint: u32) -> Decision {
        let now = self.clock.now_ms();
        let Some(slot_policy) = policy::slot_for_host(host) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::HostBlocked, None));
        };
        let Some(meter_policy) = policy::meter_policy(slot_policy.auth_meter_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };

        let mut slots = self.slots.lock().expect("egress slots lock");
        let Some(slot) = slots.get_mut(slot_policy.slot_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };

        slot.posture = slot.posture.resolved(now);
        if let Posture::Banned { until_ms } = slot.posture {
            return Decision::Refuse(RefuseReason::new(RefuseKind::Banned, Some(until_ms)));
        }
        let Some(meter) = slot.meters.get_mut(meter_policy.meter_id) else {
            return Decision::Refuse(RefuseReason::new(RefuseKind::WrongBook, None));
        };
        if let Posture::Backoff { until_ms } = meter.posture(now) {
            return Decision::Refuse(RefuseReason::new(RefuseKind::Frozen, Some(until_ms)));
        }
        if meter.at_concurrency_cap() {
            return Decision::Refuse(RefuseReason::new(RefuseKind::OverBudget, None));
        }
        if let Some(until) = meter.over_budget_until(now, lane, weight_hint) {
            return Decision::Refuse(RefuseReason::new(RefuseKind::OverBudget, until));
        }

        meter.charge(now, weight_hint);
        Decision::Admit(Permit {
            slot: slot_policy.slot_id,
            meter: meter_policy.meter_id,
            lane,
            forecast_cost: weight_hint,
            issued_at_ms: now,
            recorded: false,
        })
    }

    /// What the venue said. Consumes the permit, so one admission records once.
    pub fn record(&self, mut permit: Permit, outcome: &Outcome) {
        permit.recorded = true;
        let now = self.clock.now_ms();
        let mut changed: Option<VenuePosture> = None;

        {
            let mut slots = self.slots.lock().expect("egress slots lock");
            let Some(slot) = slots.get_mut(permit.slot) else {
                return;
            };
            let slot_policy = slot.policy;
            let before = (
                slot.posture.resolved(now),
                meter_posture(slot, permit.meter, now),
            );

            if let Some(meter) = slot.meters.get_mut(permit.meter) {
                meter.release();
            }

            match outcome {
                // A network failure is not a venue instruction. It must never be
                // mistaken for one, which is precisely the confusion that turned a
                // 418 into a 2-second retry loop.
                Outcome::Transport => {}
                Outcome::Http { status, .. } => {
                    let status = *status;
                    let retry_after = outcome.header("retry-after");

                    if slot_policy.ban_statuses.contains(&status) {
                        let idx = (slot.consecutive_bans as usize)
                            .min(slot_policy.ban_escalation_ms.len().saturating_sub(1));
                        let fallback = slot_policy
                            .ban_escalation_ms
                            .get(idx)
                            .copied()
                            .unwrap_or(slot_policy.default_ban_ms);
                        let until =
                            ban::resolve_ban_until_ms(now, outcome.body(), retry_after, fallback);
                        slot.consecutive_bans = slot.consecutive_bans.saturating_add(1);
                        slot.posture = Posture::Banned { until_ms: until };
                        if self.hosts_backstop_on() && !slot.ban_block_armed {
                            slot.ban_block_armed = true;
                            arm_hosts_backstop(slot_policy.slot_id);
                        }
                        // Every meter in the slot is behind the same IP.
                        for meter in slot.meters.values_mut() {
                            meter.spend_window(now);
                        }
                        tracing::error!(
                            venue = slot_policy.slot_id,
                            meter = permit.meter,
                            until_ms = until,
                            consecutive_bans = slot.consecutive_bans,
                            "VenueEgress: venue IP-banned — slot frozen"
                        );
                    } else if slot_policy.rate_limit_statuses.contains(&status) {
                        let base = retry_after
                            .and_then(ban::retry_after_ms)
                            .unwrap_or(slot_policy.default_backoff_ms);
                        let until = now.saturating_add(ban::jittered(base, base / 4));
                        if let Some(meter) = slot.meters.get_mut(permit.meter) {
                            // Spend the window at once so concurrent callers stop
                            // now rather than each rediscovering the limit.
                            meter.spend_window(now);
                            meter.freeze_until(until);
                        }
                        tracing::warn!(
                            venue = slot_policy.slot_id,
                            meter = permit.meter,
                            until_ms = until,
                            "VenueEgress: rate limited — meter frozen"
                        );
                    } else if (200..300).contains(&status) {
                        slot.consecutive_bans = 0;
                    }

                    // Reconcile the forecast against the venue's own count. Free,
                    // and strictly better than asking a quota endpoint.
                    if let (Some(header), Some(meter)) = (
                        policy::used_weight_header(
                            policy::METERS
                                .iter()
                                .find(|m| m.meter_id == permit.meter)
                                .unwrap_or(&policy::BINANCE_COM_API_WEIGHT),
                        ),
                        slot.meters.get_mut(permit.meter),
                    ) {
                        if let Some(used) = outcome
                            .header(header)
                            .and_then(|v| v.trim().parse::<u32>().ok())
                        {
                            meter.reconcile(now, used);
                        }
                    }
                }
            }

            let after = (
                slot.posture.resolved(now),
                meter_posture(slot, permit.meter, now),
            );
            if before != after {
                changed = Some(snapshot_slot(slot, now));
            }
        }

        if let Some(posture) = changed {
            self.publish(posture);
        }
    }

    /// Give back an admission whose call never left the Mac. Releases the
    /// concurrency slot and refunds the forecast, because nothing was spent.
    pub fn cancel(&self, mut permit: Permit) {
        permit.recorded = true;
        let mut slots = self.slots.lock().expect("egress slots lock");
        let Some(slot) = slots.get_mut(permit.slot) else {
            return;
        };
        if let Some(meter) = slot.meters.get_mut(permit.meter) {
            meter.release();
            meter.refund(permit.forecast_cost);
        }
    }

    /// Per-venue posture for Notch. Every slot is reported, always — a banned COM
    /// must not blank a live Kotak.
    pub fn postures(&self) -> Vec<VenuePosture> {
        let now = self.clock.now_ms();
        let mut slots = self.slots.lock().expect("egress slots lock");
        let mut out: Vec<VenuePosture> = slots
            .values_mut()
            .map(|slot| snapshot_slot(slot, now))
            .collect();
        out.sort_by(|a, b| a.venue.cmp(&b.venue));
        out
    }

    pub fn posture_for(&self, slot_id: &str) -> Option<VenuePosture> {
        let now = self.clock.now_ms();
        let mut slots = self.slots.lock().expect("egress slots lock");
        slots.get_mut(slot_id).map(|slot| snapshot_slot(slot, now))
    }

    fn publish(&self, posture: VenuePosture) {
        let bus = self.events.lock().expect("egress events lock").clone();
        if let Some(bus) = bus {
            bus.publish(AgentEvent::VenueEgressState {
                payload: serde_json::to_value(&posture).unwrap_or(serde_json::Value::Null),
            });
        }
    }

    /// Test seam: force a slot posture without a live venue.
    #[cfg(test)]
    pub(crate) fn force_slot_ban(&self, slot_id: &str, until_ms: i64) {
        let mut slots = self.slots.lock().expect("egress slots lock");
        if let Some(slot) = slots.get_mut(slot_id) {
            slot.posture = Posture::Banned { until_ms };
        }
    }
}

/// Ring 3 writes shell out to `sudo` and can block or prompt. They never run on
/// the caller's thread and never gate the in-process freeze, which has already
/// refused the call by the time these are reached.
fn arm_hosts_backstop(slot_id: &'static str) {
    std::thread::spawn(move || {
        if let Err(e) = crate::dns_block::arm_venue_ban(slot_id) {
            tracing::warn!(
                venue = slot_id,
                error = %e,
                "VenueEgress: hosts backstop could not be armed — in-process freeze still holds"
            );
        }
    });
}

fn clear_hosts_backstop(slot_id: &'static str) {
    std::thread::spawn(move || {
        if let Err(e) = crate::dns_block::clear_venue_ban(slot_id) {
            tracing::warn!(venue = slot_id, error = %e, "VenueEgress: hosts backstop not cleared");
        }
    });
}

fn meter_posture(slot: &Slot, meter_id: &str, now_ms: i64) -> Posture {
    slot.meters
        .get(meter_id)
        .map(|m| m.posture(now_ms))
        .unwrap_or(Posture::Live)
}

fn snapshot_slot(slot: &mut Slot, now_ms: i64) -> VenuePosture {
    slot.posture = slot.posture.resolved(now_ms);
    let slot_posture = slot.posture;
    let mut meters: Vec<MeterPosture> = slot
        .meters
        .values_mut()
        .map(|meter| {
            let posture = meter.posture(now_ms);
            let budget_limit = match meter.budget {
                super::meter::Budget::Specified { limit, .. } => Some(limit),
                super::meter::Budget::NotSpecified => None,
            };
            let used = budget_limit.map(|_| meter.used(now_ms));
            MeterPosture {
                meter: meter.id.to_string(),
                posture: posture.as_str(),
                until_ms: posture.until_ms(),
                budget_limit,
                used,
                inflight: meter.inflight(),
            }
        })
        .collect();
    meters.sort_by(|a, b| a.meter.cmp(&b.meter));
    VenuePosture {
        venue: slot.policy.slot_id.to_string(),
        posture: slot_posture.as_str(),
        until_ms: slot_posture.until_ms(),
        meters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::egress::clock::TestClock;
    use crate::egress::types::BODY_SNIPPET_LIMIT;

    const SPOT: &str = "binance-com-spot";
    const OPTS: &str = "binance-com-options";
    const CASH: &str = "kotak-nse-bse-cash";
    const NFO: &str = "kotak-nse-nfo";

    fn engine() -> (Arc<TestClock>, VenueEgress) {
        let clock = TestClock::new(1_000_000);
        let engine = VenueEgress::new(clock.clone());
        (clock, engine)
    }

    fn depth(book: &'static str) -> EgressRequest<'static> {
        EgressRequest::new(
            book,
            "api.binance.com",
            "GET",
            "/api/v3/depth",
            Lane::MarketData,
        )
        .with_query("symbol=BTCUSDT&limit=5000")
    }

    fn kotak_quote() -> EgressRequest<'static> {
        EgressRequest::new(
            CASH,
            "gw-napi.kotaksecurities.com",
            "GET",
            "/quick/user/trades",
            Lane::PrivateRead,
        )
    }

    fn admit_ok(engine: &VenueEgress, req: &EgressRequest<'_>) -> Permit {
        match engine.admit(req) {
            Decision::Admit(p) => p,
            Decision::Refuse(r) => panic!("expected admit, got {r}"),
        }
    }

    /// Would this be admitted? Cancels the permit so the probe itself does not
    /// leak a concurrency slot or leave a forecast charged.
    fn admits(engine: &VenueEgress, req: &EgressRequest<'_>) -> bool {
        match engine.admit(req) {
            Decision::Admit(p) => {
                engine.cancel(p);
                true
            }
            Decision::Refuse(_) => false,
        }
    }

    fn refusal(engine: &VenueEgress, req: &EgressRequest<'_>) -> RefuseReason {
        match engine.admit(req) {
            Decision::Admit(_) => panic!("expected refuse, got admit"),
            Decision::Refuse(r) => r,
        }
    }

    fn ban_response() -> Outcome {
        Outcome::http(
            418,
            vec![("retry-after".into(), "120".into())],
            r#"{"code":-1003,"msg":"IP(1.2.3.4) banned until 1700000000000."}"#,
        )
    }

    // ---- The failure that started this: a venue's stop must stop us. ----

    #[test]
    fn a_418_freezes_the_slot_instead_of_being_retried() {
        let (_clock, engine) = engine();
        let permit = admit_ok(&engine, &depth(SPOT));
        engine.record(permit, &ban_response());

        let refused = refusal(&engine, &depth(SPOT));
        assert_eq!(refused.kind, RefuseKind::Banned);
        assert_eq!(refused.until_ms, Some(1_700_000_000_000));
    }

    #[test]
    fn a_transport_failure_is_not_a_venue_instruction() {
        let (_clock, engine) = engine();
        // This is the distinction `error_for_status()` erased: a TCP reset must
        // not freeze anything, or a flaky network would take the desk down.
        for _ in 0..5 {
            let permit = admit_ok(&engine, &depth(SPOT));
            engine.record(permit, &Outcome::Transport);
        }
        assert!(admits(&engine, &depth(SPOT)));
        assert_eq!(engine.posture_for("binance_com").unwrap().posture, "live");
    }

    // ---- DualNoBlend: budgets and freezes never cross venues. ----

    #[test]
    fn a_com_ban_does_not_freeze_kotak() {
        let (_clock, engine) = engine();
        let permit = admit_ok(&engine, &depth(SPOT));
        engine.record(permit, &ban_response());

        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::Banned);
        // Kotak is a different venue behind a different agreement.
        assert!(admits(&engine, &kotak_quote()));
        assert_eq!(engine.posture_for("kotak_neo").unwrap().posture, "live");
    }

    #[test]
    fn a_kotak_429_does_not_freeze_com() {
        let (_clock, engine) = engine();
        let permit = admit_ok(&engine, &kotak_quote());
        engine.record(
            permit,
            &Outcome::http(429, vec![("retry-after".into(), "30".into())], "too many"),
        );

        assert_eq!(refusal(&engine, &kotak_quote()).kind, RefuseKind::Frozen);
        assert!(admits(&engine, &depth(SPOT)));
        assert_eq!(engine.posture_for("binance_com").unwrap().posture, "live");
    }

    #[test]
    fn eapi_and_api_share_a_ban_but_not_a_ledger() {
        let (_clock, engine) = engine();
        let opts_req = EgressRequest::new(
            OPTS,
            "eapi.binance.com",
            "GET",
            "/eapi/v1/ticker",
            Lane::MarketData,
        );

        // Separate ledgers: spending spot weight leaves the options meter alone.
        for _ in 0..16 {
            let p = admit_ok(&engine, &depth(SPOT));
            engine.record(p, &Outcome::http(200, vec![], "{}"));
        }
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::OverBudget);
        assert!(admits(&engine, &opts_req));

        // One IP: a ban taken on the options host stops the spot host too.
        let p = admit_ok(&engine, &opts_req);
        engine.record(p, &ban_response());
        assert_eq!(refusal(&engine, &opts_req).kind, RefuseKind::Banned);
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::Banned);
    }

    #[test]
    fn kotak_cash_and_nfo_pace_against_each_other() {
        let (_clock, engine) = engine();
        // Sharing is NOT SPECIFIED upstream, so they must share one meter.
        let cash = kotak_quote();
        let p = admit_ok(&engine, &cash);
        engine.record(p, &Outcome::http(429, vec![], "too many"));

        let nfo = EgressRequest::new(
            NFO,
            "gw-napi.kotaksecurities.com",
            "GET",
            "/quick/user/trades",
            Lane::PrivateRead,
        );
        assert_eq!(refusal(&engine, &nfo).kind, RefuseKind::Frozen);
    }

    // ---- Reserve lanes. ----

    #[test]
    fn a_market_data_flood_cannot_starve_private_reads() {
        let (_clock, engine) = engine();
        // 4200 market-data floor / 250 per depth call = 16 calls fills the floor.
        for _ in 0..16 {
            let p = admit_ok(&engine, &depth(SPOT));
            engine.record(p, &Outcome::http(200, vec![], "{}"));
        }
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::OverBudget);

        // Fill sync still gets through on the reserve.
        let my_trades = EgressRequest::new(
            SPOT,
            "api.binance.com",
            "GET",
            "/api/v3/myTrades",
            Lane::PrivateRead,
        )
        .with_query("symbol=BTCUSDT");
        assert!(admits(&engine, &my_trades));
    }

    #[test]
    fn budget_relief_reports_a_time_the_caller_can_use() {
        let (clock, engine) = engine();
        for _ in 0..16 {
            let p = admit_ok(&engine, &depth(SPOT));
            engine.record(p, &Outcome::http(200, vec![], "{}"));
        }
        let refused = refusal(&engine, &depth(SPOT));
        assert_eq!(refused.kind, RefuseKind::OverBudget);
        let until = refused.until_ms.expect("relief time");
        assert!(until > clock.now_ms());

        // And the window really does relieve it.
        clock.advance_ms(60_001);
        assert!(admits(&engine, &depth(SPOT)));
    }

    // ---- Header reconciliation. ----

    #[test]
    fn the_venue_header_corrects_an_over_forecast() {
        let (_clock, engine) = engine();
        // Depth forecasts 250 (the max of a published range). If the venue says it
        // only charged 5, the next 16 calls must not be refused.
        for _ in 0..16 {
            let p = admit_ok(&engine, &depth(SPOT));
            let used = (p.forecast_cost() / 50).to_string();
            engine.record(
                p,
                &Outcome::http(200, vec![("x-mbx-used-weight-1m".into(), used)], "{}"),
            );
        }
        assert!(admits(&engine, &depth(SPOT)));
    }

    #[test]
    fn the_venue_header_also_corrects_an_under_forecast() {
        let (_clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(
            p,
            &Outcome::http(
                200,
                vec![("x-mbx-used-weight-1m".into(), "5000".into())],
                "{}",
            ),
        );
        // Venue says we are already at 5000; market data's floor is 4200.
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::OverBudget);
    }

    #[test]
    fn a_meter_with_no_published_budget_reports_none_rather_than_a_guess() {
        let (_clock, engine) = engine();
        let posture = engine.posture_for("kotak_neo").unwrap();
        let meter = &posture.meters[0];
        assert_eq!(meter.meter, "kotak_neo:requests");
        assert_eq!(meter.budget_limit, None);
        assert_eq!(meter.used, None);

        let com = engine.posture_for("binance_com").unwrap();
        let api = com
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:api_weight")
            .unwrap();
        assert_eq!(api.budget_limit, Some(6000));
        assert!(api.used.is_some());
        let eapi = com
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:eapi")
            .unwrap();
        assert_eq!(eapi.budget_limit, None);
    }

    // ---- Fence and routing. ----

    #[test]
    fn an_unknown_book_has_no_meter_to_spend() {
        let (_clock, engine) = engine();
        let req = EgressRequest::new(
            "binance-com-stocks",
            "api.binance.com",
            "GET",
            "/api/v3/depth",
            Lane::MarketData,
        );
        assert_eq!(refusal(&engine, &req).kind, RefuseKind::WrongBook);
    }

    #[test]
    fn a_dated_contract_on_spot_depth_is_the_wrong_book() {
        let (_clock, engine) = engine();
        let before = engine.posture_for("binance_com").unwrap();
        let api_before = before
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:api_weight")
            .unwrap();
        let used = api_before.used;
        let inflight = api_before.inflight;

        let req = EgressRequest::new(
            SPOT,
            "api.binance.com",
            "GET",
            "/api/v3/depth",
            Lane::MarketData,
        )
        .with_query("symbol=BTC-260925-145000-C");
        assert_eq!(refusal(&engine, &req).kind, RefuseKind::WrongBook);

        let after = engine.posture_for("binance_com").unwrap();
        let api_after = after
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:api_weight")
            .unwrap();
        assert_eq!(api_after.used, used);
        assert_eq!(api_after.inflight, inflight);
    }

    #[test]
    fn a_spot_ticker_on_options_is_the_wrong_book() {
        let (_clock, engine) = engine();
        let req = EgressRequest::new(
            OPTS,
            "eapi.binance.com",
            "GET",
            "/eapi/v1/ticker",
            Lane::MarketData,
        )
        .with_query("symbol=BTCUSDT");
        assert_eq!(refusal(&engine, &req).kind, RefuseKind::WrongBook);
    }

    #[test]
    fn spot_depth_of_btcusdt_still_admits() {
        let (_clock, engine) = engine();
        assert!(admits(&engine, &depth(SPOT)));
    }

    #[test]
    fn the_existing_fence_still_decides_host_and_path() {
        let (_clock, engine) = engine();
        // Right book, wrong host for that book.
        let crossed = EgressRequest::new(
            SPOT,
            "eapi.binance.com",
            "GET",
            "/api/v3/depth",
            Lane::MarketData,
        );
        assert_eq!(refusal(&engine, &crossed).kind, RefuseKind::HostBlocked);

        // Mutations are refused by the fence, so no orders meter can ever be charged.
        let order = EgressRequest::new(
            SPOT,
            "api.binance.com",
            "POST",
            "/api/v3/order",
            Lane::Execution,
        );
        assert_eq!(refusal(&engine, &order).kind, RefuseKind::HostBlocked);
    }

    // ---- Concurrency and permits. ----

    #[test]
    fn recording_an_outcome_frees_the_concurrency_slot() {
        let (_clock, engine) = engine();
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(admit_ok(&engine, &depth(SPOT)));
        }
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::OverBudget);
        for p in held {
            engine.record(p, &Outcome::http(200, vec![], "{}"));
        }
        assert_eq!(
            engine
                .posture_for("binance_com")
                .unwrap()
                .meters
                .iter()
                .find(|m| m.meter == "binance_com:api_weight")
                .unwrap()
                .inflight,
            0
        );
    }

    // ---- Ban lifecycle. ----

    #[test]
    fn a_ban_expires_on_its_own_clock() {
        let (clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(
            p,
            &Outcome::http(418, vec![("retry-after".into(), "120".into())], "banned"),
        );
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::Banned);
        clock.advance_ms(120_001);
        assert!(admits(&engine, &depth(SPOT)));
    }

    #[test]
    fn repeat_bans_escalate_rather_than_re_guessing() {
        let (clock, engine) = engine();
        // No Retry-After and no parseable body: the local default must apply and
        // then grow, so a wrong guess cannot re-ban the IP forever.
        let mut lengths = Vec::new();
        for _ in 0..3 {
            let p = admit_ok(&engine, &depth(SPOT));
            let at = clock.now_ms();
            engine.record(p, &Outcome::http(418, vec![], "no expiry here"));
            let until = engine.posture_for("binance_com").unwrap().until_ms.unwrap();
            lengths.push(until - at);
            clock.advance_ms(until - at + 1);
        }
        assert_eq!(lengths, vec![300_000, 900_000, 2_700_000]);
    }

    #[test]
    fn a_clean_response_resets_the_escalation() {
        let (clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &Outcome::http(418, vec![], "no expiry"));
        clock.advance_ms(300_001);

        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &Outcome::http(200, vec![], "{}"));

        let at = clock.now_ms();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &Outcome::http(418, vec![], "no expiry"));
        let until = engine.posture_for("binance_com").unwrap().until_ms.unwrap();
        assert_eq!(until - at, 300_000, "escalation should have reset");
    }

    #[test]
    fn kotak_never_takes_an_ip_ban_it_has_no_documentation_for() {
        let (_clock, engine) = engine();
        let p = admit_ok(&engine, &kotak_quote());
        // Even if Kotak somehow returned a 418, no lock documents it as a ban.
        engine.record(p, &Outcome::http(418, vec![], "teapot"));
        assert_ne!(engine.posture_for("kotak_neo").unwrap().posture, "banned");
    }

    // ---- Notch. ----

    #[test]
    fn every_venue_is_always_reported_so_one_cannot_blank_another() {
        let (_clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &ban_response());

        let postures = engine.postures();
        assert_eq!(postures.len(), 2);
        let com = postures.iter().find(|p| p.venue == "binance_com").unwrap();
        let kotak = postures.iter().find(|p| p.venue == "kotak_neo").unwrap();
        assert_eq!(com.posture, "banned");
        assert_eq!(kotak.posture, "live");
    }

    #[test]
    fn a_posture_change_reaches_the_event_bus() {
        let (_clock, engine) = engine();
        let bus = Arc::new(EventBus::new(16));
        let mut rx = bus.subscribe();
        engine.attach_event_bus(bus);

        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &ban_response());

        let event = rx.try_recv().expect("expected a posture event");
        let (kind, payload) = event.sse_type_and_payload_json();
        assert_eq!(kind, "venue_egress_state");
        assert_eq!(payload["venue"], serde_json::json!("binance_com"));
        assert_eq!(payload["posture"], serde_json::json!("banned"));
    }

    #[test]
    fn a_steady_state_response_publishes_nothing() {
        let (_clock, engine) = engine();
        let bus = Arc::new(EventBus::new(16));
        let mut rx = bus.subscribe();
        engine.attach_event_bus(bus);

        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &Outcome::http(200, vec![], "{}"));
        assert!(rx.try_recv().is_err(), "no posture change, no event");
    }

    #[test]
    fn force_slot_ban_is_available_to_fixtures() {
        let (clock, engine) = engine();
        engine.force_slot_ban("binance_com", clock.now_ms() + 60_000);
        assert_eq!(refusal(&engine, &depth(SPOT)).kind, RefuseKind::Banned);
        assert!(admits(&engine, &kotak_quote()));
    }

    // ---- Connect-time auth traffic. ----

    #[test]
    fn auth_calls_respect_a_ban_the_data_path_took() {
        let (_clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        engine.record(p, &ban_response());

        // A connect retry during a ban is how a 2-minute ban becomes a 3-day one.
        let refused = match engine.admit_auth("api.binance.com", Lane::PrivateRead, 20) {
            Decision::Refuse(r) => r,
            Decision::Admit(_) => panic!("auth admitted during an IP ban"),
        };
        assert_eq!(refused.kind, RefuseKind::Banned);
    }

    #[test]
    fn auth_calls_are_charged_to_their_venue_and_only_their_venue() {
        let (_clock, engine) = engine();
        // Kotak's login host is not on the R0 data fence, but it is Kotak's.
        let permit = match engine.admit_auth("mis.kotaksecurities.com", Lane::PrivateRead, 0) {
            Decision::Admit(p) => p,
            Decision::Refuse(r) => panic!("expected admit, got {r}"),
        };
        assert_eq!(permit.slot(), "kotak_neo");
        engine.record(permit, &Outcome::http(429, vec![], "too many"));

        // Kotak froze; Binance did not.
        assert_eq!(refusal(&engine, &kotak_quote()).kind, RefuseKind::Frozen);
        assert!(admits(&engine, &depth(SPOT)));
    }

    #[test]
    fn auth_admission_still_refuses_an_unknown_host() {
        let (_clock, engine) = engine();
        // No slot, no metering, no send — including binance.us, which has no slot.
        for host in ["evil.example.com", "api.binance.us"] {
            match engine.admit_auth(host, Lane::PrivateRead, 20) {
                Decision::Refuse(r) => assert_eq!(r.kind, RefuseKind::HostBlocked, "{host}"),
                Decision::Admit(_) => panic!("admitted unknown host {host}"),
            }
        }
    }

    #[test]
    fn auth_weight_lands_on_the_same_ledger_as_data() {
        let (_clock, engine) = engine();
        let before = engine
            .posture_for("binance_com")
            .unwrap()
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:api_weight")
            .unwrap()
            .used;
        let p = match engine.admit_auth("api.binance.com", Lane::PrivateRead, 20) {
            Decision::Admit(p) => p,
            Decision::Refuse(r) => panic!("{r}"),
        };
        engine.record(p, &Outcome::http(200, vec![], "{}"));
        let after = engine
            .posture_for("binance_com")
            .unwrap()
            .meters
            .iter()
            .find(|m| m.meter == "binance_com:api_weight")
            .unwrap()
            .used;
        assert_eq!(before, Some(0));
        assert_eq!(after, Some(20));
    }

    #[test]
    fn a_long_ban_body_is_still_read() {
        let (_clock, engine) = engine();
        let p = admit_ok(&engine, &depth(SPOT));
        let body = format!(
            "banned until 1700000000000.{}",
            "x".repeat(BODY_SNIPPET_LIMIT * 2)
        );
        engine.record(p, &Outcome::http(418, vec![], &body));
        assert_eq!(
            engine.posture_for("binance_com").unwrap().until_ms,
            Some(1_700_000_000_000)
        );
    }
}

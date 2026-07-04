# Plan: Today Screen v1

> Source PRD: `.github/PRD-today-v1.md`  
> Grill-me: complete · Deep research: Perplexity 2026-07-02

## Architectural decisions

Durable decisions that apply across all phases:

- **Session day:** Local Mac calendar day; midnight rollover v1; boundaries stored as UTC instants with `local_date` key.
- **Engine:** Single `RoundTripEngine` in agent — WAC cost basis, Binance-compatible realized P&L, net USD fees, round-trip = inventory 0→>0→0 per symbol.
- **Source of truth:** `recent_fills` SQLite → engine → `round_trips` table + `daily_snapshots` table (new).
- **Today API:** `GET /api/daemon/today` (or `/api/daemon/station/today`) — one JSON payload for Today UI + pulse strip.
- **Signals:** Local only; four computed, two returned as `top_signals[]`; self-baseline + `learning_baseline` flag.
- **Kill switch:** Read existing SSE/state; Today banner display-only; resume via existing Notch/coordinator paths.
- **Currency:** USD for crypto v1 Station surfaces (deprecate INR formatting on Today/pulse path).
- **Stats:** Out of scope; daily snapshots written for Slice 2.
- **Degraded contract:** Locked 4-state matrix; API carries `degraded_reason` enum.

---

## Phase 1: Round-trip engine tracer bullet

**User stories:** 18, 19, (partial 1–3)

### What to build

Minimal engine that ingests fills from `RecentTradesStore`, reconstructs one symbol's buy→sell round-trip, computes WAC P&L net fees, handles unknown basis. No HTTP yet — library + unit tests only.

### Acceptance criteria

- [ ] Fixture: 3 fills open+close BTC/USDT → one round-trip, correct avg entry/exit, net P&L.
- [ ] Unknown-basis sell → `unknown_basis: true`, excluded from day aggregates helper.
- [ ] Fee in BNB/USDT/coin converted to USD at fill price.
- [ ] Tests in `agent/tests/` cover WAC math against hand-calculated + Binance doc example shape.

---

## Phase 2: Persistence + day boundary + Today API

**User stories:** 1–5, 14–15, 18, 20

### What to build

End-to-end agent path: engine runs over stored fills, persists `round_trips` + `daily_snapshots`, exposes `GET /api/daemon/today` with hero fields, degraded flags, learning_baseline, up to 50 closed round-trips for local day. Recompute or increment on new fill ingest hook.

### Acceptance criteria

- [ ] API returns JSON with `pnl_today_usd`, `trades_today`, `win_rate`, `trades[]`, `degraded_reason`, `learning_baseline`.
- [ ] Sync paused / no broker → `degraded_reason: sync_unavailable`; numeric fields null not zero.
- [ ] Day rollover assigns fills to correct `local_date`.
- [ ] Integration test: mock fills → API → expected hero numbers.

---

## Phase 3: Local behavior signals

**User stories:** 6–8, 10

### What to build

Signal detector on round-trip stream: loss chasing, revenge, overtrading, position sizing; self-baseline from rolling history; attach primary flag to each round-trip; return top 2 signals for Today payload.

### Acceptance criteria

- [ ] Cold start → `learning_baseline: true`, signal copy matches prototype B tone.
- [ ] Fixture: loss→fast re-entry→escalating size fires loss chasing + revenge.
- [ ] Table row flags match signal taxonomy (Firing > Watch > Clean).
- [ ] No brain HTTP calls for Today signals.

---

## Phase 4: Station Today UI

**User stories:** 9–13, 14–15

### What to build

Replace `StationPlaceholderView` for `.today` with `TodayView`: breaker banner, 3 hero tiles, 2 signal cards, trades table; wire to Today API via agent client; honor degraded matrix; Binance palette tokens.

### Acceptance criteria

- [ ] Matches prototype variants A/B/C (active, empty, degraded) against API/mock.
- [ ] Resume button disabled until countdown (reuse NotchViewModel kill switch state).
- [ ] Navigation/shell remains usable when agent degraded.
- [ ] `StationTests` cover presentation for four matrix states.

---

## Phase 5: Pulse strip + release gates

**User stories:** 16–17

### What to build

Wire `SessionPulseStrip` to Today API session P&L (or shared view model projection); USD formatting; same degraded rules. Backend Box release gate tests extended for Today smoke checklist.

### Acceptance criteria

- [ ] Pulse strip P&L equals Today hero when healthy.
- [ ] Pulse shows `—` when `degraded_reason` set.
- [ ] `agent/tests/today_release_gates.rs` + `station/StationTests/TodayReleaseGateTests.swift` pass.
- [ ] Manual smoke: connect Binance.US → one round-trip → Today + pulse agree.

---

## Phase 6 (deferred — Slice 2): Stats UI

**Not in this plan execution.** Prerequisites: Phase 1–5 shipped, ≥30 round-trips soak, deep research refresh for unlock copy.

- Read `daily_snapshots` + `round_trips`
- Implement Health UI per `PROTOTYPE-station.html` `#s-stats`
- Apply research unlock gates (30/100/200 RT)

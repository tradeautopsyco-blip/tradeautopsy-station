# PRD — Today Screen v1: Session Mirror + Round-Trip Engine

**Status:** Ready for agent  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Parent context:** TradeAutopsy Station v1 · Backend Box v1 (Brokers) complete · Phase 0 shell complete  
**Grill-me:** Complete (11 decisions, 2026-07-02)  
**Deep research:** Perplexity brief (2026-07-02, confidence ≈0.8) — Today-first; Stats deferred Slice 2  
**Design reference:** `station/prototypes/PROTOTYPE-station.html` (variant A/B/C)  
**Out of scope:** Stats UI (Slice 2), Pre-trade/Live/Post-trade screens, morning reflection input, brain-sourced P&L, kill-switch logic changes

---

## Problem Statement

TradeAutopsy Station has a Phase 0 shell with a **Today route placeholder** and Backend Box v1 broker sync, but traders still have no **honest same-day behavioral mirror** in the Station window:

1. **No round-trip P&L** — the agent stores raw Binance.US fills only; spot has no exchange-provided realized P&L. Today cannot show trustworthy `P&L today`, win rate, or per-trade flags without a local reconstruction engine.
2. **No single source of truth** — Notch `session_pnl` / `win_rate` come from hosted intelligence (INR / Indian-equities heritage). They do not reconcile with Binance.US spot fills and violate Backend Box redaction/opt-out posture for crypto v1.
3. **No local behavior signals for Today** — signals that depend on brain upload fail for opt-out users and add latency; Today needs local computation on the same round-trip engine.
4. **Misleading empty states** — showing `0` or stale hosted numbers when sync is paused or history is empty erodes trust in a discipline product.
5. **No history spine for Stats** — deferring Stats UI is correct (research: win rate/SQN unreliable below ~30 trades), but without daily snapshots from a proven engine, Slice 2 would recompute ad hoc and drift from Today.

Today v1 must ship the **round-trip spine + Today screen** as one vertical product: immediate session feedback (research-supported for weeks 1–4) before historical trends.

---

## Solution

Ship **Today v1** as a read-only session mirror in the Station window, backed by a **local agent round-trip + weighted-average-cost (WAC) P&L engine** (Binance Trade Analysis compatible formula).

North-star acceptance statement:

> A Binance.US-connected trader opens Today and sees honest same-day P&L, trade count, win rate, top behavior signals, flagged closed round-trips, and circuit-breaker state — all from one local engine that matches Binance realized P&L for closed round-trips (or shows `—` with reason when unknown), never stale hosted INR numbers.

### Today screen layout (v1)

1. **Circuit breaker banner** — only when kill switch actively firing/constraining; display + existing resume/countdown flow (no new agent kill logic).
2. **Hero — 3 tiles:** P&L today · Trades today · Win rate (no avg hold, no streak).
3. **Behavior signals — top 2** by severity (from four computed locally: loss chasing, revenge, overtrading, position sizing).
4. **Trades today table** — closed round-trips only, primary flag per row, ~50 cap, most recent first.

**No bottom section** — no Voice of trader quotes, no morning reflection input (capture stays Notch + Journal).

### Session boundary

- **Calendar day** in trader's Mac local timezone; **midnight rollover** in v1 (configurable rollover deferred).
- **Honest empty:** `—` + caption; never `0` / `0%` when empty or unknown; never backfill yesterday into today.

### P&L engine (agent — performance basis, not tax)

- Reconstruct **round-trips** from fills (open → flat per symbol).
- **WAC** cost basis matching Binance spot Trade Analysis:  
  `Realized PnL = sell_qty × (avg_sell − avg_buy)`, position avg resets at flat.
- **Net of fees**; fees normalized to USD at fill time.
- **Unknown basis:** sells with no synced buy history → exclude from hero aggregates; table row with `—` P&L + `Unknown basis` flag.
- Disclosure in UI: *performance basis, not tax basis* (FIFO tax export = future slice).

### Behavior signals (local, zero-config)

- Self-calibrating baselines from trader history (median trades/day, inter-trade gaps, size σ).
- **Cold start:** `< ~10 round-trips` or insufficient days → **Learning baseline** badge, conservative thresholds.
- Today shows **top 2** by severity; all four computed for flags/table.

### Degraded / empty matrix (locked)

| State | Hero | Signals | Table | Breaker |
|-------|------|---------|-------|---------|
| Agent down | `—` unavailable | `—` unavailable | empty | hidden |
| Sync paused/stale | `—` sync paused | `—` unavailable | empty | cached ≤60s if any, else hidden |
| Healthy, no trades today | `—` no trades yet | learning baseline | empty state | if firing |
| Healthy + trades | real | real / learning | rows | if firing |

### Pulse strip (v1)

- Wire to **same Today payload** for session P&L (realized today from engine).
- USD formatting for crypto v1 (replace INR `formatINR` path for Station Today/pulse).
- Same degraded matrix: `—` when agent unhealthy or broker sync unavailable.

### Daily snapshots (guts for Slice 2 — write in v1, no Stats UI)

Persist per local calendar day (minimal):

- `date`, `round_trips_closed`, `net_pnl_usd`, `wins`, `losses`
- `discipline_index` (composite placeholder OK v1), `trades_count`
- `learning_baseline`, `unknown_basis_count`

Stats UI unlock gates (Slice 2 PRD — research-backed): W charts ≥30 RT or ≥10 days; M/6M ≥100 RT; YoY ≥12mo + ≥200 RT.

### Visual tokens (Today only)

- Profit/clean: `#0ECB81` · Loss/firing: `#F6465D` · Watch: `#FF7A6B` · Neutral/empty: `#EDEDED`
- Always pair color with sign/icon/text (colorblind-safe)

---

## User Stories

### Session mirror

1. As a crypto trader, I want Today to show my **P&L today** in USD net of fees, so that I confront today's outcome without opening Binance.
2. As a trader, I want P&L today to **reconcile with Binance Trade Analysis** for closed round-trips, so that I trust the number.
3. As a trader, I want **Trades today** and **Win rate** for closed round-trips only, so that hero metrics match the trades table.
4. As a trader, when I have no closed trades today, I want **`—` not zero**, so that I am not misled into thinking I had a flat day.
5. As a trader, I want the session day to follow **my local calendar**, so that "today" matches how I live the day (24/7 crypto).

### Behavior signals

6. As a trader, I want **top behavior signals** on Today from local fills, so that feedback works when brain upload is opted out.
7. As a trader, I want signals to **learn my baselines** without configuration, so that v1 stays low friction.
8. As a trader, I want a **learning baseline** state when history is thin, so that early signals are honest not confident.

### Trades table

9. As a trader, I want **one row per closed round-trip** with avg entry/exit and net P&L, so that the table matches how I think about trades.
10. As a trader, I want **primary behavior flag** per row (Firing > Watch > Clean), so that I see which trades drove today's story.
11. As a trader, I want **open positions omitted** from the table, so that half-finished round-trips are not faked as complete.

### Circuit breaker

12. As a trader, I want the **circuit breaker banner** on Today when kill switch is firing, so that Station and Notch agree.
13. As a trader, I want **Review & resume** to use the existing dismiss/ack flow with countdown gating, so that no new resume mechanism is invented.

### Degraded honesty

14. As a trader, when broker sync is paused, I want Today to show **`—` everywhere**, so that I am never shown stale P&L.
15. As a trader, when the agent is unhealthy, I want Today navigable with placeholders, so that I can reach Brokers and retry.

### Pulse strip

16. As a trader, I want the **pulse strip session P&L** to match Today hero P&L, so that one number appears everywhere.
17. As a trader, I want pulse strip degraded to **`—`** when sync/agent unhealthy, so that stale broker numbers never show.

### Engineering / spine

18. As an engineer, I want **one round-trip engine** feeding Today API, pulse strip, flags, and daily snapshots, so that Stats Slice 2 cannot drift.
19. As an engineer, I want unknown-basis trades **excluded from aggregates** but visible in the table, so that honesty is structural.
20. As an engineer, I want Today API to include **explicit completeness/degraded flags**, so that Station never guesses state.

---

## Non-Goals (v1)

- Stats screen / Health charts (Slice 2; prototype is design ref only)
- Pre-trade declaration, declared session overlay, configurable day rollover
- Morning reflection / Voice of trader / capture on Today
- Brain-sourced P&L or behavior signals for crypto Today
- FIFO tax export, escrow match screen, open-position rows in table
- Kill switch logic changes (Slice B done)
- Order placement, withdrawals, multi-broker

---

## Research anchors (deep research — do not ship trends without these)

| Finding | Product rule |
|---------|----------------|
| Win rate / SQN unreliable <30 trades; meaningful ~100–200 | Stats deferred; Today shows same-day only |
| Immediate feedback drives behavior change weeks 1–4 | Today v1 is primary value |
| Journals caveat advanced stats; show process metrics early | Learning baseline badges; no fake YoY |
| One engine + daily aggregates | Snapshots in v1 agent; Stats reads later |
| Priority KPIs: discipline, trades/day, exit discipline | Snapshot fields prioritized accordingly |

Future slices **must run deep research** (Perplexity or equivalent) before expanding metrics, unlock thresholds, or new jurisdictions.

---

## Acceptance Criteria (release gate)

### Engine

- [ ] Closed round-trip on BTC/USDT test fixture → P&L matches hand-calculated WAC net of fees.
- [ ] P&L matches Binance Trade Analysis for same fill set within $0.01 tolerance (manual smoke).
- [ ] Unknown-basis sell excluded from hero; row shows `Unknown basis`.
- [ ] Daily snapshot written on round-trip close and at day boundary.

### API

- [ ] `GET` Today payload returns hero, signals (max 2 displayed server-side or client ranks), trades (≤50), degraded flags, learning_baseline.
- [ ] Payload documents `performance_basis_not_tax`.

### Station UI

- [ ] Today route replaces placeholder; matches prototype variants A/B/C behavior.
- [ ] Degraded matrix implemented per locked table.
- [ ] Circuit breaker banner read-only + resume uses existing NotchViewModel kill-switch paths.
- [ ] Pulse strip session P&L equals Today hero P&L when healthy.

### Tests

- [ ] Agent integration tests: round-trip reconstruction, WAC, unknown basis, day boundary.
- [ ] Station tests: presentation matrix, empty/degraded/active states.
- [ ] No hosted INR session_pnl used for Today/pulse when broker connected.

---

## Dependencies

- Backend Box v1: Binance.US sync, `recent_fills`, broker sync state, Keychain credentials.
- Phase 0: `StationAppCoordinator`, `StationRoute.today`, `SessionPulseStrip`, kill switch SSE (Slice B).
- Reference: `docs/reference/crypto/binance-us/spot/REST.md`, Binance Trade Analysis FAQ (WAC realized PnL).

---

## Further Notes

- **Stats Slice 2 PRD** follows after Today v1 release gate passes; use `PROTOTYPE-station.html` Stats half + research unlock gates.
- **Grill decisions log:** local day · WAC engine · local signals · 3+2 layout · no capture · Stats out · degraded matrix · pulse same engine · palette locked.

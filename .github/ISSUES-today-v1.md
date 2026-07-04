# Today v1 — Implementation Issues (Cursor slice prompts)

> Generated from: `.github/PRD-today-v1.md` + `plans/today-v1.md`  
> Grill-me complete · Deep research locked (Stats = Slice 2)  
> Use each issue as a **single Cursor prompt / slice**. Order matters.

---

## Issue 1 — Agent: Round-trip + WAC engine (library only)

**Phase:** 1  
**Repos:** `agent/`

**Prompt seed:**
Build the `RoundTripEngine` in the agent: ingest `BrokerFill` / recent_fills rows, reconstruct round-trips per symbol (inventory 0→flat), compute WAC realized P&L net USD fees (Binance Trade Analysis formula). Handle unknown basis, fee asset normalization. Unit tests only — no HTTP yet. Reference `docs/reference/crypto/binance-us/spot/REST.md` and PRD `.github/PRD-today-v1.md`.

**Done when:** Phase 1 acceptance criteria in `plans/today-v1.md` pass.

---

## Issue 2 — Agent: round_trips + daily_snapshots schema + Today API

**Phase:** 2  
**Repos:** `agent/`

**Prompt seed:**
Add SQLite persistence for `round_trips` and `daily_snapshots`, hook engine on fill ingest, implement `GET /api/daemon/today` returning hero + trades table + `degraded_reason` + `learning_baseline`. Local calendar day boundary (Mac TZ via request header or system default). Honest empty: null not zero. See PRD degraded matrix.

**Done when:** Integration test fill→API; sync paused returns degraded payload.

---

## Issue 3 — Agent: Local behavior signals + row flags

**Phase:** 3  
**Repos:** `agent/`

**Prompt seed:**
Add local behavior signal detector (loss chasing, revenge, overtrading, position sizing) with self-calibrating baselines and cold-start learning mode. Attach primary flag to each round-trip; include `top_signals` (max 2) in Today API. No brain calls.

**Done when:** Signal tests + flag taxonomy on table rows match PRD.

---

## Issue 4 — Station: TodayView UI

**Phase:** 4  
**Repos:** `station/`

**Prompt seed:**
Replace Today placeholder with `TodayView` per `station/prototypes/PROTOTYPE-station.html` variant A/B/C: breaker banner (display-only), 3 hero tiles, 2 signals, round-trip table, Binance palette. Wire to Today API client. Degraded matrix from PRD. Route in `StationShellView` for `.today`.

**Done when:** UI matches prototype states; StationTests for presentation matrix.

---

## Issue 5 — Station: Pulse strip wired to Today engine

**Phase:** 5  
**Repos:** `station/`, `notch/` (presentation only)

**Prompt seed:**
Wire `SessionPulseStrip` session P&L to Today API payload (same number as hero). USD formatting for crypto v1. Degraded `—` when API degraded. Remove hosted INR dependency for this path when broker connected.

**Done when:** Pulse equals hero; release gate tests pass.

---

## Issue 6 — Release gates + manual smoke checklist

**Phase:** 5  
**Repos:** `agent/tests/`, `station/StationTests/`, `.github/`

**Prompt seed:**
Add `today_release_gates.rs` and `TodayReleaseGateTests.swift` mirroring Backend Box gate pattern. Document manual Binance.US smoke steps in `.github/TODAY_V1_SMOKE_CHECKLIST.md`: connect → round-trip → P&L matches Binance → degraded when paused.

**Done when:** CI green; checklist written.

---

## Deferred — Issue 7 (Slice 2): Stats screen

**Do not start until Issues 1–6 shipped.**

**Prompt seed (future):**
Stats UI from `PROTOTYPE-station.html` `#s-stats`; read `daily_snapshots`; apply research unlock gates (30/100/200 RT). Separate PRD `.github/PRD-stats-v1.md` TBD.

---

## Quick reference — locked grill decisions

| Topic | Decision |
|-------|----------|
| Session | Local calendar day, midnight v1 |
| P&L | Agent WAC, Binance-compatible, net fees |
| Signals | Local, self-baseline, show top 2 |
| Table | Closed round-trips only |
| Capture on Today | None |
| Stats | Slice 2 |
| Degraded | 4-state matrix (PRD) |
| Pulse | Same engine as Today |
| Research | Deep research before new metrics/thresholds |

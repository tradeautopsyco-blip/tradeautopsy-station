# Plan: P5 — Live money truth

> Source: [FULL-COVERAGE-PROGRAM.md](../../issues/brokers/FULL-COVERAGE-PROGRAM.md) Phase 5 · [P5-P9-LAUNCH-EXECUTION-PLAN.md](../../issues/brokers/P5-P9-LAUNCH-EXECUTION-PLAN.md) §2  
> Status: **IN PROGRESS** (2026-09-24 IST)

## Architectural decisions

- **One owner per SHIPPING book** — `MoneyOwner::Engine(path)` or `ExplicitNone(path)` in `agent/src/money_matrix.rs`; Console mirrors in `lib/station/money-matrix.ts`.
- **Today hero** — partition fills before engines: INR cash WAC, NFO `(exit−entry)×qty×lot`, COM spot WAC only via `is_binance_com_spot_fill`; USDM/Coin-M/options excluded from spot WAC.
- **Charges** — sole IND statutory owner: Console `lib/charges/india-charge-engine.ts`; rates from locks only (NFO STT 0.15% / 0.05% post-2026-04-01; CNC 0.1% both sides).
- **Clocks** — NFO 15:40 vs cash 15:30 (`BookSessionClock`, `nseSessionForBookId`).

---

## Phase P5-1: Matrix ↔ registry (16 books)

**Status:** done (2026-09-24)

- Station `SHIPPING_BOOK_COUNT = 16` (cash×6, spot, NFO×4, options explicit-none, USDM, Coin-M, CDS, MCX).
- Console matrix extended with `kotak-nse-cds`, `kotak-mcx-future`; tests expect 16 rows.
- P4 CEX spot books **not** in matrix until CLAIM-REGISTRY SHIPPING rows exist (fence test documents this).

**Verify:** `cargo test --lib money_matrix` · `npm test -- money-matrix`

---

## Phase P5-2: Today routing

**Status:** done (core)

- `partition_fills` + `build_payload` desk selection (NFO / INR cash / COM USD).
- Tests: NFO never spot WAC; USDM never spot WAC.

**Open:** wire NFO hero when active connection book is `kotak-nse-nfo` (etc.) in live multi-desk UX — engine path exists.

**Verify:** `cargo test --lib today::service::tests`

---

## Phase P5-3: Coin-M / USDM owners

**Status:** done (owners + unit tests)

- `coinm_realized_pnl.rs` — dapi income REALIZED_PNL sum (no `None` stub).
- `usdm_realized_pnl.rs` — fapi income identity; COMMISSION/funding deferral documented in locks.

**Verify:** `cargo test --lib coinm_realized usdm_realized`

---

## Phase P5-4: Binance options

**Status:** explicit-none (by design for P5 exit)

- `options_realized_pnl.rs` — no realized hero until lock amended; not `userTrades` alone.

---

## Phase P5-5: Console charges + matrix

**Status:** in progress

- `india-charge-engine.test.ts` — qty>1 FO, official rates, banned 0.1% options sell.
- **P5-8:** pre-beta blocker #2 (hosted PnL bugs) — separate Console PR when triaged.

**Verify:** `npm test -- india-charge-engine money-matrix market-hours`

---

## Phase P5-6: Clocks

**Status:** done (Notch + Console helpers)

- `BookSessionClockTests` — 15:30 cash vs 15:40 NFO window.

---

## Phase P5-7: Docs / registry text

**Status:** done (2026-09-25)

- [x] CLAIM-REGISTRY twenty-two-book sentence (money matrix row count)
- [x] `kotak-nse-bse-cash.md` + ASSET-CLASS §9: `inr_cash_wac.rs` **SHIPPED** owner (not future)

---

## P5 exit checklist (FULL-COVERAGE)

- [x] Money matrix: 16 SHIPPING books → owner or explicit-none
- [x] Charge-lock snapshot tests (Console india-charge-engine)
- [x] Today routing: non-spot fills cannot hit spot WAC (tests)
- [ ] Console pre-beta blocker #2 closed (hosted product)
- [ ] Optional: add P4 spot books to matrix after registry SHIPPING rows for `bybit-com-spot`, etc.

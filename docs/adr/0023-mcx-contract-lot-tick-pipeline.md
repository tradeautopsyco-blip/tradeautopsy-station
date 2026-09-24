# ADR 0023: MCX commodity futures — contract lot/tick provenance

**Status:** ACCEPTED (founder execution 2026-09-24 IST — P9-C tracer)

**Date:** 2026-09-24 IST

**Number note:** P9 territory plan (`plans/multi-asset-p9-territories-2026-09-24.md`) labels
**0020** for this law; founder bundle pre-assigns **0023** to MCX lot/tick provenance. MT5
platform-family law in that plan (**0023** there) renumbers when MT5 scope is accepted.

**Program:** P9-C (MCX tracer) · `issues/compliance/CLAIM-REGISTRY.md` (MCX REFERENCE → SHIPPING
per book) · ADR 0004 (closed `asset_class` / `instrument_class`) · ADR 0019 (six-step pipeline)

**Default tracer book:** `kotak-mcx-future` (one commodity futures root on Kotak Neo; Zerodha
replication after tracer dogfood — same ladder as P6 NFO).

---

## Context

1. **MCX is a separate venue segment, not NSE F&O.** Kotak fills may carry `exSeg: mcx_fo`.
   The cash book already **refuses** F&O URLs including `mcx_fo.csv` (`kotak_scrip_master.rs`).
   The NFO named book (`kotak-nse-nfo`) owns **`nse_fo` only** (`kotak_nfo_scrip.rs`). MCX lot
   and tick cannot be borrowed from the NFO master, from cash, or from equity lot tables.

2. **Lot/tick are contract fields, not desk guesses.** Station roadmap law (R5 sizer): NFO uses
   **lots from the venue lot on the contract row**, never a standalone `fo_mktlots.csv` fiction.
   That file (and any broker-local lot summary not keyed to the same instrument identity as fills)
   is **not** provenance — it is at best a stale index and at worst wrong for the traded month.
   MCX inherits the same refusal: if the value did not come from the **venue master row** for
   that contract, it does not ship.

3. **Kotak already exposes an MCX master URL in the file-paths catalog.** Official SDK sample
   and live `filesPaths` payloads list `mcx_fo.csv` under the scripmaster host (same family as
   `nse_fo.csv`, different segment). The NFO book pattern is proven: book-scoped fence → unsigned
   GET of the allowlisted CSV path → parse with a **lock-grade header** → cache by segment → join
   fills/quotes/sizer on `instrument_id`. MCX is the next named book on that pattern, not a new
   architecture.

4. **Taxonomy is decided at the book.** Per ADR 0004, host stamps `(asset_class, instrument_class,
   is_inverse)` from the shipping **book** row. MCX futures tracer v1:
   **`commodity` + `future` + false**. MCX options (if added later) are a **separate book** or
   an explicit lock amendment — not a silent mixed stamp.

5. **Locks follow books.** NFO law lives in a book lock (conceptually:
   `issues/compliance/locks/kotak-nse-nfo.md` — segment `nse_fo`, FO CSV header, product map,
   session clock, goldens). MCX SHIPPING requires **`issues/compliance/locks/kotak-mcx-future.md`**
   (name illustrative until filed) with at minimum: segment id, allowed master URL shape,
   **`LOCK_HEADER`** (or equivalent column contract), futures instrument-type filter, tick/lot
   column semantics, and dogfood symbols. This ADR does **not** substitute for that lock; it
   records pipeline law only.

---

## Decision

### 1. Venue master file requirement (only source of lot/tick)

For `kotak-mcx-future` (and sibling MCX books on other slugs):

| Field | Provenance |
|-------|------------|
| **Lot size** | Parsed from the contract row on the **MCX FO scrip master CSV** for that book’s allowlisted file (Kotak: `mcx_fo.csv` path from `GET …/masterscrip/file-paths`, same endpoint family as NFO). |
| **Tick size** | Raw tick cell from the same row (Kotak: `dTickSize` per lock header — **scale/routing is lock-owned**; parser stores raw like NFO). |
| **Instrument identity** | Segment + token (or lock-defined id) consistent with fills and quotes on that book — **not** cash `nse_cm\|token`. |

**Hard rules:**

- **No `fo_mktlots.csv` (or equivalent orphan lot table)** in host, adapter, Notch sizer, or tests.
  CI and code review treat any import/path of that filename as STOP until an ADR amendment proves
  join keys match the venue master row-for-row.
- **No default lot** when master lookup misses: behave as **NOT SPECIFIED IN SOURCE** → reject,
  park, or dashed UI — same fail-closed posture as missing Binance `exchangeInfo` filters.
- **No cross-book master bleed:** `kotak-nse-bse-cash`, `kotak-nse-nfo`, and `kotak-mcx-future`
  each fetch, cache, and parse **their** CSV only; shared HTTP helpers are fine, shared row stores
  are not.

### 2. Extend the Kotak NFO lot pipeline pattern (implementation shape)

Replicate the **named-book scrip master** pattern established for `kotak-nse-nfo` (see
`agent/src/kotak_nfo_scrip.rs`), adapted for MCX:

1. **Book id and fence** — `kotak-mcx-future`; `authorize_book_call` / egress allowlist entries
   for file-paths + scripmaster host + **mcx** CSV path only (mirror NFO’s split: signed
   file-paths, unsigned CSV fetch).
2. **URL picker** — From file-paths JSON, keep URLs whose path is the lock-allowlisted MCX FO
   CSV (reject `nse_fo`, cash, `cde_fo`, etc., even if listed).
3. **Parser** — `from_csv_bytes` requires header **exact match** to lock `LOCK_HEADER` (MCX file
   may differ from NFO’s 79-column header — the MCX lock owns the string; do not reuse NFO’s
   header blindly).
4. **Segment filter** — Rows where exchange segment equals **`mcx_fo`** (case policy per lock).
5. **Lot cells** — Reuse **`lot_from_cells(lLotSize, iLotSize)`** semantics: both present and
   unequal → skip row (which-wins NOT SPECIFIED); one present → use it; neither → skip.
6. **Futures filter** — Tracer v1 indexes **commodity futures** rows only; options/spreads wait
   for lock + book split (same “do not treat FUT as OPT” discipline as NFO).
7. **Cache** — Segment-keyed disk cache (NFO uses `kotak_csv_cache_path(…, "nse_fo")`; MCX uses
   `"mcx_fo"` or lock-named key).
8. **Refresh** — Background refresh wired like NFO master refresh; empty fetch must not wipe a
   loaded non-empty master (`install_master_if_nonempty` pattern).
9. **Consumers** — Account split / fill stamp helpers, quote bind, contract chain, and (when
   shipped) commodity sizer read lot/tick **only** through this master + `ContractRow` (or shared
   `commodity_lot_*` read module extracted once MCX and NCDEX both need it).

Reference the NFO lock **pattern** only: segment allowlist, header pin, golden CSV fixture,
book-scoped dogfood — **do not copy** live URLs, tokens, or founder-specific symbols into this
ADR or into code comments as secrets.

### 3. Catalog and calc profile

Add (when integrator lands P9-C):

| book_id | slug | asset_class | instrument_class | is_inverse | notes |
|---------|------|-------------|------------------|------------|-------|
| `kotak-mcx-future` | `kotak_neo` | commodity | future | false | Tracer; manifest id in lock |

Calc profile family **`commodity_inr_mcx`** (or lock-named id) is **descriptive** — quoting/settlement
currency and session calendar come from the MCX lock, not from this ADR.

### 4. Session and clock

MCX trading hours and holiday calendar are **book-local**. The NSE cash session clock must not
drive MCX obtain/sync scheduling. Exact bindings are lock + host scheduler follow-up (same class
of decision as NFO vs cash clocks).

### 5. Verification (standing — tracer tier)

Tier I (ADR 0019 steps 1–5) for `kotak-mcx-future` includes:

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test --lib kotak_mcx   # module name illustrative
cargo test --test ubi_kotak_neo_component  # extend with MCX fixtures
```

**Golden assertions (required before Tier II):**

- Planted `mcx_fo` CSV fixture row → known lot/tick for a named test symbol.
- Fill normalized with that symbol → lot join succeeds; **deliberately wrong** orphan lot file
  is absent from repo and fences.
- Cash book + NFO book tests still refuse `mcx_fo` segment rows end-to-end.

Tier II adds signed dogfood plan for the tracer symbol(s) named in the MCX lock.

---

## Considered options

1. **Reuse `fo_mktlots.csv` or broker PDF lot tables.** Rejected: not contract-keyed to the same
   identity as fills; violates roadmap sizer law and creates silent qty drift across expiries.
2. **Parse MCX rows inside `kotak_nfo_scrip.rs`.** Rejected: violates named-book separation;
   NFO lock and fence are `nse_fo`-only; mixing segments reintroduces F3-style bleed.
3. **Stamp MCX fills from slug default (`equity` / `spot`).** Rejected: ADR 0004 host stamp is
   book-keyed; MCX must be `commodity` + `future` from `kotak-mcx-future`.
4. **Defer master until first live trade.** Rejected for Tier I: fixtures + header pin + lot join
   tests are step 5 deliverables; live account is step 6 only.

---

## Consequences

**Positive**

- One proven pipeline from NFO → MCX → (later) NCDEX commodity masters with shared lot-cell logic.
- Clear marketing/compliance story: lot/tick citations trace to venue CSV + lock header, not folklore.
- Sizer and display can share `ContractRow` lot with realized-PnL owners when commodity money
  books ship.

**Negative / follow-ups**

- New module, fence rows, cache key, and lock file — integrator slice; not bundled into NFO PRs.
- MCX `LOCK_HEADER` must be captured from official CSV (like NFO) before parser merges; until
  then, code stays behind feature/book gate.
- NCDEX (`P9-C2`) waits for MCX tracer green; shared `commodity_lot_*` extraction happens once
  second venue confirms column shape (ADR **0021** territory plan — not decided here).
- Plan table still says ADR **0020** for MCX; update plan index when this ADR is **ACCEPTED**.

**Does not change**

- ADR 0001 (Wasm, no secrets in component), 0004 (taxonomy), 0019 (pipeline tiers).
- NFO book scope (`kotak-nse-nfo` / `nse_fo` only).
- Refusal of `mcx_fo` on cash book and Kotak cash historical paths (unchanged).

---

## References

- Kotak NFO scrip master implementation — `agent/src/kotak_nfo_scrip.rs` (pattern only)
- Kotak file-paths / segment sample — `docs/reference/india/kotak-neo/REST.md` (`mcx_fo.csv` listed; cash v1 refuse)
- P9 MCX checklist — `plans/multi-asset-p9-territories-2026-09-24.md` (Phase P9-C)
- Sizer lot law — `plans/station-completion-roadmap-2026-09-17.md` §3.1 (`fo_mktlots.csv` refusal)
- Book lock pattern (conceptual) — `issues/compliance/locks/kotak-nse-nfo.md` (segment header, goldens — MCX lock is sibling)

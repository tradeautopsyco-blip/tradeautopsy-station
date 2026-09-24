# ADR 0021: Spot FX calc — pip size, lot size, swap/rollover display vs PnL input

**Status:** DRAFT

**Date:** 2026-09-24 IST

**Number note:** P9 territory plan (`plans/multi-asset-p9-territories-2026-09-24.md`) once labeled **0016**
for spot FX calc law; **0016–0020** are now taken (crypto session ADRs, launch ladder, IB transport
**0020**, and sibling territory slots). Founder bundle pre-assigns **0021** to the first **`fx` +
`spot`** book (P9-A). Plan index rows that still cite **0016** for this topic must be repointed when
this ADR is **ACCEPTED**.

**Blocks:** No registry **SHIPPING** flip, no `fx_spot_usd` calc profile registration, no IB spot FX
Wasm, and no Today routing change for FX until **ADR 0020** (Interactive Brokers transport) is
**ACCEPTED**. Transport choice (Flex vs Portal vs socket) is not reopened here.

**Program:** P9-A (spot forex tracer) · ADR 0004 (closed taxonomy, book-keyed catalog) · ADR 0019
(six-step pipeline) · ADR 0020 (IB W5.1 — one slug, many books) ·
`plans/multi-asset-p9-territories-2026-09-24.md` (tracer then replicate)

**Default tracer book:** `ib-fx-spot` on slug `interactive_brokers` — **one** pair and entity named in
`issues/compliance/locks/ib-fx-spot.md` (illustrative id; lock may amend `book_id` string but must
stay single-pair for tracer v1).

---

## Context

1. **First closed `(fx, spot)` book.** Station already stamps `asset_class = fx` and
   `instrument_class = spot` on the wire (ADR 0004). No shipping book uses that pair today. Crypto
   spot (`crypto_spot_usd`) and INR cash WAC are **wrong owners** for IB spot FX fills — different
   asset axis, different quoting conventions (pips, contract multipliers), and different charge rows.

2. **IB is multi-book on one slug.** ADR 0020 commits to **one transport** for US equity cash, spot
   FX, and later IB CFD books on `interactive_brokers`. Slug-level desk resolution
   (`desk_profile_for_slug` → first catalog row for slug) is **unsafe** once a second IB book exists:
   Today could show USD equity calc chrome while ingesting FX fills, or blend quote currencies. P9-A
   must route Today (and any calc-profile-gated display path) by **`book_id`**, not slug.

3. **Pip and lot are display and sizing law, not secret rates.** Spot FX autopsy needs honest pip
   distance, tick-aligned prices, and lot-normalized quantity **for UI and contract join** — without
   inventing swap points, spreads, or statutory charges in the ADR. All numeric factors live in the
   lock + B6 sheet with URL + fetch date.

4. **Swap / rollover is a common confusion surface.** Brokers report overnight financing, swap,
   “roll”, or interest lines separately from trade commissions. Users expect to **see** those amounts;
   realized-PnL engines must only **consume** rows the money owner and lock explicitly classify.
   `instrument_class = swap` (perpetuals) is a **different** axis — this ADR covers **spot FX**
   overnight financing display only.

5. **Calc profile discipline (ADR 0004).** A calc profile names **quote currency + asset class** for
   desk honesty; it is **not** a money owner. Profile id **`fx_spot_usd`** is the default for the
   tracer when quote/settlement is USD-class per lock; the lock may amend the id (e.g. non-USD quote
   on a different first book) before **ACCEPTED**.

6. **Registry posture.** Spot forex rows in `issues/compliance/CLAIM-REGISTRY.md` are **REFERENCE**
   until founder flip. Pipeline stays at ADR 0019 step 0 for IB FX until **0020 ACCEPTED** and this
   ADR **ACCEPTED** + lock **SHIPPING**.

---

## Official sources (required before ACCEPTED — placeholders)

Implementers must cite **fetch date + URL** in `issues/compliance/locks/ib-fx-spot.md` and B6
`issues/brokers/sheets/interactive_brokers.md` (FX rows). Until verified, every cell below stays **NOT
SPECIFIED IN SOURCE**.

| Topic | Intended authority | Status |
| --- | --- | --- |
| Contract size / lot multiplier for the tracer pair | IB contract specifications + Flex/trade field glossary for IDEALPRO / FX | NOT SPECIFIED IN SOURCE |
| Minimum price increment / pip convention for the pair | IB + ISO 4217 minor units; JPY-quote exception policy | NOT SPECIFIED IN SOURCE |
| Swap / financing / interest report fields on chosen transport | IB Flex Query column map (ADR 0020 Option A) or successor transport docs | NOT SPECIFIED IN SOURCE |
| Commission and tax rows affecting realized PnL | IB execution + commission reports; US/entity-specific rows B6-owned | NOT SPECIFIED IN SOURCE |
| First-book entity (IB LLC vs IBSG vs IBUK) | Founder lock + B6 jurisdiction row | NOT SPECIFIED IN SOURCE |

Oracle / OpenAlgo BAR entries for forex are **reference only** — never pip size, never swap rates.

---

## Decision

### 1. First-book identity (`ib-fx-spot`)

| Field | Law |
| --- | --- |
| **book_id** | `ib-fx-spot` unless lock amends (single pair, single entity for tracer v1) |
| **slug** | `interactive_brokers` (same Wasm adapter id and IB session family per ADR 0020) |
| **asset_class** | `fx` |
| **instrument_class** | `spot` |
| **is_inverse** | `false` unless lock proves inverse quoting for the named pair → then `true` with lock citation |
| **quote_currency** | Lock-owned (tracer default **USD** quote class → profile **`fx_spot_usd`**) |
| **calc_profile_id** | **`fx_spot_usd`** (lock may amend id string; must still resolve via `calc_profile()` after registration) |
| **compliance_profile_id** | Lock-owned IB compliance row (withdraw / trading permission posture B6-owned; not decided here) |
| **manifest_id** | Lock-owned (e.g. `interactive_brokers.fx_spot.v1` — illustrative until filed) |

Add the row to `catalog_books()` / `descriptor_for_book_id()` only after registry flip + lock
**SHIPPING** (ADR 0019 steps 1–3). Until then, code may land behind book gate with **Planned** tier.

### 2. Calc profile registration (`fx_spot_usd`)

When the tracer ships, register:

| Field | Value |
| --- | --- |
| **id** | `fx_spot_usd` (lock may amend) |
| **quote_currency** | From lock (default `USD`) |
| **asset_class** | `fx` |

**Rules:**

- Register the profile in `calc_profile()` **only** when at least one SHIPPING book references it
  (same discipline as `equities_inr_nfo`).
- Do **not** reuse `crypto_spot_usd` for FX books — DualNoBlend and money routing depend on asset
  axis honesty.
- Profile addition does **not** imply a realized-PnL owner exists; see §5.

### 3. Pip size law (display + join)

| Surface | Rule |
| --- | --- |
| **Source of truth** | Lock + IB contract spec for the tracer symbol: **pip size** (price increment for one pip) and **display decimals** are lock-owned constants per instrument, not adapter guesses. |
| **JPY and exceptions** | Pairs with quote JPY (or other non-4-decimal convention) use lock table — **no** hardcoded “always 0.0001” in components. |
| **Host join** | Sizer / quote display may read pip from instrument master or lock map keyed by normalized symbol; Wasm returns raw price/qty — host applies pip formatting. |
| **PnL display** | Unrealized / day PnL **display** may express move in pips using lock pip size; **rates** for statutory charges stay out of pip law. |
| **Refuse** | Deriving pip from float string length; copying crypto tick sizes; using INR paise rules. |

Until lock cites IB docs, pip cells in CI fixtures use **planted** values marked `lock-pending` in
test names only — not production defaults.

### 4. Lot size law (quantity normalization)

| Surface | Rule |
| --- | --- |
| **Wire quantity** | Fills carry broker-native **base currency units** (IB trade quantity as reported on Flex/transport — lock maps column names). |
| **Lot display** | **Standard lot** (or mini/micro) is a **display normalization**: `display_lots = raw_qty / contract_multiplier` where **contract_multiplier** is lock-owned per symbol (often 100_000 for major USD pairs — **NOT SPECIFIED IN SOURCE** until lock). |
| **Money engine input** | Realized-PnL owner consumes **raw qty × price** in quote currency unless lock explicitly requires lot-scaled integers; never silently multiply twice. |
| **Refuse** | Equity **share count** semantics; NFO **lot file** pipeline (ADR 0023); crypto coin step sizes. |

Lot size is **not** a calc-profile field. It lives in lock + optional IB instrument metadata join
(host-side), mirroring commodity contract rows but **without** importing MCX CSV law.

### 5. Swap / rollover — display-only vs PnL input

| Class | Law |
| --- | --- |
| **Display-only (default v1)** | Overnight **swap**, **financing**, **roll**, or **interest** amounts reported by IB on statements/Flex rows may appear in Today detail, session summary strips, and export **when mapped fields exist** on ingested rows. Label as **non-trade cashflow (display)**; quote currency from lock. |
| **PnL input (opt-in)** | Amounts in this class **must not** feed realized-PnL, WAC, or round-trip engines until: (a) lock names the **exact** report column(s) and sign convention, (b) B6 money row classifies them as **realized adjustment** or **commission-like**, and (c) `money_matrix` / owner module gains an explicit branch with golden fixtures. Until then, owners **ignore** swap rows for PnL — same posture as undocumented fee columns on crypto. |
| **Synthetic rollover** | **Forbidden.** No end-of-day “apply swap from calendar” without broker-reported rows. No OpenAlgo folklore rates. |
| **vs `instrument_class = swap`** | FX spot overnight financing is **not** a perpetual swap contract. Perps and CFD financing belong to future ADRs (`fx` perp / CFD books). |

**Tracer v1 default:** swap/rollover is **display-only**; first money owner for `ib-fx-spot` covers
**fills + commissions** (and lock-listed charge rows) only — financing lines stay visible but PnL-neutral
until a lock amendment promotes them.

### 6. Today desk routing by `book_id` (not slug)

| Surface | Rule |
| --- | --- |
| **Active desk** | Today hero, calc-profile-gated formatting, and DualNoBlend partitions resolve from the connection’s **`book_id`** (`descriptor_for_book_id` → `quote_currency`, `calc_profile_id`, `asset_class`). |
| **Deprecate for multi-book slugs** | `desk_profile_for_slug(active_broker_slug)` is **insufficient** when `interactive_brokers` carries more than one catalog row. P9-A integrator must introduce **`desk_profile_for_book_id`** (name illustrative) and thread **active book_id** from sync/obtain state into `TodayService`. |
| **IB tracer forcing function** | Even before US equity IB book ships, FX tracer work **must not** merge FX fills into a slug-default desk. Start/obtain targets **`ib-fx-spot`** explicitly (ADR 0019 book-keyed Start). |
| **Multi-book same slug** | Kotak (`kotak-nse-bse-cash` vs `kotak-nse-nfo`) already requires book-keyed stamps; Today slug shortcut was tolerable only while slug≈single desk. IB removes that tolerance — **book_id wins**. |
| **DualNoBlend** | When multiple books sync, partition by **`(book_id, quote_currency, calc_profile_id)`** — never blend FX USD desk with INR equity desk on slug string alone. |

Swift Connect continues to show slug-level session status where appropriate; **numbers** on Today follow
**book_id**.

### 7. Fills path and refuse rules

| Surface | Rule |
| --- | --- |
| **Stamp** | Host stamps `(fx, spot)` from **`ib-fx-spot`** book row only; adapter does not classify asset class. |
| **Refuse INR WAC** | `inr_cash_wac` paths must reject FX book fills (`is_inr_cash_fill` discipline extended). |
| **Refuse crypto spot engine** | `round_trip_engine` / `is_binance_com_spot_fill` heuristics must not capture FX symbols. |
| **Segment / product bleed** | US equity, CFD, and FX fills on one IB account require **book-scoped** Flex queries or post-filter fences per lock — slug-wide “all trades” ingest without book fence is **forbidden** for Tier II. |

### 8. Money owner (pointer — not rates)

Realized-PnL owner id and module path are **lock + B6** gated (`fx_*_realized_pnl.rs` in P9-A
checklist). This ADR does **not** ship charge rates or swap accrual math.

| Tracer v1 expectation | Law |
| --- | --- |
| **Owner registration** | `money_matrix` gains `ib-fx-spot` row only when B6 proves Flex (or chosen transport) commission + fill columns suffice for Tier I golden tests. |
| **Financing lines** | Until §5 opt-in, owner stays **fills + commissions**; financing remains display-only. |
| **Explicit none** | If B6 proves fills-only v1 cannot support realized PnL honestly, owner may temporarily be **ExplicitNone** with documented gap — **Enabled** still blocked by ADR 0019. |

---

## Considered options

1. **Reuse `crypto_spot_usd` profile and COM round-trip engine.** Rejected: wrong `asset_class`; pip/lot
   semantics differ; violates ADR 0004 calc profile meaning and misroutes Today.
2. **Keep Today on slug for IB until US equity book ships.** Rejected: FX tracer would display wrong desk
   the moment a second IB book row exists; violates plan “book ≠ slug”.
3. **Treat swap as mandatory PnL input on day one.** Rejected: Flex column map and sign convention are
   **NOT SPECIFIED IN SOURCE**; promotes synthetic completeness risk.
4. **Hide swap entirely until PnL owner understands it.** Rejected: operators need display-only honesty
   when IB reports financing; hiding is worse than labeled non-PnL strips.
5. **Define pip/lot only in Swift UI.** Rejected: host sizer, exports, and money tests must share the
   same lock constants — UI-only drift fails CI parity.

---

## Consequences

**Positive**

- Single law package for P9-A calc/display before adapter sprawl on IB.
- Forces book-keyed Today routing ahead of IB US equity and CFD siblings — removes slug ambiguity.
- Clear boundary: financing **visible** vs **PnL-consuming**, reducing silent money bugs.

**Negative / follow-ups**

- New calc profile row, `desk_profile_for_book_id` (or equivalent), Today state threading, lock file,
  B6 FX columns, money owner module, and golden fixtures — integrator slice after **0020 ACCEPTED**.
- Plan ADR index still lists **0016** for spot FX in places — update `P9-KICKOFF` / territory plan
  when this ADR is **ACCEPTED** to point at **0021**.
- Second FX pair or non-USD quote: replicate as **new book_id** + lock sibling; do not overload tracer
  lock with multi-pair scope.
- `descriptor_for_slug("interactive_brokers")` must remain **ambiguous or absent** when multiple IB books
  exist — callers migrate to book_id APIs.

**Does not change**

- ADR 0020 transport choice, credential shape, Kill hosts, or Wasm HTTP contract.
- ADR 0001 (Wasm sandbox), 0004 (taxonomy enums), 0019 (Tier I vs II ladder).
- India CDS / MCX / CFD territories (sibling ADRs **0022**, **0023**, …).

---

## Verification (standing — tracer Tier I)

When integrator lands P9-A tracer (steps 4–6, ADR 0019), after **0020 ACCEPTED** and this ADR
**ACCEPTED**:

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test calc_profiles_cover_dual_desk   # extended: fx_spot_usd registered
cargo test desk_profile_for_book_id        # illustrative module tests
cargo test ib_fx_spot                      # pip/lot join + refuse INR/crypto paths
cargo test --test ubi_interactive_brokers_component   # redacted Flex FX fixtures
```

**Required assertions before Tier II:**

- Start on **`ib-fx-spot`** stamps `(fx, spot)`; pip/lot display matches lock constants for planted symbol.
- Swap/financing fixture line appears in **display** payload with non-PnL labeling; realized-PnL golden
  **excludes** that line until lock opt-in.
- Today desk for active sync uses **`ib-fx-spot`** calc profile **`fx_spot_usd`**, not slug-default from
  another IB book row.
- INR cash and Binance spot engines reject FX tracer fills end-to-end.

Tier II: signed dogfood for the single pair named in `locks/ib-fx-spot.md`.

---

## References

- ADR 0004 — book-keyed catalog, `(fx, spot)` enums, calc profile discipline
- ADR 0020 — Interactive Brokers transport (blocks this ADR)
- ADR 0022 — book-keyed Start pattern (segment refuse; analogous book_id discipline)
- P9-A checklist — `plans/multi-asset-p9-territories-2026-09-24.md` (Phase P9-A)
- Today slug desk (legacy) — `agent/src/ubi/desk.rs`, `agent/src/today/service.rs`
- Calc profiles — `agent/src/ubi/catalog.rs` (`calc_profile`, `CalcProfile`)
- Money matrix — `agent/src/money_matrix.rs` (book_id owners; FX row added post-lock)

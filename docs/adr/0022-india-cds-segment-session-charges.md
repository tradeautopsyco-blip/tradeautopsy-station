# ADR 0022: India CDS (NSE CD) — segment, session clock, charge row

**Status:** ACCEPTED (founder execution 2026-09-24 IST — P9-B tracer)

**Date:** 2026-09-24 IST

**Number note:** P9 territory plan (`plans/multi-asset-p9-territories-2026-09-24.md`) once
labeled **0017** for CDS law; **0017–0021** are now taken (crypto session ADRs, launch ladder,
IB transport **0020**, MCX pipeline **0023**, and sibling territory slots). Founder bundle
pre-assigns **0022** to India currency-derivatives first-book law.

**Program:** P9-B (CDS tracer) · `issues/compliance/CLAIM-REGISTRY.md` (CDS **N-A → BUILD**
before integrator) · ADR 0004 (closed `asset_class` / `instrument_class`) · ADR 0019
(six-step pipeline) · `plans/multi-asset-p9-territories-2026-09-24.md` (tracer then replicate)

**Default tracer book:** `kotak-nse-cds` on slug `kotak_neo` (reuse Kotak Neo auth/session;
first named book for NSE **CD** / currency derivatives segment).

---

## Context

1. **CDS is a third India segment on the same Kotak slug, not cash and not NSE F&O.** Today
   `kotak_neo` splits polled fills into `kotak-nse-bse-cash` (`nse_cm` / `bse_cm`) and
   `kotak-nse-nfo` (`nse_fo` only). Currency-derivative rows on Kotak wire use segment
   **`cde_fo`** (see `agent/tests/ubi_kotak_realign.rs` — cash Start **refuses** `cde_fo` with
   `bse_fo` / `mcx_fo` / `bcs_fo`). Until a **CDS-named book** exists, those fills have no
   shipping home and must stay dropped at the cash adapter boundary.

2. **Taxonomy axis is `fx`, not `equity`.** ADR 0004 closed enums: NSE currency derivatives are
   **`asset_class = fx`**. They are not INR cash (`equity` + `spot`) and not equity-index F&O
   (`equity` + `option` / documented FUT limitation on `kotak-nse-nfo`). Host stamps from the
   connection **book** row only; adapters never classify.

3. **Instrument class is venue-defined (`future` vs `forward`).** NSE **CD** segment lists
   currency futures and currency options (and may distinguish forward-style products in master
   metadata). The closed enum has **`future`** and **`forward`** but no `mixed`. Per-fill
   derivation from venue type fields is **not** decided in this ADR (same posture as NFO FUT vs
   OPT in ADR 0004 §5); the book-level stamp must be explicit and coarse metadata only until a
   lock + follow-up names per-fill law.

4. **Session clock is not NSE cash and not NFO.** Cash books use NSE/BSE cash market hours for
   obtain/sync scheduling; NFO uses F&O session boundaries. NSE **CD** (currency derivatives)
   trading hours, partial sessions, and holiday calendar are **segment-local**. Binding the CDS
   book to the cash clock or the NFO clock without official citation is forbidden — **NOT
   SPECIFIED IN SOURCE** until the lock pins URLs + dates.

5. **Charges are statutory and segment-specific.** Realized-PnL and Today routing require a B6 /
   lock **charge row** (STT, exchange transaction charges, SEBI, stamp, GST treatment, clearing
   levies) keyed to **CDS** product types. No charge rate in this ADR is verified; placeholders
   below must be replaced from official NSE / clearing-member pages before Tier II dogfood.

6. **Registry posture.** OpenAlgo BAR lists `CDS` under many India brokers (`docs/research/openalgo-citation.md`);
   Station registry still marks CDS **N-A**. P9-B requires **BUILD + SHIPPING** row for the
   tracer before B5 adapter work (plan flip table).

---

## Official sources (required before ACCEPTED — placeholders)

Implementers must cite **fetch date + URL** in `issues/compliance/locks/kotak-nse-cds.md` (and
B6 sheet amendments). Until verified, every cell in this table stays **NOT SPECIFIED IN SOURCE**.

| Topic | Intended authority | Status |
| --- | --- | --- |
| NSE CD segment name, market hours, holiday rules | NSE circulars / market timing page for **Currency Derivatives** | NOT SPECIFIED IN SOURCE |
| Contract specs (USDINR, EURINR, …), lot, tick, expiry | NSE CD contract specification | NOT SPECIFIED IN SOURCE |
| Fee schedule (STT, txn charges, clearing) | NSE + clearing corporation tariff pages dated | NOT SPECIFIED IN SOURCE |
| Kotak `exSeg` / product map for CD fills | Kotak Neo API / SDK documentation + redacted live sample | NOT SPECIFIED IN SOURCE (lead: `exSeg: cde_fo` in Station fixtures/tests) |
| Kotak scrip master for CD | Kotak `filesPaths` entry for **`cde_fo.csv`** (same host family as `nse_fo.csv`) | NOT SPECIFIED IN SOURCE (path appears in `agent/fixtures/kotak/scrip_file_paths*.json`; header + columns lock-owned) |

Oracle leads (OpenAlgo `CDS` exchange code, Kotak `broker/kotak` mapping) are **BAR only** —
never rates, never segment strings without official match.

---

## Decision

### 1. First-book identity (`kotak-nse-cds`)

| Field | Law |
| --- | --- |
| **book_id** | `kotak-nse-cds` |
| **slug** | `kotak_neo` (same Wasm adapter id, same Keychain session family as cash/NFO) |
| **asset_class** | `fx` |
| **instrument_class** | **`future`** at book stamp for tracer v1 **unless** the CDS lock proves the tracer book is forward-only → then **`forward`**. If the lock documents **both** futures and options on one book, stamp **`future`** as representative (mirrors ADR 0004 NFO `(equity, option)` choice) and record an explicit limitation: option legs are coarse-stamped `future` at book level; per-fill precision lives in venue fields (`instrument_type`, symbol suffix, master row) untouched by this ADR. |
| **is_inverse** | `false` (INR-quoted CD contracts per lock; inverse margining is **NOT SPECIFIED IN SOURCE** for this tracer) |
| **manifest_id** | Lock-owned (e.g. `kotak_neo.cds.v1` — illustrative until filed) |

Add the row to `catalog_books()` / `descriptor_for_book_id()` only after registry flip + lock
**SHIPPING** (ADR 0019 steps 1–3). Until then, code may land behind book gate with **Planned**
tier.

### 2. Segment allowlist and refuse rules (adapter + host)

**Kotak segment token (tracer v1):** accept fills and quotes only when normalized segment equals
**`cde_fo`** (case policy per lock). Mapping from NSE marketing label **“CDS”** to Kotak
`exSeg` is **lock-owned**; do not adopt OpenAlgo’s `exchange: CDS` string inside components.

| Surface | Rule |
| --- | --- |
| **`kotak-nse-bse-cash`** | **Refuse** `cde_fo` rows end-to-end (adapter filter + host split tests). Existing guard: `cash_refuses_nrml_bo_and_non_nfo_fo_segments` — extend coverage so CDS Start on cash book never ingests `cde_fo`. |
| **`kotak-nse-nfo`** | **Refuse** `cde_fo` quotes, scrip CSV, and fill routing (NFO fence is `nse_fo` only — see `kotak_quotes.rs`, `kotak_nfo_scrip.rs` URL picker). |
| **`kotak-nse-cds`** | **Refuse** `nse_cm`, `bse_cm`, `nse_fo`, `bse_fo`, `mcx_fo`, `bcs_fo` bleed. Allowlist **`cde_fo.csv`** from Kotak file-paths on the same scripmaster host pattern as NFO; reject other segment CSVs even if listed. |
| **Poll default** | `merge_poll_book_id("kotak_neo")` stays **`kotak-nse-bse-cash`** until CDS book Start is explicit; CDS fills route only when sync/obtain targets **`kotak-nse-cds`**. |

Unknown segment on a named book → drop fill (fail closed), same posture as wrong-segment FO on
cash.

### 3. Session clock vs NSE cash / NFO

Three **independent** trading-session clocks on one slug:

| Book | Clock owner | Law |
| --- | --- | --- |
| `kotak-nse-bse-cash` | NSE/BSE cash hours | Unchanged |
| `kotak-nse-nfo` | NSE F&O hours | Unchanged |
| `kotak-nse-cds` | NSE **CD** hours | Must not reuse cash open/close or NFO `nse_fo` schedule without lock citation |

Credential TTL (Kotak JWT / trading token) remains the **auth clock** (ADR family: two clocks —
session vs credential — same class as ADR 0007 / 0009). Implementers add **`cds_session_*`**
(or book-scoped scheduler hooks) when the lock pins official open/close timestamps.

Until lock verification: all CDS session boundaries → **NOT SPECIFIED IN SOURCE** (obtain
windows, “market closed” UX, and background master refresh cadence are lock follow-ups).

### 4. Calc profile

Proposed descriptive profile id (quoting/settlement seam — **not** a money owner):

**`fx_cds_inr`**

- **INR** quote/settlement display and desk formatting follow the lock’s named currency leg.
- Renaming requires Swift session payload coordination (ADR 0004 §6 exempt list discipline).
- Alternate id **`cds_inr`** is rejected for tracer unless the lock explicitly names it over
  `fx_cds_inr`.

### 5. Charge row placeholder (lock + B6 — no verified rates)

The CDS lock must include a **charge matrix** row per product type (NRML/MIS mapping per Kotak
CD — **NOT SPECIFIED IN SOURCE**). This ADR records required **columns** only:

| Charge component | Applies to (placeholder) | Rate / basis | Source status |
| --- | --- | --- | --- |
| STT | CD futures / CD options | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |
| Exchange transaction charges | per side / per crore | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |
| SEBI turnover fee | | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |
| Stamp duty | state / product rules | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |
| GST on brokerage + statutory | | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |
| Clearing / CCIL levies (if any) | | NOT SPECIFIED IN SOURCE | NOT SPECIFIED IN SOURCE |

**Money owner:** P9-B requires **one** realized-PnL engine path for `kotak-nse-cds` (new module
or shared FX-CD module — name decided with lock). Until owner + rates are CI-pinned, book stays
**Planned** (ADR 0019 Tier I only). Options-style **ExplicitNone** is **not** the default for
CD futures tracer — founder must sign if display-only.

### 6. Contract master (pattern — details lock-owned)

Replicate the **named-book scrip master** pattern from `kotak-nse-nfo`:

1. Book-scoped fence → file-paths → unsigned **`cde_fo.csv`** fetch.
2. Parser header **exact match** to lock `LOCK_HEADER` (**NOT SPECIFIED IN SOURCE** until CSV
   captured).
3. Lot/tick from contract row only — no orphan lot tables (same refusal as ADR 0023 §1).
4. Join fills/quotes on lock-defined instrument identity (segment + token / trading symbol).

### 7. P9-B execution: tracer then replicate

**Serial law** (plan Phase P9-B):

| Phase | Deliverable |
| --- | --- |
| **Tracer** | `kotak-nse-cds` end-to-end: registry flip, this ADR **ACCEPTED**, lock **SHIPPING**, integrator row, adapter + fixtures, charge row populated, money owner green in CI, signed dogfood |
| **Replicate** | **No** second slug until tracer Tier II. Then ~24 sibling CDS books (other India slugs) as **one book per ticket**, copying P6 NFO replication checklist: catalog + allowlist + dns + component test + lock sibling |

Replication tickets must cite this ADR number and the Kotak tracer dogfood plan as template
(`plans/dogfood-*` / `issues/brokers/plans/dogfood-kotak_neo-*` family).

---

## Considered options

1. **Treat CDS as `equity` + `future` on the NFO book.** Rejected: violates ADR 0004 asset axis;
   misroutes Today partitions and would inherit NFO STT schedule not CD schedule.
2. **Ingress CDS on cash Start via adapter widen.** Rejected: breaks audit C5/C6 and teaches
   slug-keyed bleed; CDS requires explicit `book_id` Start.
3. **Stamp `(fx, forward)` without lock evidence.** Rejected: forward vs future is venue-defined;
   default **`future`** for tracer v1 with documented option-leg limitation (§1) until lock
   narrows.
4. **Single global “India session” clock for all Kotak books.** Rejected: MCX/CD/NFO/cash hours
   differ; silent reuse is NOT SPECIFIED IN SOURCE and caused NFO/cash drift in early waves.
5. **Ship charge rates in this ADR.** Rejected: money-change gate requires dated official pages
   in the lock, not draft ADR prose.

---

## Consequences

**Positive**

- Clear first-book law for category **3** (India CDS) without opening MCX or spot FX transports.
- Reuses Kotak Neo auth (no new OAuth family ADR) while keeping segment fences as strict as NFO.
- Replication path is explicit — P6 NFO wave discipline, P9-B territory checklist.

**Negative / follow-ups**

- New catalog row, fence entries, account-split arm, scrip master module, scheduler hooks, money
  owner, and `issues/compliance/locks/kotak-nse-cds.md` — integrator slice after registry flip.
- `instrument_class` book stamp may mis-label CD **option** legs until per-fill law is accepted
  (same explicit cost as ADR 0004 §5 for NFO FUT).
- Plan ADR index still lists **0017** for CDS in places — update `P9-KICKOFF` / territory plan
  when this ADR is **ACCEPTED** to point at **0022**.
- Zerodha / other slugs: segment string may differ from `cde_fo` — each lock owns mapping; do
  not copy Kotak segment token blindly.

**Does not change**

- ADR 0001 (Wasm sandbox), 0004 (taxonomy reject rules), 0019 (Tier I vs II).
- Cash and NFO book scopes on `kotak_neo`.
- CDS registry **N-A** until founder flip (no silent SHIPPING).

---

## Verification (standing — tracer Tier I)

When integrator lands P9-B tracer (steps 4–5, ADR 0019):

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test cash_refuses_nrml_bo_and_non_nfo_fo_segments
cargo test --test ubi_kotak_neo_component   # extend with cde_fo CDS book fixtures
cargo test kotak_cds                        # module name illustrative
```

**Required assertions before Tier II:**

- Start on **`kotak-nse-cds`** ingests planted `cde_fo` fills; stamps `(fx, …)` from book row.
- Start on **`kotak-nse-bse-cash`** still drops `cde_fo` after CDS book exists.
- **`kotak-nse-nfo`** quotes/fences still refuse `cde_fo`.
- At least one golden charge fixture matches lock-dated rates (no ADR placeholders in CI).

Tier II: signed dogfood for symbols named in the CDS lock.

---

## References

- ADR 0004 — book-keyed stamp, `fx` enum, calc profile discipline
- ADR 0023 — sibling pattern for segment-local master + refuse (MCX)
- Kotak segment refuse tests — `agent/tests/ubi_kotak_realign.rs`
- Kotak file-paths fixture (CDE CSV URL lead) — `agent/fixtures/kotak/scrip_file_paths.json`
- P9-B checklist — `plans/multi-asset-p9-territories-2026-09-24.md` (Phase P9-B)
- OpenAlgo CDS BAR — `docs/research/openalgo-citation.md` (exchange list only)

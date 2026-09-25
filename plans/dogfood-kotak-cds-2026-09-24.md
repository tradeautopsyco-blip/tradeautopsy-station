# Dogfood — `kotak_neo` CDS (`kotak-nse-cds`)

**Status:** DRAFT — **Tier I** path (steps 1–5; step 6 open)  
**Lock:** `/Users/bishnu/issues/compliance/locks/kotak-nse-cds.md` (**DRAFT** — **SHIPPING flip blocked** until founder dates every official source; no Tier II drills on TBD charges/session)  
**B6:** `/Users/bishnu/issues/brokers/sheets/kotak_neo.md` (CDS rows / amendments **TBD** — read cash + NFO exit rows; do not infer CD rates from sibling books)  
**ADR:** `docs/adr/0022-india-cds-segment-session-charges.md` (`DRAFT`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **Planned** + Connect on named book; not step 6 |
| **II — Live** | Step **6** signed | Drills + sign-off → **Enabled** flip |

> Decision 8: **Enabled** = **Tier II only** (ADR 0019 §C). Venue facts from B6 + **SHIPPING** lock only; STOP → ADR 0022 revisit. **Do not run charge or session drills while lock remains DRAFT.**

---

## Preconditions

- [ ] ADR 0022 **`ACCEPTED`** (segment `cde_fo`, `fx` axis, refuse bleed on cash/NFO, calc profile `fx_cds_inr`, charge columns named — no rates in ADR prose)
- [ ] Lock **`SHIPPING`** with dated URLs (`kotak-nse-cds.md` — NSE CD + Kotak Neo API fetches; session table; charge matrix populated; `cde_fo.csv` header verbatim)
- [ ] Registry flip: CDS tracer **BUILD + SHIPPING** row for `kotak-nse-cds` (P9-B) before adapter claims live path
- [ ] B6 amended for CD capability rows (segment map `cde_fo` → CD book; refuse list aligned with lock — not copied from cash/NFO rate tables)
- [ ] Bindings landed: reuse **`kotak_neo`** Keychain session family (same Wasm slug as cash/NFO); **explicit `book_id` Start** on `kotak-nse-cds` — poll default stays cash until Start targets CDS (ADR 0022 §2)
- [ ] Adapter built: first CD book **`cde_fo` only**, CalcProfile **`fx_cds_inr`**; host stamps `(fx, …)` from book row; cash adapter **refuses** `cde_fo`; NFO fence **refuses** `cde_fo`
- [x] Scrip master: book-scoped **`cde_fo.csv`** fetch (`KOTAK_CDE_FO_LANE`, `kotak_cds_scrip_master`, cache `cde_fo`) — live session still required for Tier II drills
- [ ] Money owner: **one** realized-PnL module named in lock at SHIPPING (CI green before Tier II)
- [ ] Catalog lists `kotak_neo` CDS book as **Planned** only (no `Enabled` flip before sign-off)
- [ ] Agent running on `127.0.0.1:9137` (release build)
- [ ] Live Kotak Neo account with **CD segment entitled** (production; entitlement proof via B6 validation row — **TBD** until sheet amend)
- [ ] **STOP:** any step treats NSE cash hours, NFO hours, or Zerodha/NFO STT as CD law → **STOP**, amend lock with dated NSE CD pages first
- [ ] **STOP:** any step ingests `cde_fo` on `kotak-nse-bse-cash` or `kotak-nse-nfo` Start → **STOP**, fix split tests + adapter fences (ADR 0022)

## Drills (B6 CDS rows / lock — **Tier II only after SHIPPING lock**)

| # | Drill | Steps | Pass when (cite lock + B6) |
|---|-------|-------|----------------------------|
| 1 | Session mint → CD trades poll green | Existing Kotak Neo login (TOTP + MPIN family per signed session ADR/B6) → session `baseUrl` → `GET` trades path scoped to **`cde_fo`** rows only (same `/quick/user/trades` family as cash/NFO — segment filter on book) | Fills pipeline starts: **`cde_fo`** day trades appear in Station sync when obtain/sync targets **`kotak-nse-cds`**; INR at insert; **`fx_cds_inr`** profile on wire; no `nse_cm`/`nse_fo` bleed. **STOP:** CD rows appear on cash Start or NFO book → fix host split + adapter refuse. |
| 2 | Credential expiry drill | Let Kotak trading token / JWT expiry arrive (duration per B6 — not CD session clock). Observe documented auth failure class. | Pause + reconnect (re-login), **never Kill** on slug unless Kill drill (row 6). **STOP:** dead token yields silent drop or mint storm → record + reopen session ADR if lifecycle breaks. |
| 3 | Rate / fan-out discipline | Full fills path for CD day book + any per-order fan-out — pace inside Kotak documented limits (shared type budgets per B6; CDS-specific caps **TBD** until sheet amend). | No sustained throttle errors; backoff-and-resume, never abort-and-fabricate. Cadence logged. Unknown segment on CDS book → drop fill (fail closed). |
| 4 | Day-only / history honesty | Use only endpoints the lock + B6 name for **day** CD trade history; record whether ranged backfill exists (**NOT SPECIFIED** until B6 amend). Statements/CSV recovery for pre-connect history. Second poll must not widen cursor into fake history. | Proven scope recorded in this file; candles/quotes never counted as trade history. **STOP:** multi-day backfill claimed without proof → refuse. |
| 5 | Segment + product verbatim | Capture `exSeg` / product fields verbatim on every CD row; expect **`cde_fo`** only. Watch NRML/MIS (or other) strings — **NOT SPECIFIED IN SOURCE** until lock product table. | Segment recorded verbatim; refuse list holds. **Either** new product strings evidenced on live rows → **lock amendment** **or** absent → keep refusing unevidenced strings. |
| 6 | Charge reconciliation | Reconcile fills (or broker statement fields if in-band) against **SHIPPING** lock CD charge matrix | Every levy ties to lock within tolerance or variance-logged. **Blocked while lock DRAFT.** **STOP:** any charge from NFO or cash lock → refuse. |
| 7 | Kill drill | With W0.7/Z7 hosts live, trigger Kill for slug `kotak_neo`. | L3 blocks Kotak session + scripmaster hosts for slug (same family as cash/NFO drills); slug-scoped: other brokers unaffected. |
| 8 | Strips-no-blend proof | Connect Kotak CDS INR strip alongside other Kotak INR books (cash, NFO) + sibling INR slugs + USD strip. | CDS INR is a **separate strip** — no aggregation hero across books; USD strip untouched (DualNoBlend law). |

Notes:
- Per-fill math: venue qty × price × lot from **`cde_fo.csv`** lock row — **not** cash lot=1. Charges **NOT SPECIFIED** until lock SHIPPING.
- Timestamps: follow B6/lock for trade vs session fields; do not assume cash ISO shapes without citation.
- Entitlements: validation equivalent must prove **CD segment** enabled before CDS Start — row **TBD** on B6 amend.
- Refuse list (lock + ADR 0022): cash/NFO segments on this book; execution verbs until scoped; wrong-book `cde_fo` ingress; NFO STT schedule; cash session scheduler on CD book; synthetic fills on unknown segment; OpenAlgo `CDS` string inside components; unverified product defaults; secrets in Wasm/Console; paper-as-live.

## Sign-off checklist (Planned → Enabled flip)

- [ ] Lock **SHIPPING** with founder-dated sources (no TBD rows in official table, session, or charges)
- [ ] ADR 0022 **ACCEPTED**
- [ ] Drill 1 green: explicit **`kotak-nse-cds`** Start → **`cde_fo`** poll; no bleed on cash/NFO
- [ ] Drill 2 green: auth expiry → pause + reconnect, never Kill (except drill 7)
- [ ] Drill 3 green: rate discipline held; unknown segment dropped
- [ ] Drill 4 green: day-only / history honesty recorded; cursor-never-widens holds
- [ ] Drill 5 green: segment/product verbatim watch resolved (lock amended or refusal kept)
- [ ] Drill 6 green: charges reconcile to lock CD matrix
- [ ] Drill 7 green: Kill slug-scoped on Kotak hosts
- [ ] Drill 8 green: CDS strip live with no blend
- [ ] No STOP fired, or every STOP resolved via ADR 0022 / lock amend (record resolutions here)

**Signed:** —  
**Date:** —  
**Enabled flip:** after sign-off, one ticket: catalog `Planned` → `Enabled` (agent + Swift). No flip on a DRAFT lock or unsigned dogfood.

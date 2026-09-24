# ADR 0019: Broker launch validation ladder (one pipeline, two tiers)

**Status:** ACCEPTED (founder launch bundle 2026-09-24 IST)

**Date:** 2026-09-24 IST

**Number note:** Pre-assigned **0019** for cross-cutting broker program law (not a
per-shape session ADR). Closes FULL-COVERAGE-PROGRAM open decision **8**
(dogfood-without-account).

**Program:** `issues/brokers/FULL-COVERAGE-PROGRAM.md` · P3/P4 kickoffs ·
`issues/ASSET-CLASS-INTEGRATION-SURFACE.md` §17–§18 ·
`issues/compliance/CLAIM-REGISTRY.md`

**Supersedes:** Informal “do everything before any ship” friction. Phases 0–3 stay
the roadmap; **this ADR is the only execution checklist** for landing brokers at
launch and after.

---

## Context

1. **Coverage outruns accounts.** The program targets ~34 India cash brokers plus
   multi-asset books. The founder does not hold live accounts at every venue (nor
   IB, forex, or every crypto CEX). Requiring personal G7 on every slug before
   any integration ships blocks launch and duplicates work across OAuth siblings.

2. **Two catalog surfaces already exist.** `catalog_v1()` (slug list — today only
   `kotak_neo` + `binance_com` **Enabled** for desk-live gates) vs
   `catalog_books()` (book-keyed rows — Wave 2 tracers may show **Enabled** in
   Station Connect while desk money rules still treat only the first pair as
   shipping for Today/TAI). That split caused Decision 8 drift (UI Enabled ahead
   of signed dogfood).

3. **Oracles are allowed; runtime is not.** OpenAlgo (`ad3cd54`) and Nautilus
   (citation pin in `CLAIM-REGISTRY.md`) are **lead + shape** sources. Official
   broker docs win on conflict. No OpenAlgo/Nautilus code in Station (AGPL/LGPL;
   no second control plane).

4. **Launch constraint (2026-09-24).** Ship broker connect, sync, and
   multi-asset book wiring **today** with honest product boundaries—not 34
   founder dogfoods first.

---

## Decision

Replace G0–G7 phase friction with **one six-step pipeline** and **two tiers**
of “done.” Every slug, every book, every wave uses the same pipeline; only the
tier reached differs.

### A. One pipeline (every slug, same order)

| Step | Name | Deliverable | Live account? |
|------|------|-------------|---------------|
| **1** | **Sheet** | B6 **SIGNED** at `issues/brokers/sheets/<slug>.md` | No |
| **2** | **Shape** | ADR **ACCEPTED** for auth family (or reuse row in P3 ADR index) | No |
| **3** | **Lock** | `issues/compliance/locks/<book>.md` **SHIPPING** | No |
| **4** | **Integrate** | Wasm adapter + host session + allowlist + dns + component tests **green** | No |
| **5** | **Fixtures** | Planted bodies in `agent/fixtures/` (+ optional oracle capture — §B) | No |
| **6** | **Live proof** | Signed checklist `issues/brokers/plans/dogfood-<slug>-*.md` or Station `plans/dogfood-*` | **Yes** (founder or delegate) |

**Rule:** Do not skip steps. Do not enable desk-live claims (§C) without step 6.
Steps 1–5 are sufficient to **ship Connect + sync** at Tier I.

### B. Oracle capture (optional step 5 booster — not runtime)

When the founder lacks an account but OpenAlgo/Nautilus/directory access exists:

1. On a **non-Station** machine, call the broker API (or run OpenAlgo against
   connected accounts) at pin **`ad3cd54`** (OpenAlgo) or the Nautilus citation
   pin in registry.
2. Save **redacted** JSON bodies (auth OK, trades/orders list, one error shape,
   optional quote/LTP) → `agent/fixtures/<slug>/`.
3. In the fixture header: `official_url + fetch_date` that **verified** each field;
   list any field taken only from oracle as `lead-only`.
4. Add/extend `ubi_<slug>_component` tests so normalized `FillEvent` matches the
   fixture.

**Refuse:** Synthetic fills (e.g. OpenAlgo `synthetic_*` on 404), wrapping
OpenAlgo, citing untracked OpenAlgo paths, or treating oracle capture as step 6.

### C. Two tiers (what “complete” means)

| Tier | Steps | Catalog / registry | Product promise |
|------|-------|-------------------|-----------------|
| **I — Integrator** | 1–5 done | `catalog_books` row **Planned** (Connect may ship). Do **not** add slug to `CLAIM-REGISTRY.md` “Live slugs (Enabled)” line. | User can **Connect → Keychain → Start sync → day-book fills** for that book. Desk **money hero** (Today, TAI shipping books, marketing “fully supported”) **only** for slugs in **`catalog_v1()` Enabled** (`kotak_neo`, `binance_com` until step 6 per slug). |
| **II — Live** | 1–6 signed | Flip that slug/book to **Enabled** in integrator PR + registry “Live slugs” sentence. | Same as Tier I plus **founder-signed** dogfood: loopback/consent, Harness Live without `wrong_book`, Kill drill, quote identity if desk promises quotes. |

**Decision 8 (closed):** **Enabled** in `catalog_v1()` and registry live-slugs line
= **Tier II only**. Tier I lands as **Planned** + fence-green; early **Enabled**
on book rows without step 6 is **technical debt**—fix in the same integrator PR
that signs dogfood, or revert to Planned before launch marketing goes out.

**Launch day (2026-09-24):** Multi-broker **Connect** for Tier I tracers is
allowed. Public copy must not imply Kotak-parity desk money or “fully supported”
for slugs not in `catalog_v1()` Enabled.

### D. Minimal live accounts (shapes, not ×34)

G7 (step 6) runs **once per auth shape**, not once per marketing name:

| Proof bucket | Example slug | When step 6 is required |
|--------------|--------------|-------------------------|
| Anchors | `kotak_neo`, `binance_com` | Already Tier II target |
| Kite checksum | `zerodha_kite` | Before Zerodha Tier II |
| Upstox form OAuth | `upstox` | Before Upstox Tier II |
| Fyers appIdHash | `fyers` | Before Fyers Tier II |
| Dhan consent | `dhan` | Before Dhan Tier II |
| Groww checksum | `groww` | Before Groww Tier II |
| Wave 3a TOTP/JWT | `angel` (tracer) | Before that family Tier II |
| Wave 3b XTS | `fivepaisaxts` | Before XTS siblings |
| Wave 3c Noren | `shoonya` or `flattrade` | Before Noren siblings |

OAuth siblings (Park B) and **IIFL** (Park A): **no Tier I swarm** until unparked.
**IB / forex / REFERENCE** books: pipeline frozen at step 0 until founder names
SHIPPING in chat + lock exists.

### E. End-to-end completeness (single implementation path)

For launch, “end-to-end” for a **new India cash slug** means exactly this PR
sequence (one integrator batched PR per wave-slice where files collide):

```
B6 SIGNED → ADR shape ACCEPTED → lock SHIPPING → *_session.rs + http signer
→ allowlist + dns_block → ubi-<slug>-adapter + fixtures + component test
→ Swift Connect reuse (same AuthScheme family) → dogfood plan file (step 6 open)
→ Tier I: Planned + Connect visible → (later) signed dogfood → Tier II Enabled flip
```

Multi-asset **books** on an existing slug: **lock only** (no new B6 slug), extend
existing adapter crate, same six steps with book-scoped goldens.

Desk **quotes** (e.g. `nse_cm|token`): not a substitute for step 4 fills; add
instrument + obtain binding under step 4/5 with fixture LTP; step 6 proves one
live symbol if Notch promises quotes.

### F. Verification (standing — one command block)

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test --lib b6_gate
cargo test --test b6_sheet_lint --test ubi_<slug>_component
```

Tier I merges when the above pass for the slug **and** steps 1–3 artifacts exist
in `issues/`. Tier II merges add signed dogfood plan + registry/catalog flip.

---

## Consequences

**Positive**

- One checklist for every phase; launch is not blocked on 34 accounts.
- OpenAlgo/Nautilus become a **fixture factory**, aligned with compliance.
- Clear marketing boundary: `catalog_v1()` Enabled = desk-live; Planned = Connect beta.

**Negative / follow-ups**

- Station UI may show Connect for slugs before Tier II; honesty chips and
  marketing must say **beta / sync only** where Planned.
- Wave 3 slugs without B6 sheets: step 1 blocks code—research queue, not adapter
  swarm.
- Reconcile any `catalog_books` **Enabled** without signed step 6 → **Planned**
  before external launch, **or** complete step 6 the same day for those slugs.

**Does not change**

- ADR 0001 (Wasm, no secrets in component), 0004 (book-keyed stamp), per-shape
  session ADRs 0005–0018.
- M1 single PnL writer; `ingestSignal` only; no Console broker adapters.
- Kill always-on; Stop ≠ Kill; OpenAlgo wrap refused.

---

## References

- `issues/brokers/P3-KICKOFF.md` (shape ADR index 0006–0014)
- `issues/brokers/P4-KICKOFF.md` (crypto tracers)
- `issues/compliance/CLAIM-REGISTRY.md` (OpenAlgo / Nautilus N-A rows)
- Skill A gate 0: `~/.cursor/skills/tradeautopsy-new-book/SKILL.md`

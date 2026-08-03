# Brokers SYSTEM — multi-integration research memo

**Status:** `LOCKED` — founder FINALIZE 2026-07-24  
**Date:** 2026-07-24 · FINALIZE after research pass + signed B6 sheets  
**Packs:** [BROKERS.md](../BROKERS.md) · [COMPLIANCE.md](../../compliance/COMPLIANCE.md) · [BROKERS-SYSTEM-FULL.md](./BROKERS-SYSTEM-FULL.md)  
**Plan:** `~/.cursor/plans/Multi-Broker System Rebuild-ec478ba8.plan.md`  
**Handoff:** `~/.scratch/console-station-execution/HANDOFF-BROKERS-SYSTEM.md`  
**Repos:** Station `/Users/bishnu/tradeautopsy-station` · issues mirror `/Users/bishnu/issues/brokers/`  
**Course correction:** Architecture-deepening **Issue 2** (live Binance.US) **SUPERSEDED**.  
**Sheets:** [`binance_com.md`](../sheets/binance_com.md) · [`kotak_neo.md`](../sheets/kotak_neo.md) — both **SIGNED**

### Founder locks (FINALIZE)

| Lock | Choice |
|------|--------|
| Scope | Full-stack **greenfield** of Brokers live path (Station + Enforcer + Console secret fence) |
| System shape | **Option A** — capability-sheet–gated adapters on existing seams (inside greenfield ownership) |
| First pair | **`binance_com`** (en-IN → api.binance.com) + **`kotak_neo`** equities INR |
| B4 dogfood | COM live + Kotak Neo — not US stub, not CSV-first |
| Calc / Compliance | Named seams: `crypto_spot_usd` · `equities_inr_cash` · `binance_com_compliance` · `kotak_neo_compliance` |
| D1 process | **No new adapter without a signed B6 sheet** (skill file may follow; rule binds now) |
| Dual desk | Per-connection CalcProfile strips; **no FX blend** of USD+INR into one number |
| Next code | **Phase 1 contracts (TDD)** in Station `agent/` — adapters after contracts |

---

## Question (answered)

What is the **one Brokers SYSTEM shape** that can host **many integrations** which differ by asset class, currency, calculation, and compliance — without pouring the product into “Binance forever”?

**Answer:** Option A — catalog descriptors + B6 sheets + `FillEvent` + CalcProfile/ComplianceProfile on Station/Enforcer ownership, with Keychain + SyncControl + Kill invariants. First honest pair = Binance.com + Kotak Neo.

---

## Evidence (primary — not invented)

### What exists today (Station live path)

| Layer | Mechanism | Source |
|-------|-----------|--------|
| **Catalog** | Static `BrokerCatalog.v1` — `binance_us` / `binance_com` enabled (`assetClass: crypto`); IB / Zerodha **planned** (`equities`) | `station/.../BrokerCatalog.swift` |
| **Connect** | `BrokerConnectController` → validate → Keychain save | Swift Backend Box |
| **Secrets** | `KeychainBrokerCredentialStore` — fresh read on Start | B2 direction already Station-side |
| **Sync control** | `AgentBrokerSyncControl` → agent `BrokerSyncController` start/stop | B3 partial |
| **Adapter seam** | Rust `BrokerAdapter`: `poll_fills` / `poll_balances_holdings` / `poll_open_orders` | `agent/src/broker.rs` |
| **Factory** | `build_runtime_adapter`: live **COM only**; **all** `binance_us` → `CountingPollAdapter` (empty fills) | `broker_sync_control.rs` |
| **Validation** | `BrokerValidationAdapter` + `PermissionPosture` (`ReadOnlyConfirmed` / `TradeEnabled` / `WithdrawDetected` / `Unverifiable`) | `broker_validation.rs` · COM + US validators |
| **Fill model** | `BrokerFill` — qty/price/side/fees; **no** lot, pip, settlement, currency basis, asset-class calc hooks | `broker.rs` |
| **Start request** | Carries `assetClass` string already; **HMAC-shaped credentials only** | `BrokerSyncStartRequest` |
| **Console UBI** | Parallel mega-registry + DB `api_secret` live path still exists | B5 debt |

### Compliance / create-system ties (D*)

| ID | Ask | Tie to Brokers |
|----|-----|----------------|
| **D1** | Skill: new system / new asset checklist | Must **require B6 sheet** before adapter/tech — **process rule ON** |
| **D2** | Research → asset-class skill | Forex first; crypto/equities copy pattern |
| **D3** | Feature-by-asset-class gate | No generic “trade” assumptions in Today/Risk/Journal |
| **D4** | Asset-class ready-doc | Gated; not authored until D2.1 |
| **D5** | Same doc system for everything | Brokers pack + every adapter follows spine |

---

## Options (system shape) — decision

| Option | Verdict |
|--------|---------|
| **A — Capability-sheet–gated adapters on today’s seams** | **LOCKED** |
| B — Big-bang `BrokerSession` rewrite first | Park (may deepen later inside A) |
| C — Console UBI as multi-broker spine | **Rejected** |
| D — Per–asset-class adapter traits now | **Park** until ready-docs |

### Option A (locked detail)

Keep:

- `BrokerAdapter` + `BrokerValidationAdapter`
- Keychain vault (B2)
- SyncControl Start/Stop ≠ Kill (B3)
- Enforcer owns live poll (B5)

Add **system contracts** (Phase 1 — greenfield of the live path):

1. **`BrokerCapabilitySheet`** (B6) — required before any new adapter/slug ships.
2. **`BrokerDescriptor`** — slug + asset class + currency posture + authScheme + calc/compliance profile ids.
3. **`FillEvent`** — normalized multi-asset boundary (replaces crypto-shaped `BrokerFill` as system edge).
4. **`CredentialBlob`** by `authScheme` (`hmac_api_key_secret` | `kotak_neo_totp_session`).
5. **CalcProfile / ComplianceProfile** — desk math + connect/runtime gates.

Binance COM and Kotak Neo are **first adapters**, not the architecture. Binance.US stays **parked**.

---

## Target architecture (LOCKED)

```text
┌─ Station Backend Box ─────────────────────────────────────┐
│  BrokerCatalog (descriptors)                               │
│  BrokerConnectController → ValidationAdapter → Keychain    │
│  SyncControl Start/Stop (≠ Kill)                           │
└───────────────┬────────────────────────────────────────────┘
                │ loopback wire (HMAC = machine only)
┌───────────────▼─ Enforcer ────────────────────────────────┐
│  BrokerSyncController                                      │
│  build_runtime_adapter(slug) → BrokerAdapter               │
│  FillEvent → CalcProfile(asset_class) → Today/Risk share   │
│  ComplianceProfile continuous + audit                      │
└────────────────────────────────────────────────────────────┘
         │ share-up (Bearer Caller) — status/fills consumers
┌────────▼─ Console ────────────────────────────────────────┐
│  No live secrets · no UBI live ownership (B5)              │
│  Consumes shared metrics (S2)                              │
└────────────────────────────────────────────────────────────┘
```

Hard keeps: A8 Bearer; Wire HMAC machine-only; Kill always-on; Workflow Labs; `/api/bar/v1/**` until Z; no fog restore; no Tax Center.

---

## B6 — Capability sheet template

**Rule:** No new adapter, slug enablement, or broker tech without a filled + signed sheet from **real** broker docs/API behavior.

**Path:** `/Users/bishnu/issues/brokers/sheets/<slug>.md`  
**First pair (SIGNED):** `binance_com` · `kotak_neo`

| # | Field | Answer (do not invent offline) |
|---|-------|--------------------------------|
| 0 | Slug / display / venues (COM vs US vs …) | |
| 1 | **Asset class(es)** supported when connected | |
| 2 | **Fetch path** — REST trades? user-data WS? both? poll cadence? | |
| 3 | **Post-connect options** — markets, account types, products, permission scopes | |
| 4 | **Time / volume** — max lookback, page size, rate limits, pagination | |
| 5 | **Order/trade history depth** — full / N days / from-connect-only | |
| 6 | **If incomplete history** — recovery path (incremental, CSV export, paid API, local ledger) | |
| 7 | **Currency** — quote assets, fee assets, settlement, fiat rails | |
| 8 | **Calculation factors** — fees, lots/contracts, WAC/avg, funding/rollover, PnL rules we must honor | |
| 9 | **Compliance** — permission check endpoint, withdraw detect, jurisdiction notes, audit needs | |
| 10 | **Secrets shape** — key/secret, OAuth, cert; Keychain account naming | |
| 11 | **Dogfood role** — temporary scaffold / day-1 / Phase 5 candidate | |
| 12 | **Refuse list** — what we will not support in v1 for this slug | |
| 13 | **Sources** — official docs URLs + date fetched | |

---

## Calc + compliance hooks (system contracts)

### CalcProfile (pairs S2 / Today / Risk)

| Concern | System rule |
|---------|-------------|
| Owner | Station/Enforcer for desk-speed numbers; Console deep only where S2 matrix says so |
| Input | Normalized FillEvent + descriptor.calc_profile_id |
| Output | Shared metrics (day PnL, fees handled/unhandled, basis flags) — **no recalc** on the other side |
| Dual desk | When both COM + Kotak sync: **two profiles, two currencies** — no FX blend in v1 |
| Variance by class | Profiles, not `if broker == binance` |

### ComplianceProfile

| Concern | System rule |
|---------|-------------|
| Connect-time | COM: withdraw detect blocks Start. Kotak: TOTP+MPIN session mint success |
| Runtime | COM: re-validate restrictions. Kotak: session expiry → pause + reconnect (not Kill) |
| Stop vs Kill | Stop = pause poll only; Kill + audit + SSE **stay on** (B3) |
| Jurisdiction | Descriptor + sheet field; SEBI/algo policy later — hook exists |

---

## Map to B1–B6 (LOCKED)

| ID | Role under Option A |
|----|---------------------|
| **B1** | Demoted. US parked — not dogfood |
| **B2** | Deepen Keychain-only; fence Console DB secret live path |
| **B3** | Deepen SyncControl honesty (Start/Stop ≠ Kill) behind system |
| **B4** | **LOCKED** — Binance.com + Kotak Neo |
| **B5** | Cull/park Console UBI live ownership; Enforcer adapters = live SoT |
| **B6** | Sheet gate — **first pair signed**; every future adapter needs a sheet |

## Map to D1–D5

| ID | What this memo contributes | What stays gated |
|----|----------------------------|------------------|
| **D1** | Process rule: require B6 before adapter | Skill authoring until founder says build |
| **D2** | CalcProfile dimensions list what asset-class skills must define | Forex skill + ready-doc |
| **D3** | Feature gate: cite class ready-doc + broker sheet when touching fills/PnL | Enforcement skill |
| **D4** | Brokers consumes ready-docs; does not replace them | FOREX-READY.md |
| **D5** | Brokers pack + `sheets/` follow same research → ready → issues spine | Meta rollout |

---

## Implementation order (after this FINALIZE)

1. **Phase 1 — Freeze contracts (TDD, Station cwd)** — `FillEvent`, descriptors, auth blobs, Calc/Compliance stubs, sync start credential blob.  
2. **Phase 2 — Backend Box** — catalog + dual connect (HMAC vs Kotak TOTP) + Keychain.  
3. **Phase 3 — Adapters** — COM on FillEvent + new `kotak_neo` Rust.  
4. **Phase 4 — Console B5 fence.**  
5. **Phase 5 — Desk honesty + dual dogfood.**

**Explicit non-goals**

- Shipping architecture-deepening Issue 2 as the product slice  
- Authoring full FOREX-READY / D1–D2 skills in this pass  
- Marketplace / arbitrary endpoint plugins  
- Tax Center / fog restore  
- Inventing B6 facts that contradict signed sheets  

---

## Blast radius (when building)

| Area | Touch |
|------|-------|
| Station Swift | `BrokerCatalog`, Connect, Keychain, SyncControl, Brokers UI |
| Enforcer | `BrokerAdapter`, validation, `build_runtime_adapter`, poll loop, Today calc flags |
| Console | Status/ingest only for live; UBI park (B5); no new DB-secret live path |
| Issues | `brokers/sheets/*`, COMPLIANCE D1 checklist entry, plan Phase 4/5 gates |

---

## What we stop doing (LOCKED)

1. Treating “live Binance.US adapter” as the next Brokers north star.  
2. Designing the system around one venue’s API quirks.  
3. Adding adapters without a B6 sheet.  
4. Letting Console UBI own Station live secrets/fetch.  
5. Inventing asset-class PnL rules inside Binance-shaped `if` branches.  
6. Blending USD + INR into one Notch/Today number.  
7. Assuming Kotak `/quick/user/trades` is full historical ledger.

---

## FINALIZE (LOCKED 2026-07-24)

> Brokers SYSTEM = **Option A** — capability-sheet–gated adapters on existing `BrokerAdapter` / validation / Keychain / SyncControl seams, rebuilt as a **full-stack greenfield of the live Brokers path** (contracts, catalog, credential shapes, FillEvent, calc/compliance profiles, ownership). Catalog descriptors carry asset class + currency + authScheme + calc/compliance profile ids. B6 sheets gate every integration. First pair = **`binance_com`** + **`kotak_neo`** (sheets signed). Architecture-deepening Issue 2 superseded. Deepen B2/B3/B5 against this shape. Dual desk = per-connection CalcProfile, no FX blend. D1 process rule: no adapter without B6. **Next: Phase 1 TDD contracts in Station.**

| # | Decision | Lock |
|---|----------|------|
| F1 | System shape | **Option A** |
| F2 | B6 path + template | **OK as written**; first pair signed |
| F3 | CalcProfile / ComplianceProfile named seams | **OK** |
| F4 | B4 dogfood | **COM + Kotak Neo** |
| F5 | Named next (non-Binance) broker | **`kotak_neo`** (first equities) |
| F6 | D1 require B6 now | **Yes** |

---

## Exit checklist

- [x] Course correction recorded (Issue 2 superseded as next slice)
- [x] Options A–D + **Option A LOCKED**
- [x] B6 template + first-pair sheets **SIGNED**
- [x] Calc + compliance hooks mapped to S2 / B3 / D*
- [x] B1–B6 and D1–D5 remap
- [x] Founder FINALIZE paragraph + F1–F6
- [x] Research checklist complete ([FULL pack §8](./BROKERS-SYSTEM-FULL.md))
- [x] BROKERS.md status flipped after FINALIZE (this pass)
- [ ] Phase 1 contracts (implement) — **NEXT**

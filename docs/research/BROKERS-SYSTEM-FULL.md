# Brokers SYSTEM — FULL research pack (reconciled)

**Status:** `RESEARCH EXIT COMPLETE` — reconciled 2026-07-25 (UBI Wasm pivot + R1–R11)  
**Architecture:** Station [ADR 0001](file:///Users/bishnu/tradeautopsy-station/docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md) — **uniform Wasm/WASI** adapters  
**Parts index:** [PARTS.md](./PARTS.md) — R1–R11 **DONE**  
**Sheets (canonical):** `/Users/bishnu/issues/brokers/sheets/` — [binance_com.md](../sheets/binance_com.md) · [kotak_neo.md](../sheets/kotak_neo.md)  
**System memo:** [BROKERS-SYSTEM.md](./BROKERS-SYSTEM.md) — amend locks for Wasm (see §0)  
**Handoff (next thread):** `~/.scratch/console-station-execution/HANDOFF-UBI-WASM.md`

---

## 0. Reconcile note (2026-07-25)

| Prior claim | Current truth |
|-------------|----------------|
| Option A = deepen native `BrokerAdapter` seams | **Superseded for execution** by ADR 0001: every adapter is a Wasm component |
| Phase 3 = reshape native COM + new native Kotak | **Gated:** WIT + Wasmtime host + `broker_http_call` first; COM/Kotak = Wasm components; native `.rs` = reference only |
| Console B5 “fence secrets” | **Stronger:** Console **zero broker code** (`lib/brokers/**` full removal) — R9 |
| Backend Box = brokers | Broker portion renamed **UBI**; Market Data / AI keys stay Backend Box |
| `origin` / community_reviewed | Field exists; **v1 = `first_party` only**; reviewed tier deferred |
| B6 path | Canonical = `issues/brokers/sheets/` only |
| Research “COMPLETE” before R5–R10 | Wave 2–3 now done; this pack is the reconciled SoT |

Venue facts (COM + Kotak) from R1–R4 and signed sheet content **remain**; only the **execution model** and Console ownership changed.

---

## 1. Product objective (locked)

| Pillar | Meaning |
|--------|---------|
| **Multi-broker** | Many integrations behind Station UBI + Enforcer |
| **Multi-asset** | Crypto spot ≠ equities — no generic “trade” lie |
| **Multi-currency** | USD crypto · INR equities — CalcProfile owns math; **no FX blend** |
| **Compliance-aware** | Host-side posture; Stop ≠ Kill; B6 human gate |
| **Extensible safely** | Traders/AI can author adapters — same Wasm sandbox as first-party |
| **Not Binance forever** | First pair = Binance.com + Kotak Neo |

**Hard keeps:** A8 Bearer; Wire HMAC machine-only; Kill always-on; Workflow Labs; `/api/bar/v1/**` until Z; no fog/Tax.

---

## 2. Target architecture (UBI + Wasm)

```text
Station UBI (was Backend Box · brokers only)
  BrokerCatalog (descriptors + origin=first_party v1)
  Connect flows (HMAC vs Kotak TOTP session)
  Keychain (scheme-tagged blobs — host only)

Enforcer
  Wasmtime Adapter Sandbox
  WIT: fetch_fills(cursor) -> FillEvent[]
  WIT import: broker_http_call(request) -> response
       → host attaches auth from Keychain
       → host allowlist + Kill DNS
  CalcProfile / ComplianceProfile (host)
  SyncControl Start/Stop (≠ Kill)

Console
  ZERO broker adapter / credential / CSV-broker code (R9)
  May keep history rows + protective broker slug strings only
```

### Contracts (Phase 1 — first code)

1. WIT world: `FillEvent` + cursor + `broker_http_call` (R5, R6)
2. Host stub: fixtures → contract tests
3. `BrokerDescriptor` fields incl. `origin` (R7)
4. CalcProfile / ComplianceProfile stubs
5. Keychain tagged blob design (no secrets into Wasm)

**Do not** ship live COM/Kotak Wasm until host + WIT green.

---

## 3. First pair (venue summary)

### 3.1 `binance_com`

| Field | Lock |
|-------|------|
| Host | `https://api.binance.com` (en-IN / global) — **not** US |
| Auth | HMAC key/secret in Keychain; host signs via `broker_http_call` |
| Fills | `GET /api/v3/myTrades` — symbol required; 24h window; limit ≤1000 |
| Bootstrap | Balance → `{ASSET}USDT` |
| Desk | `crypto_spot_usd` |
| Compliance | WithdrawDetected blocks connect; host-side |
| Kill DNS | **GAP:** `dns_block.rs` has **no** Binance hosts today — must fix before COM Kill dogfood (R8) |
| Native code | `binance_com_spot_*.rs` = **reference** for Wasm rebuild, not reshape-in-place |

### 3.2 `kotak_neo`

| Field | Lock |
|-------|------|
| Auth | TOTP + MPIN → session; tagged Keychain blob |
| Fills | `/quick/user/trades` — **day/session only**; no historical API |
| Products | CNC + MIS; cash `nse_cm`/`bse_cm`; refuse F&O |
| Desk | `equities_inr_cash` |
| Sync v1 | Read-only fills |
| Session | Daily IST posture; `stCode` 1003 → reconnect (not Kill) |
| Kill DNS | Kotak hosts already listed |
| Console prior art | Port protocol into Station, then **delete** Console broker code |

### 3.3 Dual desk

Per-connection CalcProfile strips; **no FX blend** USD+INR into one number. No `origin` badge on Notch/Today in v1.

---

## 4. Research parts map (all DONE)

| ID | Topic | File |
|----|-------|------|
| R1–R4 | Venue facts COM + Kotak | `parts/R1`…`R4` |
| R5 | WIT FillEvent fields | `parts/R5-fill-model-gap.md` |
| R6 | Host-mediated credentials | `parts/R6-credentials-start.md` |
| R7 | Catalog + dual desk | `parts/R7-catalog-dual-desk.md` |
| R8 | Host compliance + Kill DNS gap | `parts/R8-compliance-postures.md` |
| R9 | Console zero-broker checklist | `parts/R9-console-live-fence-map.md` |
| R10 | Stop ≠ Kill uniform | `parts/R10-sync-vs-kill.md` |
| R11 | Extensibility → ADR 0001 | `parts/R11-…` |

---

## 5. Build phases (post-reconcile)

| Phase | Work | Gate |
|------:|------|------|
| 0 | Docs: ADR · CONTEXT · PARTS · this pack · B6 amend · handoff | **THIS EXIT** |
| **1** | **Wasm/WIT/host** + FillEvent contract tests (TDD) | Next thread |
| 2 | UBI catalog + dual connect + tagged Keychain | After Phase 1 |
| 3 | Wasm `binance_com` + Wasm `kotak_neo` components | After Phase 2 + B6 still signed |
| 4 | Console zero-broker removal (R9 checklist) | After Station can dogfood or paths parked |
| 5 | Desk honesty + dual dogfood + COM Kill DNS fix | Before claiming Phase 5 |

---

## 6. Explicit refuse / do-not

- Native first-party tier / two-tier Wasm
- Live Binance.US as product slice
- Console `lib/brokers` as live SoT
- Secrets inside Wasm components
- FX-blended dual PnL in v1
- Adapter without signed B6
- Treating sandbox pass as B6 substitute

---

## 7. Status line for next thread

> Research exit complete (R1–R11 + ADR 0001). **Next code = Phase 1 Wasm/WIT/host** in `tradeautopsy-station`. Paste `HANDOFF-UBI-WASM.md`. Do not re-research venue facts or A8.

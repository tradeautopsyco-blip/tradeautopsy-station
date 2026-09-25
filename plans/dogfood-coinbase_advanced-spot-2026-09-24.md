# Dogfood — `coinbase_advanced` spot (`coinbase-advanced-spot`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (mechanical prep 2026-09-25); **Tier II** step 6 open  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/coinbase-advanced-spot.md` (**SHIPPING**)  
**B6:** `/Users/bishnu/issues/brokers/sheets/coinbase_advanced.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0018-coinbase-jwt-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6 |
| **II — Live** | Step **6** signed | **LIVE** on **`api.coinbase.com`** + sign-off → desk-live flip |

> Decision 8: **desk-live** = **`catalog_v1()` + registry live-slugs** (ADR 0019 §C).  
> `coinbase_advanced` is **Connect beta Enabled** on the spot book row + UI badge; not desk-live until step-6 sign-off. **Refuse** `api-sandbox.coinbase.com`.

---

## OFFLINE — Tier I steps 1–5 (integrator — no live Coinbase session)

### Pipeline steps

| Step | Deliverable | Status |
|------|-------------|--------|
| **1** Sheet | B6 `coinbase_advanced` **SIGNED** | [x] |
| **2** Shape | ADR **0018** **ACCEPTED** | [x] |
| **3** Lock | `coinbase-advanced-spot.md` **SHIPPING** | [x] |
| **4** Integrate | Wasm + host JWT mint + allowlist + dns + component tests | [x] |
| **5** Fixtures | `agent/fixtures/coinbase_advanced/` + `ubi_coinbase_advanced_component` | [x] |

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| ADR 0019 connect beta | `cargo test -p tradeautopsy-agent --lib adr_0019_connect_beta` | book row **Enabled**; slug **not** in `catalog_v1()` |
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `signed_coinbase_covers_surface_rows_0_to_25` |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_coinbase_advanced_component` | fills map + secrets-never-seen |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | prod host only; sandbox refused |

- [x] `adr_0019_connect_beta` (includes `coinbase_advanced`)
- [x] B6 lint
- [x] `ubi_coinbase_advanced_component`
- [x] lib `dns_block`

### Fence (`rg`)

```bash
rg -n 'coinbase|api-sandbox\.coinbase' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [x] `/api/v3/brokerage/*` paths on `api.coinbase.com` only for live book
- [x] Sandbox host absent from live allowlist + dns_block tests

### Wasm build

```bash
cargo build -p ubi-coinbase-advanced-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_coinbase_advanced_adapter.wasm`

### Swift Connect beta (no network)

- [x] CDP API key name + EC PEM in Keychain (`coinbase_jwt_es256_session`); PEM never logged
- [x] `BrokerDogfoodProgram.showsConnectBetaBadge(slug: "coinbase_advanced")` (`CoinbaseAdvancedConnectLifecycleTests`)

### Quote-filter unit behavior

- [x] `fetch_fills_maps_usd_pairs_and_stamps_identity`

### Catalog / registry (Tier I — **no desk-live flip**)

- [x] `catalog_books()` row `coinbase-advanced-spot`: **Enabled** (Connect beta)
- [x] `catalog_v1()` unchanged
- [x] `CLAIM-REGISTRY.md` **Live slugs** unchanged; Coinbase under **Connect beta** only

---

## LIVE — Tier II step 6 (CDP Advanced Trade key)

**Gate:** CDP **Advanced Trade** API key + EC private key on **`api.coinbase.com`**. Founder may defer.

### Preconditions

- [ ] CDP key name + ES256 PEM (view-only or trade per B6)
- [ ] Agent on **`127.0.0.1:9137`**
- [ ] At least one `-USD` and one `-USDT` product with fills for G7 cross-check

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |

### Connect runbook (UI)

1. **Brokers** → **Coinbase Advanced** (**Connect beta**).
2. **Connect** → key name + PEM → **Start sync** on `coinbase-advanced-spot`.
3. Optional JWT probe: `GET /api/v3/brokerage/accounts`.

### LIVE — Drills (B6 row 25)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | JWT probe + fills | Connect → Start | Accounts probe OK; fills paginate with cursor | |
| 2 | Historical fills + cursor | Multi-page | B6 row 2 honesty | |
| 3 | Rate limit discipline | Normal sync | B6 row 4 | |
| 4 | Kill L3 on `api.coinbase.com` | Kill slug `coinbase_advanced` | Prod blocked; sandbox never used | |
| 5 | UI cross-check | Manual | ≥1 `-USD` and ≥1 `-USDT` product booked correctly | |
| 6 | DualNoBlend | Coinbase + Binance USD | Separate strips | |

**STOP:** If sandbox host or wrong JWT `uri` appears in logs, stop and revisit ADR 0018.

---

## Sign-off

**Tier I (steps 1–5 — offline / mechanical):**

- [x] All OFFLINE pipeline + CI rows ticked except release wasm (2026-09-25 prep)
- [ ] LIVE preconditions + drills (deferred — no account)

**Tier II (step 6 — live):** sign only after drills green.

**Signed:** —  
**Date:** — (IST)

---

## Desk-live flip (Tier II only — **post-sign integrator PR**)

### 1. Rust `catalog_v1()`

Append from `catalog_books()` row `coinbase-advanced-spot`:

- `slug`: `coinbase_advanced`
- `book_id`: `coinbase-advanced-spot`
- `manifest_id`: `tradeautopsy:coinbase-advanced-spot@0.1.0`
- `auth_scheme`: `CoinbaseJwtEs256Session`
- `availability`: `Enabled`

Run `cargo test --lib catalog_v1`, `adr_0019_connect_beta`, `b6_gate`.

### 2. Registry

- Append `` `coinbase_advanced` `` to **Live slugs**; remove from **Connect beta** when graduated.

### 3. Swift

- Remove `"coinbase_advanced"` from `p4CryptoSpotSlugs` / `connectBetaSlugs`; update `CoinbaseAdvancedConnectLifecycleTests`.

### 4. PR checklist

- [ ] Dogfood signed; LIVE results recorded
- [ ] No auth/fence change unless drill bug

**book_id:** `coinbase-advanced-spot`

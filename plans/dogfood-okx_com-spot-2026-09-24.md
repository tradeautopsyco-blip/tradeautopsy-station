# Dogfood — `okx_com` spot (`okx-com-spot`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (mechanical prep 2026-09-25); **Tier II** step 6 open  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/okx-com-spot.md` (**SHIPPING**)  
**B6:** `/Users/bishnu/issues/brokers/sheets/okx_com.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0016-okx-passphrase-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6 |
| **II — Live** | Step **6** signed | **LIVE** drills on **www.okx.com** + sign-off → desk-live flip |

> Decision 8: **desk-live** = **`catalog_v1()` + registry live-slugs** (ADR 0019 §C).  
> `okx_com` is **Connect beta Enabled** on the spot book row + UI badge; not desk-live until step-6 sign-off. Refuse `us.okx.com`, `eea.okx.com`, and `x-simulated-trading: 1`.

---

## OFFLINE — Tier I steps 1–5 (integrator — no live OKX session)

Station cwd: `/Users/bishnu/tradeautopsy-station/agent`

### Pipeline steps

| Step | Deliverable | Status |
|------|-------------|--------|
| **1** Sheet | B6 `okx_com` **SIGNED** | [x] |
| **2** Shape | ADR **0016** **ACCEPTED** | [x] |
| **3** Lock | `okx-com-spot.md` **SHIPPING** | [x] |
| **4** Integrate | Wasm + host signing + allowlist + dns + component tests | [x] |
| **5** Fixtures | `agent/fixtures/okx_com/` + `ubi_okx_component` | [x] |

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| ADR 0019 connect beta | `cargo test -p tradeautopsy-agent --lib adr_0019_connect_beta` | book row **Enabled**; slug **not** in `catalog_v1()` |
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `signed_okx_covers_surface_rows_0_to_25` |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_okx_component` | `www.okx.com` only + secrets-never-seen |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `okx_com` host matrix green |

- [x] `adr_0019_connect_beta` (includes `okx_com`)
- [x] B6 lint
- [x] `ubi_okx_component`
- [x] lib `dns_block`

### Fence (`rg`)

Live host **`www.okx.com`** only (B6 row 0).

```bash
rg -n 'okx|www\.okx|us\.okx|eea\.okx|simulated-trading' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [x] Regional hosts refused
- [x] Spot fills path `GET /api/v5/trade/fills?instType=SPOT` allowlisted for `okx-com-spot`

### Wasm build

```bash
cargo build -p ubi-okx-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_okx_adapter.wasm`

### Swift Connect beta (no network)

- [x] Three-field connect: API key + secret + passphrase (`OkxPassphraseSession`); vault never logs secrets
- [x] `BrokerDogfoodProgram.showsConnectBetaBadge(slug: "okx_com")` (`OkxConnectLifecycleTests`)

### Catalog / registry (Tier I — **no desk-live flip**)

- [x] `catalog_books()` row `okx-com-spot`: **Enabled** (Connect beta)
- [x] `catalog_v1()` unchanged — Kotak + Binance desk-live pair only
- [x] `CLAIM-REGISTRY.md` **Live slugs** unchanged; OKX under **Connect beta** only

---

## LIVE — Tier II step 6 (global www.okx.com account)

**Gate:** OKX **global** mainnet keys on **`www.okx.com`** (not US/EEA regional). Founder may defer until keys available.

### Preconditions

- [ ] API key + secret + passphrase (read/trade as required for fills; no simulated-trading header)
- [ ] Agent on **`127.0.0.1:9137`**
- [ ] Spot fills visible on desk `instId` quotes (USDT/USDC/USD policy per B6 row 7)

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |
| **OKX region** | Global `www.okx.com` (no secrets) |

### Connect runbook (UI)

1. **Brokers** → **OKX** (**Connect beta**).
2. **Connect** → key + secret + passphrase → **Start sync** on `okx-com-spot`.
3. Optional probe: balance `GET /api/v5/account/balance` before fills poll (integrator path).

### LIVE — Drills (B6 row 25)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | Connect + fills sync | Connect → Start | Fills on `instType=SPOT`; USD desk strip | |
| 2 | 3-day retention honesty | Backfill attempt | No invented `fills-history`; honest pagination `after` | |
| 3 | Rate limit / pagination | Multi-page day | B6 row 4 discipline | |
| 4 | Regional / sim header refuse | N/A if only global key | No egress to refused hosts | |
| 5 | Kill L3 on `www.okx.com` | Toggle kill for `okx_com` | Blocks OKX prod; siblings OK | |
| 6 | DualNoBlend | OKX USD + Binance USD | Separate strips | |

**STOP:** If `x-simulated-trading: 1` or regional host appears in agent logs, stop and fix host policy before sign-off.

---

## Sign-off

**Tier I (steps 1–5 — offline / mechanical):**

- [x] All OFFLINE pipeline + CI rows ticked except release wasm (2026-09-25 prep)
- [ ] LIVE preconditions + drills (deferred — no account)

**Tier II (step 6 — live):** sign only after drills green.

- [ ] Drills 1–6 green (see table)
- [ ] No STOP fired

**Signed:** —  
**Date:** — (IST)

---

## Desk-live flip (Tier II only — **post-sign integrator PR**)

Do **not** open from Tier I ticks alone.

### 1. Rust `catalog_v1()` (`agent/src/ubi/catalog.rs`)

Append descriptor from `catalog_books()` row `okx-com-spot`:

- `slug`: `okx_com`
- `book_id`: `okx-com-spot`
- `manifest_id`: `okx_com.s1.v1`
- `auth_scheme`: `OkxPassphraseSession`
- `availability`: `Enabled`

Update `adr_0019_connect_beta` / `catalog_v1` / `b6_gate` tests per flip (assert `okx_com` in `catalog_v1()` after graduation).

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test --lib catalog_v1
cargo test --lib adr_0019_connect_beta
cargo test --lib b6_gate
```

### 2. Registry (`issues/compliance/CLAIM-REGISTRY.md`)

- Append `` `okx_com` `` to **Live slugs** sentence.
- Remove from **Connect beta** bullet when graduated.

### 3. Swift (`BrokerDogfoodProgram.swift`)

- Remove `"okx_com"` from `p4CryptoSpotSlugs` / `connectBetaSlugs`.
- Update `OkxConnectLifecycleTests` badge expectation after flip.

### 4. PR checklist

- [ ] Dogfood **Signed** + **Date**; LIVE Result cells filled
- [ ] No auth/fence changes unless drill bug

**book_id:** `okx-com-spot`

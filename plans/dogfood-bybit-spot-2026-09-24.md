# Dogfood — `bybit` spot (`bybit-com-spot`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (mechanical prep 2026-09-25); **Tier II** step 6 open  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/bybit-com-spot.md` (**SHIPPING**)  
**B6:** `/Users/bishnu/issues/brokers/sheets/bybit.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0015-bybit-header-hmac-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6 |
| **II — Live** | Step **6** signed | **LIVE** drills + sign-off → desk-live flip |

> Decision 8: **desk-live** = **`catalog_v1()` + registry live-slugs** (ADR 0019 §C).  
> `bybit` is **Connect beta Enabled** on the spot book row + UI badge (`BrokerDogfoodProgram.connectBetaSlugs`); not desk-live until step-6 sign-off.

---

## OFFLINE — Tier I steps 1–5 (integrator — no live Bybit session)

### Pipeline steps

| Step | Deliverable | Status |
|------|-------------|--------|
| **1** Sheet | B6 `bybit` **SIGNED** | [x] |
| **2** Shape | ADR **0015** **ACCEPTED** | [x] |
| **3** Lock | `bybit-com-spot.md` **SHIPPING** | [x] |
| **4** Integrate | Wasm + host HMAC + allowlist + dns + component tests | [x] |
| **5** Fixtures | `agent/fixtures/bybit/` + `ubi_bybit_component` | [x] |

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| ADR 0019 connect beta | `cargo test -p tradeautopsy-agent --lib adr_0019_connect_beta` | book row **Enabled**; slug **not** in `catalog_v1()` |
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `signed_bybit_covers_surface_rows_0_to_25` |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_bybit_component` | fills map + secrets-never-seen + quote filter |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `bybit` hosts + sibling refuse cases green |

- [x] `adr_0019_connect_beta` (includes `bybit`)
- [x] B6 lint
- [x] `ubi_bybit_component`
- [x] lib `dns_block`

### Fence (`rg`)

Prod host **`api.bybit.com`** only; refuse **`api-testnet.bybit.com`**, **`api-demo.bybit.com`** (B6 rows 0/12).

```bash
rg -n 'bybit|api-testnet|api-demo' agent/src/ubi/allowlist.rs agent/src/dns_block.rs
```

- [x] Allowlist paths scoped to `bybit-com-spot` / v5 spot reads only
- [x] `hosts_for_broker("bybit")` → prod host; testnet/demo absent from live allowlist

### Wasm build

```bash
cargo build -p ubi-bybit-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_bybit_adapter.wasm`

### Swift Connect beta (no network)

- [x] `BrokerCatalog` spot row: `bybit`, `availability: .enabled`, `hmacApiKeySecret`
- [x] `BrokerDogfoodProgram.showsConnectBetaBadge(slug: "bybit")` (`BybitConnectLifecycleTests`)

### Catalog / registry (Tier I — **no desk-live flip**)

- [x] `catalog_books()` row `bybit-com-spot`: **Enabled** (Connect beta)
- [x] `catalog_v1()` unchanged — still `kotak_neo` + `binance_com` only
- [x] `CLAIM-REGISTRY.md` **Live slugs** unchanged; Bybit listed under **Connect beta** only

---

## LIVE — Tier II step 6 (mainnet read-only API key)

**Gate:** Bybit **mainnet** API key (read-only preferred per B6 row 9). Refuse testnet/demo hosts. Founder may defer; do not tick Tier II until drills run on **`api.bybit.com`**.

### Preconditions

- [ ] Mainnet API key + secret (no withdraw permission; `query-api` probe green when wired)
- [ ] Agent listening on **`127.0.0.1:9137`** (Station embedded agent or release agent)
- [ ] Unified account (`UNIFIED`) with spot execution history on desk pairs (USDT/USDC/USD quote)

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |
| **Bybit account type** | UNIFIED / SPOT (no secrets) |

### Connect runbook (UI)

1. Open **Brokers** → **Bybit** (badge **Connect beta**).
2. **Connect** → enter API key + secret → host stores `HmacApiKeySecret` vault blob (Wasm never sees secret).
3. **Start sync** on book `bybit-com-spot`.
4. Confirm credentials present via loopback `POST /api/daemon/broker/credentials/present` with `brokerSlug` `bybit` and the connection id for this desk.

### Z11 — Drills (B6 row 25)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | Connect + execution poll | Connect → Start → wait for sync | `GET /v5/execution/list?category=spot` fills on desk pairs; USD strip on `bybit-com-spot` | |
| 2 | 7-day window / pagination | Second sync or forced backfill | Honest 7d windows + `cursor`; no invented single-call full history | |
| 3 | Rate discipline | Normal sync day | No sustained `retCode` 10006; respect 50/s execution list (B6 row 4) | |
| 4 | Quote filter honesty | Account with mixed quotes | Non-USDT/USDC/USD pairs refused or flagged; no silent USD | |
| 5 | Kill | Kill switch L3 for slug `bybit` | Blocks `api.bybit.com`; Binance/Kotak OK | |
| 6 | DualNoBlend USD strip | Bybit USD + Binance USD (+ Kotak INR if connected) | COM USD hero unchanged; each book its own strip | |

**STOP:** If responses come from testnet/demo hosts, or execution POST paths succeed, stop and reopen lock + ADR 0015 — do not widen allowlist.

---

## Sign-off

**Tier I (steps 1–5 — offline / mechanical):**

- [x] All OFFLINE pipeline + CI rows ticked except release wasm artifact (2026-09-25 prep)
- [ ] LIVE preconditions + drills (deferred — no mainnet key in dogfood session)

**Tier II (step 6 — live):** sign only after drills 1–6 green.

- [ ] Drill 1 green (fills + USD insert)
- [ ] Drill 2 green (window / pagination)
- [ ] Drill 3 green (rate discipline)
- [ ] Drill 4 green (quote filter)
- [ ] Drill 5 green (Kill hosts)
- [ ] Drill 6 green (DualNoBlend)
- [ ] No STOP fired, or STOP recorded with lock revisit

**Signed:** —  
**Date:** — (IST)

---

## Desk-live flip (Tier II only — **post-sign integrator PR**)

Do **not** open this PR from Tier I ticks alone. Requires founder signature above + green LIVE drills.

### 1. Rust `catalog_v1()` (`agent/src/ubi/catalog.rs`)

Append a `BrokerDescriptor` (copy fields from the existing `catalog_books()` row for `bybit-com-spot`):

- `slug`: `bybit`
- `book_id`: `bybit-com-spot`
- `manifest_id`: `bybit.spot.v1`
- `auth_scheme`: `HmacApiKeySecret`
- `availability`: `Enabled`

Update tests in the same file:

- `catalog_v1_first_pair_only_enabled` → expect **+1** desk-live slug (or rename to `catalog_v1_desk_live_slugs`)
- `b6_gate_only_signed_first_pair_is_enabled` → extend enabled vec when desk-live policy expands beyond Kotak/Binance
- `adr_0019_connect_beta_slugs_enabled_on_book_rows` → assert `bybit` **is** in `catalog_v1()` after flip (remove from connect-beta exclusion set)

Run:

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test --lib catalog_v1
cargo test --lib adr_0019_connect_beta
cargo test --lib b6_gate
```

### 2. Registry (`issues/compliance/CLAIM-REGISTRY.md`)

- **Live slugs (desk-live Tier II):** append `` `bybit` `` to the sentence (keep existing desk-live slugs until intentionally replaced).
- **Connect beta** bullet: remove `` `bybit` `` (or footnote “graduated … IST”).
- Bybit SHIPPING row: parenthetical → desk-live Tier II when flipped.

### 3. Swift (`station/StationApp/Broker/Models/BrokerDogfoodProgram.swift`)

- Remove `"bybit"` from `p4CryptoSpotSlugs` / `connectBetaSlugs` so UI drops **Connect beta** badge after desk-live.
- `BrokerCatalog` spot row stays `availability: .enabled`.

Update tests:

- `BybitConnectLifecycleTests` — `#expect(!BrokerDogfoodProgram.showsConnectBetaBadge(slug: "bybit"))` after flip

### 4. PR checklist

- [ ] Dogfood plan: **Signed** + **Date** filled; all LIVE drill Result cells noted
- [ ] `P3-KICKOFF.md` / Wave 4 row: **desk-live** for `bybit` when intended for Today hero
- [ ] No change to auth, allowlist, or wasm unless a drill exposed a bug

**book_id:** `bybit-com-spot`

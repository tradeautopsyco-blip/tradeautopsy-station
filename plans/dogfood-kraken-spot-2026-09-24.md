# Dogfood — `kraken` spot (`kraken-com-spot`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (mechanical prep 2026-09-25); **Tier II** step 6 open  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/kraken-com-spot.md` (**SHIPPING**)  
**B6:** `/Users/bishnu/issues/brokers/sheets/kraken.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0017-kraken-spot-nonce-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6 |
| **II — Live** | Step **6** signed | **LIVE** drills + sign-off → desk-live flip |

> Decision 8: **desk-live** = **`catalog_v1()` + registry live-slugs** (ADR 0019 §C).  
> `kraken` is **Connect beta Enabled** on the spot book row + UI badge; not desk-live until step-6 sign-off.

---

## OFFLINE — Tier I steps 1–5 (integrator — no live Kraken session)

### Pipeline steps

| Step | Deliverable | Status |
|------|-------------|--------|
| **1** Sheet | B6 `kraken` **SIGNED** | [x] |
| **2** Shape | ADR **0017** **ACCEPTED** | [x] |
| **3** Lock | `kraken-com-spot.md` **SHIPPING** | [x] |
| **4** Integrate | Wasm + host nonce signing + allowlist + dns + component tests | [x] |
| **5** Fixtures | `agent/fixtures/kraken/` + `ubi_kraken_component` | [x] |

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| ADR 0019 connect beta | `cargo test -p tradeautopsy-agent --lib adr_0019_connect_beta` | book row **Enabled**; slug **not** in `catalog_v1()` |
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `signed_kraken_covers_surface_rows_0_to_25` |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_kraken_component` | secrets-never-seen + quote filter |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `kraken` + `futures.kraken.com` refuse |

- [x] `adr_0019_connect_beta` (includes `kraken`)
- [x] B6 lint
- [x] `ubi_kraken_component`
- [x] lib `dns_block`

### Fence (`rg`)

Prod **`api.kraken.com`** only; refuse **`futures.kraken.com`** on spot book (B6 rows 0/12/21).

```bash
rg -n 'kraken|futures\.kraken' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [x] Spot private paths on `api.kraken.com` only
- [x] Futures host refused for `kraken-com-spot`

### Wasm build

```bash
cargo build -p ubi-kraken-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_kraken_adapter.wasm`

### Swift Connect beta (no network)

- [x] Key-entry → vault blob; host signs `API-Key` / `API-Sign` (`KrakenConnectLifecycleTests`)
- [x] `BrokerDogfoodProgram.showsConnectBetaBadge(slug: "kraken")`

### Quote-filter unit behavior

- [x] `quote_filter_skips_non_desk_pairs_without_failing_sync`

### Catalog / registry (Tier I — **no desk-live flip**)

- [x] `catalog_books()` row `kraken-com-spot`: **Enabled** (Connect beta)
- [x] `catalog_v1()` unchanged
- [x] `CLAIM-REGISTRY.md` **Live slugs** unchanged; Kraken under **Connect beta** only

---

## LIVE — Tier II step 6 (Kraken spot API key)

**Gate:** Kraken **spot** API key on **`api.kraken.com`**. Founder may defer.

### Preconditions

- [ ] API key + base64 secret (query/trade permissions per B6)
- [ ] Agent on **`127.0.0.1:9137`**
- [ ] Monotonic nonce discipline understood (B6 row 16)

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |

### Connect runbook (UI)

1. **Brokers** → **Kraken** (**Connect beta**).
2. **Connect** → key + secret → **Start sync** on `kraken-com-spot`.

### LIVE — Drills (B6 row 25)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | `TradesHistory` poll + nonce | Connect → Start | Desk pairs map; monotonic nonce | |
| 2 | Pagination `ofs` | Multi-page history | B6 rows 4–6 honesty | |
| 3 | Invalid nonce / 401 | Force bad nonce or revoke key once | Reconnect prompt, **not** Kill | |
| 4 | Rate limit discipline | Normal sync | B6 row 4 | |
| 5 | Kill L3 on `api.kraken.com` | Kill slug `kraken` | Spot host blocked; futures not used on this book | |
| 6 | DualNoBlend USD strip | Kraken + Binance | Separate strips | |

**STOP:** If `futures.kraken.com` appears in egress for this book, stop before sign-off.

---

## Sign-off

**Tier I (steps 1–5 — offline / mechanical):**

- [x] All OFFLINE pipeline + CI rows ticked except release wasm (2026-09-25 prep)
- [ ] LIVE preconditions + drills (deferred — no account)

**Tier II (step 6 — live):** sign only after drills 1–6 green.

**Signed:** —  
**Date:** — (IST)

---

## Desk-live flip (Tier II only — **post-sign integrator PR**)

### 1. Rust `catalog_v1()`

Append from `catalog_books()` row `kraken-com-spot`:

- `slug`: `kraken`
- `book_id`: `kraken-com-spot`
- `manifest_id`: `kraken.spot.v1`
- `auth_scheme`: `KrakenSpotNonceSession`
- `availability`: `Enabled`

Run `cargo test --lib catalog_v1`, `adr_0019_connect_beta`, `b6_gate`.

### 2. Registry

- Append `` `kraken` `` to **Live slugs**; remove from **Connect beta** when graduated.

### 3. Swift

- Remove `"kraken"` from `p4CryptoSpotSlugs` / `connectBetaSlugs`; update `KrakenConnectLifecycleTests`.

### 4. PR checklist

- [ ] Dogfood signed; LIVE results recorded
- [ ] No fence/auth change unless drill bug

**book_id:** `kraken-com-spot`

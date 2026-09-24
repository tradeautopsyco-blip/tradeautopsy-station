# Dogfood — `bybit` spot (`bybit-com-spot`)

**Status:** DRAFT — **Tier I** accountless path (steps 4–5 offline; step 6 open)  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/bybit-com-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/bybit.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0015-bybit-header-hmac-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** (B6, shape ADR, lock SHIPPING, adapter green, fixtures + component tests) | **OFFLINE** section — not step 6; `catalog_books` stays **Planned**; no `catalog_v1()` **Enabled** / registry live-slugs line |
| **II — Live** | Step **6** signed live proof | **LIVE** drills + founder sign-off below |

> Decision 8: **Enabled** = **Tier II only**. Offline ticks do not substitute for live step 6 (ADR 0019 §B).

---

## OFFLINE — Tier I steps 4–5 (required now — no live account)

Tick when green in CI or local `tradeautopsy-station/agent` cwd.

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `bybit` sheet row green |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_bybit_component` | all tests green; `assert_component_never_saw_secrets` |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `bybit` hosts + sibling refuse cases green |

- [ ] B6 lint
- [ ] `ubi_bybit_component`
- [ ] lib `dns_block` (slug + crypto matrix)

### Fence (`rg`)

Prod host **`api.bybit.com`** only; refuse **`api-testnet.bybit.com`**, **`api-demo.bybit.com`** (B6 rows 0/12).

```bash
rg -n 'bybit|api-testnet|api-demo' agent/src/ubi/allowlist.rs agent/src/dns_block.rs
```

- [ ] Allowlist paths scoped to `bybit-com-spot` / v5 spot reads only
- [ ] `hosts_for_broker("bybit")` → prod host; testnet/demo absent from live allowlist

### Wasm build

```bash
cargo build -p ubi-bybit-adapter --target wasm32-wasip2 --release
```

Artifact: `target/wasm32-wasip2/release/ubi_bybit_adapter.wasm` (packaging path recorded in integrator PR).

- [ ] Release wasm present for `ubi-bybit-adapter`

### Swift fake connect (no network)

Fake agent / vault stub — key-entry validators only; no call to `api.bybit.com`.

- [ ] Connect flow stores `HmacApiKeySecret` blob; Wasm never receives secret (ADR 0015)
- [ ] StationTests lifecycle green when `BybitConnectLifecycleTests` (or family HMAC stub) lands; until then: manual stub checklist in offline G7 table

### Quote-filter unit behavior

B6 row 7 — USDT/USDC/USD desk only; `ETHBTC`-class pairs refused.

- [ ] `cargo test -p tradeautopsy-agent --test ubi_bybit_component quote_filter_refuses_non_usd_stable_quotes`

---

## LIVE — Tier II step 6 (deferred — no account)

| Drill | Status | Notes |
|-------|--------|-------|
| Connect + execution poll (`GET /v5/execution/list?category=spot`) | **DEFERRED — no account** | Mainnet read-only key when available |
| 7-day window / pagination honesty | **DEFERRED — no account** | B6 rows 4–6 |
| Rate limit discipline (50/s execution list) | **DEFERRED — no account** | B6 row 4 |
| Kill drill L3 on `api.bybit.com` | **DEFERRED — no account** | B6 row 22 |
| DualNoBlend USD strip | **DEFERRED — no account** | B6 row 24 |

---

## Sign-off

**Tier I (steps 4–5 — offline verification only):**

- [ ] All OFFLINE rows ticked
- [ ] LIVE table explicitly **DEFERRED — no account**
- [ ] Registry + CORRECTNESS unchanged **SHIPPING**; live-slugs note still excludes `bybit` until Tier II

**Tier II (step 6 — live):** founder signs after LIVE drills green (do not sign until live section is executed).

**Signed:** — **Date:** — (IST)

**Enabled flip (Tier II only):** integrator PR — `catalog_v1()` + `catalog_books` **Enabled** for `bybit` / `bybit-com-spot`, registry “Live slugs” sentence, Swift `availability: .enabled`. Not part of accountless / Tier I exit.

**book_id:** `bybit-com-spot`

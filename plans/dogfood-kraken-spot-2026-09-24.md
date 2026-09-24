# Dogfood — `kraken` spot (`kraken-com-spot`)

**Status:** DRAFT — **Tier I** accountless path (steps 4–5 offline; step 6 open)  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/kraken-com-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/kraken.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0017-kraken-spot-nonce-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6; **Planned** only |
| **II — Live** | Step **6** signed | **LIVE** drills + sign-off |

> Decision 8: **Enabled** = **Tier II only** (ADR 0019 §C).

---

## OFFLINE — Tier I steps 4–5 (required now — no live account)

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `kraken` sheet row green |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_kraken_component` | secrets-never-seen + fixture sync |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `kraken` + `futures.kraken.com` refuse |

- [ ] B6 lint
- [ ] `ubi_kraken_component`
- [ ] lib `dns_block`

### Fence (`rg`)

Prod **`api.kraken.com`** only; refuse **`futures.kraken.com`** on spot book (B6 rows 0/12/21).

```bash
rg -n 'kraken|futures\.kraken' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [ ] Spot private paths on `api.kraken.com` only
- [ ] Futures host refused for `kraken-com-spot`

### Wasm build

```bash
cargo build -p ubi-kraken-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_kraken_adapter.wasm`

### Swift fake connect (no network)

- [ ] Key-entry → base64 secret vault blob; host signs `API-Key` / `API-Sign`; no network in StationTests stub
- [ ] Bad nonce / 401 → reconnect posture documented (B6 row 16) — assert in lifecycle test when present

### Quote-filter unit behavior

B6 row 7 — USDT/USDC/USD/EUR desk; non-desk pairs skipped/refused.

- [ ] `cargo test -p tradeautopsy-agent --test ubi_kraken_component quote_filter_skips_non_desk_pairs_without_failing_sync`

---

## LIVE — Tier II step 6 (deferred — no account)

| Drill | Status | Notes |
|-------|--------|-------|
| Signed `TradesHistory` poll + monotonic nonce | **DEFERRED — no account** | B6 row 16 |
| Pagination `ofs` honesty | **DEFERRED — no account** | B6 rows 4–6 |
| Invalid nonce / permission → reconnect not Kill | **DEFERRED — no account** | B6 row 16 |
| Rate limit discipline | **DEFERRED — no account** | B6 row 4 |
| Kill drill L3 on `api.kraken.com` | **DEFERRED — no account** | B6 row 22 |
| DualNoBlend USD strip | **DEFERRED — no account** | B6 row 24 |

---

## Sign-off

**Tier I (steps 4–5 — offline verification only):**

- [ ] All OFFLINE rows ticked
- [ ] LIVE table **DEFERRED — no account**
- [ ] Registry unchanged; `kraken` not in live Enabled set until Tier II

**Tier II (step 6 — live):** sign only after LIVE drills green.

**Signed:** — **Date:** — (IST)

**Enabled flip (Tier II only):** integrator PR — `catalog_v1()` + registry live-slugs + Swift; not from Tier I / accountless exit.

**book_id:** `kraken-com-spot`

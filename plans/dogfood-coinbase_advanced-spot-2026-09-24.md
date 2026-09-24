# Dogfood — `coinbase_advanced` spot (`coinbase-advanced-spot`)

**Status:** DRAFT — **Tier I** accountless path (steps 4–5 offline; step 6 open)  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/coinbase-advanced-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/coinbase_advanced.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0018-coinbase-jwt-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6; **Planned** only |
| **II — Live** | Step **6** signed | **LIVE** on **`api.coinbase.com`** + sign-off |

> Decision 8: **Enabled** = **Tier II only**. **Refuse** `api-sandbox.coinbase.com`.

---

## OFFLINE — Tier I steps 4–5 (required now — no live account)

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `coinbase_advanced` row green |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_coinbase_advanced_component` | fills map + secrets-never-seen |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | prod host only; sandbox refused |

- [ ] B6 lint
- [ ] `ubi_coinbase_advanced_component`
- [ ] lib `dns_block` (`coinbase_advanced` + sandbox deny)

### Fence (`rg`)

```bash
rg -n 'coinbase|api-sandbox\.coinbase' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [ ] `/api/v3/brokerage/*` paths on `api.coinbase.com` only for live book
- [ ] Sandbox host absent from live allowlist + dns_block tests

### Wasm build

```bash
cargo build -p ubi-coinbase-advanced-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_coinbase_advanced_adapter.wasm`

### Swift fake connect (no network)

CDP API key name + EC PEM in Keychain (`coinbase_jwt_es256_session`).

- [ ] Connect never logs PEM; host mints ES256 JWT per ADR 0018 (uri + 120s expiry) in unit/stub tests
- [ ] Wrong PEM → 401 reconnect posture — live only; stub documents expected behavior

### Quote-filter unit behavior

B6 row 7 — named `-USD`, `-USDT`, `-USDC` rows; no silent USD for USDT products.

- [ ] `cargo test -p tradeautopsy-agent --test ubi_coinbase_advanced_component fetch_fills_maps_usd_pairs_and_stamps_identity`
- [ ] Adapter `quote_from_product` longest-suffix match covered on `-USDT` fixture when added

---

## LIVE — Tier II step 6 (deferred — no account)

| Drill | Status | Notes |
|-------|--------|-------|
| JWT probe `GET /api/v3/brokerage/accounts` | **DEFERRED — no account** | CDP key + PEM |
| Historical fills + cursor pagination | **DEFERRED — no account** | B6 row 2 |
| Rate limit discipline | **DEFERRED — no account** | B6 row 4 |
| Kill drill L3 on `api.coinbase.com` | **DEFERRED — no account** | slug `coinbase_advanced` |
| UI cross-check ≥1 `-USD` and ≥1 `-USDT` product | **DEFERRED — no account** | G7 money honesty |

---

## Sign-off

**Tier I (steps 4–5 — offline verification only):**

- [ ] All OFFLINE rows ticked
- [ ] LIVE table **DEFERRED — no account**

**Tier II (step 6 — live):** sign only after LIVE drills green.

**Signed:** — **Date:** — (IST)

**Enabled flip (Tier II only):** integrator PR — `catalog_v1()` **Enabled**, `catalog.rs` / Swift components, `CLAIM-REGISTRY` live-slugs sentence; not from Tier I offline ticks.

**book_id:** `coinbase-advanced-spot`

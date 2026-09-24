# Dogfood — `okx_com` spot (`okx-com-spot`)

**Status:** DRAFT — **Tier I** accountless path (steps 4–5 offline; step 6 open)  
**Accountless exit:** [/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md](/Users/bishnu/issues/brokers/P4-ACCOUNTLESS-EXIT.md)  
**Offline checklist (all four slugs):** [/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md](/Users/bishnu/issues/brokers/plans/P4-offline-g7-checklist.md)  
**Lock:** `/Users/bishnu/issues/compliance/locks/okx-com-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/okx_com.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0016-okx-passphrase-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6; stay **Planned**; no desk-live **Enabled** flip |
| **II — Live** | Step **6** signed | **LIVE** drills on **www.okx.com** + sign-off |

> Decision 8: **Enabled** = **Tier II only**. Refuse `us.okx.com`, `eea.okx.com`, and `x-simulated-trading: 1`.

---

## OFFLINE — Tier I steps 4–5 (required now — no live account)

Station cwd: `/Users/bishnu/tradeautopsy-station/agent`

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `okx_com` row green |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_okx_component` | `calls_target_www_okx_com_only`, `sibling_hosts_are_not_allowlisted`, secrets-never-seen |
| Kill + crypto fence | `cargo test -p tradeautopsy-agent --lib dns_block` | `okx_com` host matrix green |

- [ ] B6 lint
- [ ] `ubi_okx_component`
- [ ] lib `dns_block`

### Fence (`rg`)

Live host **`www.okx.com`** only (B6 row 0).

```bash
rg -n 'okx|www\.okx|us\.okx|eea\.okx|simulated-trading' agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [ ] Regional hosts refused
- [ ] Spot fills path `GET /api/v5/trade/fills?instType=SPOT` allowlisted for `okx-com-spot`

### Wasm build

```bash
cargo build -p ubi-okx-adapter --target wasm32-wasip2 --release
```

- [ ] Release wasm: `target/wasm32-wasip2/release/ubi_okx_adapter.wasm`

### Swift fake connect (no network)

Three-field key entry: API key + secret + passphrase (ADR 0016).

- [ ] Vault blob `okx_passphrase_session`; Swift never logs secret/passphrase
- [ ] Fake agent probe path stubbed — no egress to `www.okx.com`

### Quote-filter unit behavior

B6 row 7 — USDT/USDC/USD (and EUR flag policy) via `quote_from_inst_id`; no silent USD for BTC-quoted pairs.

- [ ] `cargo test -p tradeautopsy-agent --test ubi_okx_component fills_row_maps_to_fill_event` (desk quote on fixture)
- [ ] Adapter unit: non-desk `instId` refused or flagged per lock (extend test if missing)

---

## LIVE — Tier II step 6 (deferred — no account)

| Drill | Status | Notes |
|-------|--------|-------|
| Connect + balance probe (`GET /api/v5/account/balance`) | **DEFERRED — no account** | www.okx.com global account |
| Fills sync + 3-day retention honesty | **DEFERRED — no account** | B6; no invented `fills-history` |
| Rate limit / pagination `after` | **DEFERRED — no account** | B6 row 4 |
| Kill drill L3 on `www.okx.com` | **DEFERRED — no account** | slug `okx_com` |
| DualNoBlend | **DEFERRED — no account** | B6 row 24 |

---

## Sign-off

**Tier I (steps 4–5 — offline verification only):**

- [ ] All OFFLINE rows ticked
- [ ] LIVE table **DEFERRED — no account**

**Tier II (step 6 — live):** sign only after LIVE drills on mainnet.

**Signed:** — **Date:** — (IST)

**Enabled flip (Tier II only):** integrator PR — `catalog_v1()` + `catalog_books` **Enabled** for `okx_com`, registry live-slugs line, Swift catalog; not from Tier I offline ticks.

**book_id:** `okx-com-spot`

# Dogfood — `bybit` spot (`bybit-com-spot`)

**Status:** DRAFT — unsigned, founder executes on live mainnet  
**Lock:** `/Users/bishnu/issues/compliance/locks/bybit-com-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/bybit.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0015-bybit-header-hmac-session.md` (`ACCEPTED`)

> Phase 4 scaffold. Catalog/components integrator lands separately.

## Preconditions

- [ ] B6 SIGNED rows 0–25 (quote-filter row 7; testnet refuse rows 0/23)
- [ ] ADR 0015 ACCEPTED (header HMAC; no OAuth API routes)
- [ ] Lock SHIPPING `bybit-com-spot`
- [ ] Wasm built: `ubi-bybit-adapter` + host fence `bybit-com-spot`
- [ ] **`HmacApiKeySecret`** in Keychain (mainnet key, read-only preferred)
- [ ] Agent on `127.0.0.1:9137`
- [ ] Catalog still **Planned** until sign-off below

## Drills

| # | Drill | Pass when |
|---|-------|-----------|
| 1 | Connect + execution poll | Key-entry → `GET /v5/execution/list?category=spot` returns fills; host attaches `X-BAPI-*`; no secret in Wasm |
| 2 | 7-day window honesty | Incremental sync uses ≤7d windows; no single call claims full history |
| 3 | Quote-filter | `BTCUSDT` books; `ETHBTC` refused/flagged per row 7 |
| 4 | Fee honesty | `execFee` + `feeCurrency` on fills; flag when unhandled |
| 5 | Kill | L3 blocks `api.bybit.com` for slug `bybit` |
| 6 | DualNoBlend | USD strip alongside INR strips without blend |

## Sign-off

- [ ] Drills 1–6 green
- [ ] **Signed:** — **Date:** —
- [ ] Enabled flip ticket (catalog + components) after sign-off

**book_id:** `bybit-com-spot`

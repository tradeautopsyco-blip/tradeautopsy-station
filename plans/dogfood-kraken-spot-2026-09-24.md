# Dogfood — `kraken` spot (`kraken-com-spot`)

**Status:** DRAFT — founder executes on a live account (catalog stays **Planned** until signed)  
**Lock:** `/Users/bishnu/issues/compliance/locks/kraken-com-spot.md` (SHIPPING)  
**B6:** `/Users/bishnu/issues/brokers/sheets/kraken.md` (`SIGNED` 2026-09-24 IST)  
**ADR:** `docs/adr/0017-kraken-spot-nonce-session.md` (`ACCEPTED`)

> Docs only. No invented venue facts — every drill cites its B6 row.

## Preconditions

- [ ] B6 `SIGNED` (rows 7/12/22 quote-filter + futures refusal read fully)
- [ ] ADR 0017 `ACCEPTED` (SHA512 + monotonic nonce; no OAuth)
- [ ] Adapter built: `ubi-kraken-adapter` + host signer on `api.kraken.com`
- [ ] Lock SHIPPING for `kraken-com-spot`
- [ ] Catalog lists `kraken` as **Planned** only (no integrator flip in this slice)
- [ ] Agent running on `127.0.0.1:9137` (release build)
- [ ] Live Kraken spot API key + **base64 secret** (production; no spot testnet per B6 row 23)

## Drills

| # | Drill | Pass when |
|---|-------|-----------|
| 1 | Key-entry → signed `TradesHistory` poll | Host POST attaches `API-Key`/`API-Sign`; fills map with `broker_slug=kraken`; nonce strictly increases across consecutive private calls (row 16) |
| 2 | Quote-filter | `XBTUSDT` / `ETHUSDT` book; `ETHXBT` (or other non USDT/USDC/USD/EUR quote) skipped or refused — never silently USD (row 7) |
| 3 | Pagination honesty | `ofs` paging until empty; no claim of unbounded single-call history (rows 4–6) |
| 4 | Futures host refusal | Fence rejects `futures.kraken.com` on book `kraken-com-spot` (rows 0/12/21) |
| 5 | Invalid nonce / permission | Force bad nonce or revoked key → pause + reconnect prompt, **never Kill** (row 16 posture) |
| 6 | Kill drill | L3 blocks `api.kraken.com` for slug `kraken`; slug-scoped (row 22) |
| 7 | DualNoBlend | Kraken USD strip live alongside COM/Bybit USD + INR strips with no blend hero (row 24) |

## Sign-off checklist (Planned → Enabled)

- [ ] Drills 1–7 green or STOP documented
- [ ] **Signed:** — **Date:** —
- [ ] Enabled flip: separate integrator ticket (catalog/components) — not in Wave 4c scaffold

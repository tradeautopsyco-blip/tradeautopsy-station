# Dogfood — `zerodha_kite` cash (`zerodha-nse-bse-cash`)

**Status:** OPEN — founder Z11 (catalog stays **Planned** until this record is signed)  
**Lock:** `/Users/bishnu/issues/compliance/locks/zerodha-nse-bse-cash.md`  
**B6:** `/Users/bishnu/issues/brokers/sheets/zerodha_kite.md` (`SIGNED`)  
**ADR:** `docs/adr/0005-zerodha-redirect-callback.md` (`ACCEPTED`)

## Preconditions

- [ ] Kite Connect app redirect registered exactly: `http://127.0.0.1:9137/api/daemon/broker/zerodha/callback`
- [ ] Agent running on `127.0.0.1:9137` (release build)
- [ ] Live Kite account (production; no paper env per B6 row 23)

## Drills (B6 row 25 / Z11)

| # | Drill | Pass when |
|---|--------|-----------|
| 1 | Redirect login → checksum → `/trades` poll | Fills appear in Station sync; INR at insert |
| 2 | Daily token / 06:00 expiry or forced logout | `403` / `TokenException` → reconnect prompt, **not** Kill |
| 3 | Rate discipline | No sustained 429; obey 10 rps others / 1 rps quote |
| 4 | Day-book cursor | Second poll does not widen duplicate fills |
| 5 | Kill | L3 blocks `api.kite.trade`, `kite.zerodha.com`, `ws.kite.trade` |
| 6 | DualNoBlend | COM USD hero unchanged; INR cash separate strip |

## Sign-off

**Signed:** —  
**Date:** —  
**Enabled flip:** after sign-off, one ticket: catalog `Planned` → `Enabled` (agent + Swift).

# Dogfood — `zerodha_kite` cash (`zerodha-nse-bse-cash`)

**Status:** OPEN — founder Z11 (catalog stays **Planned** until this record is signed)  
**Lock:** `/Users/bishnu/issues/compliance/locks/zerodha-nse-bse-cash.md`  
**B6:** `/Users/bishnu/issues/brokers/sheets/zerodha_kite.md` (`SIGNED`)  
**ADR:** `docs/adr/0005-zerodha-redirect-callback.md` (`ACCEPTED`)

> Station shows **Planned** but **Connect / Start** are on for `zerodha_kite` only (`BrokerDogfoodProgram`) until you sign this file and we flip catalog **Enabled**.

## Preconditions

- [ ] Kite Connect app redirect registered **exactly** (portal requires `https://`):  
  `https://127.0.0.1:9140/api/daemon/broker/zerodha/callback`  
  (developers.kite.trade → your app → Redirect URL; **Postback** left empty)
- [ ] Agent listening on **`127.0.0.1:9137`** (run Station.app so the embedded agent is healthy, or release agent on that port)
- [ ] Live Kite account (production; no paper env per B6 row 23)
- [ ] Kite Connect **paid** app with **api_key** + **api_secret** (daily login token model)

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |
| **Kite app name** | (no secrets) |

## Runbook — Connect (UI)

1. Open **Brokers** → **Zerodha Kite** (badge **Planned**).
2. **Connect** → enter API key + secret → browser opens Kite login.
3. Approve login; browser hits loopback callback; Station should auto-start sync.
4. Confirm agent vault: loopback `POST /api/daemon/broker/credentials/present` with body  
   `{"environment":"prod","brokerSlug":"zerodha_kite","brokerConnectionId":"00000000-0000-4000-8000-000000000004"}` → `"present": true` (fixed connection id in `BrokerConnectionIdentity`).

## Runbook — Connect (loopback only, optional)

If UI is unavailable, same flow via HMAC-protected daemon routes (Station signs requests when agent is paired).

1. `POST /api/daemon/broker/zerodha/connect/begin` with `brokerSlug` `zerodha_kite`, connection id above, `apiKey`, `apiSecret`.
2. Open `loginUrl` from response in browser; finish login.
3. Callback: `GET /api/daemon/broker/zerodha/callback?request_token=…&state=…`

## Drills (B6 row 25 / Z11)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | Redirect login → checksum → `/trades` poll | Connect → Start if needed → wait for sync | Fills appear in Station sync; INR at insert on `zerodha-nse-bse-cash` | |
| 2 | Daily token / 06:00 expiry or forced logout | Next session after 06:00 IST **or** revoke session in Kite console | `403` / `TokenException` → **reconnect** prompt, **not** Kill | |
| 3 | Rate discipline | Normal sync day; watch agent logs | No sustained 429; obey 10 rps others / 1 rps quote (B6) | |
| 4 | Day-book cursor | Two sync runs same calendar day | Second poll does not duplicate/widen fills | |
| 5 | Kill | Kill switch L3 for slug `zerodha_kite` / broker `zerodha` | Blocks `api.kite.trade`, `kite.zerodha.com`, `ws.kite.trade`; other brokers OK | |
| 6 | DualNoBlend | Zerodha INR + Binance USD (+ Kotak INR if connected) | COM USD hero unchanged; each INR book its own strip | |

**STOP:** If redirect URL is rejected by Kite, or callback never reaches `127.0.0.1:9137`, stop and reopen ADR 0005 — do not invent a public redirect.

## Sign-off checklist (Planned → Enabled)

- [ ] Drill 1 green (fills + INR insert)
- [ ] Drill 2 green (session expiry → reconnect, not Kill)
- [ ] Drill 3 green (rate discipline)
- [ ] Drill 4 green (cursor / no duplicate widen)
- [ ] Drill 5 green (Kill hosts)
- [ ] Drill 6 green (DualNoBlend)
- [ ] No STOP fired, or STOP recorded with ADR revisit

**Signed:** —  
**Date:** — (IST)  
**Enabled flip:** after sign-off — `BrokerAvailability::Enabled` in `agent/src/ubi/catalog.rs` + `availability: .enabled` in `BrokerCatalog.swift` for `zerodha_kite` only; remove from `BrokerDogfoodProgram.connectWhilePlannedSlugs` (or leave until Enabled).

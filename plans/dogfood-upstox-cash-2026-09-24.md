# Dogfood — `upstox` cash (`upstox-nse-bse-cash`)

**Status:** OPEN — catalog **Planned** until signed  
**Lock:** `issues/compliance/locks/upstox-nse-bse-cash.md`  
**B6:** `issues/brokers/sheets/upstox.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0006-upstox-oauth-form-session.md` (`ACCEPTED`)

## Preconditions

- [ ] Upstox app redirect: `http://127.0.0.1:9137/api/daemon/broker/upstox/callback`
- [ ] Agent on `127.0.0.1:9137`
- [ ] Live Upstox account (not sandbox for G7)

## Drills

| # | Drill | Pass when |
|---|--------|-----------|
| 1 | OAuth → day trades poll | Fills in sync; INR at insert; products `I`/`D` verbatim |
| 2 | 3:30 AM token expiry | Reconnect, not Kill |
| 3 | Rate limits | Standard API bucket respected |
| 4 | Kill | `api.upstox.com`, `api-hft.upstox.com`, `assets.upstox.com` blocked at L3 |
| 5 | DualNoBlend | USD hero unchanged |

**Signed:** —  
**Enabled flip:** after sign-off (agent catalog + Swift).

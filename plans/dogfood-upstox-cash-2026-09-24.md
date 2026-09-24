# Dogfood — `upstox` cash (`upstox-nse-bse-cash`)

**Status:** DRAFT — **Tier I** path (steps 1–5; step 6 open)  
**Lock:** `issues/compliance/locks/upstox-nse-bse-cash.md`  
**B6:** `issues/brokers/sheets/upstox.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0006-upstox-oauth-form-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **Planned** + Connect; not step 6 |
| **II — Live** | Step **6** signed | Drills + sign-off → **Enabled** flip |

> Decision 8: **Enabled** = **Tier II only** (ADR 0019 §C).

---

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

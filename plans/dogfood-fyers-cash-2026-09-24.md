# Dogfood — `fyers` cash (`fyers-nse-bse-cash`)

**Status:** DRAFT — **Tier I** path (steps 1–5; step 6 open)  
**Lock:** `issues/compliance/locks/fyers-nse-bse-cash.md`  
**B6:** `issues/brokers/sheets/fyers.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0007-fyers-appidhash-session.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **Planned** + Connect; not step 6 |
| **II — Live** | Step **6** signed | Drills + sign-off → **Enabled** flip |

> Decision 8: **Enabled** = **Tier II only** (ADR 0019 §C).

---

## Preconditions

- [ ] myapi app redirect: `http://127.0.0.1:9137/api/daemon/broker/fyers/callback`
- [ ] Agent on `127.0.0.1:9137`
- [ ] Live Fyers account

## Drills

| # | Drill | Pass when |
|---|--------|-----------|
| 1 | OAuth → tradebook poll | Fills sync; INR; CNC/INTRADAY or 1/2 products |
| 2 | Daily token expiry | Reconnect, not Kill |
| 3 | Kill | `api-t1.fyers.in` blocked; no live order POST |
| 4 | DualNoBlend | USD strip unchanged |

**Signed:** —  
**Enabled flip:** after sign-off.

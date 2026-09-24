# Dogfood — `fyers` cash (`fyers-nse-bse-cash`)

**Status:** OPEN — catalog **Planned**  
**Lock:** `issues/compliance/locks/fyers-nse-bse-cash.md`  
**B6:** `issues/brokers/sheets/fyers.md` (`SIGNED` 2026-09-24)  
**ADR:** `docs/adr/0007-fyers-appidhash-session.md` (`ACCEPTED`)

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

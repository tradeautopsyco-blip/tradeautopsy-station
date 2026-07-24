# Station-wire Contract ACK — architecture-deepening v1 / A8 Phase IV

**Date:** 2026-07-24  
**Station tip:** `eba1ed2` + this contract bump  
**Console tip:** `bc1a93c9` (A8 Phase IV Bearer)

## ACK

Both repos treat `tradeautopsy-station/station-wire/v1.json` as the shared hop + auth contract.

| Concern | Locked |
|---------|--------|
| Loopback Notch→agent | Wire v1 HMAC + `x-daemon-secret`; `x-user-id` = machine hint only |
| Agent→Console brain | `Authorization: Bearer` Station Caller JWT (`aud=station`) only |
| Bar hops | Notch uses `/api/daemon/bar/*`; brain uses `/api/bar/v1/*` (see dual tables in v1.json) |
| Fill ingress | `POST /api/internal/bar/v1/broker-ingest/fill` with Bearer; required fields listed in wire file |
| Kill command_type | `fog_of_war`, `clear_fog` only (agent `parse_daemon_command_type`) |

## Console confirmation

Console A8 Phase III–IV already reject `DAEMON_SECRET` + `x-user-id` as who-am-I (`lib/daemon/verify-request.ts`). Bar/broker/capital/TAI gates use Bearer presence, not secret presence.

## Next desk slice

Architecture-deepening **Issue 2** — live Binance.US `BrokerAdapter` (Station agent).

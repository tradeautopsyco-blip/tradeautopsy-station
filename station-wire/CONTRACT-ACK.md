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

**SUPERSEDED (2026-07-24):** Architecture-deepening Issue 2 (live Binance.US) is not the next product slice.

**Next:** Brokers SYSTEM research → founder FINALIZE — `docs/research/BROKERS-SYSTEM.md` · issues pack `/Users/bishnu/issues/brokers/research/BROKERS-SYSTEM.md` (multi-broker · asset class · currency · calc · compliance · B1–B6 + D1–D5). Temporary Binance only.

## T1 — Station ↔ Enforcer bridge harden

**Date:** 2026-07-26
**Station tip:** `8cec50a` (`chore/commit-landed-work`) — `feat(bridge): T1 single Wire client + wire≠identity harden`

Locks the dual-hop auth already frozen above with zero protocol change:

| Concern | Harden |
|---------|--------|
| Duplicate loopback Wire v1 HMAC signers | Collapsed into one shared Swift type — `StationWireClient` (`notch/StationWireClient.swift`, part of the `Notch` package both Notch and StationApp already depend on). `notch/NotchViewModel.swift` and `station/StationApp/Broker/Services/LocalAgentBrokerRuntimeClient.swift` (plus `AgentSupervisor.swift`, `LocalDeviceLoginAgentClient.swift`, `LocalTodayAgentClient.swift`) all now call it; the old private `AgentWireSigner` / `makeWireSignature` duplicates are deleted. |
| `x-user-id` wire hint | Unchanged fixed UUID (`loopbackWireUserId` = `00000000-0000-4000-8000-000000000002`), now defined once on `StationWireClient` — every other copy in Swift references it instead of re-declaring the literal. |
| `STATION_ACCESS_TOKEN` test/bootstrap fallback | Tightened in `agent/src/lib.rs`: `UpstreamClient::brain_authorization_header` now only honors this env fallback when `UpstreamConfig::is_loopback_http_bootstrap()` is true (loopback http base + token set) — it can never substitute for the Keychain Bearer against a real (https) Console, even if the env var is stray-set in a production process. |
| Regression coverage | New `agent/tests/station_bridge_harden.rs` — asserts the capture-accept→outbox and `/instruments/ltp` upstream hops send `Authorization: Bearer …` and never forward `x-daemon-secret` / `x-user-id`. New `agent/src/lib.rs` unit tests lock the loopback-only gating. Existing `bar_forward.rs`, `phase7_screenshot_proxy.rs`, `station_wire_contract.rs`, `wire_phase2.rs` all still green. |

No hop paths, header names, or the wire v1 canonical string changed — see `v1.json` notes for the same clarification inline.

## T2.1 — B6 sheet gate (Wasm vs signed sheets)

**Date:** 2026-07-26  
**Station branch:** `chore/commit-landed-work` — B6 audit + COM cursor honesty + named next stub (commit when asked)

| Concern | Lock |
|---------|------|
| Sheet SoT | `/Users/bishnu/issues/brokers/sheets/` — audit in `B6-AUDIT-T2.md` |
| Enabled slugs | `binance_com` + `kotak_neo` only (SIGNED sheets); gate tests in `catalog` / `components` |
| Sheet fix | COM Wasm never combines `fromId` + `startTime` (B6 §4 / R1 `-1128`) |
| Named next | `zerodha_kite` — `sheets/zerodha_kite.md` Status=`RESEARCH`; catalog stays Planned; **no Wasm** |
| Don't | Invent community_reviewed · enable unsheeted slug · Binance.US as next |

**Next:** T2 step 2 — **B2** Keychain-only Start by connection id.

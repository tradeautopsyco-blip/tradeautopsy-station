# TRD — Architecture Deepening v1

**Status:** Ready for implementation  
**Derived from:** [PRD-architecture-deepening-v1.md](./PRD-architecture-deepening-v1.md)  
**Cross-repo plan:** [`plans/architecture-deepening-cross-repo-handoff.md`](../plans/architecture-deepening-cross-repo-handoff.md)  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Paired repo:** `FExEVIL/tradeautopsy`  
**Scope:** Live Binance.US adapter, validation on sync start, pause authority, loopback wire client, BarSurfacePhase bridge, broker session deepen, kill-switch module, bar proxy + station-wire, Bar session runtime, cross-repo contract alignment

---

## 1. Purpose

Translate Architecture Deepening v1 into implementable technical requirements: modules, seams, adapters, wire contracts, state ownership, sequencing, and verification. An engineer or agent should implement without re-deriving product rationale from the architecture review.

This TRD does **not** invent new product surfaces. It deepens and connects existing ones so deep implementations (poll loop, round-trip engine, wire middleware, Keychain, presentations) receive real inputs and honest UI.

---

## 2. Goals and Non-Goals

### 2.1 Goals

| ID | Goal |
|----|------|
| G1 | Live `binance_us` BrokerAdapter for non-test keys |
| G2 | Runtime ExchangeInfoSymbolCache population from Binance.US |
| G3 | Validation + withdrawDetected gate on `POST .../broker/sync/start` |
| G4 | Agent `user_paused` sole pause authority; Station reconciles |
| G5 | Single Swift LoopbackWireClient for all loopback callers |
| G6 | Single BarSurfacePhase authority → Notch + StationAppCoordinator |
| G7 | Broker session module absorbs shallow pass-throughs |
| G8 | KillSwitch module: fire/dismiss/ack parity + crypto DNS maps |
| G9 | BarProxy module + station-wire dual hop table |
| G10 | BarSessionRuntime replaces undecomposed Notch hub (phased last) |
| G11 | Cross-repo contract ACK + joint smoke |

### 2.2 Non-Goals

Same as PRD Out of Scope: marketplace, desk content, Stats, UDS auth, port change, order placement, inventing key-level withdraw APIs, Console credential UI, behavioral weight changes.

---

## 3. System Context

```
┌──────────────────────────── Station.app ────────────────────────────┐
│  StationAppCoordinator                                              │
│    ├─ BarSurfacePhase authority ──────────────┐                     │
│    ├─ Broker session module ── Keychain        │                     │
│    ├─ LoopbackWireClient (shared) ─────────────┼── all HTTP/SSE      │
│    ├─ Today projection ◄── agent /today + SSE  │                     │
│    └─ BarSessionRuntime (Notch) ───────────────┘                     │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ wire-v1 HMAC + x-daemon-secret
                                ▼
┌──────────────────── tradeautopsy-agent :9137 ───────────────────────┐
│  wire middleware                                                    │
│  BrokerSyncController ──► BinanceUSSpotBrokerAdapter (live)          │
│                       └──► CountingPollAdapter (TA_TEST_/TA_FAKE_)   │
│  broker_validation on sync/start                                    │
│  ExchangeInfoSymbolCache (booted)                                   │
│  TodayService + RoundTripEngine                                     │
│  KillSwitch module ── dns_block + audit + event_bus                 │
│  BarProxy ──► UpstreamClient ──► brain                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ x-daemon-secret (+ existing brain auth)
                                ▼
                     tradeautopsy.in (bar v1, daemon cmd/ack, fill ingest)
```

### 3.1 Invariants (must not violate)

1. Agent listen address remains `127.0.0.1:9137`.
2. Protected loopback routes: `x-daemon-secret` + wire-v1 HMAC.
3. No secrets in logs, SSE payloads, or UI models.
4. Keychain is the only broker credential store.
5. Withdraw detected → hard block (connect + sync/start).
6. Stop pauses broker sync only; kill switch / audit / SSE remain.
7. Notch/Station never call tradeautopsy.in directly.
8. Financial/sync code cites `docs/reference/crypto/binance-us/spot/` (or records NOT SPECIFIED).

---

## 4. Sequencing (hard)

Implement in this order. Later phases may depend on earlier interfaces existing.

| Phase | Deliverable | Primary tree |
|-------|-------------|--------------|
| 0 | Contract freeze + station-wire hop tables + ACK template | `station-wire/`, docs, brain ACK |
| 1 | BinanceUS spot client/adapter + exchangeInfo boot | `agent/` |
| 2 | Validation on start + pause authority | `agent/`, `station/` |
| 3a | LoopbackWireClient | `station/`, `notch/` |
| 3b | Brain tolerance (other repo) | tradeautopsy |
| 4 | BarSurfacePhase authority bridge | `notch/`, `station/` |
| 5 | Broker session deepen | `station/` |
| 6 | KillSwitch module + Today kill seam | `agent/`, `station/`, `notch/` |
| 7 | BarProxy + wire doc | `agent/`, `station-wire/` |
| 8 | BarSessionRuntime | `notch/` |
| 9 | Joint smoke + audit doc update | both repos |

**Gate:** Do not start Phase 8 until Phase 1 smoke shows non-empty fills for a real or recorded fixture path.

---

## 5. Module Requirements

Use architecture vocabulary: each subsection is a **module** with an **interface** (what callers must know) and **implementation** notes. **Adapters** are named only where two implementations justify a **seam**.

### 5.1 BinanceUSSpotClient + BinanceUSSpotBrokerAdapter (Phase 1)

**Seam:** existing `BrokerAdapter` trait.

**Adapters:**
- Live: `BinanceUSSpotBrokerAdapter`
- Test: `CountingPollAdapter` for keys prefixed `TA_TEST_` or `TA_FAKE_`

**Interface obligations (adapter):**
- `poll_fills` / balances / open orders per existing BrokerAdapter contract used by `broker_sync` poll loop
- Map Binance.US myTrades / account / openOrders JSON → internal fill and balance types
- Surface rate-limit and auth failures in forms the poll loop already classifies
- Never log API secret or signature query

**Factory (`build_runtime_adapter`):**
- `binance_us` + test/fake prefix → CountingPoll
- `binance_us` + otherwise → live US adapter
- `binance_com` unchanged (live vs test prefix)

**Reference:** `docs/reference/crypto/binance-us/spot/REST.md`, `MECHANICS.md`, `MANIFEST.md` blockers.

**Verification:**
- Unit: fixture JSON → fills
- Integration: start sync with mock HTTP or recorded client → `recent_trades` non-empty
- Manual: founder account smoke

### 5.2 ExchangeInfoSymbolCache boot (Phase 1)

**Interface:** TodayService / RoundTripEngine already accept a cache; boot must not leave production on `empty()` forever.

**Behavior:**
- Fetch Binance.US `GET /api/v3/exchangeInfo` (public) at agent start and/or on first US sync start
- Populate cache used by fee/pair lookup
- On fetch failure: keep heuristic fallback; expose degraded signal if Today already has a hook; do not crash agent

**Verification:** Test that boot path assigns non-empty cache from fixture; failure path leaves heuristic without panic.

### 5.3 Sync-start validation (Phase 2)

**Seam:** `BrokerSyncController::start` (or equivalent control interface).

**Behavior:**
- Before `build_runtime_adapter` for live keys, run `BrokerValidationAdapter` / Live Binance.US validation
- `withdrawDetected` → do not start poll loop; return error posture Station can map to Failed
- Test/fake key prefixes may skip live HTTP validation if existing test contract requires it — document which prefixes skip
- Do not persist secrets; credentials arrive in start payload over wire-protected loopback (existing pattern) then held only for adapter lifetime

**Verification:** Integration test: withdrawDetected fixture → start rejected; no poll task spawned.

### 5.4 Pause authority (Phase 2)

**Source of truth:** agent `user_paused` (or equivalent atomic in BrokerSyncController).

**Station behavior:**
- Stop → call agent stop → then update any local cache from agent status
- Status/snapshot load → prefer agent runtime status when agent online
- Must not force `.paused` solely from UserDefaults when agent reports syncing (or the inverse after stop)
- Offline: may show last-known metadata with Unavailable: Agent Offline per Backend Box taxonomy

**Verification:** Extend pause lifecycle tests: after stop, kill UI process state, reload from agent → still Paused; start clears only after agent start OK.

### 5.5 LoopbackWireClient (Phase 3a)

**Module:** shared Swift loopback wire client.

**Interface (caller must know):**
- Requires daemon secret for session
- Produces URLRequest with wire-v1 headers: proto version, request id, timestamp, nonce, signature, daemon secret, user id as required by agent
- Optional: daemon-source / caller integrity fields when agent enforces them
- Never returns or logs the secret or signature to UI models

**Adapters:**
- Live HMAC signer
- Test fake that injects headers or bypasses for in-process fakes (StationTests)

**Call sites to migrate:** AgentSupervisor health probes, LocalAgentBrokerRuntimeClient, LocalTodayAgentClient, Notch authorized requests, BarSettingsView signed requests.

**Delete:** duplicate `makeWireSignature` / inline HMAC copies after migration.

**Verification:** Agent rejects tampered canonical string from shared client tests; all former call sites compile against one type.

### 5.6 BarSurfacePhase authority (Phase 4)

**Module:** phase authority (may live adjacent to Bar session runtime initially, then move inside Phase 8).

**Interface:**
- Current `BarSurfacePhase`
- Observation/subscription for changes
- Derivation inputs: live-state projection, pending declaration, optimistic armed, debrief flags, open positions — same rules as today’s `recomputeBarSurfacePhase` semantics (preserve behavior; concentrate locality)

**Consumers:**
- BarNotchShell / phase routing
- StationAppCoordinator via NavigationPolicy (replace production wiring that constructs a stuck `.declaration` provider with no notch link)

**Preserve:** NavigationPolicy stickiness rules as a pure module.

**Verification:** Coordinator tests flip phase through authority and assert route policy outcomes; production launch path does not hardcode eternal declaration.

### 5.7 Broker session module (Phase 5)

**Module:** broker session (Station-side).

**Interface (small):**
- connect/validate/save (existing BrokerConnectController responsibilities may remain inside or behind this module)
- startSync / stopSync / retry
- deleteConnection
- statusSnapshot for UI presentation

**Absorb:** shallow pass-through types whose deletion test currently only moves lines (e.g. thin sync control wrappers).

**Keep seams:**
- KeychainBrokerCredentialStore adapter
- Agent runtime HTTP adapter (LoopbackWireClient underneath)
- Fake* adapters for tests

**Document:** agent single active sync adapter vs UI `brokerConnectionId` — interface must not pretend multi-active sync if agent still global.

**Verification:** Existing lifecycle tests pass against session interface; grep shows pass-through type removed or internalized.

### 5.8 KillSwitch module (Phase 6)

**Module:** agent-side KillSwitch.

**Interface:**
- `fire(level, broker_slug, …)`
- `dismiss(…)`
- `ack(…)` — after successful brain ack forward **or** as local completion path: must perform same local cleanup as dismiss for DNS/audit/SSE/fog

**Implementation owns:**
- dns_block enable/disable
- kill_switch_audit append
- event_bus KillSwitchState publish
- fog flag: either read by a documented consumer or deleted entirely

**DNS maps:**
- `binance_us` → Binance.US API/web hosts required for L3 efficacy (cite reference or ops list; do not use Kotak fallback)
- `binance_com` → appropriate Global hosts if kill supported
- Unknown slug: fail closed or explicit safe default — **never** silently Kotak for crypto product path

**Today / Station:**
- Circuit banner inputs from agent SSE or `/api/daemon` kill projection — not NotchViewModel stored properties
- Dismiss/resume actions call KillSwitch dismiss path

**Brain:** ack route path unchanged unless Phase 0 says otherwise; Station must not require brain to clear DNS.

**Verification:**
- Unit/integration: ack and dismiss both clear DNS in test env
- Hosts map test for `binance_us`
- fog_active either asserted by a reader test or absent from codebase

### 5.9 BarProxy module (Phase 7)

**Module:** agent bar proxy.

**Interface:** path table mapping daemon route → brain path + method; forward with existing 429 retry helper; map upstream errors to daemon JSON errors.

**Absorb:** near-copy handlers in bar API module.

**station-wire/v1.json:**
- Document `daemon_bar_*` paths (`/api/daemon/bar/...`)
- Document `brain_bar_*` paths (`/api/bar/v1/...`)
- Document auth difference: loopback wire-v1 vs agent→brain secret header
- Bump `compatibility.station_version` / `phase` when this program ships

**Verification:** Existing bar_forward tests target proxy; wire JSON contains both hop tables.

### 5.10 BarSessionRuntime (Phase 8)

**Module:** Bar session runtime (Notch).

**Interface (illustrative — keep small):**
- start/stop session observation (SSE + poll as today)
- declare / stop-me / clear actions
- current projection: live-state fields needed by UI
- `barSurfacePhase` (may own or wrap Phase 4 authority)
- kill overlay actions → KillSwitch / agent

**Implementation:** may compose former NotchViewModel logic; reducers become private.

**Adapters:**
- LoopbackWireClient
- BarDeclareHTTPExecuting (or successor)
- Optional clock/UUID for tests

**Verification:** Tests through runtime interface for declare → phase transition; no requirement to unit-test private sequencing of deleted public reducers separately if covered at runtime seam.

---

## 6. Wire and HTTP Contracts

### 6.1 Unchanged (must remain)

| Route | Role |
|-------|------|
| `POST /api/daemon/broker/sync/start` | Start sync (gains validation) |
| `POST /api/daemon/broker/sync/stop` | Pause sync |
| `POST /api/daemon/broker/sync/retry` | Retry degraded classes |
| `GET /api/daemon/broker/sync-state` | Runtime status |
| `GET /api/daemon/today` | Today payload |
| `GET /api/daemon/events/stream` | SSE |
| `POST /api/daemon/kill-switch` | Fire |
| `POST /api/daemon/dismiss-kill-switch` | Dismiss |
| `POST /api/daemon/kill-switch/ack` | Ack proxy + local cleanup parity |
| `POST /api/daemon/bar/*` | Daemon bar proxy |
| Brain `/api/bar/v1/*` | Hosted bar |

### 6.2 Behavioral contract changes

| Change | Detail |
|--------|--------|
| sync/start | May return validation failure without starting; Station maps to Failed |
| sync-state | Paused reflects agent user_paused |
| kill ack | Local DNS/audit/SSE cleared on success (parity with dismiss) |
| SSE kill | Today and Notch both consume |
| station-wire | Dual hop tables + auth notes |

### 6.3 Agent → brain

- Headers: existing `x-daemon-secret`, `x-user-id`, `x-request-id` (no requirement to add loopback HMAC upstream)
- Fill ingress: preserve redaction boundary; include connection identity fields Backend Box already requires

### 6.4 Cross-repo ACK artifact

Minimum fields in both repos’ ACK note:
- station-wire version hash or semver
- bar hop table copy
- kill command_type list
- fill ingest required fields
- “crypto slug `binance_us` supported” confirmation

---

## 7. State Ownership

| State | Owner |
|-------|--------|
| Broker API secrets | Keychain |
| Runtime sync / pause / data-class freshness | Agent |
| Today round-trips / hero | Agent TodayService |
| BarSurfacePhase | Phase authority / BarSessionRuntime |
| Kill switch DNS + audit | Agent KillSwitch module |
| Kill switch UX projection | SSE / agent status → Station + Notch |
| Uploaded behavioral / bar history | Brain |
| UI presentation models | Station/Notch presenters only |

---

## 8. Error and Degraded Taxonomy

Align with Backend Box + Today matrices; additions:

| Condition | User-visible |
|-----------|--------------|
| sync/start withdrawDetected | Failed — withdraw permission; do not sync |
| exchangeInfo fetch fail | Today may degrade fee precision; sync can continue |
| empty live polls with healthy HTTP | Do not treat as Syncing-complete if completeness flags say otherwise — existing data-class rules apply; adapter must not report success with silently unimplemented endpoints |
| kill ack upstream fail | Do not clear local DNS; surface error |
| wire signature drift | 401 from agent; client fix |

---

## 9. Repository Touch Map (durable, not file laundry lists)

| Tree | Phases |
|------|--------|
| `agent/src/` broker + validation + sync control + exchange info + kill + api/bar | 1, 2, 6, 7 |
| `agent/tests/` | 1, 2, 6, 7 |
| `station/StationApp/` broker, today, navigation, supervisor, wire client | 2, 3a, 4, 5, 6 |
| `station/StationTests/` | 2–6, 8 as needed |
| `notch/` wire usage, phase, runtime, settings | 3a, 4, 6, 8 |
| `station-wire/v1.json` | 0, 7 |
| `docs/reference/crypto/binance-us/` | cite only; update if live verify closes MANIFEST blockers |
| `plans/` + `.github/` smoke checklist | 0, 9 |
| tradeautopsy (brain) | 0, 3b, 6 ACK, 9 |

Avoid drive-by refactors outside the active phase’s module.

---

## 10. Testing Plan

### 10.1 Automated (CI)

- Adapter fixture tests (US)
- CountingPoll still selected for TA_* prefixes
- exchangeInfo populate path
- sync/start withdrawDetected
- pause reconcile
- LoopbackWireClient vs agent reject/accept
- Phase authority → NavigationPolicy
- Broker session lifecycle fakes
- Kill ack/dismiss parity + binance_us hosts
- Bar proxy forward
- BarSessionRuntime declare→phase (Phase 8)
- Secret redaction / release gates extended if patterns exist

### 10.2 Manual joint smoke

See handoff Phase 9 checklist; publish as `.github/ARCHITECTURE_DEEPENING_SMOKE_CHECKLIST.md` when Phase 9 starts (create then, not before).

Minimum:
1. Connect Binance.US read-only key
2. Confirm withdraw block with a withdraw-enabled fake/fixture
3. Observe fills + Today hero
4. Pulse matches hero
5. Notch declare → phase follows in shell
6. Stop → pause persists across Station relaunch
7. Kill L3 → Binance.US hosts blocked in test/observability path
8. Ack and dismiss both restore access consistently
9. Brain shows expected bar/fill activity without raw secrets

---

## 11. Rollout and Compatibility

- Ship behind normal Station releases; no feature flag required if CountingPoll remains for tests and live path is additive.
- Bump `station-wire` compatibility.phase to a named deepening phase (e.g. `architecture-deepening-v1`) when Phases 0+7 land.
- Brain min version: only bump if 3b requires new fields; otherwise keep `brain_min_version` and document ACK.

---

## 12. Issue Breakdown Guide (for `/to-issues`)

Create issues in dependency order. Suggested titles (map to PRD story groups):

| # | Title | PRD groups | Blocked by |
|---|-------|------------|------------|
| 1 | Cross-repo contract freeze + station-wire hop tables | A | — |
| 2 | Live Binance.US spot client + BrokerAdapter | B | 1 |
| 3 | exchangeInfo boot into Today/round-trip | C | 2 |
| 4 | Validation gate on sync/start | D | 2 |
| 5 | Pause single source of truth | E | 4 |
| 6 | Swift LoopbackWireClient unification | F | — (can parallel after 1) |
| 7 | Brain: US fill/bar/kill tolerance | G | 1 |
| 8 | BarSurfacePhase authority + shell bridge | H | 6 optional |
| 9 | Broker session module deepen | I | 5, 6 |
| 10 | KillSwitch module + crypto DNS + Today seam | J | 1, 7 |
| 11 | BarProxy collapse + wire doc | K | 1 |
| 12 | BarSessionRuntime deepen | L | 2, 5, 8, 10 |
| 13 | Joint smoke + audit doc update | M | 2–12 |

Each issue body must include: parent PRD link, story numbers, acceptance checkboxes, blocked-by, primary repo, and group N invariants.

---

## 13. Open Questions (do not block Phase 1)

1. Exact Binance.US host list for L3 — confirm against current docs.binance.us / ops needs during Phase 6.
2. MANIFEST Blocker 1 (key-level withdraw) — remains account-level until primary source exists.
3. Whether exchangeInfo refresh is periodic or boot-only — choose simplest that keeps cache non-empty; document in issue 3.
4. Brain fill ingest rate limits under 90-day backfill — monitor in Phase 1/3b; throttle in Station if brain requires.

---

## 14. Success Definition

Technical program is done when program-level PRD acceptance criteria are checked, CI is green without live secrets, joint smoke is signed, and `AUDIT_CURRENT_VS_PLANNED.md` no longer lists Binance.US sync as CountingPoll-only for production keys.

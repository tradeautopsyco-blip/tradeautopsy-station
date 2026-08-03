# PRD — Architecture Deepening v1: Live Binance.US Loop + Coherent Station Seams

**Status:** Ready for agent / ready for `/to-issues`  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Paired repo:** `FExEVIL/tradeautopsy` (brain / Console)  
**Parent context:** Architecture review 2026-07-09; `AUDIT_CURRENT_VS_PLANNED.md`; Backend Box v1 + Today v1 already shipped as control plane / session mirror  
**Cross-repo plan:** [`plans/architecture-deepening-cross-repo-handoff.md`](../plans/architecture-deepening-cross-repo-handoff.md)  
**TRD:** [`TRD-architecture-deepening-v1.md`](./TRD-architecture-deepening-v1.md)  
**Grill / review:** Architecture deepening review (improve-codebase-architecture), 2026-07-09  
**Scope:** Close all Strong / Worth exploring / Speculative friction from the architecture review so Station + brain operate without stubs, split pause truth, triplicated wire signing, dead phase provider, kill-switch divergence, or empty Binance.US polls

---

## Problem Statement

TradeAutopsy Station already has a Phase 0 shell, Backend Box control plane, Today engine, Notch HUD, wire-v1 auth, and kill switch — but several seams are shallow, duplicated, or disconnected. From the trader’s and operator’s perspective:

1. **Binance.US sync looks connected but produces no fills.** The AGENTS.md test broker resolves to a counting stub adapter. Today, pulse strip, and behavioral upload paths run on empty polls while Binance.com has a real adapter — the product’s north-star broker is theater.
2. **Withdraw hard block is only enforced on the Swift connect path.** The agent accepts raw keys on sync start without re-validation, so a bypass or stale Keychain path can start sync without the invariant.
3. **Stop / Paused can lie.** Station stores `syncPaused` in metadata while the agent stores `user_paused`. After restart or overlay logic, UI can show Paused without asking the agent — or the reverse.
4. **Shell navigation ignores live trading phase.** Notch computes `BarSurfacePhase` from live state; Station shell still reads a provider stuck at `.declaration`, so Desk/Session routing does not follow the real session.
5. **Wire signing is copied three ways in Swift.** Supervisor, Notch, and Settings can drift from `agent` wire verification; auth bugs appear as intermittent 401s that unit tests on one copy never catch.
6. **Kill switch is not one module.** Fire, dismiss, and ack take different local side effects; unknown broker slugs (including `binance_us`) default DNS block hosts to Kotak; Today’s circuit banner reads Notch internals instead of agent state.
7. **Bar declare failures are hard to diagnose.** Notch → agent daemon bar routes → brain bar routes are copy-paste proxies with a vocabulary mismatch in `station-wire/v1.json`.
8. **Notch session orchestration is undecomposed.** Understanding one live session requires reading a multi-thousand-line view model; small reducers exist for testability but real bugs live in call sequencing — and there are no tests on that orchestration.

Until these are closed, Station and the TradeAutopsy brain cannot merge into a frictionless loop: brain receives empty or inconsistent traffic, Station UI overclaims health, and safety systems are not trustworthy for the crypto test user.

---

## Solution

Ship **Architecture Deepening v1**: make the existing deep modules (wire middleware, broker poll loop, round-trip engine, Keychain, BrokerConnectController, presentation builders) receive real data and sit behind coherent seams.

North-star acceptance statement:

> A trader connects Binance.US once, Station validates and hard-blocks withdraw, the agent runs a live spot adapter with exchangeInfo-backed fee/pair metadata, Today and pulse show real session P&L, Notch declare and shell phase stay in lockstep, Stop pauses sync without lying after restart, kill switch targets Binance.US hosts and clears consistently on dismiss/ack, and every loopback client shares one wire signer — while brain accepts bar and fill traffic under the frozen cross-repo contract.

Product flow after this program:

1. Trader connects Binance.US (existing Brokers flow).
2. Validation + withdraw block run at connect **and** again at agent sync start.
3. Live adapter polls fills / balances / open orders; exchangeInfo boots at agent start.
4. Today + pulse update from real fills; Notch live-state and shell phase agree.
5. Trader can Stop (agent pause truth) and Start (fresh Keychain read) without agent restart.
6. Kill switch fire/dismiss/ack keep DNS, audit, and SSE aligned for `binance_us`.
7. Engineers maintain one Swift wire client and one bar-proxy path table documented in station-wire.

Work is sequenced per the cross-repo handoff plan: **live adapter before Notch deepen**.

---

## Goals

| ID | Goal |
|----|------|
| G1 | Live Binance.US `BrokerAdapter` for real keys; CountingPoll only for test/fake key prefixes |
| G2 | Boot and use Binance.US `exchangeInfo` for pair/fee lookup in Today / round-trip paths |
| G3 | Withdraw hard block on agent `sync/start` as well as Swift connect |
| G4 | Single pause source of truth in the agent; Station UI reflects agent |
| G5 | One Swift loopback wire client shared by all Station/Notch callers |
| G6 | Single `BarSurfacePhase` authority driving Notch chrome and Station shell navigation |
| G7 | Broker session module collapses shallow pass-throughs without losing Keychain / fake test seams |
| G8 | Kill-switch module with fire/dismiss/ack parity; Binance.US DNS host map; Today reads agent kill state |
| G9 | Bar brain-proxy deepened; station-wire lists daemon and brain hop tables |
| G10 | Bar session runtime deepened from NotchViewModel; tests cross that interface |
| G11 | Cross-repo contract ACK with tradeautopsy so merge is frictionless |
| G12 | Joint Binance.US smoke checklist signed off |

---

## Non-Goals / Out of Scope

- Broker marketplace / App-Store catalog (`plans/broker-marketplace-handoff.md`)
- Desk route content beyond phase-driven navigation (Pre-trade, Live, Journal screens)
- Stats screen (Today Slice 2)
- UDS / 0600 token-file auth
- Changing agent port away from 9137
- Order placement / withdraw / deposit sync
- Replacing account-level withdraw detection with invented key-level APIs (MANIFEST Blocker 1 stays open until primary source confirms)
- Console/web credential UI for Binance.US
- Rewriting brain behavioral engine weights
- Multi-window Station; customizable hotkeys

---

## Actors

| Actor | Needs |
|-------|--------|
| Crypto trader (Binance.US) | Honest sync, real Today P&L, safe credentials, working Notch session |
| Security-conscious trader | Withdraw hard block, Keychain-only secrets, no secret leakage |
| Notch user in a live session | Phase, declare, kill overlay, live-state that match shell |
| Station engineer / agent | Deep modules, one wire client, testable seams |
| Brain / daemon engineer | Stable contracts, crypto-tolerant fill/bar/kill payloads |
| QA / release owner | Fake adapters in CI; real Binance.US smoke; cross-repo checklist |
| Founder | Frictionless Station↔brain merge; AGENTS.md invariants held |

---

## User Stories

Stories are grouped for `/to-issues` / issue scaling. **Each group maps to one or more tracer-bullet issues.** Do not merge groups across phase boundaries in the handoff plan without updating the plan.

### A. Cross-repo contract (Phase 0)

1. As a Station engineer, I want a frozen hop table for Notch → agent daemon bar routes → brain bar routes, so that path drift cannot break declare/live-state.
2. As a brain engineer, I want the same hop table in `station-wire` and brain docs, so that both repos implement one contract.
3. As a Station engineer, I want fill-ingress field requirements frozen (connection id, slug, asset class, environment, completeness, redaction), so that live US fills are not rejected after adapter ship.
4. As a brain engineer, I want kill-switch command_type and ack semantics frozen, so that Station kill-switch deepen does not desync fog/command poll.
5. As a release owner, I want a written Contract ACK in both repos, so that merge order is explicit.
6. As an engineer, I want agent→brain auth documented as secret-header (not loopback HMAC), so that nobody “fixes” upstream by adding wire headers the brain ignores.
7. As an engineer, I want compatibility.phase / station_version bump rules defined for this program, so that mismatched builds fail loudly.

### B. Live Binance.US adapter (Phase 1) — Strong

8. As a crypto trader, I want Binance.US sync to fetch real fills, so that Today and journal reflect my trading.
9. As a crypto trader, I want balances and open orders synced as current snapshots, so that exposure is visible.
10. As a trader, I want first connect to perform the existing 90-day backfill against real myTrades (within reference-doc limits), so that recent history is useful.
11. As a trader, I want ongoing sync incremental from last seen fill, so that Station does not re-pull everything.
12. As an engineer, I want `binance_us` real keys to use a live spot adapter, so that CountingPoll is not production behavior.
13. As QA, I want `TA_TEST_*` / `TA_FAKE_*` keys to keep CountingPoll, so that CI stays deterministic without secrets.
14. As an engineer, I want the US client/adapter to mirror the proven Binance.com client→adapter layering, so that two adapters justify the BrokerAdapter seam.
15. As an engineer, I want all US REST usage cited from `docs/reference/crypto/binance-us/spot/`, so that no mechanic is invented from memory.
16. As an engineer, I want gaps marked NOT SPECIFIED IN SOURCE to remain unimplemented or explicitly degraded, so that we do not guess retention or rate limits.
17. As a trader, I want Syncing to mean data classes are actually current with non-empty capability when the account has trades, so that “connected” is not empty-poll success.
18. As an intelligence consumer, I want live fills eligible for existing bar fill ingress / redaction paths, so that brain receives real activity metadata when enabled.
19. As QA, I want integration tests with a fake HTTP Binance.US surface, so that adapter mapping is proven without live keys.
20. As a release owner, I want a manual smoke step: connect founder Binance.US → see fills in recent trades, so that stub removal is proven.

### C. exchangeInfo boot (Phase 1)

21. As a trader, I want Today P&L fee/pair handling to use exchangeInfo when available, so that non-heuristic fee assets are correct.
22. As an engineer, I want ExchangeInfoSymbolCache populated at agent boot (or first sync) from Binance.US exchangeInfo, so that round-trip engine is not permanently on empty cache.
23. As an engineer, I want empty-cache heuristic fallback to remain only as degraded behavior, so that missing boot is visible in tests/metrics.
24. As QA, I want a test that runtime population path is invoked (fixture or mock HTTP), so that empty() at boot cannot silently return.

### D. Validation on sync start (Phase 2) — Worth exploring → required

25. As a security-conscious trader, I want agent sync/start to refuse withdrawDetected credentials, so that the hard block cannot be bypassed by calling the agent directly.
26. As a trader, I want sync/start validation failures to return a clear failed posture without starting the poll loop, so that I know why sync did not start.
27. As an engineer, I want the existing broker_validation module invoked on the sync-start path, so that library code is no longer orphaned.
28. As a release owner, I want a test that withdrawDetected on sync/start never builds a live adapter, so that the invariant is CI-gated.
29. As an engineer, I want Swift connect validation to remain, so that bad keys never hit Keychain; agent check is defense in depth.
30. As an engineer, I want permission taxonomy (readOnlyConfirmed, tradeEnabled, unverifiable, withdrawDetected) shared in meaning across Swift and Rust, so that UI and agent agree.

### E. Pause single source of truth (Phase 2)

31. As a trader, I want Paused to mean the agent has user_paused set, so that Stop is honest.
32. As a trader, I want app restart after Stop to remain Paused until I Start, so that Station does not auto-resume against my intent.
33. As an engineer, I want Station metadata not to override agent status when overlaying Paused, so that dual stores cannot disagree.
34. As a trader, I want Start to clear pause only via agent start success, so that UI cannot show Syncing while agent is still paused.
35. As QA, I want lifecycle tests that restart Station UI state and re-fetch agent status, so that pause lies are caught.
36. As an engineer, I want Stop to continue not restarting the agent, so that kill switch, audit, and SSE stay up (AGENTS.md).

### F. Loopback wire client (Phase 3a) — Strong

37. As an engineer, I want one Swift loopback wire client, so that canonical string, timestamp, nonce, ULID, and headers cannot drift.
38. As an engineer, I want AgentSupervisor, broker runtime client, Today client, Notch, and Settings to use that client, so that no third HMAC copy remains.
39. As an engineer, I want the client interface to match agent wire verification behavior, so that positive and negative tests share one seam.
40. As QA, I want a test where a deliberately wrong canonical string is rejected by the agent, so that the shared client is the one under test.
41. As an engineer, I want consistent handling of x-daemon-source / caller integrity fields where required, so that Notch and Station do not differ accidentally.
42. As an engineer, I want unprotected routes (if any remain) explicitly documented, so that “sometimes unsigned” is not accidental.
43. As a security engineer, I want secrets never logged by the wire client, so that AGENTS.md no-secrets-in-logs holds.

### G. Brain tolerance for live US traffic (Phase 3b — brain repo)

44. As a brain engineer, I want bar v1 routes to accept Station-forwarded declare/live-state for crypto sessions, so that Notch sessions work with Binance.US.
45. As a brain engineer, I want fill ingest to accept Binance.US symbols and fee assets Station sends, so that ingress does not drop live fills.
46. As a brain engineer, I want daemon command / kill-switch paths to accept broker slug binance_us, so that fog/kill is not India-broker-only.
47. As a release owner, I want brain issues for 3b linked from Station Phase 0 ACK, so that cross-repo tracking is visible.
48. As a Station engineer, I want brain-breaking payload changes to require wire revision bump, so that mismatched Station builds fail closed.

### H. BarSurfacePhase single authority (Phase 4) — Strong

49. As a trader, I want Station shell session routes to follow the same BarSurfacePhase as the Notch, so that opening the window matches my live session.
50. As a trader, I want phase to move declaration → armed/livePlan → debrief from live-state and local rules, so that chrome and shell stay aligned.
51. As an engineer, I want one phase authority module, so that DefaultBarSurfacePhaseProvider cannot stay stuck at declaration in production.
52. As an engineer, I want NavigationPolicy stickiness rules preserved, so that deepening phase does not rewrite navigation semantics.
53. As QA, I want coordinator tests driven by live phase changes (not only test setters on a dead provider), so that the bridge is real.
54. As an engineer, I want BarNotchPhaseRouting and shell mapping to consume the same phase enum, so that duplicate mapping cannot diverge.

### I. Broker session module (Phase 5) — Strong

55. As an engineer, I want connect → validate → Keychain → start/stop → status behind one broker session interface, so that callers do not traverse seven shallow hops.
56. As an engineer, I want AgentBrokerSyncControl-style pass-throughs deleted or absorbed, so that the deletion test concentrates complexity.
57. As QA, I want FakeBrokerControlClient / FakeBrokerAgentRuntimeClient to remain adapters at the session seam, so that lifecycle tests stay fast.
58. As an engineer, I want Keychain to remain the only credential store, so that deepening does not introduce plaintext prefs for secrets.
59. As an engineer, I want the single active adapter / global sync slot behavior documented in the session interface, so that per-connection identity in UI is not a false promise.
60. As a trader, I want Brokers UI behavior unchanged except honesty of status/pause, so that deepening is not a UX redesign.

### J. Kill-switch module (Phase 6) — Worth exploring

61. As a trader, I want kill-switch fire to apply DNS, audit, and SSE together, so that safety is not partial.
62. As a trader, I want dismiss and ack to clear the same local side effects, so that brain ack cannot leave DNS blocking while SSE says inactive.
63. As a crypto trader, I want L3 for binance_us to block Binance.US hosts, so that Kotak defaults never apply to my broker.
64. As an engineer, I want fog_active either consumed by a real reader or removed, so that dead state cannot imply safety.
65. As an engineer, I want one kill-switch module interface (fire/dismiss/ack), so that HTTP handlers are not the domain layer.
66. As a trader, I want Today circuit-breaker banner to reflect agent kill-switch state, so that I am not coupled to NotchViewModel fields.
67. As a trader, I want resume/dismiss from Today to call the same dismiss path as Notch overlay, so that one seam clears state.
68. As a brain engineer, I want Station ack handling to remain compatible with hosted ack, so that cross-repo fog flows keep working.
69. As QA, I want tests that ack clears DNS in test env the same way dismiss does, so that split-brain cannot ship.
70. As a release owner, I want smoke proof that Binance.US L3 touches US hosts, so that crypto kill switch is trustworthy.

### K. Bar brain-proxy (Phase 7) — Speculative → included

71. As an engineer, I want one bar-proxy module with a path table, so that nine near-copy handlers are not the maintenance surface.
72. As an engineer, I want 429 retry behavior preserved in that module, so that deepening does not regress capture/bar resilience.
73. As an engineer, I want station-wire to list both `/api/daemon/bar/*` and `/api/bar/v1/*`, so that vocabulary matches runtime.
74. As QA, I want existing bar_forward tests to target the proxy module, so that behavior stays covered.
75. As a brain engineer, I want path changes to require dual-repo ACK, so that Notch cannot call routes brain deleted.

### L. Bar session runtime (Phase 8) — Strong

76. As an engineer, I want Bar session runtime to own loopback client use, SSE/daemon FSM, live-state projection, phase derivation, declare, and kill overlay state, so that NotchViewModel is not an undecomposed hub.
77. As an engineer, I want pure reducers colocated inside that module’s implementation, so that they are not shallow public pass-throughs.
78. As QA, I want tests that exercise declare → phase armed → livePlan through the runtime interface, so that sequencing bugs are caught where they live.
79. As an engineer, I want BarDeclareHTTPExecuting (or successor) to remain an injectable adapter at the HTTP seam, so that declare tests do not need a live agent.
80. As a trader, I want Notch UX behavior preserved while internals deepen, so that deepening is not a visual redesign.
81. As an engineer, I want Today to depend on agent/SSE kill and sync signals, not private Notch fields, so that desk and notch share locality.

### M. Joint smoke and release (Phase 9)

82. As QA, I want a written joint smoke checklist covering Station + brain, so that “done” is evidence-based.
83. As a release owner, I want CI green on Station without real Binance.US credentials, so that deepening stays safe in CI.
84. As a release owner, I want no secrets in logs, SSE, or UI models proven by release gates, so that AGENTS.md holds.
85. As a founder, I want founder-account Binance.US smoke after Phases 1–8, so that the north star is real.
86. As an engineer, I want AUDIT_CURRENT_VS_PLANNED updated or superseded when this program ships, so that plan docs stop claiming a stub adapter.

### N. Invariants and safety (cross-cutting — every issue inherits)

87. As a security-conscious trader, I want Keychain-only broker secrets, so that prefs/files never hold API keys.
88. As a security-conscious trader, I want no API key, secret, HMAC signature, or auth header in logs, SSE, or UI models.
89. As a trader, I want Stop to pause broker sync only — never disable kill switch, audit, or SSE.
90. As an engineer, I want agent port 9137 unchanged.
91. As an engineer, I want all agent auth on protected routes to keep x-daemon-secret + wire-v1 HMAC.
92. As a trader, I want withdraw permission to remain a hard block with no save/start when detected.
93. As an engineer, I want reference docs cited before new financial/sync code.
94. As an engineer, I want existing deep modules (wire.rs, broker_sync poll loop, round_trip_engine, kill_switch_audit, outbox, Keychain store, BrokerConnectController, presentation builders, NavigationPolicy) preserved and fed — not rewritten for sport.

---

## Implementation Decisions

### Program shape

- This PRD is the product contract for Architecture Deepening v1; the TRD is the technical contract; the handoff plan is the cross-repo sequencing contract.
- Implementation proceeds as tracer-bullet phases in the handoff plan — not a big-bang rewrite.
- Prefer deepening existing seams over inventing new ones. Ideal: feed BrokerAdapter, wire middleware, and phase/navigation seams that already exist.

### Architecture vocabulary (for issues and reviews)

Use exactly: **module**, **interface**, **implementation**, **depth**, **seam**, **adapter**, **leverage**, **locality**.  
Do not substitute “service,” “API” (for interface), or “boundary” (for seam) in architecture discussion inside issues.

### Cross-repo

- Station owns local broker execution, loopback wire, Today engine, Notch runtime, kill-switch DNS/audit.
- Brain owns hosted bar v1, daemon command/ack, fill ingest acceptance, behavioral storage.
- `station-wire/v1.json` is the shared contract file living in Station; brain references the same revision in ACK.
- Phase 0 ACK is a hard gate before claiming Phase 3b/6/7 done across repos.

### Binance.US adapter

- Add live US spot client + adapter parallel to Binance.com.
- `build_runtime_adapter`: real keys → live US adapter; test/fake prefixes → CountingPoll.
- Cite `docs/reference/crypto/binance-us/spot/REST.md` and MECHANICS.md.
- Do not invent key-level withdraw endpoints; use documented account-level signals consistent with Backend Box.
- Rate limit / retention uncertainties stay degraded or NOT SPECIFIED — no guessed constants.

### exchangeInfo

- Populate cache from Binance.US exchangeInfo for the syncing environment.
- Round-trip / Today continue to support heuristic fallback only as degraded.

### Validation

- Invoke Rust broker_validation (or shared posture classifier) on sync/start before adapter build.
- withdrawDetected → do not start; return failed posture to Station.
- Swift connect validation remains mandatory before Keychain write.

### Pause truth

- Agent `user_paused` is authoritative.
- Station may cache for offline display but must reconcile from agent when online.
- Remove or neutralize overlays that force Paused solely from local metadata when agent disagrees.

### Loopback wire

- One Swift module: signing + header attachment + request id/nonce/timestamp.
- All Station/Notch loopback callers depend on it.
- Two adapters justify the seam: live signer vs test fake/passthrough used in StationTests.

### BarSurfacePhase

- One authority derives phase from live-state + local session rules.
- Coordinator/NavigationPolicy consume that authority.
- Delete or repurpose dead always-declaration provider behavior in production wiring.
- Preserve NavigationPolicy stickiness semantics.

### Broker session

- Deepen by absorbing shallow pass-throughs into one session module.
- Keep Keychain adapter and agent HTTP adapter as real seams (fakes in tests).
- Document single global sync slot vs UI connection identity.

### Kill switch

- One module: fire, dismiss, ack — identical local cleanup obligations for dismiss and successful ack.
- Map `binance_us` (and `binance_com` if in scope) to correct host lists; never fall through to Kotak for crypto slugs.
- Resolve fog_active (wire to reader or delete).
- Today reads kill state from agent/SSE (or a small shared projection), not Notch private properties.

### Bar proxy

- Collapse daemon bar handlers into one proxy module + path table.
- Update station-wire with both hop layers.
- Preserve 429 retry helper behavior.

### Bar session runtime

- Deepen after live data and phase/kill seams exist.
- External interface small: session projection, declare, stop-me, phase, kill overlay actions.
- Reducers become internal implementation details.
- Tests at runtime interface, not only reducer unit tests.

### Testing philosophy

- Test external behavior through module interfaces (the interface is the test surface).
- Prefer highest existing seam (BrokerAdapter, wire middleware, session fakes, presentation builders).
- CI never requires real Binance.US credentials.
- Manual smoke uses founder Binance.US read-only (no withdraw) account.

### Issue scaling rules (for `/to-issues`)

1. One GitHub/markdown issue ≈ one handoff phase workstream (A–M groups above), further split only if a slice is not demoable alone.
2. Each issue body must list: parent PRD, user story numbers covered, acceptance criteria checkboxes, blocked-by phase, primary repo.
3. Prefactor issues (wire client, pause truth) may precede feature issues that depend on them.
4. Do not create a single “rewrite NotchViewModel” issue that includes adapter work — violate sequencing.
5. Brain issues for groups A/G/J (ACK parts) live on tradeautopsy tracker; Station issues link them.
6. Every issue inherits group N invariants as definition-of-done bullets.
7. Label vocabulary: prefer existing org labels (`ready-for-agent` if present) plus repo hints from the handoff ownership matrix.

---

## Testing Decisions

### What makes a good test

- Assert observable behavior at a module interface (HTTP status, sync state taxonomy, phase enum, DNS test-env side effect, Today payload fields).
- Do not assert private call order inside NotchViewModel/runtime except via the runtime interface.
- Do not require live Binance.US in CI.
- Prefer fakes/adapters already used in StationTests and agent tests/common wire harness.

### Modules under test (by phase)

| Phase | Test focus |
|-------|------------|
| 0 | Contract doc presence / wire schema fields (lightweight) |
| 1 | Adapter mapping from fixture JSON → BrokerFill; CountingPoll still for TA_*; exchangeInfo populate path |
| 2 | sync/start withdrawDetected; pause authority after stop/restart reconcile |
| 3a | Shared wire client positive/negative against agent |
| 3b | Brain-side (other repo) |
| 4 | Coordinator phase bridge; NavigationPolicy with live phase stream |
| 5 | Broker session lifecycle via fakes; pass-through modules gone |
| 6 | fire/dismiss/ack parity; binance_us hosts; Today banner inputs |
| 7 | bar_forward through proxy module; wire path table |
| 8 | Runtime declare→phase tests; Today independent of Notch privates |
| 9 | Manual checklist + release gates |

### Prior art

- `agent/tests/wire_phase2.rs`, `tests/common` wire harness
- `agent/tests` broker sync with CountingPoll / TA_TEST_
- `BinanceUSCredentialValidationTests`, `BrokerPauseLifecycleTests`, `BrokerDeleteLifecycleTests`
- `TodayScreenPresentationTests`, `BrokerScreenPresentationTests`
- `StationAppCoordinatorTests`, `NavigationPolicyTests`
- `bar_forward.rs`, kill-switch DNS tests (extend to crypto hosts)
- Backend Box / Today release gate patterns

---

## Acceptance Criteria (program-level)

- [ ] Live Binance.US adapter returns fills/balances/orders for real keys; CountingPoll only for test prefixes
- [ ] ExchangeInfoSymbolCache populated in runtime path; Today not permanently heuristic-only
- [ ] sync/start enforces withdrawDetected hard block
- [ ] Paused/Syncing UI matches agent after Stop and after app relaunch
- [ ] Single Swift loopback wire client; no third HMAC implementation in Notch/Settings
- [ ] Shell session routes follow live BarSurfacePhase
- [ ] Broker session shallow pass-throughs absorbed; fakes still work
- [ ] Kill-switch fire/dismiss/ack parity; binance_us DNS hosts correct; fog_active resolved
- [ ] Today circuit banner uses agent/SSE kill state
- [ ] Bar proxy + station-wire hop tables aligned
- [ ] Bar session runtime testable at interface; sequencing covered
- [ ] Cross-repo Contract ACK recorded
- [ ] Joint smoke checklist completed on founder Binance.US
- [ ] CI green without real broker secrets
- [ ] AGENTS.md invariants held (port, wire, Keychain, withdraw, kill switch always on, no secrets in observables)

---

## Further Notes

- **Audit debt:** `AUDIT_CURRENT_VS_PLANNED.md` already documents the CountingPoll gap — this PRD is the remediation program.
- **MANIFEST blockers:** Account-level withdraw detection remains until primary source confirms key-level; do not block this program on Blocker 1 invention.
- **Marketplace:** Explicitly parked; do not pull Console registry work into these issues.
- **Ordering reminder:** Feeding deep financial modules (Phase 1–2) before deepening Notch (Phase 8) is mandatory.
- **Share path:** Give brain engineers the handoff plan first; give implementers PRD + TRD; scale issues from PRD story groups A–M.

---

## Traceability

| Architecture review candidate | PRD story groups | Handoff phase |
|------------------------------|------------------|---------------|
| Live Binance.US adapter | B, C | 1 |
| Validation on sync start | D | 2 |
| Broker session + pause split | E, I | 2, 5 |
| Loopback wire unify | F | 3a |
| BarSurfacePhase authority | H | 4 |
| Kill-switch deepen + Today coupling | J, L | 6, 8 |
| Bar brain-proxy | K | 7 |
| NotchViewModel / Bar session runtime | L | 8 |
| Cross-repo frictionless merge | A, G, M | 0, 3b, 9 |

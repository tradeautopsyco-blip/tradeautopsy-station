# TradeAutopsy Station — Current vs Planned Audit

**Audit date:** 2026-07-07  
**Method:** Full read of all listed planning/status docs; code verification in `station/`, `agent/`, `notch/`; `cargo test` and `swift test` executed locally on macOS.

---

## Executive summary (read this first)

The repo is **not** where its PRDs say it is on the headline milestone line. Phase 0 shell, Today v1 (engine + API + UI), and Backend Box **control plane** (Keychain, validate, start/stop/delete, status taxonomy) are largely real and tested. The critical gap: **Binance.US broker sync — the PRD’s north-star broker — has no live poll adapter.** Runtime sync for `binance_us` resolves to `CountingPollAdapter`, which increments a counter and returns empty fills. Binance Global (`binance_com`) has a real spot client/adapter, but that contradicts Backend Box v1’s “Binance.US only” scope lock.

**Production-real today:** menu-bar Station app, agent supervision, wire HMAC auth, kill switch + DNS block + Ed25519 audit, Notch HUD (brain-proxied), Today round-trip engine + `/api/daemon/today`, Brokers UI + credential lifecycle, Session pulse strip (USD, Today-sourced).

**Stubbed-but-tested:** broker sync state machine, data-class completeness, redaction/behavioral gates — proven with fakes/mocks, not live Binance.US fills.

**Placeholder / zero backing:** most Desk/Session routes (Pre-trade, Live, Journal, etc.), Market Data & AI/Workflow key validation, Stats UI, broker marketplace handoff, live Binance.US fill ingestion.

---

## 1. Plan and PRD narratives (with verification)

### 1.1 `AGENTS.md` (repo agent context)

**Stated goal:** Define architectural invariants for Station: agent on port 9137, `x-daemon-secret` + wire-v1 HMAC, no secrets in observable surfaces, kill switch always on, Keychain-only credentials, withdraw-permission hard block.

**Decisions recorded:** Ephemeral `AGENT_DAEMON_SECRET` per launch via `SecRandomCopyBytes`; UDS/0600 token-file auth explicitly **not implemented**.

| Claim | Status | Evidence |
|-------|--------|----------|
| Agent port 9137 | **VERIFIED** | `station/StationApp/AgentSupervisor.swift:6` (`defaultPort = 9137`); `station-wire/v1.json:6` |
| Ephemeral daemon secret per launch | **VERIFIED** | `station/StationApp/AgentDaemonSecret.swift:11-19,22-26` |
| Wire HMAC on loopback | **VERIFIED** | `agent/src/wire.rs:1-74`; middleware on protected routes in `agent/src/api/mod.rs:58+` |
| UDS token auth not implemented | **VERIFIED** | `station-wire/v1.json:22`; no UDS code in agent |
| Keychain-only credential store | **VERIFIED** | `station/StationApp/Broker/Services/KeychainBrokerCredentialStore.swift:9-47` |
| Withdraw hard block | **VERIFIED** (validation path) | `agent/src/broker_validation.rs:49-51`; Station tests `withdrawPermissionBlocksSaveAndStart` |
| Reference-library discipline before financial code | **VERIFIED** (Today) | `agent/src/round_trip_engine.rs:3` cites `MECHANICS.md`; engine tests exist |

**Gap vs invariant:** Withdraw block uses account-level `enable_withdrawals` from `/sapi/v1/account/apiRestrictions`, not key-level permission — `docs/reference/crypto/MANIFEST.md:63-64` flags this as an open blocker. Implemented anyway at account level.

---

### 1.2 `README.md`

**Stated goal:** Describe repo as macOS companion with Rust agent (9137) + Notch HUD; architecture invariants (Notch → agent only, no UI in agent, no LLM in Station, no entry orders).

| Claim | Status | Evidence |
|-------|--------|----------|
| `agent/` Rust loopback daemon | **VERIFIED** | `agent/src/main.rs`, Axum router |
| `notch/` Swift Notch HUD | **VERIFIED** | `notch/Package.swift`, 80+ Swift sources |
| `station/` mentioned in AGENTS but not README table | **STALE** | README table lists only agent + notch; `station/` is the shippable `.app` |
| macOS 14+, Rust, Xcode 15+ | **VERIFIED** | `station/Package.swift:8`, `station/project.yml:5`, CI |
| `sudoers.d/99-tradeautopsy-dns` for L3 DNS | **VERIFIED** (code path) | `agent/src/dns_block.rs:53-58` uses `sudo` for hosts flush on macOS |
| Notch never calls tradeautopsy.in directly | **VERIFIED** | NotchViewModel uses loopback + `webBaseURL` for brain via agent proxy patterns |

---

### 1.3 `.github/PRD-phase0-station-shell.md` + `TRD-phase0-station-shell.md`

**Stated goal:** Ship menu-bar `TradeAutopsy Station.app` with coordinator, agent supervision, 1100×700 window, 10-route sidebar placeholders, shared Notch + pulse strip, hotkeys, Login Item — **no screen content**, **no Backend Box**.

**What was decided:** `LSUIElement`, single `NotchViewModel`, `StationRoute` canonical nav, Desk route persistence, 60s Session stickiness, degraded-but-navigable shell, embed `tradeautopsy-agent`.

**Built vs deferred:**

| Deliverable | Built? | Status | Evidence |
|-------------|--------|--------|----------|
| Menu-bar app, no Dock | Yes | **VERIFIED** | `station/StationApp/Info.plist:25-26` (`LSUIElement=true`) |
| Agent spawn/health/restart/teardown | Yes | **VERIFIED** | `AgentSupervisor.swift:34-91,53-64`; coordinator tests |
| 1100×700 window, frame persist | Yes | **VERIFIED** | `StationWindowControllerTests` |
| Hotkeys ⌥Space / ⌥⇧Space | Yes | **VERIFIED** | `HotkeyRegistrarTests` (11 tests) |
| Login at Login opt-in | Yes | **VERIFIED** | `SettingsView.swift:18-21`, `LoginItemServiceTests` |
| Ten route placeholders only | Partial | **STALE** | Shell now has **12 routes** + Backend Box section; real Today, Brokers, Settings, Market Data, AI/Workflow — `StationRoute.swift:9-21`, `StationShellView.swift:64-79` |
| No Backend Box in Phase 0 | Violated | **STALE** | Backend Box shipped inside Phase 0 shell window |
| Session pulse strip (Notch live-state INR path) | Superseded | **STALE** | Pulse strip now uses Today USD path — `SessionPulseStrip.swift:14-27` |
| `station_version` pin | Yes | **VERIFIED** | `station-wire/v1.json:35-36` (`0.2.0`, `phase-0-shell`) |
| CI Station tests | Yes | **VERIFIED** | `.github/workflows/ci.yml:49-55` |
| Acceptance checklist all [ ] | Manual | **ASPIRATIONAL** | `PHASE0_SMOKE_CHECKLIST.md` — no sign-off in repo |

**Phase 0 verdict:** Core shell **shipped**. PRD’s “placeholders only” boundary is **obsolete** — subsequent slices landed without updating Phase 0 status.

---

### 1.4 `.github/PRD-backend-box-v1.md`

**Stated goal:** Brokers screen as safe Binance.US sync control plane — validate, Keychain store, auto-start read sync, honest status, stop/start/delete, behavioral upload with redaction, internal environments.

**North star:** *“Connect Binance.US once … feed redacted broker activity …”*

**Built vs deferred:**

| Area | PRD claim | Actual | Status |
|------|-----------|--------|--------|
| Binance.US only enabled broker | Locked v1 | **Binance.US + Binance.com enabled** in catalog | **STALE** | `BrokerCatalog.swift:29-41` |
| Live validation before Keychain save | Required | Swift validators + agent `LiveBinanceUSValidationAdapter` | **VERIFIED** | `broker_validation.rs:83-138` |
| Withdraw hard block | Required | Account-level `enable_withdrawals` | **VERIFIED** (with MANIFEST caveat) |
| Auto-start after connect | Required | `BrokersViewModel` + `AgentBrokerSyncControl` | **VERIFIED** | lifecycle tests |
| Stop/Start without agent restart | Required | `broker_sync_control.rs:195-207` | **VERIFIED** | `broker_sync_control.rs` tests |
| Delete removes Keychain | Required | `KeychainBrokerCredentialStore.delete` | **VERIFIED** | `BrokerDeleteLifecycleTests` |
| Sync fills, balances, open orders | Required | Poll loop calls all three data classes | **PARTIAL** | Loop exists `broker_sync.rs:337-339`; **Binance.US adapter returns nothing** |
| 90-day backfill | Required | Config + first-poll floor logic | **STUBBED** | `broker_sync.rs` + `initial_backfill_days: 90`; useless without real adapter |
| Rate limit / degraded / partial sync states | Required | State machine + tests with `ConfigurableDataClassAdapter` | **VERIFIED** (fake adapters) |
| Behavioral upload + redaction | Required | `broker_redaction.rs`, `broker_behavioral.rs` | **VERIFIED** (unit/release gates) |
| Environments (internal only) | Required | `TradeAutopsyEnvironment`, per-env Keychain namespace | **VERIFIED** | `BrokerEnvironmentTests` |
| Manual Binance.US smoke sign-off | Release blocker | Checklist exists, unchecked | **ASPIRATIONAL** | `BACKEND_BOX_SMOKE_CHECKLIST.md` |

**Critical code finding — Binance.US sync is a no-op:**

```225:244:agent/src/broker_sync_control.rs
fn build_runtime_adapter(
    broker_slug: &str,
    api_key: &str,
    api_secret: &str,
) -> anyhow::Result<Arc<dyn BrokerAdapter>> {
    match broker_slug {
        "binance_us" if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_") => {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        "binance_us" => Ok(Arc::new(CountingPollAdapter::new())),
        "binance_com"
            if api_key.starts_with("TA_TEST_SYNC") || api_key.starts_with("TA_FAKE_COM_") =>
        {
            Ok(Arc::new(CountingPollAdapter::new()))
        }
        "binance_com" => Ok(Arc::new(crate::binance_com_spot_adapter::BinanceComSpotBrokerAdapter::new(
            api_key, api_secret,
        ))),
        other => anyhow::bail!("unsupported broker slug: {other}"),
    }
}
```

There is **no** `binance_us_spot_client.rs` or `BinanceUsSpotBrokerAdapter` in the repo (glob confirms zero files). Validation talks to Binance.US; sync does not.

**Backend Box v1 verdict:** Control plane and safety rails are **real and tested**. The product promise of **live Binance.US data** is **not implemented** — card may show “Syncing” while ingesting zero fills.

---

### 1.5 `.github/PRD-today-v1.md` + `plans/today-v1.md` + `ISSUES-today-v1.md`

**Stated goal:** Local WAC round-trip engine, Today API, local behavior signals, Today UI, pulse strip on same payload, daily snapshots for future Stats. Stats = Slice 2 deferred.

**Parent context claim:** *“Phase 0 shell complete · Backend Box v1 (Brokers) complete”* — **STALE** on Backend Box (see §1.4).

**Phase-by-phase (`plans/today-v1.md`):**

| Phase | Scope | Status | Evidence |
|-------|-------|--------|----------|
| 1 — Round-trip engine | WAC, fees, unknown basis, unit tests | **VERIFIED** | `round_trip_engine.rs`; `round_trip_engine_tests.rs` (12 tests); `today_release_gates.rs` |
| 2 — Persistence + Today API | SQLite, `GET /api/daemon/today`, degraded matrix | **VERIFIED** | `today/store.rs`, `today/service.rs`, `api/today.rs:7-25`; `today_api.rs` |
| 3 — Local behavior signals | 4 signals, top 2, row flags | **VERIFIED** | `today/signals.rs`; unit test loss-chasing+revenge |
| 4 — Station TodayView | Replace placeholder, 3+2 layout, breaker banner | **VERIFIED** | `TodayView.swift`; `StationShellView.swift:73-74` |
| 5 — Pulse strip + release gates | Same P&L as hero, USD | **VERIFIED** | `SessionPulseStrip.swift:14-27`; `TodayReleaseGateTests.swift` |
| 6 — Stats UI | Deferred | **ASPIRATIONAL** | Not started |

**Acceptance criteria spot-check:**

| Criterion | Status | Notes |
|-----------|--------|-------|
| WAC matches hand calc | **VERIFIED** | Engine tests |
| Binance Trade Analysis within $0.01 (manual) | **ASPIRATIONAL** | Blocked without live US fills |
| Unknown basis excluded from hero | **VERIFIED** | `today_release_gates.rs:24-48` |
| Daily snapshot on close + day boundary | **VERIFIED** | `today/service.rs` tests |
| Degraded matrix (4 states) | **VERIFIED** | `TodayScreenPresentationTests` (5 tests) |
| No hosted INR for Today/pulse when broker connected | **PARTIAL** | Pulse/Today use USD; **Notch collapsed chrome still uses `formatINR(sessionPnL)`** — `notch/NotchViewModel.swift:332,2816` |
| Circuit breaker banner + resume countdown | **VERIFIED** | `TodayViewModel.swift:33-50`, `TodayView.swift` |

**Today v1 verdict:** Agent + Station vertical is **substantially shipped** for any fills that reach `recent_fills`. With Binance.US stub adapter, end-to-end value for the PRD’s primary broker is **blocked at ingestion**.

---

### 1.6 `plans/broker-marketplace-handoff.md`

**Stated goal:** Next priority = broker marketplace in Backend Box (App-Store-style catalog, per-broker requirements schema). Locked decisions from grilling session in wrong repo; must re-explore Station code first.

| Claim | Status | Evidence |
|-------|--------|----------|
| Marketplace not started | **VERIFIED** | No requirements-schema types, no marketplace list/detail UI beyond static `BrokerCatalog` |
| Curated catalog (not arbitrary endpoints) | **PARTIAL** | `BrokerCatalog.swift` — 2 enabled + 2 planned cards, no per-broker auth schema |
| Console TS registry does not carry over | **VERIFIED** | Rust `BrokerAdapter` + Swift Keychain model used instead |
| Parked: Dock/Cmd-Tab, HTML restyle | **ASPIRATIONAL** | `LSUIElement=true` still set — intentional per Phase 0 |

---

### 1.7 Smoke checklists (`.github/*_SMOKE_CHECKLIST.md`)

| Doc | Narrative | Status |
|-----|-----------|--------|
| `PHASE0_SMOKE_CHECKLIST.md` | 10 manual sections for shell release | **ASPIRATIONAL** — all `[ ]`; claims BrokersView real (**VERIFIED** §1.3) |
| `BACKEND_BOX_SMOKE_CHECKLIST.md` | 10-step real Binance.US manual path | **ASPIRATIONAL** — unchecked; step 4 (90-day backfill) would fail with stub adapter |
| `TODAY_V1_SMOKE_CHECKLIST.md` | Manual + CI gates | CI items marked `[x]`; manual Binance round-trip still `[ ]` |

---

### 1.8 `docs/reference/` (product decisions only)

**`README.md` — Reference library discipline**  
Establishes: no formula from AI memory; cite authoritative sources; categories (broker-mechanics, market-structure, accounting, behavioral-science). **VERIFIED** as process; **`behavioral-science/` is empty** (`.gitkeep` only).

**`crypto/CRYPTO-STANDARD.md` — Exchange isolation, citation rules, 90-day staleness review**  
**VERIFIED** as enforced standard; Binance.US vs Global separation reflected in code (`binance_com_*` vs `binance_us` paths).

**`crypto/MANIFEST.md` — Status table + Binance.US blockers**  
Documents three **open blockers** for Backend Box: (1) key-level withdraw endpoint uncertain, (2) 90-day myTrades backfill unconfirmed in source, (3) Binance.US snapshot stale (2023-09-06). **VERIFIED** — these remain unresolved; code shipped despite blocker 1.

**`crypto/binance-us/spot/MECHANICS.md` — WAC, round-trip, unknown basis, fee rules for Today v1**  
Product decisions: performance-not-tax basis, unknown basis honesty, non-USD quote exclusion, fee_unhandled flag. **VERIFIED** — implemented in `round_trip_engine.rs` and Today service tests.

---

## 2. Current feature inventory (code truth)

### 2.1 Station shell (`station/`)

**Sidebar routes (12 total)** — `StationRoute.swift:9-21`, `StationSidebar.swift:14-28`

| Route | Section | Implementation | Backing |
|-------|---------|----------------|---------|
| Today | Session | `TodayView` + `TodayViewModel` | Agent `/api/daemon/today` |
| Pre-trade | Session | `StationPlaceholderView` | None |
| Live trade | Session | Placeholder | None (Notch has live tabs) |
| Post-trade | Session | Placeholder | None |
| Brokers | Backend Box | `BrokersView` + full VM | Agent sync control + Keychain |
| Market Data | Backend Box | `MarketDataKeysView` | Keychain only; **validation stubbed** (UI copy line 18) |
| AI / Workflow | Backend Box | `AIWorkflowKeysView` | Keychain only; **validation stubbed** (line 18) |
| Journal | Desk | Placeholder | None |
| Escrow match | Desk | Placeholder | None (Notch has `BarEscrowMatchView`) |
| Patterns | Desk | Placeholder | None |
| Fidelity score | Desk | Placeholder | None |
| Settings | Desk | Launch at Login toggle only | `SMAppService` |

**Shell infrastructure (real):** `StationAppCoordinator`, `AgentSupervisor`, `HotkeyRegistrar`, `StatusItemController`, `NavigationPolicy`, warnings, Login Item prompt, shared Notch hosting, desk route persistence.

**PRD vs routes:** Phase 0 specified 10 routes / 2 sections. Code has **12 routes / 3 sections** (added Backend Box group + Market Data + AI/Workflow). **STALE** relative to Phase 0/TRD sidebar spec.

---

### 2.2 Agent (`agent/`)

**HTTP surface (representative):** health, SSE, kill switch, bar proxy, broker sync start/stop/retry/state, recent trades, today, daemon commands, instruments, capture/outbox — `agent/src/api/mod.rs`.

**Broker adapters (`BrokerAdapter` trait — `broker.rs:42-54`):**

| Adapter | Purpose | Live broker data? |
|---------|---------|-------------------|
| `CountingPollAdapter` | Test / **production binance_us** | **No** — empty fills |
| `BinanceComSpotBrokerAdapter` | Binance Global spot | **Yes** — myTrades + balances |
| `ConfigurableDataClassAdapter` | Integration tests | Fake |
| `SeqMockBrokerAdapter` | Test queues | Fake |

**Validation adapters (separate from sync):**

| Adapter | Exchange | Live API? |
|---------|----------|-----------|
| `LiveBinanceUSValidationAdapter` | Binance.US | Yes — `/sapi/v1/account/apiRestrictions` |
| `LiveBinanceComValidationAdapter` | Binance Global | Yes |
| `FakeBinanceUSValidationAdapter` / `FakeBinanceComValidationAdapter` | CI | Magic keys |

**Today stack:** `RoundTripEngine` → `TodayStore` (round_trips, daily_snapshots) → `TodayService` → `GET /api/daemon/today`. Behavior signals in `today/signals.rs`. **Real** for stored fills.

**Security / enforcement:**

| Mechanism | Implemented? | Location |
|-----------|--------------|----------|
| Wire v1 HMAC + nonce replay window | Yes | `wire.rs` |
| SSE signing | Yes | `sse_signing.rs` |
| Kill switch + DNS L3 hosts block | Yes | `kill_switch` API + `dns_block.rs` |
| Ed25519 kill-switch audit log | Yes | `kill_switch_audit.rs` |
| Broker redaction boundary | Yes | `broker_redaction.rs` |
| Behavioral opt-out gate | Yes | release gate tests |

**Other modules:** `instruments/` (SQLite symbol store — Kotak segment map present but **unused**, dead_code warnings), `exchange_info` cache, bar forward proxy to brain, capture outbox.

---

### 2.3 Notch (`notch/`)

Separate Swift package — ambient overlay (pill, expanded shell, kill switch overlay, declaration flows, debrief, positions, TAI, journal capture, on-device dictation).

| Capability | Status | Notes |
|------------|--------|-------|
| Loopback agent polling/SSE | **VERIFIED** | `NotchViewModel`, `DaemonConnection.swift` |
| Kill switch overlay | **VERIFIED** | `KillSwitchOverlayController.swift` |
| Pre-trade / live / post-trade **content** | **VERIFIED in Notch** | Not Station window routes |
| Hosted mode (no duplicate hotkeys) | **VERIFIED** | `NotchLauncher.swift:24-30,69-71` |
| Shared `StationShellView` when expanded | **VERIFIED** | Station hosts via `HostedExpandedNotchShell` |
| Session P&L display in Notch chrome | **STALE vs Today PRD** | Still INR via brain `session_pnl` — `NotchViewModel.swift:2280,2816` |
| BarNotchScreen (9 screens) | **VERIFIED** | Parallel legacy nav; Station uses `StationRoute` (12) |

Notch is the **most feature-complete** surface for trading-session UX; Station window is the **structural shell + Today + Backend Box**.

---

## 3. Tech stack (declared vs actual)

| Layer | Declared | Actual imports / usage |
|-------|----------|------------------------|
| Agent | Rust 2021, Axum 0.7, Tokio, rusqlite, reqwest, ed25519-dalek, hmac/sha2 | Matches `agent/Cargo.toml` |
| Station | Swift 6.0 package, macOS 14, depends on Notch + swift-testing | `station/Package.swift`; AppKit, Security, ServiceManagement linked |
| Notch | Swift 5.9, macOS 14, AVFoundation, Speech | Dictation/journal capture |
| Xcode app | xcodegen, embed release agent binary | `station/project.yml:37-48` |
| CI | macOS 14: agent + station swift test; macOS 15: `.app` build | **Notch tests not in CI** — only `swift build` (`ci.yml:36-39`) |
| Wire contract | `station-wire/v1.json` v1.0.0, station_version 0.2.0 | Endpoints match agent routes; Today endpoint not listed in wire JSON (**STALE** contract doc) |

**README omits `station/`** — documentation drift.

---

## 4. Security posture (implemented vs documented)

| Control | Docs claim | Implementation | Gap |
|---------|------------|----------------|-----|
| Loopback-only from Station/Notch UI | Yes | Agent client URLs use 127.0.0.1:9137 | Brain egress only from agent |
| Ephemeral daemon secret | Yes | `AgentDaemonSecret.swift` | Dev attach allows legacy `AGENT_SECRET` |
| Wire HMAC on protected routes | Yes | `wire.rs` middleware | Health may differ — supervisor uses `/api/daemon/health` |
| Keychain for broker secrets | Yes | Generic password, per-env account string | **No app sandbox** — no `.entitlements`, no App Sandbox in project |
| Keychain for provider API keys | Backend Box extension | `KeychainProviderAPIKeyStore.swift` | Validation stubbed — keys stored unvalidated |
| Withdraw permission block | Hard block | Account-level apiRestrictions | Key-level block unconfirmed (MANIFEST blocker 1) |
| No secrets in logs/SSE/UI | Structural | Presentation tests scan for leakage; redaction module | Runtime enforcement depends on discipline |
| Ed25519 audit trail | Slice B | SQLite append-only + verify tests | Seed from env or ephemeral generate |
| DNS kill switch (L3) | README sudoers | `/etc/hosts` manipulation + broker host maps | Requires sudo / sudoers file |
| Kill switch survives Stop sync | PRD invariant | Stop only pauses broker poll — `broker_sync_control.rs:195-207` | **VERIFIED** |
| Code signing | Production expectation | CI: `CODE_SIGN_IDENTITY="-"`, signing disabled | Not production-hardened |

**Sandboxing:** **Not implemented.** Station runs unsandboxed with Keychain access — typical for macOS utilities but unstated in PRDs.

---

## 5. Test and performance state

### 5.1 Rust (`cd agent && cargo test`)

**Run on 2026-07-07 (this audit machine):**

| Run | Result |
|-----|--------|
| Run 1 | **FAILED** — `broker_data_classes::all_data_classes_current_reports_syncing` timeout on `POST .../broker/sync/start` (3s client timeout) |
| Run 2 | **FAILED** — same test |
| Run 3 | **PASSED** — all suites green |
| `--list` count | **142** tests |

**Suite breakdown (representative):** 44 lib unit tests; integration tests including `round_trip_engine_tests` (12), `phase6_recent_trades` (9), `capture_delivery_phase3` (11), `bar_forward` (7), `binance_us_validation` (7), `backend_box_release_gates` (4), `today_release_gates` (3), `today_api` (2), plus kill switch, wire, SSE, broker sync suites.

**Flaky / timing-sensitive:**

- `broker_data_classes.rs:all_data_classes_current_reports_syncing` — uses 3s HTTP timeout while agent may still be starting; **failed 2/3 runs**. Likely race under load, not logic failure.
- Several `AgentSupervisorTests` take **10–13s** each (port collision probes) — slow but deterministic.

**Warnings:** Multiple `unused_imports` / `dead_code` in agent lib (instruments, broker_sync spawn_broker_stack) — no test impact.

### 5.2 Station (`cd station && swift test`)

**166 tests passed**, 0 failed (~35s; AgentSupervisor suite dominates).

Covers: coordinator lifecycle, navigation policy, broker credential lifecycle, Backend Box release gates, Today presentation + release gates, pulse strip, hotkeys, window controller, Binance US/COM validation.

### 5.3 Notch (`cd notch && swift test`)

**6 tests passed** — `BarNotchScreenTests`, `BarNotchPhaseRoutingTests` only. **Not run in CI.**

### 5.4 Performance

No benchmarks or perf gates in repo. Broker poll loop uses configurable intervals + rate-limit backoff (`broker_sync.rs`) — not profiled here.

---

## 6. Real picture summary

### Production-real (would work in a founder demo with correct preconditions)

1. **Station.app shell** — menu bar, window, hotkeys, agent supervision, warnings, Login Item.
2. **Notch HUD** — intervention overlay, declarations, kill switch UX, brain-proxied live state (INR heritage).
3. **Agent core** — Axum on 9137, wire auth, SSE, kill switch fire/dismiss, DNS block, audit log.
4. **Backend Box control plane** — connect UI, live credential validation (US + COM), Keychain CRUD, start/stop/delete, status taxonomy, environments (debug-gated).
5. **Today v1 stack** — WAC engine, signals, API, UI, degraded matrix, pulse strip USD wiring — **when fills exist in SQLite**.
6. **Safety tests** — withdraw block (fake keys), redaction, opt-out, secret leakage guards in presentation tests.

### Stubbed-but-tested (passes CI, fails product intent)

1. **Binance.US runtime sync** — `CountingPollAdapter` for all non-test keys; sync state machine runs on empty data.
2. **90-day backfill** — logic present; never fetches real US trades.
3. **Market Data / AI Workflow keys** — Keychain storage; UI states “validation is stubbed in v1”.
4. **Data-class “Syncing” status** — can report healthy completeness with zero fills from counting adapter.

### Placeholder / no backing code

1. **Station routes:** Pre-trade, Live trade, Post-trade, Journal, Patterns, Fidelity score (shell placeholders).
2. **Stats / Slice 2** — daily_snapshots written; no Stats UI.
3. **Broker marketplace** — handoff doc only; static catalog without requirements schema.
4. **FIFO tax export, multi-broker UI, order placement** — PRD non-goals; still absent.
5. **Manual smoke sign-offs** — all three checklists unchecked in repo.
6. **Wire doc** — missing Today endpoint listing.

### Highest-risk doc/code contradictions (for next PRD)

| # | Contradiction | Severity |
|---|---------------|----------|
| 1 | PRD-today-v1 assumes “Backend Box complete” + Binance.US fills | **Ship-blocker** for Today value prop |
| 2 | `binance_us` → `CountingPollAdapter` while `binance_com` has real adapter | **Scope inversion** vs Backend Box v1 |
| 3 | Phase 0 “placeholders only” vs 5 non-placeholder routes | **Plan drift** |
| 4 | MANIFEST blockers vs shipped withdraw check | **Compliance ambiguity** |
| 5 | TODAY PRD “no INR on pulse” vs Notch INR session P&L | **UX inconsistency** |
| 6 | `TODAY_V1_SMOKE_CHECKLIST` CI `[x]` without mandatory manual Binance.US path | **Release honesty** |

### Suggested immediate engineering truth label

**Station is a real macOS shell + agent + Notch product with a complete Today engine on synthetic/real-ingested fills, and a Backend Box control plane that validates Binance.US credentials but does not yet sync Binance.US trades.**

---

## Appendix: doc inventory read

| File | Read |
|------|------|
| `AGENTS.md` | Full |
| `README.md` | Full |
| `plans/today-v1.md` | Full |
| `plans/broker-marketplace-handoff.md` | Full |
| `.github/PRD-phase0-station-shell.md` | Full |
| `.github/PRD-today-v1.md` | Full |
| `.github/PRD-backend-box-v1.md` | Full |
| `.github/TRD-phase0-station-shell.md` | Full |
| `.github/ISSUES-today-v1.md` | Full |
| `.github/TODAY_V1_SMOKE_CHECKLIST.md` | Full |
| `.github/PHASE0_SMOKE_CHECKLIST.md` | Full |
| `.github/BACKEND_BOX_SMOKE_CHECKLIST.md` | Full |
| `docs/reference/README.md` | Full |
| `docs/reference/crypto/CRYPTO-STANDARD.md` | Full |
| `docs/reference/crypto/MANIFEST.md` | Full |
| `docs/reference/crypto/binance-us/spot/MECHANICS.md` | Full (product sections) |

Raw Binance API reference trees under `docs/reference/crypto/**/REST.md` etc. were not narrated per audit scope.

# TRD — Phase 0: Station App Shell + Window Skeleton

**Status:** Ready for implementation  
**Derived from:** [PRD-phase0-station-shell.md](./PRD-phase0-station-shell.md)  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Parent context:** TradeAutopsy Station v1 (`FExEVIL/tradeautopsy` #161)  
**Scope:** macOS app wrapper, coordinator, Station window chrome, nav placeholders — **no screen content**, **no Backend Box**, **no Notch HUD internals**

---

## 1. Purpose

This document translates the Phase 0 PRD into implementable technical requirements. It defines modules, types, protocols, state machines, wire contracts, persistence, build artifacts, and verification criteria sufficient for an engineer or agent to implement **TradeAutopsy Station.app** without re-reading product rationale.

---

## 2. Goals and Non-Goals

### 2.1 Goals

| ID | Goal |
|----|------|
| G1 | Ship a menu-bar macOS `.app` (`LSUIElement`, no Dock icon) that owns `NSApplication` lifecycle |
| G2 | Spawn, health-check, supervise, and tear down `tradeautopsy-agent` on loopback `127.0.0.1:9137` |
| G3 | Provide one canonical navigation model (`StationRoute`) shared by Notch expanded surface and Station window |
| G4 | Render empty labeled placeholders for all ten routes; shell remains navigable when agent is unhealthy |
| G5 | Register global hotkeys (`⌥Space`, `⌥⇧Space`) centrally via coordinator |
| G6 | Persist window frame and Desk route; recompute Session route from trading phase on launch |
| G7 | Produce CI-verifiable unit tests via injectable coordinator protocols |

### 2.2 Non-Goals

Same as PRD §Out of Scope: screen content, Backend Box, Notch HUD internals, kill switch logic changes, new wire endpoints, non-macOS targets, customizable hotkeys, multiple Station windows, HTTP graceful agent shutdown.

---

## 3. System Context

```
┌─────────────────────────────────────────────────────────────────┐
│              TradeAutopsy Station.app (station/)                │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              StationAppCoordinator (@MainActor)           │  │
│  │  • NotchViewModel (single instance)                       │  │
│  │  • StationRoute + NavigationPolicy                        │  │
│  │  • AgentSupervisor                                        │  │
│  │  • HotkeyRegistrar / StatusItemController                 │  │
│  │  • StationWindowController                                │  │
│  └───────────────┬─────────────────────┬─────────────────────┘  │
│                  │                     │                        │
│     ┌────────────▼──────────┐  ┌───────▼──────────────────┐    │
│     │ NotchLauncher (hosted)│  │ Station NSWindow          │    │
│     │ • NSPanel overlay     │  │ • SessionPulseStrip       │    │
│     │ • shared shell view   │  │ • StationShellView        │    │
│     └────────────┬──────────┘  └───────────────────────────┘    │
└──────────────────┼──────────────────────────────────────────────┘
                   │ HTTP/SSE loopback only
                   ▼
        ┌──────────────────────────┐
        │ tradeautopsy-agent       │
        │ 127.0.0.1:9137           │
        │ (embedded in .app bundle)│
        └────────────┬─────────────┘
                     │ x-daemon-secret + HMAC (wire v1)
                     ▼
              tradeautopsy.in (brain — agent only)
```

### 3.1 Architecture Invariants (must not violate)

1. Notch and Station shell call **only** loopback agent endpoints — never `tradeautopsy.in` directly.
2. Agent never renders UI.
3. No LLM inference in Station.
4. Signals to brain via existing `ingestSignal()` path only; Phase 0 adds no new egress.
5. Single `NotchViewModel` instance owned by coordinator — no duplicate polling/SSE pipes.

---

## 4. Repository Layout

### 4.1 New: `station/`

```
station/
├── TradeAutopsy Station.xcodeproj
├── StationApp/
│   ├── StationApp.swift              # @main entry
│   ├── StationAppCoordinator.swift
│   ├── AgentSupervisor.swift
│   ├── StationWindowController.swift
│   ├── StatusItemController.swift
│   ├── HotkeyRegistrar.swift
│   ├── LoginItemService.swift
│   ├── Navigation/
│   │   ├── StationRoute.swift
│   │   └── NavigationPolicy.swift
│   ├── Views/
│   │   ├── StationShellView.swift
│   │   ├── StationSidebar.swift
│   │   ├── StationPlaceholderView.swift
│   │   ├── SessionPulseStrip.swift
│   │   └── WarningBannerView.swift
│   └── Models/
│       ├── AgentHealthWarning.swift
│       └── InputMonitoringWarning.swift
└── StationTests/
    ├── StationAppCoordinatorTests.swift
    ├── NavigationPolicyTests.swift
    └── Fakes/
        ├── FakeAgentSupervisor.swift
        ├── FakeHotkeyRegistrar.swift
        ├── FakeStationWindowController.swift
        ├── FakeStatusItemController.swift
        └── FakeLoginItemService.swift
```

### 4.2 Modified: `notch/`

| File | Change |
|------|--------|
| `NotchLauncher.swift` | Accept injected `NotchViewModel`; skip hotkey monitors when `isHostedByStation == true`; optional shared shell adapter |
| `BarNotchShell.swift` | Migrate from local `@State activeScreen: BarNotchScreen` to coordinator-owned `StationRoute` binding (Phase 0: binding injection, content unchanged) |
| `Package.swift` / Xcode | Link `notch` as dependency of `station` target |

### 4.3 Unchanged: `agent/`

No feature changes. Binary embedded by Xcode copy build phase. Health endpoint used for launch poll.

### 4.4 Wire contract

Pin `station_version` in `station-wire/v1.json` on Phase 0 ship (currently `"0.1.0"`).

---

## 5. Platform and Toolchain

| Requirement | Value |
|-------------|-------|
| macOS deployment target | 14+ |
| Xcode | 15+ |
| Swift | 5.9+ (match existing `notch/`) |
| Rust | stable (agent build) |
| App type | `LSUIElement = true` (menu-bar utility, no Dock icon) |
| CI runner | `macos-14` (extend `.github/workflows/ci.yml`) |

---

## 6. Core Types

### 6.1 `StationRoute`

Canonical ten-case navigation enum. **Labels must match `Station.html` design handoff before merge**; provisional mapping below.

```swift
enum StationRoute: String, CaseIterable, Codable, Hashable {
    // Session (auto-follow phase unless manual stickiness)
    case today = "Today"
    case preTrade = "Pre-trade"
    case liveTrade = "Live trade"
    case postTrade = "Post-trade"

    // Desk (persist across relaunch; no auto phase-follow)
    case journal = "Journal"
    case brokers = "Brokers"
    case escrowMatch = "Escrow match"
    case patterns = "Patterns"
    case fidelityScore = "Fidelity score"
    case settings = "Settings"

    var isDesk: Bool { ... }
    var isSession: Bool { !isDesk }
    var sidebarSection: SidebarSection { ... }
    var sfSymbol: String { ... }
}

enum SidebarSection: String {
    case session = "Session"
    case desk = "Desk"
}
```

**Migration from `BarNotchScreen`:**

| `BarNotchScreen` | `StationRoute` |
|------------------|----------------|
| `.morning` | `.today` |
| `.pretrade` | `.preTrade` |
| `.live` | `.liveTrade` |
| `.posttrade` | `.postTrade` |
| *(none — new)* | `.journal` |
| *(none — new)* | `.brokers` |
| `.escrow` | `.escrowMatch` |
| `.patterns` | `.patterns` |
| `.fidelity` | `.fidelityScore` |
| `.triage` | *(resolve vs Settings per design handoff)* |
| `.settings` | `.settings` |

Provide `StationRoute.init?(barNotchScreen:)` for transitional mapping. Retire `BarNotchScreen` over subsequent slices.

### 6.2 `NavigationPolicy`

Pure static functions — unit-testable without UI.

```swift
enum NavigationPolicy {
    /// Map BarSurfacePhase → default Session route.
    static func routeForPhase(_ phase: BarSurfacePhase) -> StationRoute

    /// Launch route: restore Desk if saved; else phase-derived Session route.
    static func launchRoute(saved: StationRoute?, phase: BarSurfacePhase) -> StationRoute

    /// Returns new route if auto-follow should apply; nil = no change.
    /// Session stickiness: if user manually picked a Session route within 60s, do not auto-follow.
    static func shouldAutoFollowPhase(
        active: StationRoute,
        phase: BarSurfacePhase,
        manualSessionPickAt: Date?,
        now: Date
    ) -> StationRoute?
}
```

**Phase → route mapping (implement per existing `BarNotchShell.syncActiveScreenFromPhase` logic):**

| `BarSurfacePhase` | Default `StationRoute` |
|-------------------|------------------------|
| `.declaration` | `.preTrade` |
| `.armed` | `.liveTrade` |
| `.livePlan` | `.liveTrade` |
| `.debrief` | `.postTrade` |

**Today route:** shown when morning brief is available and not yet consumed (mirror existing morning-brief gating in `BarNotchShell`); otherwise fall through to phase mapping.

**Desk routes:** `journal`, `brokers`, `escrowMatch`, `patterns`, `fidelityScore`, `settings`.

### 6.3 Warning Models

```swift
struct AgentHealthWarning: Equatable {
    enum Reason: Equatable {
        case launchTimeout
        case portCollisionNonAgent
        case crashLoopExceeded
        case runtimeDisconnected
    }
    var reason: Reason
    var message: String
    var logPath: URL?
    var canRetry: Bool
}

struct InputMonitoringWarning: Equatable {
    var message: String
    var systemSettingsURL: URL  // x-apple.systempreferences:... Input Monitoring
}
```

Warnings render in a **persistent banner/strip slot** — not one-shot `NSAlert`. Shell navigation remains enabled.

---

## 7. `StationAppCoordinator`

`@MainActor final class StationAppCoordinator: ObservableObject`

### 7.1 Owned State

| Property | Type | Notes |
|----------|------|-------|
| `activeRoute` | `@Published StationRoute` | Single source of truth for Notch + Station |
| `notchViewModel` | `NotchViewModel` | Single instance; injected into hosts |
| `agentHealthWarning` | `@Published AgentHealthWarning?` | nil when healthy |
| `inputMonitoringWarning` | `@Published InputMonitoringWarning?` | nil when granted |
| `manualSessionPickAt` | `Date?` | Set on user Session nav click |
| `isFirstLaunch` | `Bool` | UserDefaults flag |
| `launchAtLoginEnabled` | `Bool` | Synced with `SMAppService` |

### 7.2 Dependencies (injectable protocols)

```swift
protocol AgentSupervising: AnyObject {
    var onHealthChange: ((Bool) -> Void)? { get set }
    func start() async
    func retry() async
    func shutdown() async
    var isHealthy: Bool { get }
    var currentWarning: AgentHealthWarning? { get }
}

protocol HotkeyRegistering: AnyObject {
    func registerToggleNotch(_ handler: @escaping () -> Void)
    func registerOpenStation(_ handler: @escaping () -> Void)
    func unregisterAll()
}

protocol StationWindowControlling: AnyObject {
    func show(orderFrontOnly: Bool)      // first launch / silent boot
    func showAndActivate()               // menu, ⌥⇧Space, pulse click
    func hide()                          // red close button
    func restoreFrame() / func persistFrame()
    var isVisible: Bool { get }
}

protocol StatusItemControlling: AnyObject {
    func install(coordinator: StationAppCoordinator)
    func updateAgentStatus(isHealthy: Bool)
}

protocol LoginItemServicing: AnyObject {
    var isRegistered: Bool { get }
    func setRegistered(_ enabled: Bool) throws
    func syncStatusOnLaunch()
}
```

Production implementations use AppKit; tests use fakes from `StationTests/Fakes/`.

### 7.3 Lifecycle Sequence

```
applicationDidFinishLaunching
  ├─ sync LoginItem status (SMAppService)
  ├─ restore window frame from UserDefaults
  ├─ compute launchRoute(savedDeskRoute, viewModel.barSurfacePhase)
  ├─ agentSupervisor.start()
  │    ├─ STATION_DEV_ATTACH=1 → skip spawn, attach to existing listener
  │    ├─ else spawn Bundle.main/.../tradeautopsy-agent
  │    └─ poll GET /health until 200 or ~10s timeout
  ├─ on healthy OR degraded:
  │    ├─ notchLauncher.configure + start() (hosted, no duplicate hotkeys)
  │    └─ viewModel.startPolling() if not already started by NotchLauncher
  ├─ register hotkeys (⌥Space toggle Notch, ⌥⇧Space open/front Station)
  ├─ install NSStatusItem menu
  ├─ detect Input Monitoring → set inputMonitoringWarning if missing
  ├─ first launch: show Station window (orderFront, no activate)
  └─ first-run Login Item prompt (non-blocking Enable / Skip)

applicationWillTerminate / Quit menu
  ├─ unregister hotkeys
  ├─ agentSupervisor.shutdown()  // SIGTERM → 3s → SIGKILL
  ├─ notchLauncher.dismiss()
  └─ persist window frame + Desk activeRoute
```

### 7.4 Navigation Behavior

| Event | Behavior |
|-------|----------|
| User selects Session sidebar item | Set `activeRoute`; record `manualSessionPickAt = now` |
| User selects Desk sidebar item | Set `activeRoute`; clear manual Session stickiness concern |
| `barSurfacePhase` changes | If `NavigationPolicy.shouldAutoFollowPhase(...)` returns route, update `activeRoute` even when Station window hidden |
| Phase change while Notch collapsed | Update route silently — **do not auto-expand Notch** (Phase 0) |
| Pulse strip click | Set `activeRoute = .liveTrade`; `showAndActivate()` Station window |
| Relaunch | Restore saved Desk route from UserDefaults; Session routes from current phase |

### 7.5 Persistence Keys (UserDefaults)

| Key | Type | Content |
|-----|------|---------|
| `station.window.frame` | `Data` (CGRect archive) | Window position/size |
| `station.route.desk` | `String` (raw `StationRoute`) | Last Desk route only |
| `station.firstLaunch.completed` | `Bool` | Skip first-run window + Login prompt |
| `station.loginItem.promptDismissed` | `Bool` | Skip Login Item onboarding prompt |

Do **not** persist Session routes.

---

## 8. `AgentSupervisor`

### 8.1 Spawn

| Parameter | Source |
|-----------|--------|
| Binary path | `Bundle.main.url(forAuxiliaryExecutable: "tradeautopsy-agent")` or `Contents/MacOS/tradeautopsy-agent` |
| Port | `9137` (env `AGENT_PORT` forwarded if set) |
| Secret | `AgentDaemonSecret.resolveForSession()` — ephemeral per launch, injected as `AGENT_DAEMON_SECRET` on spawn; wire-v1 health/API probes use the same in-memory value |
| Dev attach | `STATION_DEV_ATTACH=1` → no spawn; probe loopback only |

### 8.2 Launch Health Poll

```
GET http://127.0.0.1:9137/health
Expected: 200 JSON { "status": "ok", "daemon": "agent", ... }
Timeout: ~10 seconds with exponential backoff (e.g. 200ms → 500ms → 1s)
Failure: AgentHealthWarning(reason: .launchTimeout)
```

Reference: `agent/src/api/health.rs`.

### 8.3 Port Collision Handling

When spawn fails with address-in-use OR health poll finds existing listener:

1. Probe `GET /health` on `127.0.0.1:9137`.
2. Verify response identifies TradeAutopsy agent (`daemon == "agent"`, valid wire handshake / secret verification per existing conventions).
3. **If verified:** attach (supervisor does not own process; no SIGTERM on quit unless spawned by Station).
4. **If not verified:** `AgentHealthWarning(reason: .portCollisionNonAgent)` with actionable message (check Tauri `DAEMON_PORT`, other processes).

### 8.4 Runtime Supervision

| Parameter | Value |
|-----------|-------|
| Health signal | Coordinator notified on up/down for SSE reconnect via `NotchViewModel` |
| Auto-restart | ≤ 3 attempts per rolling 60s window |
| Backoff | Exponential between restarts |
| Crash loop stop | After 3 failures in window → `crashLoopExceeded`; require manual Retry |
| Retry action | Reset counter; respawn or re-attach |

Follow test style of `DaemonConnectionFSM` in `notch/DaemonConnection.swift` for health FSM.

### 8.5 Shutdown (Station-spawned agent only)

```
1. SIGTERM to child PID
2. Wait up to 3 seconds
3. SIGKILL if still alive
4. Clear child PID reference
```

On Quit menu only — closing Station window does **not** shutdown agent.

---

## 9. Hotkeys

| Shortcut | Action | Registration |
|----------|--------|--------------|
| `⌥Space` | Toggle Notch overlay | `HotkeyRegistrar` via coordinator only |
| `⌥⇧Space` | Open or front Station window (`showAndActivate`) | Same |

When `NotchLauncher.isHostedByStation == true`:

- Remove `installHotkeyMonitorsIfNeeded()` calls from `NotchLauncher.start()`.
- Coordinator owns all global/local monitors.
- Unregister all monitors in `applicationWillTerminate`.

**Input Monitoring:** global `⌥Space` requires Accessibility/Input Monitoring permission. Detect via `CGEvent.tapIsEnabled` or equivalent; surface `InputMonitoringWarning` with System Settings deep link. Local monitor may still work when Station is focused.

---

## 10. Station Window

### 10.1 `StationWindowController`

| Property | Value |
|----------|-------|
| Default size | 1100 × 700 pt |
| Minimum size | 1100 × 700 pt |
| Style mask | `.titled`, `.closable`, `.miniaturizable`, `.resizable` |
| Titlebar | `titlebarAppearsTransparent = true`, `fullSizeContentView = true`, hidden title text |
| Traffic lights | Standard macOS buttons (dark unified toolbar chrome) |
| Instance count | **1** — reopen actions call `show`/`showAndActivate` on existing window |
| Close (red) | `orderOut` — app keeps running |
| Frame persistence | Save on move/resize debounced; restore on launch |

### 10.2 Layout (top → bottom)

```
┌──────────────────────────────────────────────────────────── 1100px ────┐
│ [traffic lights]          unified toolbar (dark)                       │
├────────────────────────────────────────────────────────────────────────┤
│ SessionPulseStrip (full width)                                         │
├──────────┬─────────────────────────────────────────────────────────────┤
│ Sidebar  │ StationPlaceholderView(activeRoute)                         │
│ 220px    │                                                             │
│ Session  │  Title: "<Route label>"                                     │
│ Desk     │  Body: "Coming in a future release." (Phase 0)              │
└──────────┴─────────────────────────────────────────────────────────────┘
```

Use `BarDS` fill/text tokens from `notch/BarDesignSystem.swift`.

### 10.3 Focus Policy

| Trigger | Window API |
|---------|------------|
| First launch | `orderFront(nil)` — visible, no key steal |
| Login-at-boot silent start | No window unless previously visible; if shown, `orderFront` only |
| Menu "Open Station", `⌥⇧Space`, pulse strip click | `makeKeyAndOrderFront` |

---

## 11. `SessionPulseStrip`

Shared SwiftUI view used in Station window and Notch expanded surface.

### 11.1 Data Source

Bind to coordinator's single `NotchViewModel`:

| Field | Source |
|-------|--------|
| Session P&L (realized today) | `viewModel` live-state projection |
| Total unrealized P&L | Sum of open positions |
| Open position count | Positions array count |
| Single-position symbol chip | Show when `count == 1` |

### 11.2 Degraded Display

When agent unhealthy or broker data unavailable:

- Show `—` for numeric fields
- Amber indicator (never stale numbers)
- Strip remains clickable → navigates to `.liveTrade` when shell navigable

### 11.3 Interaction

Tap/click → `coordinator.navigate(to: .liveTrade)` + `showAndActivate()` if Station host.

---

## 12. Notch Hosting

### 12.1 Integration

```swift
// Coordinator owns view model
let viewModel = NotchViewModel()
let notchLauncher = NotchLauncher(viewModel: viewModel, isHostedByStation: true)
notchLauncher.configure(secret:port:webBase:)
```

Modify `NotchLauncher` to accept optional injected view model (default preserves standalone behavior for C ABI path).

### 12.2 Shared Shell

Extract or adapt sidebar + placeholder router from `BarNotchShell` into `StationShellView` accepting:

```swift
@ObservedObject var viewModel: NotchViewModel
@Binding var activeRoute: StationRoute
var onSessionNavTap: () -> Void  // records manualSessionPickAt
```

Notch expanded surface embeds same `StationShellView` + `SessionPulseStrip` below existing top bar.

### 12.3 Start Condition

Call `notchLauncher.start()` after agent reaches healthy **or** degraded-warning state (shell must function in degraded mode).

---

## 13. Status Item Menu

NSStatusItem with template icon. Menu items:

| Item | Action |
|------|--------|
| Open Station | `showAndActivate()` |
| Toggle Notch | Toggle `NotchPanelController` |
| Agent status | Display-only (● green / ● amber / ● red) |
| Launch at Login | Checkmark toggle → `LoginItemService.setRegistered` |
| Quit | `coordinator.shutdown()` → terminate app |

Update agent status indicator on `agentSupervisor.onHealthChange`.

---

## 14. Login Item

| Requirement | Implementation |
|-------------|----------------|
| API | `SMAppService.mainApp` (macOS 13+) |
| Default | Off — opt-in only |
| Toggle surfaces | Settings placeholder + status menu |
| First-run prompt | Non-blocking banner/sheet: Enable / Skip |
| Re-sync | On launch, read `SMAppService.mainApp.status`; update `launchAtLoginEnabled` if user changed in System Settings |

---

## 15. Agent Wire Surface (Phase 0 usage)

Shell uses **existing** endpoints only:

| Endpoint | Method | Phase 0 use |
|----------|--------|-------------|
| `/health` | GET | Launch poll, port collision verification |
| `/api/bar/v1/live-state` | GET | Polling via `NotchViewModel` |
| SSE live-state | GET (SSE) | Real-time P&L after connect |
| Positions projection | via live-state | Pulse strip counts/chips |

Full contract: `station-wire/v1.json`. Pin `compatibility.station_version` on release.

**Reconnect:** when supervisor reports agent recovered, `NotchViewModel` resumes polling/SSE without app relaunch (existing reconnect logic; coordinator triggers `startPolling` if stopped).

---

## 16. Build and Release

### 16.1 Xcode Project

- Target: **TradeAutopsy Station** (macOS app)
- Link: `notch` Swift package / framework
- Embed: `tradeautopsy-agent` binary
- Info.plist: `LSUIElement = YES`, bundle ID per org convention

### 16.2 Agent Embed Build Phase

```
1. cargo build --release -p tradeautopsy-agent (or workspace equivalent)
2. Copy agent/target/release/tradeautopsy-agent → Contents/MacOS/tradeautopsy-agent
3. Code-sign embedded binary with app bundle
```

Support Debug configuration copying debug agent binary for local dev.

### 16.3 CI Extensions (`.github/workflows/ci.yml`)

| Job | Command |
|-----|---------|
| `station-build` | `xcodebuild -scheme "TradeAutopsy Station" -destination 'platform=macOS' build` |
| `station-tests` | `xcodebuild test -scheme "TradeAutopsy Station" -destination 'platform=macOS'` |
| Existing `notch-build` | Retain `swift build` in `notch/` |
| Existing `agent-tests` | Retain `cargo test` |

Optional follow-up job: spawn real agent for integration smoke.

---

## 17. Testing Requirements

### 17.1 Primary Seam

**`StationAppCoordinator` + injectable protocols** — all external behavior tests drive coordinator with fakes.

### 17.2 Required Test Cases

| ID | Scenario | Assert |
|----|----------|--------|
| T1 | Launch with healthy agent fake | No warning; notch start called; polling started |
| T2 | Launch timeout | `AgentHealthWarning.launchTimeout`; shell navigable |
| T3 | Port collision non-agent | Blocking warning with retry |
| T4 | Phase transition, no manual pick | `activeRoute` updates via `NavigationPolicy` |
| T5 | Manual Session pick | No auto-follow for 60s |
| T6 | Desk route persistence | Save/load round-trip |
| T7 | Launch route | Saved Desk restored; Session from phase |
| T8 | Retry after crash loop | Supervisor retry invoked; counter reset |
| T9 | Quit | Shutdown sequence: hotkeys unregistered, agent SIGTERM, notch dismiss |
| T10 | `⌥⇧Space` fake | `showAndActivate` recorded |
| T11 | Close window | `hide` only; agent still running |

### 17.3 `NavigationPolicy` Unit Tests

Pure function tests — no AppKit. Cover all `BarSurfacePhase` values, 60s stickiness boundary, Desk exclusion from auto-follow.

### 17.4 Deferred

- UI snapshot tests at 1100×700 (manual QA Phase 0)
- Full AppKit window server integration tests

### 17.5 Manual Smoke Checklist

1. Launch `.app` → status item visible, no Dock icon  
2. First launch → Station window opens once (no focus steal)  
3. Agent spawn → status ● green; strip populates when broker data available  
4. `⌥Space` / `⌥⇧Space` with and without Input Monitoring  
5. Nav placeholders switch; phase change updates Session route  
6. Kill agent → warning + degraded strip; Retry recovers  
7. Quit → no orphan `tradeautopsy-agent` process  

---

## 18. Implementation Order

Per PRD build order:

1. `StationAppCoordinator` + protocol fakes + `AgentSupervisor`
2. `StatusItemController`
3. `StationWindowController` + window chrome
4. `StationRoute` + placeholders + `NavigationPolicy`
5. `SessionPulseStrip` + shared `NotchViewModel` wiring
6. `HotkeyRegistrar` centralization
7. Host `NotchLauncher` (`isHostedByStation`)
8. Login Item toggle + first-run prompt
9. Quit / teardown path

---

## 19. Acceptance Criteria (Phase 0 Done)

Phase 0 is complete when all of the following hold:

- [ ] `TradeAutopsy Station.app` builds in CI and embeds `tradeautopsy-agent`
- [ ] App runs as menu-bar utility (no Dock icon)
- [ ] Agent spawned, health-checked, restarted (≤3/min), and torn down on Quit
- [ ] `STATION_DEV_ATTACH=1` skips spawn for local dev
- [ ] Port 9137 collision detected with attach-or-warn behavior
- [ ] Persistent agent warning with Retry, port guidance, log path
- [ ] Input Monitoring warning when global hotkeys unavailable
- [ ] Single `NotchViewModel`; no duplicate hotkeys from `NotchLauncher`
- [ ] `⌥Space` toggles Notch; `⌥⇧Space` opens/fronts Station
- [ ] Station window 1100×700, frame persisted, single instance, close hides only
- [ ] Ten route placeholders with Session/Desk sidebar (220px)
- [ ] `NavigationPolicy` enforces phase follow + 60s Session stickiness + Desk persistence
- [ ] `SessionPulseStrip` in Notch and Station with degraded `—` mode
- [ ] Launch at Login opt-in toggle + first-run prompt + SMAppService re-sync
- [ ] `StationTests` green in CI; manual smoke checklist passed
- [ ] `station-wire/v1.json` `station_version` pinned for release

---

## 20. Open Items (resolve before coding)

| Item | Owner | Blocker for |
|------|-------|-------------|
| Confirm ten `StationRoute` labels vs `Station.html` | Design | Sidebar labels, `StationRoute` enum |
| Triage vs Settings mapping from `BarNotchScreen` | Design | Route migration |
| Daemon secret env var names (match agent launch) | Engineering | `AgentSupervisor` spawn env |
| Bundle ID + code signing identity | Release | Xcode project |
| Agent log file path for warning UI | Engineering | `AgentHealthWarning.logPath` |

---

## 21. References

- [PRD-phase0-station-shell.md](./PRD-phase0-station-shell.md)
- `station-wire/v1.json` — wire contract
- `notch/BarNotchShell.swift` — prior nav/phase logic to migrate
- `notch/DaemonConnection.swift` — FSM test prior art
- `notch/NotchLauncher.swift` — hosting modifications
- `agent/src/api/health.rs` — health poll response shape
- Parent PRD: `FExEVIL/tradeautopsy` #161

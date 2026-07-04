# PRD — Phase 0: Station App Shell + Window Skeleton

**Status:** Ready for agent  
**Repo:** `FExEVIL/tradeautopsy-station`  
**Parent context:** TradeAutopsy Station v1 (`FExEVIL/tradeautopsy` #161)  
**Grill-me:** Complete (19 decisions, 2026-06-27)  
**Scope:** macOS app wrapper, coordinator, Station window chrome, nav placeholders — **no screen content**, **no Backend Box**, **no Notch HUD internals**

---

## Problem Statement

TradeAutopsy today ships as a Rust loopback **agent** and a Swift **Notch** library, but there is no shippable macOS **application**. Without a native app shell:

1. **No always-on entry point** — traders cannot Login-at-boot, menu-bar launch, or global hotkeys without a host process owning `NSApplication`.
2. **No main workspace** — the Notch pill is optimized for glance-and-intervene; sustained work (journal, brokers, patterns) needs a proper window with persistent nav — not a floating panel alone.
3. **No process supervision** — the agent must be spawned, health-checked, restarted, and torn down with the UI; leaving that to manual `cargo run` breaks the circuit-breaker promise at 9:15 AM.
4. **Split navigation risk** — `BarNotchShell` already implements sidebar routing inside the Notch; a separate Station window without a shared router will duplicate nav state and show conflicting screens.
5. **Silent failure modes** — missing Input Monitoring, port 9137 collisions, and agent crashes must surface actionable warnings, not empty UI or orphan processes.

Traders and engineers need a **Phase 0 shell**: one `.app` that owns lifecycle, routes Notch ↔ Station window ↔ agent, and renders empty placeholders for every nav route before screen PRDs land.

---

## Solution

Ship **TradeAutopsy Station.app** — an `LSUIElement` menu-bar utility (no Dock icon) whose core is **`StationAppCoordinator`**:

```
TradeAutopsy Station.app (station/)
├── StationAppCoordinator
│   ├── spawns/supervises tradeautopsy-agent (127.0.0.1:9137)
│   ├── owns single NotchViewModel (one polling/SSE pipe)
│   ├── owns StationRoute + NavigationPolicy
│   ├── registers global hotkeys (⌥Space, ⌥⇧Space)
│   └── drives NSStatusItem menu
├── NotchLauncher (hosted — hotkeys removed when embedded)
├── Station NSWindow (1100×700, unified toolbar, SessionPulseStrip)
│   ├── Sidebar 220px — Session + Desk sections
│   └── Empty placeholder per StationRoute
└── embedded agent binary at Contents/MacOS/tradeautopsy-agent
```

**One shared shell tree, two hosts:** the same nav + `SessionPulseStrip` render inside the Notch expanded surface and the Station window. `⌥Space` toggles Notch only; `⌥⇧Space` and status menu open/front the Station window.

**Degraded-but-running:** agent unhealthy → persistent warning with fix steps (not a hard block). Shell nav remains usable; pulse strip shows `—` without stale P&L.

**Phase 0 delivers the skeleton only** — every screen body is a labeled placeholder. Backend Box (broker/API key control plane) is a separate slice.

---

## User Stories

### App lifecycle & discoverability

1. As a macOS trader, I want TradeAutopsy Station to run without a Dock icon, so that it stays a utility alongside my broker terminal.
2. As a trader, I want a menu-bar status item with Open Station, Toggle Notch, Agent status, and Quit, so that I can control Station without hunting for a Dock tile.
3. As a new installer, I want Station to auto-open its window once on first launch, so that I can verify the app is running after install.
4. As a returning user, I want Login-at-boot to start Station silently (no window steal), so that the agent is ready before I open Kite without pulling focus from my broker.
5. As a trader, I want to press `⌥Space` to toggle the Notch overlay globally, so that intervention is reachable from any app when Input Monitoring is granted.
6. As a trader, I want to press `⌥⇧Space` to open or front the Station window, so that I can jump to the main workspace without dismissing the Notch.
7. As a trader, I want explicit open actions (menu, `⌥⇧Space`, pulse-strip click) to bring Station front and key, so that intentional navigation activates the workspace.
8. As a trader, I want first-run and silent boot window opens to use `orderFront` without stealing key from my broker, so that Station is visible but Kite stays focused when I didn't ask for focus.

### Agent supervision

9. As a trader, I want Station to spawn and supervise the local agent on launch, so that I never manually run `cargo run` in production.
10. As a trader, I want Station to health-check loopback `127.0.0.1:9137` on launch, so that I know the agent is alive before trading.
11. As a developer, I want `STATION_DEV_ATTACH=1` to skip spawn and attach to an existing `cargo run` agent, so that local dev stays fast.
12. As a trader with TradeAutopsy Tauri also installed, I want Station to detect port 9137 collisions, verify the existing listener is a TradeAutopsy agent, and attach — or show a clear error if not, so that I am never confused by a silent wrong process.
13. As a trader, I want a persistent warning when the agent is unhealthy with actionable fixes (Retry, check port conflict, show logs), so that I know how to recover without support.
14. As a trader, I want the shell to stay navigable while the agent is down, so that I can reach Settings and retry without the app feeling frozen.
15. As a trader, I want the agent to auto-restart up to three times per minute with backoff after a crash, so that transient failures self-heal.
16. As a trader, I want auto-restart to stop after repeated failure until I press Retry, so that runaway crash loops do not hide the problem.
17. As a trader, I want SSE/polling to reconnect automatically after agent recovery, so that live P&L resumes without relaunching Station.
18. As a trader, I want Quit to terminate the agent child cleanly (SIGTERM, 3s grace, SIGKILL), so that orphan processes do not hold port 9137.
19. As a trader, I want closing the Station window (red button) to hide only — not quit — so that the menu-bar app keeps running during live sessions.

### Login Item

20. As a trader, I want Launch at Login to be opt-in (default off), so that I explicitly consent before Station starts at boot.
21. As a trader, I want a Launch at Login toggle in Settings placeholder and status menu, so that I can enable it without System Settings hunting.
22. As a trader, I want a non-blocking first-run prompt (Enable / Skip) for Login at Login, so that onboarding is not a modal wall.
23. As a trader, I want in-app toggle state to re-sync if I disable Login Items in System Settings, so that UI reflects reality on next launch.

### Navigation & shared shell

24. As a trader, I want one canonical nav (`StationRoute`) for all ten screens, so that Notch and Station window never disagree on which screen is active.
25. As a trader, I want sidebar sections **Session** and **Desk**, so that time-bound flow is separated from reference work.
26. As a trader, I want Desk routes (Journal, Brokers, etc.) to stay put when trading phase changes, so that reading journal entries is not interrupted by auto-navigation.
27. As a trader, I want Session routes to follow trading phase automatically unless I manually picked a Session screen within the last 60 seconds, so that the shell reflects live context without fighting intentional navigation.
28. As a trader, I want `activeRoute` updated in the coordinator even when the Station window is hidden, so that opening Station later lands on the correct phase screen.
29. As a trader, I want collapsed Notch to not auto-expand on phase change in Phase 0, so that phase updates happen silently until I toggle the overlay.
30. As a trader, I want Desk routes restored on relaunch and Session routes resolved from current phase, so that I return to Journal but not to stale Pre-trade during live market.
31. As a trader, I want every nav item in Phase 0 to show a labeled empty placeholder, so that engineers can land screen PRDs without re-plumbing routing.

### Session pulse strip

32. As a trader, I want a Session pulse strip below the titlebar showing session P&L (realized today) and total unrealized P&L, so that I see risk at a glance in both Notch and Station hosts.
33. As a trader, I want open-position count in the strip, so that I know exposure without opening Live.
34. As a trader, I want a single-position symbol chip when only one position is open, so that the strip is informative without clutter.
35. As a trader, I want unhealthy agent/broker state to show `—` and amber indicator — never stale numbers — so that I am not misled during outages.
36. As a trader, I want clicking the pulse strip to navigate to Live trade, so that the strip is a shortcut to full positions detail.

### Window chrome

37. As a trader, I want the Station window default size 1100×700 with minimum 1100×700, so that layout matches the design canvas.
38. As a trader, I want window position and size persisted, so that Station reopens where I left it.
39. As a trader, I want only one Station window — reopen actions front the existing window, so that I never get duplicate shells.
40. As a trader, I want unified toolbar dark chrome with hidden title text and standard traffic lights, so that Station matches the Bar design system.
41. As a trader, I want the pulse strip full width above the sidebar, so that session status spans the entire window header.

### Permissions & security

42. As a trader, I want Station to detect missing Input Monitoring and show how to enable global `⌥Space`, so that global hotkeys are not silently broken.
43. As a security-conscious trader, I want the Notch and Station shell to call only loopback agent endpoints, so that README architecture invariants hold.
44. As an engineer, I want a single `NotchViewModel` instance owned by the coordinator, so that duplicate polling does not leak credentials or waste CPU.

### Engineering / release

45. As an engineer, I want `station/` with `@main` and a minimal Xcode project linking `notch/`, so that CI produces one `TradeAutopsy Station.app` artifact.
46. As an engineer, I want the release `.app` to embed `tradeautopsy-agent`, so that Login Item launches a self-contained bundle.
47. As QA, I want Phase 0 verifiable via menu bar, hotkeys, agent health, nav placeholders, and quit teardown without requiring screen content PRDs.

---

## Implementation Decisions

### Modules to build

- **`station/`** — new macOS app target: `StationApp` (`@main`), `StationAppCoordinator`, `AgentSupervisor`, `StationWindowController`, `StatusItemController`, `HotkeyRegistrar`, `SessionPulseStrip`, `StationShellView` (sidebar + placeholder router), `NavigationPolicy`, `StationRoute` enum.
- **`notch/`** — modify `NotchLauncher` to skip hotkey registration when hosted; optional thin adapter for shared shell injection; no new screen content.
- **`agent/`** — no feature changes; binary embedded by Xcode build phase. Health endpoint used for launch poll (existing agent HTTP surface).
- **Build** — minimal `TradeAutopsy Station.xcodeproj`; copy release/debug agent binary into `Contents/MacOS/tradeautopsy-agent`.

### StationRoute (canonical nav)

Provisional ten-case enumeration — **labels must match `Station.html` design handoff before implementation**; adjust only if handoff differs:

**Session**
1. Today  
2. Pre-trade  
3. Live trade  
4. Post-trade  

**Desk**
5. Journal  
6. Brokers  
7. Escrow match  
8. Patterns  
9. Fidelity score  
10. Settings  

`BarNotchScreen` maps to `StationRoute` for migration; `BarNotchScreen` retires over time. Sidebar width **220px** (Station window); reuse `BarDS` fill/text tokens.

### StationAppCoordinator responsibilities

- Owns single `NotchViewModel`; inject into `NotchLauncher` and `StationShellView`.
- Owns `@Published activeRoute: StationRoute` and applies `NavigationPolicy`.
- Delegates agent spawn/health/restart to `AgentSupervisor`.
- Sole registrar for `⌥Space` / `⌥⇧Space`; unregisters on `willTerminate`.
- Surfaces `AgentHealthWarning` and `InputMonitoringWarning` models for shared warning UI.
- Persists: window frame (UserDefaults), `activeRoute` for Desk routes only; Session routes recomputed from phase on launch.

### NavigationPolicy (prototype shape)

```swift
enum NavigationPolicy {
    static func routeForPhase(_ phase: BarSurfacePhase) -> StationRoute { ... }
    static func launchRoute(saved: StationRoute?, phase: BarSurfacePhase) -> StationRoute {
        if saved.isDesk { return saved }
        return routeForPhase(phase)
    }
    static func shouldAutoFollowPhase(
        active: StationRoute,
        phase: BarSurfacePhase,
        manualSessionPickAt: Date?,
        now: Date
    ) -> StationRoute? { ... } // nil = no change; 60s manual Session stickiness
}
```

Desk routes = Journal, Brokers, Escrow match, Patterns, Fidelity score, Settings. Session routes = Today, Pre-trade, Live trade, Post-trade.

### AgentSupervisor

- Spawn `Bundle.main` agent path; env for daemon secret/port from existing Station conventions.
- Launch poll: HTTP health until ready or ~10s timeout → `AgentHealthWarning`.
- Port collision: probe listener; attach only if wire handshake/secret verifies TradeAutopsy agent; else blocking warning.
- Runtime: auto-restart ≤3 attempts / 60s exponential backoff; cancel on Quit; notify coordinator on up/down for SSE reconnect.
- Shutdown: SIGTERM → 3s → SIGKILL.

### Station window

- Style: titled, closable, miniaturizable, resizable; `titlebarAppearsTransparent` + `fullSizeContentView`.
- `SessionPulseStrip` row below titlebar, full width; sidebar + content below strip.
- Close button → `orderOut`; Quit menu → full `coordinator.shutdown()`.

### Notch hosting

- `NotchLauncher.start()` after agent healthy (or degraded warning state).
- Remove duplicate global/local hotkey monitors from `NotchLauncher` when `isHostedByStation == true`.
- Expanded Notch: same `SessionPulseStrip` below existing top bar; shared `activeRoute` binding.

### Login Item

- `SMAppService` mainApp; register only on user toggle (Settings placeholder + status menu).
- Re-sync `status` on launch; first-run Enable/Skip prompt (non-blocking).

### Warnings (Grill #5 model)

Persistent banner or strip slot — not one-shot alert. Include: Retry, port-9137 guidance, log path, Input Monitoring link. Shell navigable; pulse strip degraded.

### Architecture invariants (unchanged)

- Notch/Station shell never call `tradeautopsy.in` directly.
- Agent never renders UI.
- No LLM inference in Station.
- Signals to brain via `ingestSignal()` only (shell does not add new egress).

---

## Testing Decisions

### Seam strategy (primary — one seam)

**`StationAppCoordinator` + injectable protocols** is the single highest seam for Phase 0 shell behavior. All external behavior tests drive the coordinator with fakes:

| Protocol | Fake for tests |
|----------|----------------|
| `AgentSupervising` | Simulate healthy / timeout / crash / port collision / restart success |
| `HotkeyRegistering` | Record registered handlers; simulate key events |
| `StationWindowControlling` | Record show/front/hide/frame without `NSWindow` |
| `StatusItemControlling` | Record menu actions |
| `LoginItemService` | Toggle without `SMAppService` |

**Goal:** one test target exercises launch, degraded warning, nav policy, quit teardown, and route persistence without AppKit window server.

### Secondary seams (existing — do not duplicate)

- **`NavigationPolicy`** — pure functions; unit tests only; no UI.
- **`DaemonConnectionFSM`** (`notch/DaemonConnection.swift`) — prior art for agent connection state; `AgentSupervisor` health FSM should follow same test style.
- **Pure reducers/presentations in `notch/`** — unchanged; not Phase 0 scope except injectable `NotchViewModel` deps (`BarDeclareHTTPExecuting`, `BarArchetypeStore`).
- **Rust `agent/tests/`** — `TestAgentOptions` harness for contract tests; shell smoke can spawn real agent in CI optional job.

### What makes a good test

- Test **observable shell behavior**: route after phase transition, warning visible when supervisor reports down, Retry invokes supervisor respawn, Quit calls shutdown sequence, Desk route persists mock save/load.
- Do **not** test private method order inside coordinator; test state outputs and fake protocol calls.
- UI snapshot tests for 1100×700 chrome **deferred** to manual QA in Phase 0.

### New test target

Add `NotchTests` or `StationTests` target to SwiftPM / Xcode with macOS 14+; run in CI alongside `swift build` (extend `.github/workflows/ci.yml`).

### Manual smoke checklist

1. Launch `.app` → status item visible, no Dock icon.  
2. First launch → Station window opens once.  
3. Agent spawn → status ● green; strip populates when broker data available.  
4. `⌥Space` / `⌥⇧Space` with and without Input Monitoring.  
5. Nav placeholders switch; phase change updates Session route.  
6. Kill agent → warning + degraded strip; Retry recovers.  
7. Quit → no orphan `tradeautopsy-agent` process.

---

## Out of Scope

- **Screen content** — Today, Journal, Brokers, Patterns, declare flows, kill switch UI internals, TAI, capture panels (placeholders only).
- **Backend Box** — OpenBB-style Backends / API Keys / Environments window (separate slice; coordinator may stub `openBackendBox()` no-op).
- **Notch HUD internals** — collapsed pill, warning cards, kill switch overlay (already in `notch/`).
- **Kill switch logic** — Station Slice B (done in agent + notch).
- **New wire protocol endpoints** — shell uses existing agent health/live-state/positions surfaces.
- **Windows/Linux Station** — macOS v1 only.
- **User-customizable hotkeys** — fixed `⌥Space` / `⌥⇧Space` in Phase 0.
- **Multiple Station windows** — single instance only.
- **HTTP graceful agent shutdown endpoint** — SIGTERM sufficient for Phase 0.

---

## Further Notes

- **Parent PRD** `FExEVIL/tradeautopsy` #161 covers full Station v1 (Kill Switch + Notch + Backend Box). This PRD is **Phase 0 only** — shippable shell before screen PRDs land.
- **Design handoff** `Station.html` — confirm exact ten `StationRoute` labels and section membership before coding sidebar; provisional list above may differ (e.g. Triage vs Settings).
- **Build order:** coordinator + agent supervisor → status item → window chrome → `StationRoute` placeholders → pulse strip + shared `NotchViewModel` → hotkey centralization → host `NotchLauncher` → Login Item toggle → quit path.
- **Wire contract:** pin `station_version` in `station-wire/v1.json` when Phase 0 ships.
- **C ABI** `tradeautopsy_notch_*` entry points in `NotchLauncher` may remain for transitional agent-driven launch; Phase 0 primary path is Swift coordinator calling `NotchLauncher` directly.

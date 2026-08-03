# Handoff: Station + PLAN-only Notch (keep / delete / next plans)

**Date:** 2026-07-28  
**Why this exists:** Disk is tight on this Mac. This doc is the single handoff so you can **delete unused trees safely** and still resume Station + Notch work without losing the plan stack.

**Canonical product tree to keep:** `/Users/bishnu/tradeautopsy-station/`  
**Approved UI mock:** `station/prototypes/NOTCH-ui.html` (PLAN-only expanded surface)

---

## 1. Where we are (code status)

### Done (do not redo)

| Work | Status | Notes |
|---|---|---|
| **Notch severance from Station desk** | DONE | Desk uses `SessionModel` + `StationDS`; no Station shell inside Notch |
| **Normal macOS window chrome** | DONE | Dock app, close hides, ⌘Q quits |
| **PLAN HTML identity** | DONE / APPROVED | `NOTCH-ui.html` = BarNotchShell only (no PULSE/BRIEF/CAPTURE tabs) |
| **PLAN-only floating Notch reintegration** | DONE (code) | Phases 1–4 implemented |

**What shipped in reintegration:**
- Notch package: `planSurfaceOnly` when `NotchLauncher(isHostedByStation: true)` — expand always lands on `BarNotchShell`
- Station SPM: `.package(path: "../notch")` again
- `FloatingNotchHost` → owns launcher; **no** `setHostedExpandedContent`
- Launch (agent healthy): `configure` + `start()` → collapsed pill
- **⌥Space** → floating Notch toggle; **⌥⇧Space** → open Station
- Quit → `floatingNotch.dismiss()`
- Tests: coordinator / hotkey / plan-surface-only green

**Manual QA (2026-07-28):** DONE
- [x] Collapsed pill after Station launch
- [x] ⌥Space → BarNotchShell (Morning brief default)
- [x] No other Notch tab chrome
- [x] Kill-switch overlay + Fn dictation
- [x] Desk + ⌥⇧Space still work

**PLAN realign (2026-07-28):** DONE — fail-closed `planSurfaceOnly` → always `BarCircuitPanelView`; Settings loss-limits via agent `/api/daemon/bar/profile/loss-limits` → Console `/api/bar/v1/profile/loss-limits`.

### Source of truth (Console + Station)

| Layer | Owner now |
|---|---|
| Desk (Today / Brokers / Settings window) | Station `SessionModel` / `StationDS` |
| Expanded Notch | **PLAN only** → `BarNotchShell` / `BarCircuitPanelView` (no PULSE/BRIEF/… tabs) |
| Live PLAN data | Agent `/api/daemon/bar/*` → Console `/api/bar/v1/*` (**until Phase Z**) |
| Approved UI mock | `station/prototypes/NOTCH-ui.html` |
| Web `/dashboard/bar` | Culled — do not restore |

### Architecture (locked)

```
Station desk window     → SessionModel / StationDS / SessionPollingHost
Floating Notch pill     → NotchViewModel / BarNotchShell (PLAN only)
Agent (port 9137)       → dual pollers OK; bar routes proxy to Console until Phase Z
Console /api/bar/v1     → hosted PLAN brain (declare, live-state, loss-limits, …)
```

Do **not** re-host full-tab Notch or embed Station desk routes inside expanded Notch.
---

## 2. All plans — what they are, keep or ignore

### A. Active / authoritative for Station+Notch

| Plan | Path | Role |
|---|---|---|
| **Station Notch — PLAN-only floating panel** | `~/.cursor/plans/Notch PLAN reintegration-a6d98bab.plan.md` | **Authoritative.** PLAN-only host. **Code done;** manual QA left. |
| Approved mock | `tradeautopsy-station/station/prototypes/NOTCH-ui.html` | Visual identity for expanded PLAN |

### B. Superseded / historical (keep one copy if you want history; not needed to build)

| Plan / handoff | Path | Status |
|---|---|---|
| Full-tab Notch reintegration | `tradeautopsy-station/plans/notch-floating-panel-reintegration.md` | **SUPERSEDED** by PLAN-only plan (full PULSE…CAPTURE tabs cancelled for Station) |
| Notch severance handoff | `tradeautopsy-station/NOTCH_SEVERANCE_HANDOFF.md` | **DONE but STALE** mid-doc (says Phases 2–5 not started). Severance finished; then Notch was **re-linked** for floating panel only. Safe to archive/delete after reading §1 above. |
| Severance design (Claude) | `~/.claude/plans/sprightly-yawning-oasis.md` | Historical design for severance |

### C. Other Station plans (orthogonal — not Notch)

| Plan | Path | Notes |
|---|---|---|
| Today v1 | `tradeautopsy-station/plans/today-v1.md` | Desk Today screen |
| Broker marketplace handoff | `tradeautopsy-station/plans/broker-marketplace-handoff.md` | Brokers |
| Architecture deepening | `tradeautopsy-station/plans/architecture-deepening-cross-repo-handoff.md` | Cross-repo |
| Audit current vs planned | `tradeautopsy-station/AUDIT_CURRENT_VS_PLANNED.md` | Snapshot doc |

### D. Next plans (what we are going to do after cleanup)

Order recommended:

1. ~~**Manual QA of PLAN-only Notch**~~ — **DONE** 2026-07-28.
2. **Polish / bugs from QA** — only if regressions appear vs `NOTCH-ui.html`.
3. **Optional later (not started):**
   - Status menu “Toggle Notch” (hotkey is source of truth today)
   - Reduce dual-poller cost if battery/CPU matters (not required for parity)
   - Sync/align local `notch/` with tauri `TradeAutopsyNotch` if that tree returns (Untitled Sources was empty)
4. **Do not** put Station desk routes inside expanded Notch.
5. **Do not** restore PULSE/BRIEF/… tabs for Station-hosted Notch unless product explicitly reverses PLAN-only.

---

## 3. Disk map — what eats space (~21G in tradeautopsy-station alone)

| Path | ~Size | Keep? |
|---|---|---|
| `tradeautopsy-station/agent/target` | **~18G** | **DELETE OK** (Rust build cache). Rebuild with `cargo build` when needed. |
| `tradeautopsy-station/station/.build` | **~1.5G** | **DELETE OK** (SwiftPM). Rebuild with `cd station && swift build`. |
| `tradeautopsy-station/notch/.build` | **~1.0G** | **DELETE OK**. Rebuild with `cd notch && swift build`. |
| `~/Library/Developer/Xcode/DerivedData` | **~1.6G** | **DELETE OK** (Xcode cache). |
| `tradeautopsy-daemon-tauri` | **~6.4G** | **Not required** for Station+Notch day-to-day if you only ship Station. Delete/archive if you are not actively developing that app. |
| `tradeautopsy-relay` | **~685M** | Keep only if you need relay; else archive. |
| `tradeautopsy-behavioral-engine` | **~135M** | Keep if agent/brain work; else archive. |
| `tradeautopsy-tauri` | **~3.5M** | Tiny; optional. `Sources/TradeAutopsyNotch` was missing before — Station uses **local** `notch/`. |
| `~/.cursor/plans` | **~5M** | Many unrelated plans (aero-glass, etc.). Safe to prune old `.plan.md` files; keep the PLAN-only Notch plan if you still want it. |
| `~/.cursor/projects` | **~250M** | Cursor project metadata/transcripts. Can prune old projects carefully. |

### Safe reclaim commands (build caches only — ~20G+)

```bash
# From tradeautopsy-station — regenerable build artifacts
rm -rf agent/target
rm -rf station/.build notch/.build
rm -rf ~/Library/Developer/Xcode/DerivedData/*

# Optional: cargo clean if you prefer
# (cd agent && cargo clean)
```

**Do not delete these source trees if you want to continue Station+Notch:**

```
tradeautopsy-station/station/     # Station app
tradeautopsy-station/notch/       # Notch package (required path dep)
tradeautopsy-station/agent/src/   # Agent sources (not target/)
tradeautopsy-station/station/prototypes/NOTCH-ui.html
tradeautopsy-station/Package / SPM sources under station/ + notch/
```

### Prototypes (tiny; keep vs delete)

| File | Keep? |
|---|---|
| `NOTCH-ui.html` | **KEEP** — approved PLAN identity |
| `CURRENT-station.html` | Keep if desk UI still referenced |
| `TERMINAL-station-shell.html` | **DELETE OK** — wrong direction (CLI shell mock) |
| `PROTOTYPE-*.html` / `.js` | Optional; not required for Notch |

### Docs that are stale / deletable after this handoff

| File | Action |
|---|---|
| `NOTCH_SEVERANCE_HANDOFF.md` | Archive/delete — superseded by this handoff + code |
| `plans/notch-floating-panel-reintegration.md` | Archive/delete — full-tab plan superseded by PLAN-only |
| This file | **KEEP** until next major milestone |

---

## 4. Key files to resume coding (after reclaim)

**Station**
- `station/StationApp/FloatingNotchHost.swift`
- `station/StationApp/StationApp.swift` — configure + inject host
- `station/StationApp/StationAppCoordinator.swift` — start/toggle/quit/hotkeys
- `station/Package.swift` — `../notch` dependency
- `station/StationApp/Session/` — desk `SessionModel` (do not merge into Notch)

**Notch**
- `notch/NotchLauncher.swift` — `isHostedByStation`
- `notch/NotchViewModel.swift` — `planSurfaceOnly` / `enablePlanSurfaceOnly()`
- `notch/BarNotchShell.swift` + `BarCircuitPanelView.swift` — PLAN UI
- `notch/Tests/NotchTests/PlanSurfaceOnlyTests.swift`

**Hotkeys**
- ⌥Space → `FloatingNotchHost.toggle()`
- ⌥⇧Space → `openStation()`

---

## 5. Resume checklist (new machine / after delete)

1. Keep `tradeautopsy-station` git repo (or push first, then clone).
2. Delete `agent/target`, `station/.build`, `notch/.build`, DerivedData (see §3).
3. Rebuild when ready:
   ```bash
   cd /Users/bishnu/tradeautopsy-station/notch && swift test --filter PlanSurfaceOnlyTests
   cd /Users/bishnu/tradeautopsy-station/station && swift build && swift test --filter 'StationAppCoordinatorTests|HotkeyRegistrarTests'
   ```
4. Run agent (rebuild if `target` wiped), then StationApp → manual QA checklist §1.
5. Next product work = QA fixes only unless a new plan is written.

---

## 6. One-line summary

**Sever desk from Notch → re-host floating PLAN-only Notch in Station → code + manual QA + PLAN realign done (2026-07-28). Live PLAN data stays agent→Console `/api/bar/v1` until Phase Z. Keep `station/`, `notch/` sources, and `NOTCH-ui.html`.**

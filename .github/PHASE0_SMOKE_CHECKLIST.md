# Phase 0 Smoke Checklist

Manual verification steps for Phase 0 Station App Shell release.
Run on a real Mac (not simulator) before closing the Phase 0 milestone.

---

## 1. Menu bar + no Dock icon

- [ ] Build and run. App appears in menu bar (TradeAutopsy icon).
- [ ] App does NOT appear in the Dock.
- [ ] App does NOT appear in Cmd+Tab switcher.

## 2. First launch window

- [ ] On first launch, Station window opens automatically.
- [ ] Window is 1100×700, titled "TradeAutopsy Station".
- [ ] Window has sidebar nav with Session + Desk groups.

## 3. Agent health + pulse strip

- [ ] Pulse strip is visible in Station window header.
- [ ] If agent is healthy: pulse strip shows active state (not degraded).
- [ ] If agent fails to start: pulse strip degrades and warning banner appears.
- [ ] Kill the agent process manually → warning banner appears within ~5 seconds.
- [ ] Agent restarts automatically after kill.
- [ ] Pulse strip recovers (leaves degraded state) after agent restart — no manual action needed.

## 4. Hotkeys

- [ ] ⌥Space — toggles Notch open/closed.
- [ ] ⌥⇧Space — opens Station window (brings to front if already open).
- [ ] Hotkeys work when Station window is NOT focused.
- [ ] Hotkeys work when app is in background.
- [ ] If Input Monitoring permission not granted: warning banner appears in Station window.
- [ ] Granting Input Monitoring permission clears the banner.

## 5. Navigation + placeholders

- [ ] All 10 sidebar routes are present: Today, Pre-trade, Live trade, Post-trade, Journal, Brokers, Escrow match, Patterns, Fidelity score, Settings.
- [ ] Clicking each non-Brokers, non-Settings route shows placeholder view.
- [ ] Brokers route shows real BrokersView (Backend Box v1).
- [ ] Settings route shows Launch at Login toggle.

## 6. Notch

- [ ] ⌥Space opens expanded Notch.
- [ ] Expanded Notch shows same pulse strip and nav as Station window.
- [ ] Navigating in Notch updates Station window route (shared state).
- [ ] Navigating in Station window updates Notch route (shared state).
- [ ] Notch does NOT auto-expand when route changes programmatically.

## 7. Kill agent → warning + Retry

- [ ] Kill agent manually (Activity Monitor or `pkill`).
- [ ] Warning banner appears in Station window.
- [ ] "Retry" button is visible.
- [ ] Clicking Retry restarts agent and clears warning.
- [ ] Pulse strip recovers after Retry.

## 8. Launch at Login

- [ ] Open Settings route.
- [ ] Toggle "Launch at Login" → ON.
- [ ] Quit app. Reboot (or log out/in).
- [ ] App appears in menu bar automatically on login.
- [ ] Toggle "Launch at Login" → OFF.
- [ ] Reboot. App does NOT appear on login.
- [ ] Status menu toggle and Settings toggle stay in sync.

## 9. Quit → no orphan agent

- [ ] Quit from status menu → app terminates.
- [ ] `pgrep tradeautopsy-agent` (or equivalent) returns nothing — no orphan process.

## 10. Wire pin

- [ ] `cat station-wire/v1.json` shows `"station_version": "0.2.0"` and `"phase": "phase-0-shell"`.

---

_Sign off: [ ] All checks passed. Date: _______ Build: _______

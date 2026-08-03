# Console + Station — share twin (for review on phone or desktop)

**Date:** 2026-07-28  
**Audience:** Friend / reviewer who needs the **current** Console + Station shape without installing the Mac app.  
**Open this:** [`CONSOLE-STATION-SHARE.html`](./CONSOLE-STATION-SHARE.html) (AirDrop the HTML, or open in Safari/Chrome).

---

## What this is

A **full offline UI twin** of:

| Product | What you get |
|--------|----------------|
| **Station** | Desk window — pulse strip, all sidebar routes, operable Today / Brokers / settings-style panels |
| **Console** | Thin personal brain — Journal · Trades · Workspace · Insights · thin Settings (**not** a second desk) |
| **Notch** | PLAN-only floating surface — pill, expand, Morning brief → Settings (no PULSE/BRIEF/CAPTURE tabs) |

No live agent, Keychain, TOTP, or Console HTTP. Sample data + local clicks only.

---

## Current ownership (locked)

| Layer | Owner now |
|-------|-----------|
| Desk (Today / Brokers / Settings window) | **Station** (`SessionModel` / `StationDS`) |
| Expanded Notch | **PLAN only** → `BarNotchShell` / `BarCircuitPanelView` |
| Live PLAN data (real app) | Agent `:9137` → Console `/api/bar/v1/*` **until Phase Z** |
| Web `/dashboard/bar` | Culled — do not restore |
| Console Keep | Journal, Trades, Workspace, Insights, thin prefs — **not** Org / Algo Labs fat / fog dashboard |

Authoritative plan: `Tradeautopsy1/Untitled/tradeautopsy/founder-issues/CONSOLE-STATION-EXECUTION-PLAN.md`  
Station+Notch handoff: `tradeautopsy-station/STATION_NOTCH_SPACE_HANDOFF.md`

---

## What’s inside (data-rich)

Console tab is seeded from TradeAutopsy repo  
`Tradeautopsy1/Untitled/tradeautopsy/lib/data/sampleTrades.ts` (~46 Indian market trades):

| Screen | Data |
|--------|------|
| **Trades** | Full table + net P&L / win rate / filters (wins, losses, revenge, options) + search |
| **Journal** | 28 entries with clean / revenge / review tabs |
| **Calendar** | Day cells with P&L from the sample month |
| **Insights Hub** | 4 impact cards |
| **Pattern Registry** | 6 patterns (confirmed + pending) |
| **Performance** | By-symbol and by-strategy breakdowns |
| **Workspace** | 8 Day-trader widgets |

Nav matches Console Keep set: Journal · Trades · Import · Workspace · Calendar · Insights · Patterns · Settings.

---

## How to open on a phone

1. **AirDrop / Files** — send `CONSOLE-STATION-SHARE.html` alone (self-contained). Open in Safari.
2. **Same Wi‑Fi server** (optional):
   ```bash
   cd tradeautopsy-station/station/prototypes
   python3 -m http.server 8765
   ```
   Then on phone: `http://<your-mac-lan-ip>:8765/CONSOLE-STATION-SHARE.html`
3. Use the top switcher: **Station | Console | Notch**.

On phone the UI goes **full-bleed** (no fixed 1100×700 desktop frame).

---

## What should work (checklist)

- [ ] Switch Station / Console / Notch
- [ ] Station: every sidebar route opens a real panel; dual / degraded / breaker toggles affect Today
- [ ] Station Brokers: Connect / Start / Stop / Re-auth flip local status
- [ ] Console: Journal add note; Trades filter; Workspace select cards; Insights / Settings toggles
- [ ] Notch: expand/collapse pill; all PLAN sidebar screens; + New trade → Pre-trade; gear → Settings
- [ ] Notch **Capture to Journal** → jumps to Console Journal with a seeded capture entry

---

## Related mocks (desktop-sized)

| File | Role |
|------|------|
| `CURRENT-station.html` | Station shell SoT (desktop frame) |
| `NOTCH-ui.html` | PLAN Notch SoT (desktop frame) |
| `PROTOTYPE-station.html` | **Older** speculative mock — not current plan |

This share file is the one to send when someone needs **phone + full product twin**.

# PROTOTYPE — TradeAutopsy Station

**Notch · summon motion (UI, 2026-09-28):**  
[`PROTOTYPE-notch-spotlight-summon.html`](./PROTOTYPE-notch-spotlight-summon.html) — Spotlight glass / island morph / current cut-off close. Notes: [`PROTOTYPE-notch-spotlight-summon.NOTES.md`](./PROTOTYPE-notch-spotlight-summon.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-spotlight-summon.html
```

**Current plan twin (phone + desktop):**  
[`CONSOLE-STATION-SHARE.html`](./CONSOLE-STATION-SHARE.html) + [`CONSOLE-STATION-SHARE.md`](./CONSOLE-STATION-SHARE.md)  
Station desk · thin Console · PLAN Notch — fully clickable offline.

**Notch · Apple HIG × trading desk (UI, 2026-09-12):**  
[`PROTOTYPE-notch-desk.html`](./PROTOTYPE-notch-desk.html) — ticker, ticket, ladder, blotter on glass. Notes: [`PROTOTYPE-notch-desk.NOTES.md`](./PROTOTYPE-notch-desk.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-desk.html
```

**Notch · Harness top bar (UI, 2026-09-17):**  
[`PROTOTYPE-harness-topbar.html`](./PROTOTYPE-harness-topbar.html) — Apple chrome for books + holes. **Verdict A shipped** in Notch `BarNotchShell`. Notes: [`PROTOTYPE-harness-topbar.NOTES.md`](./PROTOTYPE-harness-topbar.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-harness-topbar.html
```

**Notch · closed pill at 1:1 menu-bar scale (UI, 2026-09-18):**  
[`PROTOTYPE-notch-closed-pill.html`](./PROTOTYPE-notch-closed-pill.html) — 24pt strip, Apple fluid press/drag, A island+ears / B menu extra / C split ears. **Verdict A shipped** in Notch closed chrome. Notes: [`PROTOTYPE-notch-closed-pill.NOTES.md`](./PROTOTYPE-notch-closed-pill.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-closed-pill.html
```

**Notch · Options planning board (UI, 2026-09-18):**  
[`PROTOTYPE-notch-options-planning.html`](./PROTOTYPE-notch-options-planning.html) — Session · OI · Payoff · Chain · Depth · Greeks + full Plan on one linked board. A mosaic / B hero+twins / C focus+floor. Notes: [`PROTOTYPE-notch-options-planning.NOTES.md`](./PROTOTYPE-notch-options-planning.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-options-planning.html
```

**Notch · Options dashboard builder (UI, 2026-09-18):**  
[`PROTOTYPE-notch-options-dashboard.html`](./PROTOTYPE-notch-options-dashboard.html) — customizable Pre-trade. **Plan on the cockpit; risk board after Confirm** (at-stop + at-target, equity-if-SL + equity-if-TP). Plan dock pinned. DualNoBlend. Notes: [`PROTOTYPE-notch-options-dashboard.NOTES.md`](./PROTOTYPE-notch-options-dashboard.NOTES.md).

```bash
cd station/prototypes && ./serve-prototype.sh
```

`?asset=options&board=cockpit`

**Notch · Risk layout zoo (UI, 2026-09-18):**  
[`PROTOTYPE-notch-risk-engine.html`](./PROTOTYPE-notch-risk-engine.html) — A/B/C **layout zoo only** (not the product flow). SL-side hierarchy experiments: A mosaic / B account-path / C HUD-on-session. Product path is the options dashboard after Confirm (at-stop + at-target). Do not merge into the dashboard host. Notes: [`PROTOTYPE-notch-risk-engine.NOTES.md`](./PROTOTYPE-notch-risk-engine.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-risk-engine.html
```

**Notch · Options plot density (UI, 2026-09-18):**  
[`PROTOTYPE-notch-options-plot.html`](./PROTOTYPE-notch-options-plot.html) — OpenAlgo Charts density (lines / DOM / last-N), not their feed. Live Notch chrome. Notes: [`PROTOTYPE-notch-options-plot.NOTES.md`](./PROTOTYPE-notch-options-plot.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-options-plot.html
```

**Notch · Spot session OMS (UI, 2026-09-18):**  
[`PROTOTYPE-notch-spot-session.html`](./PROTOTYPE-notch-spot-session.html) — live Notch sidebar. Options default: OI bar chart of venue rows. Spot/Equity form unchanged. Notes: [`PROTOTYPE-notch-spot-session.NOTES.md`](./PROTOTYPE-notch-spot-session.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-spot-session.html
```

**Station · desk shell (UI, 2026-09-18):**  
[`PROTOTYPE-station-threads.html`](./PROTOTYPE-station-threads.html) — broker + TradingView density × WWDC 26 Liquid Glass. DualNoBlend. Not WhatsApp. Notes: [`PROTOTYPE-station-threads.NOTES.md`](./PROTOTYPE-station-threads.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-station-threads.html
```

**Notch · Apple HIG catalog (UI, 2026-09-12):**  
[`PROTOTYPE-notch-hig.html`](./PROTOTYPE-notch-hig.html) — collapsed pill + expanded PLAN, SF-style symbols. Notes: [`PROTOTYPE-notch-hig.NOTES.md`](./PROTOTYPE-notch-hig.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-notch-hig.html
```

**Today · one day + now (UI, 2026-09-12):**  
[`PROTOTYPE-today-one-day.html`](./PROTOTYPE-today-one-day.html) — fuse of Overview (one local day) and risk-layers A (open-now blotter). Notes: [`PROTOTYPE-today-one-day.NOTES.md`](./PROTOTYPE-today-one-day.NOTES.md).

```bash
open station/prototypes/PROTOTYPE-today-one-day.html
```

**Binance Options Pre-trade lights (logic, 2026-09-10):**  
[`PROTOTYPE-binance-options-pretrade-light.html`](./PROTOTYPE-binance-options-pretrade-light.html) — which panels may paint venue strings vs lock-dark. Plan: [`../../plans/binance-options-pretrade-light.md`](../../plans/binance-options-pretrade-light.md). Double-click the HTML. Not the BANKNIFTY `notch-options-declare` fixture toggle.

```bash
open station/prototypes/PROTOTYPE-binance-options-pretrade-light.html
```

**Question:** What should Today v1 + Stats look like, fully interactive?

**UI run:**
```bash
open station/prototypes/PROTOTYPE-station.html
```

**Engine run (round-trip + WAC logic):**
```bash
cargo run --manifest-path agent/Cargo.toml --bin prototype-today-engine
```

See `agent/prototypes/LOGIC.md` for engine scenarios.

Must open HTML from the `prototypes/` folder (or use a local server) so `PROTOTYPE-station-metrics.js` loads.

---

## Screens

### Today (`?variant=A|B|C` + bottom bar / ← →)

| Variant | State |
|---------|--------|
| **A** | Active — circuit breaker + countdown resume, hero, 2 signals, trades |
| **B** | Honest empty — all `—`, learning baseline |
| **C** | Degraded — sync paused |

**Functional:**
- Sidebar navigation
- Circuit breaker countdown (72s) → enables Review & resume → dismisses banner
- Pulse strip updates per variant

### Stats (click **Stats** in sidebar)

Apple Health aesthetic scoped to `#s-stats`:
- Orange `#ff6a13`, cards `#1c1c1e` / `#161618`, blue `#0a84ff`
- Left rail 252px — Behavior + Performance metric lists
- Click row → `stSelect()` → chart + 3 highlight cards update
- D / W / M / 6M / Y segmented control → chart only
- Bar colors: green / amber / red by threshold (inverted for Trades/day)
- Animated bar heights on switch
- Export button → toast "not wired"

---

## Files

| File | Purpose |
|------|---------|
| `PROTOTYPE-station.html` | Main interactive mock |
| `PROTOTYPE-station-metrics.js` | Stats METRICS data (from design handoff) |
| `PROTOTYPE-today-screen.html` | Superseded — use `PROTOTYPE-station.html` |

---

## Verdict

_(fill after review)_

**When done:** Delete `station/prototypes/` or fold into SwiftUI.

# PROTOTYPE — TradeAutopsy Station

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

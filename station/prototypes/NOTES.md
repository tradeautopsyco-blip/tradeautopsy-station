# PROTOTYPE — TradeAutopsy Station

**Current plan twin (phone + desktop):**  
[`CONSOLE-STATION-SHARE.html`](./CONSOLE-STATION-SHARE.html) + [`CONSOLE-STATION-SHARE.md`](./CONSOLE-STATION-SHARE.md)  
Station desk · thin Console · PLAN Notch — fully clickable offline.

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

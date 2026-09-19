# PROTOTYPE — throwaway

**Question:** Given three SL-only board hierarchies (A mosaic / B path / C HUD), which layout reads clearest? This file is layout comparison only — not the shipping risk product.

Not answering: production math, `docs/reference/` (none exists yet for fixed-fraction), remaining-risk as live truth (T8), margin / SPAN / σ rungs, Kelly plugins, a new sidebar “Risk” screen, shipping Swift.

**Plan:** Three structurally different boards on `PROTOTYPE-notch-risk-engine.html?variant=&asset=&role=`. Host is live Notch chrome (SESSION rail, island). Only hierarchy swaps. Qty is **authored** by a Nautilus-shaped fixed-risk helper in this file (demo). Calm ≥ 4 actually halves. No stop → do not invent one; Confirm / Add blocked.

**Merged host:** keep **this** file for A vs B vs C. Sizer + gate also live on the dashboard cockpit — [`PROTOTYPE-notch-options-dashboard.html?asset=options&board=cockpit`](./PROTOTYPE-notch-options-dashboard.html?asset=options&board=cockpit). TP / at-target / equity-if-TP live only on the dashboard after Confirm. Do not add them here unless founder asks.

Locked from the grill (2026-09-18): sizer + gate; detect is a circuit when a stop exists; one physics, role **circuits** not role formulas. Q9-C “cockpit is the risk product” is **superseded** by dashboard Confirm-split (planning cockpit → risk board after declare).

## What we already have (live, not invented)

| Zone | Live / lock |
|------|-------------|
| Notch chrome | `BarNotchShell` · Pre-trade |
| Rung 1 | typed qty × (stop − entry) · `BarPlanLadder` |
| Detect | plan vs live SL · does not invent a stop · DualNoBlend |
| Calm ≥ 4 | copy only in production (“size halved”) |
| Margin | unavailable (venue engine) |
| Options σ | rungs 2–5 dark |

This prototype **changes the hierarchy**: risk numbers are the board; glance charts shrink.

## Variants

| Key | Name | Structure |
|-----|------|-----------|
| A | Risk mosaic | Four risk cells own the grid. Charts are a filmstrip. Thin ticket rail. Confirm pinned. |
| B | Account path | One chart: equity if SL. Ticket is a floor strip. No OI / payoff. |
| C | HUD on session | Session full-bleed (decoration you can still read). Sizer is a glass HUD. OI/payoff ghosts. Confirm in the HUD. |

## Law (demo)

- DualNoBlend: switching Spot (USDT) / Equity (INR) / Options (USDT) never blends %.
- Equity / spot / option **buyer**: stop-distance sizer. Option **seller**: measurement dark (Q8).
- Override qty above authored → Confirm disabled (gate).
- Fill with no SL → detect `noForm`; cannot add.
- Demo · not live. Formula is not a `docs/reference/` source.

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-risk-engine.html
```

`?variant=A|B|C&asset=equity|spot|options`

**Verdict:** _(fill after you choose)_

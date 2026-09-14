# PROTOTYPE — throwaway

**Question:** What should Station look like as a Mac app when Today is **one local day** (happened + open now), and Journal / Backend Box / Settings are real destinations — Apple HIG, not a stub?

**Fuse of**
- [`STATION-OVERVIEW.html`](./STATION-OVERVIEW.html) — remaining, one chart, trips. No week strip.
- [`PROTOTYPE-risk-layers.html?variant=A`](./PROTOTYPE-risk-layers.html?variant=A) — open-now blotter, detection, Console drawer.
- [`STATION-MERGED.html`](./STATION-MERGED.html) / [`STATION-STUDIO.html`](./STATION-STUDIO.html) — Journal save-state, Backend Box workbench.

**HIG (2026 fetch):** macOS split-view sidebar; inset grouped lists; Liquid Glass on chrome, thicker/more opaque sidebar; system font + size-specific tracking; mini switches in grouped forms; materials thin / regular / thick; Settings as grouped Form in the sidebar destination (not a toolbar gear).

**Not answering:** T8 remaining-risk as live truth, blending closed P&L into open MTM, TradingView, Pre/Live/Post as Station tabs.

**Run**
```bash
open station/prototypes/PROTOTYPE-today-one-day.html
# Journal / Backend / Settings:
# file://…/PROTOTYPE-today-one-day.html?variant=A&route=journal
```

**Today variants** (← →)

| Key | Name | Hierarchy |
|-----|------|-----------|
| A | Day spine | Chart + remaining first. Open now under it. |
| B | Now is the desk | Blotter A first. Day chart is a strip. |
| C | Split clocks | Left = happened. Right = open now. |

**Sidebar routes:** Today · Journal · Backend Box · Settings.

**Law:** hero P&L is closed round trips only. DualNoBlend INR. Limits in Settings sync to Notch; they do not fire Kill. Capture happens on Notch. Journal is every **pre-trade declaration** that POSTed to Console (matched, pending, expired, cancelled) plus pre / live / post, fidelity, day sheet — not save-chips only.

**Journal (from Console + Notch, 2026-09-12):** frozen snapshot (kind, setup chip, calm 1–5, confidence 1–5, SL, target, invalidation, SL consent), match status, impulsive fill with no `declarationId`, fidelity dimensions (process vs plan, not P&L), notebook day sheet, emotion / signal tags, chart shots, voice, search + facets. Today A/B/C stay; **C already holds happened + now.**

**Verdict:** _(fill after review)_

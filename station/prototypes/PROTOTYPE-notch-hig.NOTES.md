# PROTOTYPE — throwaway

**Question:** What should TradeAutopsy Notch look like as Apple HIG **once every production surface is on the desk** — not a skinny chrome mock?

**Rule:** list the Notch, then draw it. Production SoT is `BarNotchShell` + collapsed host. This file is the interactive twin.

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-hig.html
```

**Variants** (← →) are **chrome only**. Content is the same catalog.

| Key | Name | Hierarchy |
|-----|------|-----------|
| A | PLAN sidebar | Production shell, HIG materials |
| B | Icon rail | 56px symbols · HUD |
| C | Segmented | No sidebar · session in the title |

---

## Catalog (must all exist in the HTML)

### Host
- Collapsed pill 32×176: impact track (`N↓` / `—`) **or** intervention keyword (`EXPIRED` / `COOLING` / `LIMIT`)
- Press 0.97 · 10pt drag hysteresis (click expands)
- Expand grabber · outside/Esc collapse
- Station ⌥Space = expand/collapse (does not hide)
- Hide row = hide pill until ⌥Space

### Shell
- Sidebar 176: Session (Morning, Pre-trade, Live, Post-trade) · More analysis · Hide · archetype + gear
- Morning badge `1` · Live `!` + unposted count
- Top bar: title · composite `0.72` · multiplier `×0.85` · broker · Quote · Instruments · Account · state pill
- Loading strip · error + Open Station · Bar features gate

### Morning
- Pre-market: Nifty / BNF / VIX
- What matters + yesterday P&L takeaway
- KEY METRICS: Session P&L · Adherence · Win rate · Left on table
- Start trading → Pre-trade

### Pre-trade (four surfaces)
- Standard: calm 1–5 · confidence 1–5 · entry/SL/target/symbol · BUY/SELL · qty · setup chips · invalidation · auto-place SL · Confirm
- NFO three-zone: Analytics (session/chain/OI holes) · Consequence (legs, greeks, payoff, ladder) · Plan
- Crypto Options: USDT · catalog bind · klines · settlement · no lots
- Swing: 6-step wizard (check-in → risk → hold → invalidation → review → overnight)

### Live
- Detect card (undeclared / no form)
- Metric strip + plan snapshot
- Interference No / Maybe / Yes
- Live capture (Voice · Screenshot · unposted tray)
- Protective SL status only (no Set SL)
- Kill → warning Confirm/Cancel → overlay countdown + I'm Calm
- Exit gate Q1–Q3
- Armed: declaration on file · waiting for fill
- Desk B only: equity-if-SL box (not remaining-risk)

### Post
- Moments A (outcome 2:00) · B (process Yes/No) · C (one sentence) · Submit debrief · Dismiss

### Analysis
- Escrow: 7 ledger slots + 60pt ring
- Patterns: setup mix (placeholder)
- Fidelity: 14-session line + escrow %
- Triage: empty vs open positions

### Settings tabs
- Broker (sync + account pulse/ledger DualNoBlend)
- Risk limits
- Behavior (archetype)
- Desk A/B/C
- Notifications

**Not answering:** T8 remaining-risk as live truth, blending books, folding without a founder chrome choice.

**Verdict:** _(fill after you choose A/B/C)_

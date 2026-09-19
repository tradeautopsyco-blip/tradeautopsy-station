# PROTOTYPE — throwaway

**Question:** With the **live Notch sidebar** (SESSION / More · analysis / Hide / Intraday · F&O), what should **Options Pre-trade** look like when it is **data-first** — OI and chain as charts of venue strings — while Spot / Equity keep the current form?

**Plan:** Same file `PROTOTYPE-notch-spot-session.html`. A = Pre-trade · B = Live · C = Post. Default class = **Options**.

**How options actually trade (Station locks, not Sensibull)**

| Fact | Source | UI consequence |
|------|--------|----------------|
| European, dated, USDT premium | `MECHANICS.md` | Expiry / DTE first-class. P&L is settlement, not fill WAC. |
| Buyer max loss = premium | same | At-expiry polyline is identity, not Black-76. Short-call wing unbounded. |
| OI = LatestState per symbol | `GET /eapi/v1/openInterest` `sumOpenInterest` | Bar chart of **rows**. Never a summed PCR / max pain. |
| Chain = catalog | `optionSymbols` | Table, not CE/PE/IV strike grid. `showsStrikeGrid = false`. |
| Greeks | `GET /eapi/v1/mark` VenuePublished | Copy strings. NFO greeks stay dark. |
| σ rungs 2–5 / max pain / IV smile | OPTIONS-PRICING.md blocker | Stay dark. |

**Sidebar (from live Notch screenshot)**

SESSION: Morning brief · Pre-trade · Live trade · Post-trade → MORE · ANALYSIS → Hide notch · ⌥Space → `Intraday · F&O` + gear.

**Not answering:** Shipping Swift. Inventing PCR/max pain. Spot/Equity form rewrite.

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-spot-session.html
```

| Key | Name | Structure |
|-----|------|-----------|
| A | Pre-trade | Options: Analytics (OI chart + chain) · Consequence · Plan. Spot/Equity: current form |
| B | Live trade | Working + OI still in view |
| C | Post-trade | Moments A–C |

**Verdict:** _(fill after you choose)_

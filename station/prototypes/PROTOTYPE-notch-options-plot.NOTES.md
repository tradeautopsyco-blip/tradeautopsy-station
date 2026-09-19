# PROTOTYPE — throwaway

**Question:** What should **Options Pre-trade’s plot** look like if we steal **OpenAlgo Charts density** (price lines, volume pane, DOM strip) — **not** the AGPL OpenAlgo feed — inside **live Notch chrome**?

**Plan:** Three variants on `PROTOTYPE-notch-options-plot.html?variant=`. Host is Notch Pre-trade · Options. Only the plot subtree swaps.

**Steal / refuse**

| Take | Refuse |
|------|--------|
| Entry / SL / TP as labelled lines; drag; ghost; commit on release | `OpenAlgoDataFeed` / WS / `localhost:5000` |
| Volume histogram from venue kline volume | 102 indicators, 85 drawings, widget rail |
| Depth as a right strip of the same snapshot | Click-to-live order (toast only) |
| Last-N visible bars | Station-built resample / Renko / footprint |

**WWDC 26:** glass on sidebar/toolbar only. Plot is standard material. Press 0.97. Reduce motion = no drag spring.

**Plan zone (live `BarCryptoOptionsDeclareView.planZone`):** State check → Contract → Risk → Horizon → Invalidation → Confirm. Plot lines stay bound to Risk (entry / SL / TP). σ rungs 2–5 stay dark.

**Law:** Demo · not live. DualNoBlend USDT. No PCR / max pain / strike grid.

**Not answering:** Shipping Swift. Wrapping OpenAlgo.

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-options-plot.html
```

| Key | Name | Structure |
|-----|------|-----------|
| A | Lines on candles | OHLC + volume + draggable BUY/SL/TP |
| B | DOM dock | Chart owns · depth heatmap right |
| C | Last-N overlay | Fit last 40 · Buy/Sell on plot · ghost |

**Verdict:** _(fill after you choose)_

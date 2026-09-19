# PROTOTYPE — throwaway

**Question:** When a trader is **planning** an Options trade, Session / OI / Payoff / Chain / Depth / Greeks and the **full Plan flow** already exist — but they live in one long scroll. What should Pre-trade look like if they all sit **in the same place**, stay **linked**, and stay **interactive**?

**Plan:** Three structurally different boards on `PROTOTYPE-notch-options-planning.html?variant=`. Host is live Notch chrome (SESSION rail, island, Options · Intraday). Only the board layout swaps.

## What we already have (live, not invented)

From `BarCryptoOptionsDeclareView` + `BarDeskTemplate.glanceKinds(.options)`:

| Zone | Surface | Source / lock |
|------|---------|----------------|
| Analytics | Last | `market/quote` · prefills entry |
| Analytics | Session / History | `GET /eapi/v1/klines` · this contract, not spot BTCUSDT |
| Analytics | Chain catalog | `optionSymbols` · `showsStrikeGrid = false` |
| Analytics | Open interest | `GET /eapi/v1/openInterest` `sumOpenInterest` rows. Never `{oi:0}`. Never PCR / max pain. |
| Analytics | Depth | `market/order_book` |
| Consequence | Legs | local plan |
| Consequence | Greeks Δ Γ Θ Vega | `GET /eapi/v1/mark` VenuePublished strings. NFO stays dark. Not Black-76. |
| Consequence | At expiry | European cash-settle identity. S from eapi index. Short-call wing unbounded. |
| Consequence | Ladder | Rung 1 = typed numbers. σ rungs 2–5 stay dark. |
| Consequence | Margin | unavailable (venue engine, not ours) |
| Plan | State check → Contract → Risk → Horizon → Invalidation → Confirm | live `planZone` |

**Not on this board:** PCR, max pain, IV smile, strike grid, OpenAlgo feed, lot / NRML, shipping Swift.

## Linked interactions (all three variants)

- Click an **OI bar** or **chain row** → bind contract; Last, greeks, payoff, session last, and Plan contract fields follow.
- Drag **BUY / SL / TP** on Session → Risk fields. Commit on release.
- Call/Put · Buy/Sell · contracts → **payoff polyline** is identity, not a pricer.
- Add / remove **leg** → Consequence list + payoff uses the first leg.
- Hover payoff → USDT PnL at that S.
- Confirm gated: calm + confidence + contract + invalidation. Second click holds `clientToken`.

**WWDC 26 / HIG:** glass on sidebar + island only. Charts are standard material. Color only on Confirm. Press 0.97. Larger sidebar more opaque.

**Law:** Demo · not live. DualNoBlend USDT. No PCR / max pain / strike grid.

**Not answering:** Shipping Swift. Wrapping OpenAlgo.

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-options-planning.html
```

| Key | Name | Structure |
|-----|------|-----------|
| A | Cockpit mosaic | Four charts at once (Session · OI · Payoff · Depth) + Plan rail |
| B | Hero + twins | Session owns; OI and Payoff as a following pair; chain strip |
| C | Focus + floor | One large chart; live thumbs; Plan as a floor (not a rail) |

**Verdict:** _(fill after you choose)_

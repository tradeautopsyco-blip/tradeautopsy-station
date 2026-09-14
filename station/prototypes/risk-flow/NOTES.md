# PROTOTYPE — capital-preservation flow

**Question:** Does this flow help a trader preserve capital, or is it a stats product wearing a risk costume?

**Wrong last prototype:** three competing desks (blotter / flip / tape). That was layout. This is the **path a fill takes**.

## Focus (from the transcript)

Capital preservation. Risk management. Systems beat willpower.

## The flow (canonical)

1. **Detect** — every fill gets a card: max downside and max upside of *this* trade, as money and % of account.
2. **Plan ≠ live SL** — reminder both ways. 13% → 14% is worse (more account at the stop). Tighten is better. Same card, opposite tone. Info on the live circle repeats planned vs actual max loss (Reliance 1300, form 1100, live 1150).
3. **No form** — circle / card is not a nag. Open Notch or a small form so they can *see* risk, then plan. Do not invent a stop.
4. **Today (and Console Trades), toggle** — shallow: this trade’s P&amp;L as % of account, toward/away from goal. Button **Advanced statistics** opens the Console *shell window* or the web journal for that trade.
5. **Console / journal** — that trade only: pre, live, post, notes, equity **before vs after**, max drawdown / max risk of that trade. Simple and data-dense. Not Sharpe on Today.
6. **Notch pre** — two views: direct (OHLCV, no chart) or charted. Broker Connect first. Form beside, least friction.
7. **Notch after declare** — the big box **becomes** account: margin, capital, equity path to the SL, max drawdown of this plan, max risk.
8. **Circle** — **plan now, implement last.** Live floating exit-criteria after detect card, Today shallow, journal depth, and Notch box-flip all exist. One trade is a ring. **Many trades: UI not decided.** Do not ship the circle in T3/Today.

## Fog (do not build in this flow)

Hotkey Notch workspace, Workflow Labs widgets, custom order-flow / FII-DII / “whole applications” in the Notch. Essence is glanceable custom data. Check later whether that helps in the heated moment or it is a second terminal.

## Helps the trader? (psychology)

Yes, if the card is a **gate** (stop + size as % of account) and Today stays small. No, if 13% of account at the stop is treated as normal — that already fails ~1% risk-per-trade. The mismatch reminder is useful; it does not fix a strategy with no edge. Compulsive trading → size zero, not a richer circle.

Verify live lot sizes, margins, expiry, charges. Qty 130 @ 1300 / SL 1100 ≈ 13% on ₹2L is an illustration.

**Verdict (2026-08-21):** Founder **yes** — this is the flow.
**Verdict (2026-09-08):** Founder **perfect** again (`?variant=detect`). Path-of-a-fill stays canonical.
**Production (2026-09-08):** After S7 (#60), switchable A/B/C chrome landed around this path (epic #62). Circle remains last — #63. Do not delete this prototype until #62 closes.

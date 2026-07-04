# Futures USDⓈ-M — Mechanics Reference

**Exchange:** Binance Global only (`fapi.binance.com`) — NOT available on Binance.US  
**Source:** `futures-usdm/ENUMS-FILTERS.md`, `futures-usdm/WEBSOCKET.md`, `futures-usdm/CHANGELOG-NOTES.md`, web search (Tier 4 — marketing/review sources, not official docs)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No  
**TradeAutopsy status:** NOT BUILDING — reference only, no broker connection supports this asset class today

---

## 1. What a "position" is

A contract position, not asset ownership. Trader holds a **notional exposure** to an asset's price via a perpetual or dated contract, settled in USDT/USDC (hence "USDⓈ-Margined").

Position fields confirmed from source: `positionAmt`, `positionSide` (`BOTH`/`LONG`/`SHORT`), `leverage`, `marginType` (cross/isolated), `isolatedMargin`, `isolatedWallet`.

**Flat** = net position quantity = 0 for a symbol (in one-way mode) or both LONG and SHORT legs closed (in hedge/dual-side mode).

## 2. Position lifecycle

- Opens on first order that establishes non-zero contract quantity
- **Hedge Mode** (`dualSidePosition`) allows simultaneous LONG and SHORT positions on the same symbol — NOT a net position model in this mode
- One-way mode: net position only, BUY/SELL orders net against existing position
- Closes when quantity returns to zero, OR via `closePosition=true` order flag which closes 100% of current position

## 3. P&L Calculation

**NOT SPECIFIED IN SOURCE** — the exact realized PnL formula for futures was not extracted from `schema__3_.yaml` in this thread (13,621 lines, not fully read). What's confirmed structurally:

- `unrealizedProfit` is a live-tracked field (from `common-definition.md` enums context) — meaning futures P&L is **mark-to-market continuously**, unlike spot's realize-on-sell model
- `markPrice` (not last trade price) is used for PnL calculation, liquidation checks, and PERCENT_PRICE filter — confirmed field exists (`GET /fapi/v1/premiumIndex`)
- **This is categorically different from spot's WAC formula.** A spot-style "realized on close" calculation would be wrong for futures because:
  - Funding payments occur throughout the position's life, not just at close
  - Mark price (not fill price) determines running P&L
  - `breakEvenPrice` field exists — accounts for fees + funding in the break-even calc, confirmed from `common-definition.md`

**Before building:** must extract the exact PnL formula from `schema__3_.yaml`, specifically the `positionRisk` and `income` endpoint response schemas.

## 4. Fees

`GET /fapi/v1/commissionRate` — user-specific commission rate. Fees charged per fill, same asset-normalization problem as spot.

## 5. Margin / Leverage / Liquidation

**Core to this product — spot has none of this.**

- **Leverage:** user-set per symbol via `POST /fapi/v1/leverage`. Confirmed range up to 125x (Tier 4 source — marketing page, needs official confirmation from schema)
- **Margin type:** `CROSSED` or `ISOLATED`, set via `POST /fapi/v1/marginType`
- **Cross margin:** shared wallet balance across all cross positions — a loss in one position can be covered by margin from another
- **Isolated margin:** each position has its own allocated margin (`isolatedMargin`, `isolatedWallet` fields); loss capped to that allocation
- **Liquidation:** triggered when margin ratio breaches maintenance threshold. `GET /fapi/v1/leverageBracket` returns notional/leverage brackets — `NOT SPECIFIED IN SOURCE` in this thread: the exact liquidation price formula
- **ADL (Auto-Deleveraging):** confirmed concept exists (`ADL_QUANTILE`, `symbolAdlRisk` endpoint) — a queue-based forced-close mechanism when insurance fund is insufficient during extreme liquidation events. This is a Binance-specific risk not present in spot at all.

**This entire section needs a dedicated live-source pull before any liquidation-aware feature is built.** Getting this wrong isn't a display bug — it's telling a trader their liquidation risk is different than it actually is, which is the opposite of what a circuit-breaker product should ever do.

## 6. Funding

**Confirmed mechanism, not in official schema depth here:**
- Perpetual contracts have no expiry; funding rate mechanism keeps contract price aligned to spot
- Funding exchanged between long and short holders periodically (Tier 4 source says every 8 hours — needs official confirmation)
- `lastFundingRate`, `nextFundingTime` fields confirmed in `GET /fapi/v1/premiumIndex`
- `FUNDING_FEE` is a confirmed `ACCOUNT_UPDATE` reason type (from `futures-usdm/WEBSOCKET.md`)
- Funding fee in **crossed position**: pushes balance update only
- Funding fee in **isolated position**: pushes balance + related position update

**RPI (Retail Price Improvement)** — new order book exclusion mechanism, confirmed added 2025-11-18. Affects fill price for retail-flagged orders. Relevant to accurate fill reconstruction if ever built.

## 7. Expiry / Exercise

- `PERPETUAL` contracts: no expiry (majority of futures volume)
- `CURRENT_MONTH` / `NEXT_MONTH` / `CURRENT_QUARTER` / `NEXT_QUARTER`: dated contracts, confirmed via `contractType` enum. Settlement mechanics `NOT SPECIFIED IN SOURCE` in this thread.

## 8. Session / time concept

24/7 like spot, but funding events (every ~8h) are structurally significant timestamps a futures-aware Today screen would need to account for — position P&L isn't just "buy vs sell price," it's continuously affected by funding regardless of whether the trader takes any action.

## 9. Data required from broker API (partial — needs full schema extraction)

Confirmed relevant endpoints:
- `GET /fapi/v1/positionRisk` (v2/v3) — live position + unrealized PnL + liquidation-relevant fields
- `GET /fapi/v1/income` — funding fee history, realized PnL history, commission history (income types)
- `GET /fapi/v1/userTrades` — fill-level data (window now limited to 6 months per changelog)
- `GET /fapi/v1/premiumIndex` — mark price, funding rate

## 10. Why this can't be "spot logic + a flag"

A `RoundTripEngine` built for spot cannot be extended to futures by adding a `leverage` field. Futures P&L requires:
1. Continuous mark-to-market, not fill-to-fill realization
2. Funding fee accrual as a first-class P&L component (not a fee, not a trade — a distinct income type)
3. Liquidation price as a **live risk number**, not a historical record
4. Separate handling for hedge mode (two simultaneous positions per symbol)

This confirms Rule 8 architecture guidance from `CRYPTO-STANDARD.md`: the engine needs an `AssetClass` seam, and futures needs its own implementation, not a parameterized spot engine.

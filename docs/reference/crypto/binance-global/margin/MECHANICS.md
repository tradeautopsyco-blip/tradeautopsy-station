# Margin Trading (Cross / Isolated) — Mechanics Reference

**Exchange:** Binance Global only — **confirmed NOT available on Binance.US** (regulatory restriction)  
**Source:** `margin/WEBSOCKET.md` (Risk Data Stream), web search (Tier 4 — marketing/review sources)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No  
**TradeAutopsy status:** NOT BUILDING — reference only, and moot for the currently-connected broker

---

## 1. What a "position" is

Unlike futures, margin trading is **still spot trading** — the trader owns real assets, buys/sells at real market prices. The difference: some of the purchasing power is **borrowed** from the exchange against posted collateral.

Confirmed from source: `sideEffectType` enum includes `MARGIN_BUY` and `AUTO_REPAY`, indicating order-level flags for borrowing behavior.

## 2. Position lifecycle

Same round-trip logic as spot (buy→sell, inventory-based) **plus** a borrowing/repayment lifecycle running in parallel:

- Trader deposits collateral into margin wallet
- Borrows against it (`MARGIN_BUY` side effect) to trade larger size than collateral alone allows
- Interest accrues on borrowed amount continuously
- Repayment can be automatic (`AUTO_REPAY`) or manual
- Closing a position doesn't necessarily close the loan — trader can still owe borrowed funds after selling

**This means margin P&L is not the same question as "what did this round-trip make."** There's a second, ongoing number: net borrowing cost.

## 3. P&L Calculation

**NOT SPECIFIED IN SOURCE in this thread** for the exact formula. Structurally, margin P&L has two components not present in plain spot:

1. Trading P&L — same WAC-style calculation as spot, on the actual buy/sell fills
2. **Interest cost** — continuously accruing charge on borrowed funds, which reduces net P&L even if the trade itself was profitable

A margin-aware Today screen showing only trading P&L (spot-style) would overstate the trader's actual result by ignoring interest owed.

## 4. Fees

Same trading fee structure as spot, **plus** interest on borrowed funds (a distinct fee category, not a trading commission).

## 5. Margin / Leverage / Liquidation

**Confirmed from Tier 4 sources (needs official verification):**
- Cross Margin: leverage up to 20x (varies by pair)
- Isolated Margin: leverage up to 10x
- Binance US does not offer margin trading — confirmed, stricter US regulations cited as reason
- Borrowed funds **cannot be withdrawn** — stay in margin account until repaid; only unborrowed deposited funds can move back to spot wallet

**Liquidation:** confirmed concept via `RISK_LEVEL_CHANGE` event (from `margin/WEBSOCKET.md` context, actually documented under Portfolio Margin but the risk mechanism is margin-account-wide): types include `MARGIN_CALL`, `REDUCE_ONLY`, `FORCE_LIQUIDATION`.

- **Cross margin liquidation:** entire margin balance shared across positions — a loss in one position draws down collateral for all; liquidation risks the whole balance
- **Isolated margin liquidation:** risk capped to that position's allocated margin only

`NOT SPECIFIED IN SOURCE`: exact margin call / liquidation trigger formula (margin level threshold calculation).

## 6. Funding

**Not applicable** — funding rate is a futures/perpetual concept. Margin has **interest** instead, which is conceptually different (continuously accruing borrowing cost vs periodic long/short payment exchange).

## 7. Expiry / Exercise

Not applicable — margin positions don't expire, but loans can trigger forced liquidation if margin level drops too low, functioning as an implicit forced-close mechanism.

## 8. Risk Data Stream (confirmed from `margin/WEBSOCKET.md`)

- Base: `wss://margin-stream.binance.com`
- **Cross Margin only** — isolated margin not supported on this stream
- Events: `MARGIN_LEVEL_STATUS_CHANGE`, `USER_LIABILITY_CHANGE` (borrowing, repayment, interest)

## 9. Data required from broker API (partial)

Confirmed relevant endpoints (from earlier session context, Portfolio Margin doc which shares margin endpoints):
- `GET /papi/v1/margin/order`, `/openOrders`, `/allOrders`, `/myTrades` (Portfolio Margin path — standalone Margin API path is `NOT SPECIFIED IN SOURCE` in this thread, needs separate extraction from `schema__1_.yaml`)

## 10. Why this needs its own engine, not a spot extension

Interest accrual is a **time-based, continuously-running cost** — unlike a trading fee charged once per fill. An engine that only understands "fills" cannot compute this without also modeling the loan balance over time. This is closer to an accounting ledger problem than a trade-reconstruction problem.

**Moot for now:** the currently connected broker (Binance.US) doesn't offer margin at all. This document exists so that if TradeAutopsy ever adds a Binance Global connection, the engineering scope is understood before work starts — not so it gets built now.

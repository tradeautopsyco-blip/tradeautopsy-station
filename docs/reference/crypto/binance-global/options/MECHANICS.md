# Options (European) — Mechanics Reference

**Exchange:** Binance Global only (`eapi.binance.com`) — NOT available on Binance.US  
**Source:** `options/REST.md`, web search (Tier 4)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No  
**TradeAutopsy status:** NOT BUILDING — reference only

---

## 1. What a "position" is

A contract giving the right (not obligation) to buy (call) or sell (put) an underlying asset at a fixed strike price, on or by an expiration date. The trader pays or receives a **premium** at trade time.

Binance offers **European-style options only** — confirmed from Tier 4 source: can only be exercised at expiration, not before. This is a deliberate simplification vs American-style options (exercisable any time before expiry). Simplified options structure: European options reduce complexity.

## 2. Position lifecycle

- **Buyer:** pays premium upfront. Position lifecycle ends at expiration — either exercised (if in-the-money) or expires worthless (if out-of-the-money). Maximum loss = premium paid.
- **Seller (writer):** receives premium upfront. Obligated to fulfill the contract if exercised against them at expiry. Loss potentially unbounded (for calls) or substantial (for puts).
- No liquidation mechanism for options buyers — confirmed: Binance options do not involve margin calls or forced liquidations, unlike futures. (Sellers/writers may still have margin requirements — see Section 5.)

## 3. P&L Calculation

**NOT SPECIFIED IN SOURCE in this thread** for the exact formula, but structurally:

- **Buyer P&L** = `(settlement_value − premium_paid)` if exercised, or `−premium_paid` if expired worthless
- **Seller P&L** = `premium_received − settlement_payout` (inverse of buyer)
- Settled in USDT — confirmed: contracts settled in USDT rather than the underlying asset, simplifying P&L math vs settling in the crypto itself
- **Time decay (theta)** works against option buyers structurally — value erodes as expiration approaches, independent of underlying price movement. Confirmed conceptually: options lose value as they approach expiration, even if the market doesn't move against you.

This is a **fundamentally different P&L shape** than spot or futures — no continuous fill-based reconstruction applies. P&L is determined by a single settlement event at (or before, for early close) expiry, not by matching buy/sell fills.

## 4. Fees

Commission rate query confirmed: `GET /eapi/v1/commission`. Structurally similar to other products — fee per trade/exercise.

## 5. Margin / Leverage / Liquidation

- **Options buyers:** no margin, no leverage in the traditional sense — premium paid is the full capital commitment, max loss is capped there
- **Options sellers (writers):** DO have margin requirements (`GET /eapi/v1/marginAccount` — confirmed field `riskLevel` exists), since their potential loss is uncapped/large
- `RISK_LEVEL_CHANGE` event confirmed for options margin accounts — same risk-level concept as margin/portfolio margin
- **No forced liquidation for options positions themselves** — confirmed distinguishing feature vs futures. But margin account backing a short options position could still face risk-level escalation.

## 6. Funding

**Not applicable** — no funding rate mechanism for options. Time decay (theta) is the closest analog but is a fundamentally different mechanism (deterministic decay curve, not a periodic long/short payment).

## 7. Expiry / Exercise

**This is the core mechanic that doesn't exist in spot/futures at all.**

- Every contract has a fixed expiration date
- European style: exercise only possible at expiry, not before
- `GET /eapi/v1/exerciseHistory` — confirmed endpoint for exercise records
- Auto-exercise likely occurs for in-the-money contracts at expiry (standard industry practice — **not confirmed from Binance source in this thread**, flagging for verification)

## 8. Session / time concept

Unlike spot/futures' continuous 24/7 model, options have a **hard temporal boundary**: the expiration date/time. A Today-style screen for options would need to surface "days to expiry" and theta decay as first-class signals, not just P&L — the passage of time itself is a risk factor, not just a session boundary.

## 9. Block trades

Confirmed separate execution path: `POST /eapi/v1/block/order/create` etc. — large trades negotiated off standard order book. Would need separate handling if ever built (different fill semantics than retail order matching).

## 10. Self-Trade Prevention & Kill Switch relevance

`POST /eapi/v1/countdownCancelAll` — confirmed auto-cancel/kill-switch mechanism exists at the exchange level for options, conceptually adjacent to TradeAutopsy's own circuit-breaker philosophy. Worth referencing if options are ever built — the exchange already has a heartbeat-based auto-cancel pattern TradeAutopsy's own kill switch design could learn from or interoperate with.

## 11. Why this is the most structurally different asset class

Spot and futures are both "continuous exposure with P&L that moves as price moves." Options are **discrete, time-bound, and asymmetric** (buyer capped loss / seller uncapped loss). A behavioral circuit-breaker model built around "position size" and "loss chasing" (designed for spot/futures) may not map cleanly onto options at all — the relevant behavioral risks for options traders (over-leveraging via cheap OTM options, ignoring theta decay, doubling down on losing premium) are a **different risk taxonomy**, not a variant of the existing one.

**If options are ever built, this needs its own behavioral signal research pass, not just its own P&L engine.**

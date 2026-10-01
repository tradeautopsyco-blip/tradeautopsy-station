# Position sizing — pre-trade budget to quantity

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Pre-trade position size from a risk budget, for every Station book |
| **Primary source** | None. Internal plans and a Tradeture screenshot were read and rejected as sources. |
| **Snapshot date** | 2026-10-01 |
| **Source version** | n/a |
| **Staleness warning** | Do not implement a budget→qty formula, a default risk percent, a pre-trade fee rate, or a leverage multiplier until this file cites a primary source. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> The following facts are needed before a calculator may author quantity, fees, or leverage-adjusted notional. They are **NOT SPECIFIED IN SOURCE**:
>
> - A primary source for `quantity = risk_budget / distance` (or any other identity), including rounding and the unit (shares, base asset, lots, contracts).
> - Whether the default budget is a typed amount, a percent of obtain(funds), or both, and what the percent is.
> - A pre-trade fee schedule per calc profile (what is included, the rate, the currency, and whether reward is net of fees).
> - Whether venue leverage changes the quantity identity on `crypto_usdm_usd` / `crypto_coinm_usd`, and the formula if it does.
> - Option-seller max loss. The options mechanics note says the spot/futures sizing taxonomy may not apply.
>
> Feature code must leave authored quantity unset. Display of a trader-typed quantity times a trader-typed stop distance may follow the existing Notch ladder tests; that measurement is not this document’s formula.

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Station design options | [`docs/design/risk-engine.md`](../../design/risk-engine.md) | 2026-10-01 | Maps code. Not a primary source. |
| Completion roadmap | [`plans/station-completion-roadmap-2026-09-17.md`](../../../plans/station-completion-roadmap-2026-09-17.md) | 2026-10-01 | States a candidate identity and leaves the default percent and the R:R gate as open founder locks. Internal plan, not a primary source. |
| Prototype sizer | [`station/prototypes/risk-demo-logic.mjs`](../../../station/prototypes/risk-demo-logic.mjs) | 2026-10-01 | File header: not a `docs/reference` source. Role fractions 0.005 / 0.01 / 0.02. |
| Tradeture screenshot | Founder attachment, 2026-10-01 (ad frame, exchange OKX, risk 1.1%) | 2026-10-01 | UX inspiration in the design doc. Not a formula source. |
| Kotak margin calculator | [`docs/reference/india/kotak-neo/MARGIN-CALCULATOR.md`](../india/kotak-neo/MARGIN-CALCULATOR.md) | 2026-08-27 | Calculator path unspecified. Not a sizer. |
| Binance options mechanics | [`docs/reference/crypto/binance-global/options/MECHANICS.md`](../crypto/binance-global/options/MECHANICS.md) | 2026-10-01 | Says a spot/futures position-size model may not map onto options. |
| OpenAlgo citation | [`docs/research/openalgo-citation.md`](../../research/openalgo-citation.md) | 2026-09-23 | Vocab oracle. No position-size formula in that pin. |

---

## Concepts

---

### Budget to quantity

**Source:** No primary source.

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| risk budget | NOT SPECIFIED IN SOURCE | money in the book quote currency, or a percent |
| distance | NOT SPECIFIED IN SOURCE | price |
| quantity | NOT SPECIFIED IN SOURCE | book-specific unit |

**Gaps (NOT SPECIFIED IN SOURCE):**

- The identity itself.
- Rounding to tick, step, or lot.
- Behavior when funds obtain is dark.
- Behavior when stop is missing.
- A default percent.

---

> **OUR INTERPRETATION**
>
> - Do not treat the roadmap sentence `size = planned_risk / distance_to_invalidation` as sourced. It is a candidate written in an internal plan.
> - Do not treat the Tradeture frame’s 1.1% or its fee amounts as sourced.
> - Existing `BarPlanLadder` arithmetic measures a quantity the trader already typed. It is not an authorization to solve for quantity.

---

### Pre-trade fees

**Source:** No primary source for a pre-trade schedule. Post-fill fee fields exist on some fill paths; India locks often still say charges are unspecified until the lock is SHIPPING.

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| fee | NOT SPECIFIED IN SOURCE | money |
| risk + fees | NOT SPECIFIED IN SOURCE | money |
| reward − fees | NOT SPECIFIED IN SOURCE | money |
| R:R including fees | NOT SPECIFIED IN SOURCE | ratio |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Which levies apply before the fill (brokerage, exchange, stamp, GST, funding, option premium).
- Whether they are a function of notional, premium, or lot.
- Maker versus taker.

---

> **OUR INTERPRETATION**
>
> - A calculator may show a Fees row whose value is absent and whose reason is `fee_schedule_unspecified`.
> - Reusing the last fill’s `fee_amount` as a rate is a guess. Leave it unimplemented.

---

### Leverage as a sizing input

**Source:** No primary source that leverage rescales quantity. Station reads a leverage string from a futures position row when the account envelope includes it, and refuses to invent one.

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| leverage | NOT SPECIFIED IN SOURCE as a multiplier on size | number from the venue, or absent |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether size is notional, margin, or contracts.
- Cross versus isolated.
- A maximum the product may assume.

---

> **OUR INTERPRETATION**
>
> - Show the venue’s leverage string when the position row has one.
> - Do not default to 50 or 100.
> - Do not POST a leverage change. That path is a broker mutation.

---

## Verification Checklist

- [ ] A primary source (exchange rulebook, broker doc, or peer-reviewed method the founder accepts) is pasted into “Budget to quantity” before any code sets `authored_qty`.
- [ ] Founder lock: typed money vs percent of funds, recorded in this file after the source, under OUR INTERPRETATION.
- [ ] Founder lock: R:R below 1 is a color, a warn, or a block — recorded the same way.
- [ ] Each calc profile that shows a fee number has its own reference doc or a dated section here.
- [ ] USDM/Coin-M leverage formula, if any, cites Binance exchange rules rather than a UI screenshot.

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Copy `size = planned_risk / distance` from the completion roadmap into this file as law | The roadmap is an internal plan. It also says the formula waits on a reference doc, and it leaves the default percent open. | Left NOT SPECIFIED. |
| Use Tradeture 1.1% risk, fee 1.21, RR 2.7 / 2.0 | Those are pixels on an ad. No exchange or broker document. | Described as UX in the design doc. Not copied as rates. |
| Use prototype role fractions 0.5% / 1% / 2% and “calm ≥ 4 halves” | `risk-demo-logic.mjs` says it is not a reference source. | Not copied. |
| Treat OpenAlgo “about 1%” as the gate | The OpenAlgo citation pin in this repo does not state a position-size formula. | Not used. |
| Invent option-seller max loss from premium | Options mechanics reference says the taxonomy may not map. Options money owner is an explicit none. | Left unspecified. |

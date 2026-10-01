# Position sizing — pre-trade budget to quantity

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Pre-trade position size from a risk budget, for every Station book |
| **Primary source** | None. Internal plans and a Tradeture screenshot were read and rejected as sources. The **FOUNDER CANDIDATE** section is a product proposal for a yes/no lock. It is not a primary source. |
| **Snapshot date** | 2026-10-01 |
| **Source version** | n/a — candidate section added 2026-10-01; still no external source version |
| **Staleness warning** | Primary source is still missing. The FOUNDER CANDIDATE section does not license implementation. Leave authored quantity unset, and do not ship a default risk percent, a fee, or a leverage multiplier, until the founder records YES or NO on that section’s rows. Fees stay out of this version after a size lock. A default risk percent is a separate lock and is not set here. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> The FOUNDER CANDIDATE section writes concrete identities so each one can be accepted or rejected. A dated YES in that section is the product lock. Until that YES is written, feature code leaves authored quantity unset. A YES is not a citation of an exchange, a broker, or a paper.
>
> The following facts are needed before a calculator may author quantity, fees, or leverage-adjusted notional. They are **NOT SPECIFIED IN SOURCE**:
>
> - A primary source for `quantity = risk_budget / distance` (or any other identity), including rounding and the unit (shares, base asset, lots, contracts).
> - Whether the default budget is a typed amount, a percent of obtain(funds), or both, and what the percent is.
> - A pre-trade fee schedule per calc profile (what is included, the rate, the currency, and whether reward is net of fees). Fees are out of this version. Do not invent a schedule. A later manual fee box in Station only is future work and is not part of any identity in the candidate section.
> - Whether venue leverage changes the quantity identity on `crypto_usdm_usd` / `crypto_coinm_usd`, and the formula if it does.
> - Option-seller max loss. The options mechanics note says the spot/futures sizing taxonomy may not apply.
> - A product default for `risk_pct`. The candidate takes a trader-typed percent and names no default.
>
> Feature code must leave authored quantity unset. Display of a trader-typed quantity times a trader-typed stop distance may follow the existing Notch ladder tests; that measurement is not this document’s formula. The candidate identities do not change that.

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
| Founder candidate section | This file, section **FOUNDER CANDIDATE** | 2026-10-01 | Product proposal for a yes/no lock. Not a primary source. |

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
> - The FOUNDER CANDIDATE section is where a product lock would be recorded. It is not sourced law.

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
> - Fees are out of this version. Candidate identities ignore fees. A later version may add manual fee entry in Station only. That box is not specified here and is not a schedule.

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

## FOUNDER CANDIDATE — SL suggestor / budget→qty / stop from margin %

**Status:** proposal. Not locked. Not a primary source. Not licensed to implement.

**How to lock:** write a date and YES or NO in the lock log at the end of this section. One row at a time is enough. A YES on a row licenses only that row. A NO leaves that row dashed and does not borrow another row’s identity.

**What stays true after any YES:** fees stay out of the identity. No product default for `risk_pct`. Dark funds author no quantity. Leverage is not a term. Coin-M and short options stay on their own rows.

This section is the concrete proposal behind the Plan control “SL suggestor (% of margin)” (`notch/BarPlanSlSuggestor.swift`). That control still prints **—** for stop and size. `POST /api/daemon/risk/preview` still returns `authored_qty: null`.

Design map of books and profiles: [`docs/design/risk-engine.md`](../../design/risk-engine.md) §2. Declare tabs that exist today (`notch/BarDeskInstruments.swift` `BarDeclareAssetClass.supported`): Binance → Spot, Options, USDM, Coin-M; Kotak → Equity, Options. Other catalog books are named here so a later tab does not invent a second formula. DualNoBlend: one book, one quote currency. Never add an INR free figure to a USD free figure.

### Shared prelude (row C0)

The suggestor’s word “margin” means one number: the lit obtain-funds **free** figure for **this book**. It is not `account/margin_estimate` (that envelope stays unavailable, reason `margin_calculator_unspecified`). It is not `MarginUsed`. It is not a blend of collateral and cash. Unrealized P&L stays off this base.

| Book family | Lit free figure the candidate uses | When it is absent |
| --- | --- | --- |
| Kotak cash `kotak-nse-bse-cash` | Glance free. Host copies RMS sample key `Net` into `free` ([`FUNDS-LIMITS.md`](../india/kotak-neo/FUNDS-LIMITS.md)). Definition of `Net` is **NOT SPECIFIED IN SOURCE**. | **—** |
| Kotak NFO `kotak-nse-nfo` | Glance free for **that** book. Limits body is `seg=FO`, not the cash `ALL` snapshot (same funds note). | **—** |
| Binance spot, USDM, Coin-M, options | Glance free for **that** book’s quote holding. USDM and Coin-M obtain tests copy venue `availableBalance` into the funds row that becomes `free`. | **—** |
| CDS, MCX, and any other book | Glance free only when the envelope is that book. Do not reuse cash `Net` or NFO `Net`. | **—** |

```
budget_base    = lit free figure for this book
                 else —  (reason funds_dark; no authored quantity)

risk_pct       = trader-typed percent. 1.25 means 1.25 percent.
                 Empty, non-finite, or ≤ 0 → —
                 No product default. A default percent is a separate founder lock.

risk_budget    = budget_base × (risk_pct / 100)
                 when budget_base is lit and > 0 and risk_pct is lit
                 else —

stop_distance  = |entry − stop|
                 when both prices are finite and the distance is > 0
                 else —
```

Fees do not appear. Do not subtract a fee from `risk_budget`. Do not add a fee to `stop_distance`.

Non-positive `budget_base` is the same as dark: dash. Do not author quantity 0.

No cap on `risk_pct` is proposed. A cap would be another lock.

Rounding to tick, step, or lot is **not** part of a YES on quantity. `LOT_SIZE.stepSize` in the Binance filter notes is a filter, not a sizing multiplier. Until a separate rounding lock, show no rounded authored size (there is no authored size).

Leverage is not in the algebra. A futures row may still show the venue leverage string when a position row carries one. Do not multiply by it. Do not default it.

### Two solves (only where a profile row evaluates)

Size given a loss-side stop, when `stop_distance` and `multiplier` are both lit:

```
quantity = risk_budget / (stop_distance × multiplier)
```

Stop given a typed size and `risk_budget`, when `quantity > 0` and `multiplier` is lit:

```
stop_distance = risk_budget / (quantity × multiplier)
BUY  stop = entry − stop_distance
SELL stop = entry + stop_distance
```

A solved stop that is not finite or not strictly positive stays **—**.

A typed stop on the profitable side of entry is not a loss. Do not solve a size from it. That matches the existing ladder rule: max planned loss exists only when the typed geometry is a loss (`notch/BarPlanLadder.swift`).

If `multiplier` is **NOT SPECIFIED** or the lot row is dark, both results stay **—**. Do not drop `multiplier` to 1 to force a division, except on a row whose YES explicitly sets the multiplier to 1.

### Per profile

Declare-tab coverage is noted so the founder can see what Plan can paint today. Catalog books without a tab still have a row.

#### C1 — `equities_inr_cash` (shares)

Books: `kotak-nse-bse-cash` and the other `*-nse-bse-cash` catalog rows. Declare tab today: Kotak **Equity**.

| | |
| --- | --- |
| Budget base | C0. Kotak cash free ← RMS `Net`. |
| Unit | Shares. |
| Multiplier | **Candidate value: 1.** One currency unit of price on one share is one currency unit of planned loss. **NOT SPECIFIED IN SOURCE** (no `docs/reference` page states this for India cash). |
| Size given stop | `shares = risk_budget / stop_distance` |
| Stop given size | `stop_distance = risk_budget / shares`, then the BUY/SELL placement in the prelude. |
| Lot file | Do not read an F&O lot onto cash. |

**YES** licenses multiplier 1 and both solves. **Until YES, dash.**

#### C2 — `equities_inr_nfo` future (lots)

Same calc profile as NFO options. Branch on the instrument. This row is a future (FUT) on `*-nse-nfo`. The Kotak **Options** tab is C3/C4, not this row. The catalog stamp on `kotak-nse-nfo` is `instrument_class: option` because the book is mixed CE/PE/FUT; the fill’s instrument type picks the row.

| | |
| --- | --- |
| Budget base | C0 for the NFO book (`seg=FO` free). |
| Unit | Lots (the quantity the desk types). |
| Lot | [`NFO-SCRIP-MASTER.md`](../india/kotak-neo/NFO-SCRIP-MASTER.md): use `lLotSize` and `iLotSize` when both are present and equal. Sample value 65 is one fixture row, not a NIFTY constant. Disagreement or a missing column → skip, multiplier dark. Do not invent 25, 50, or 75. |
| `lMultiplier` | Header name on that CSV. **NOT SPECIFIED** as a sizing multiplier. Leave it out. |
| Size given stop | `lots = risk_budget / (stop_distance × lot)` |
| Stop given size | `stop_distance = risk_budget / (lots × lot)`, then BUY/SELL placement. |

The multiplication “price distance × lots × lot = INR” is the candidate. It is the inverse of the shape the NFO realized owner uses in code. That code comment is not a primary source. **YES** accepts the algebra and the agreeing-lot field. **Until YES, dash.** A YES still dashes when the lot columns disagree.

#### C3 — NFO option, long (premium)

Kotak **Options** tab, book `kotak-nse-nfo`, long premium.

Entry and stop are **premiums**, not the underlying index. `stop_distance = |premium_entry − premium_stop|`.

| | |
| --- | --- |
| Budget base | C0 for the NFO book. |
| Unit | Lots. |
| Lot field | Same agreeing `lLotSize` / `iLotSize` rule as C2. |
| Rupee chain | [`OPTIONS-PRICING.md`](../india/nfo/OPTIONS-PRICING.md): scaling premium into ₹ (per-lot vs per-unit, and any multiplier) is **NOT SPECIFIED IN SOURCE**. Lot columns are not a greek formula. |
| Size given stop | `lots = risk_budget / (premium_distance × lot)` |
| Stop given size | `premium_distance = risk_budget / (lots × lot)`, then place the premium stop on the loss side of the entry premium. |

This is planned loss if premium moves from entry to the typed stop. It is not expiry payoff and not a greek.

**YES** accepts that algebra **knowing the rupee chain is unspecified**. **NO**, or no answer, leaves the suggestor on **—**. Do not apply C2’s future identity to a CE/PE.

#### C4 — NFO option, short

**Candidate: stay blocked.**

[`OPTIONS-PRICING.md`](../india/nfo/OPTIONS-PRICING.md): a numeric max-loss bound for a short call is **NOT SPECIFIED IN SOURCE**. The word “unlimited” is not a number. Premium received is not max loss.

No size solve. No stop solve. Reason when the suggestor is asked: `option_short_max_loss_unspecified`. The trader may still type a max planned loss on declare. The suggestor does not fill it.

**YES** locks the block. **NO** does not unlock a formula. A different candidate would have to be written first.

#### C5 — `fx_cds_inr` (lots)

Book `kotak-nse-cds`. No declare tab today. ADR 0022 is draft. Charge rows there are unspecified; fees are out of this version anyway.

| | |
| --- | --- |
| Budget base | C0 for `kotak-nse-cds` only. |
| Unit | Lots. |
| Lot | **NOT SPECIFIED IN SOURCE.** No `docs/reference` contract spec names the CDS lot field. |
| Algebra, if a later reference names `lot` | `lots = risk_budget / (stop_distance × lot)` and the matching stop solve. |

**YES** means “use that algebra on the day a reference names `lot`.” **Until the lot is cited, dash,** including after YES. **NO** means do not keep this algebra waiting.

#### C6 — `commodity_inr_mcx` (lots)

Book `kotak-mcx-future`. No declare tab today. ADR 0023 describes a lot pipeline. That ADR is not a `docs/reference` contract spec.

| | |
| --- | --- |
| Budget base | C0 for `kotak-mcx-future` only. |
| Unit | Lots. |
| Lot | **NOT SPECIFIED IN SOURCE.** |
| Algebra, if a later reference names `lot` | Same as C5. |

Same YES / dash rule as C5.

#### C7 — `crypto_spot_usd` (base asset)

Books: `binance-com-spot` and the other `*-spot` catalog rows. Declare tab today: Binance **Spot**.

[`binance-us/spot/MECHANICS.md`](../crypto/binance-us/spot/MECHANICS.md) records a **post-fill** identity for Binance.US, `sell_qty × (avg_sell_price − avg_buy_price)`, on USD-quoted symbols. That page is a different venue and a different question. It is **not** the primary source for this pre-trade rule. The candidate reuses the multiplier-1 shape for a book whose symbol quote is the book quote.

| | |
| --- | --- |
| Budget base | C0. Quote-asset free for this book (the glance labels the asset, often USDT). Do not convert that figure into a second “USD” number. Catalog `quote_currency` is the desk label, not a conversion rate. |
| Unit | Base asset. |
| Multiplier | **Candidate value: 1** (one base unit × one quote unit of price = one quote unit). **NOT SPECIFIED** as a pre-trade Binance Global rule. |
| Quote mismatch | Symbol quote different from the book quote (an ETHBTC-style pair on a USD book): **—**. The Binance.US note already refuses a USD P&L for `quote_not_usd`. |
| `quoteOrderQty` | Not solved. A quote-sized ticket stays **—** (`quote_size_ticket`). |
| Size given stop | `base_qty = risk_budget / stop_distance` |
| Stop given size | `stop_distance = risk_budget / base_qty`, then BUY/SELL placement. |

**YES** licenses multiplier 1 for matching-quote spot. **Until YES, dash.**

#### C8 — `crypto_usdm_usd` (contracts)

Book `binance-com-usdm`. Declare tab: Binance **USDM**. `is_inverse` is false.

[`futures-usdm/MECHANICS.md`](../crypto/binance-global/futures-usdm/MECHANICS.md) describes a contract position and does not state a P&L identity. [`ENUMS-FILTERS.md`](../crypto/binance-global/futures-usdm/ENUMS-FILTERS.md) `MIN_NOTIONAL` is a filter (`price * quantity`), not a P&L multiplier. No `contractSize` is quoted in `docs/reference`.

| | |
| --- | --- |
| Budget base | C0. Lit USDM free (`availableBalance` copied into the funds row). Not a cross-book wallet sum. |
| Unit | Contracts. |
| Multiplier | **Candidate value: 1.** One contract × one quote unit of price = one quote unit of planned loss. Leverage is not applied. **NOT SPECIFIED IN SOURCE.** |
| Size given stop | `contracts = risk_budget / stop_distance` |
| Stop given size | `stop_distance = risk_budget / contracts`, then BUY/SELL placement. |

**YES** licenses that linear identity with multiplier 1 and no leverage. **NO**, or no answer, leaves multiplier dashed and both outputs **—**.

#### C9 — `crypto_coinm_usd` (contracts, inverse)

Book `binance-com-coinm`. Declare tab: Binance **Coin-M**. `is_inverse` is true. Margin on this book is the coin, not USDT ([`futures-coinm/MECHANICS.md`](../crypto/binance-global/futures-coinm/MECHANICS.md)).

That same note records an inverse shape, `contracts × contract_value × (1/entry_price − 1/exit_price)`, and says it is general knowledge, **not confirmed from an official Binance source**, and must not be used. This candidate does not adopt that line.

| | |
| --- | --- |
| Budget base | Would be C0 in the coin, and only for this book. Unused while the identity is absent. |
| Unit | Contracts. |
| Identity | **None.** Multiplier and contract value are **NOT SPECIFIED IN SOURCE.** |
| Size and stop | **—** |

**YES** confirms the block: Coin-M stays dashed until a Binance primary quote is pasted into this file and a new row is written. **NO** does not license the unverified inverse line.

#### C10 — `crypto_options_usd`, long (premium)

Book `binance-com-options`. Declare tab: Binance **Options**. Realized owner is an explicit none.

[`options/MECHANICS.md`](../crypto/binance-global/options/MECHANICS.md): buyer maximum loss is the premium paid; the exact P&L formula in that note is marked **NOT SPECIFIED IN SOURCE** in the same section. The Plan fields are premium in USDT (`BarCryptoOptionsDeclareView`). Contract size that would scale a per-underlying premium is **NOT SPECIFIED**. No numeric contract multiplier is proposed beyond the candidate below.

| | |
| --- | --- |
| Budget base | C0 for `binance-com-options`. Buyers are described as having no futures-style margin. The base is still the lit free figure, not `marginAccount`. |
| Unit | Contracts. |
| Prices | Premiums. `premium_distance = \|premium_entry − premium_stop\|`. |
| Multiplier | **Candidate value: 1**, and only if the typed premium is already per contract in quote currency. **NOT SPECIFIED IN SOURCE.** |
| Size given stop | `contracts = risk_budget / premium_distance` |
| Stop given size | `premium_distance = risk_budget / contracts`, placed on the loss side of the entry premium. |

This is the loss if premium moves from entry to the stop. It is not settlement value at expiry.

**YES** licenses multiplier 1 on per-contract premium. **Until YES, dash.**

#### C11 — `crypto_options_usd`, short

**Candidate: stay blocked.**

[`options/MECHANICS.md`](../crypto/binance-global/options/MECHANICS.md): seller loss is potentially unbounded (calls) or substantial (puts). That sentence is not a finite max-loss number. Do not treat premium received as max loss. Do not copy the NFO short row onto this book as a formula, and do not copy this venue’s words onto NFO as a number.

No size solve. No stop solve. Reason: `option_short_max_loss_unspecified`.

**YES** locks the block. **NO** does not unlock a formula.

### Fees (this version)

Ignored in every row above. The Fees cell on the risk strip stays empty with `fee_schedule_unspecified`.

Future, not this version: a manual fee amount typed in Station only. No schedule, no rate, no import from the last fill. Not part of C0–C11.

### Lock log

Empty until the founder writes it. Do not pre-fill YES.

| Row | Profile / case | Founder | Date | YES or NO |
| --- | --- | --- | --- | --- |
| C0 | Shared prelude: lit free × typed %, fees ignored, dark funds dash, no default % | | | |
| C1 | `equities_inr_cash` multiplier 1, shares | | | |
| C2 | `equities_inr_nfo` future, agreeing lot | | | |
| C3 | NFO option long, premium × agreeing lot | | | |
| C4 | NFO option short blocked | | | |
| C5 | `fx_cds_inr` algebra waiting on a cited lot | | | |
| C6 | `commodity_inr_mcx` algebra waiting on a cited lot | | | |
| C7 | `crypto_spot_usd` multiplier 1, base qty, no quote-size solve | | | |
| C8 | `crypto_usdm_usd` multiplier 1, no leverage | | | |
| C9 | `crypto_coinm_usd` no identity | | | |
| C10 | `crypto_options_usd` long, per-contract premium, multiplier 1 | | | |
| C11 | `crypto_options_usd` short blocked | | | |

---

## Verification Checklist

- [ ] A primary source (exchange rulebook, broker doc, or peer-reviewed method the founder accepts) is pasted into “Budget to quantity” before any code sets `authored_qty`. A YES in the FOUNDER CANDIDATE lock log is a product lock, not this checkbox.
- [ ] Founder lock log (C0–C11) has a date and YES or NO before any code sets `authored_qty` from that row. Unfilled rows stay dashed.
- [ ] Founder lock: typed money vs percent of funds, recorded in this file after the source, under OUR INTERPRETATION. C0 is the percent path only. It does not set a default percent.
- [ ] Founder lock: R:R below 1 is a color, a warn, or a block — recorded the same way.
- [ ] Fees stay out of this version. No profile shows a fee number from a schedule. A future manual Station fee box is not implemented from this file.
- [ ] USDM/Coin-M leverage formula, if any, cites Binance exchange rules rather than a UI screenshot. C8’s candidate does not multiply by leverage. C9 has no identity.

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Copy `size = planned_risk / distance` from the completion roadmap into this file as law | The roadmap is an internal plan. It also says the formula waits on a reference doc, and it leaves the default percent open. | Left NOT SPECIFIED. |
| Use Tradeture 1.1% risk, fee 1.21, RR 2.7 / 2.0 | Those are pixels on an ad. No exchange or broker document. | Described as UX in the design doc. Not copied as rates. |
| Use prototype role fractions 0.5% / 1% / 2% and “calm ≥ 4 halves” | `risk-demo-logic.mjs` says it is not a reference source. | Not copied. |
| Treat OpenAlgo “about 1%” as the gate | The OpenAlgo citation pin in this repo does not state a position-size formula. | Not used. |
| Invent option-seller max loss from premium | Options mechanics reference says the taxonomy may not map. Options money owner is an explicit none. NFO pricing says a short-call bound is unspecified. | C4 and C11 stay blocked. Long rows use premium distance only as a candidate, still dashed until YES. |
| Treat the FOUNDER CANDIDATE algebra as a primary source | No exchange, broker, or paper is quoted as the source of those identities | Left as a proposal. BLOCKER stays. Display stays dashed until a lock-log YES. |
| Use the Binance.US post-fill line `sell_qty × (avg_sell_price − avg_buy_price)` as the pre-trade law for every book | That sentence is Binance.US spot, after the fill, USD-quoted | Named under C7 as a shape the founder may reuse for matching-quote spot. Not copied into other profiles. |
| Paste the Coin-M line `contracts × contract_value × (1/entry − 1/exit)` into the candidate | `futures-coinm/MECHANICS.md` says that line is unverified general knowledge and must not be used | C9 has no identity. Outputs stay —. |
| Default `risk_pct` to 1% or 1.1% | No source. The default-percent lock is still open | No default. Empty percent stays —. |
| Put a fee into `risk_budget` | Fees are out of this version | Identities ignore fees. |
| Use `lMultiplier`, or a remembered 25/50/75 lot | NFO scrip master: agreeing `lLotSize`/`iLotSize` only; disagreement skips; do not invent those constants. `lMultiplier` has no sizing definition in `docs/reference` | Constant lot dashed. `lMultiplier` left out. |
| Treat Kotak `Net` as SPAN margin | FUNDS-LIMITS: `Net` is copied to `free`; its definition is unspecified; `margin_estimate` stays unavailable | Budget base is the lit free figure for that book. |

# TradeAutopsy Station — Reference Library

## Purpose

This is the project's authoritative foundation library. Every feature that involves a financial
calculation, market mechanic, or behavioral-science claim **must cite a document here** (or add
one) before implementation begins.

---

## The Core Rule

> **No feature is built on a guess or on AI memory. Every formula, mechanic, or standard is
> captured from an authoritative source, cited, and dated before code is written against it.**

Gaps are never guessed. If the source does not state something, write `NOT SPECIFIED IN SOURCE`
and leave it blank until a primary source is found. AI recall, blog posts, and intuition are not
sources.

---

## How To Use This Library

1. **Before building a feature**, check for a relevant reference doc in the categories below.
2. **If no doc exists**, create one from an authoritative source using [`_TEMPLATE.md`](./_TEMPLATE.md)
   as part of the feature work — not after it.
3. **Cite the doc** in the feature's PRD and TRD under a "Foundation" or "References" section.
4. **Build the code** to match the cited source, not intuition. When the source is ambiguous,
   document the interpretation in an `OUR INTERPRETATION` block — never silently decide.

---

## Categories

### `broker-mechanics/`

How brokers actually compute P&L, fees, margin, and settlement.

- Authoritative sources: Zerodha Varsity, Interactive Brokers docs, Binance.US API docs,
  broker help-center pages.
- What belongs here: fill price mechanics, fee calculation formulas, margin call rules,
  settlement timelines, open-order semantics, rate limits.
- [`india/kotak-neo/MARGIN-CALCULATOR.md`](./india/kotak-neo/MARGIN-CALCULATOR.md) — Kotak Neo `account/margin_estimate` (calculator path unspecified; dark envelope).
- [`india/kotak-neo/FUNDS-LIMITS.md`](./india/kotak-neo/FUNDS-LIMITS.md) — Kotak Neo cash `obtain(funds)` from RMS `POST /quick/user/limits` (`Net` / `MarginUsed` copied; not SPAN).

### `market-structure/`

Order types, fill mechanics, settlement cycles, and market hours as defined by exchanges.

- Authoritative sources: exchange rulebooks (NSE, BSE, Binance), SEBI investor education,
  SEC investor education, exchange trading hour notices.
- What belongs here: order type definitions, partial fill semantics, T+1/T+2 settlement,
  trading session boundaries, circuit-breaker rules.
- [`india/nfo/OPTIONS-PRICING.md`](./india/nfo/OPTIONS-PRICING.md) — NFO options pricing / greeks (named trader model unspecified; no fixture ΔΓΘ; S5 G0).
- [`crypto/binance-global/spot/OHLCV-RESAMPLE.md`](./crypto/binance-global/spot/OHLCV-RESAMPLE.md) — derived OHLCV resample (aggregation unspecified; venue kline interval wins).
- India F&O instrument master (Kotak Neo, **v1 refused**): [`india/kotak-neo/NFO-SCRIP-MASTER.md`](./india/kotak-neo/NFO-SCRIP-MASTER.md).

### `accounting-standards/`

Realized vs. unrealized P&L, cost basis methods (FIFO / LIFO / weighted-average), and tax lots.

- Authoritative sources: accounting standards bodies, broker tax documentation (Zerodha
  P&L statement methodology), IRS Publication 550 for US context.
- What belongs here: cost basis calculation rules, short-sale treatment, wash-sale rules,
  realized P&L event definitions, tax-lot assignment logic.

### `behavioral-science/`

The empirical science behind TradeAutopsy's scoring and intervention model.

- Authoritative sources: peer-reviewed papers (Kahneman, Thaler, Barber & Odean), the
  NB00–NB06 validation notebooks, dual-process theory literature, conformal prediction
  calibration references.
- What belongs here: signal weight justifications, threshold calibration citations,
  behavioral bias definitions, intervention-timing science.

---

## Authority Hierarchy

Prefer sources in this order:

1. Official primary source (exchange rulebook, broker API docs, peer-reviewed paper)
2. Official regulatory guidance (SEBI, SEC investor education)
3. Broker help-center documentation (dated snapshot)
4. Academic secondary literature (cite the original paper it cites)

**Never cite:** AI-generated content, untitled blog posts, or undated web pages.

**Date every snapshot.** Mark a reference doc as stale if the source has been updated since
the snapshot. Re-verify against the live source before building against a stale doc.

---

## Exemplar

The canonical model for this discipline is the Binance.US API reference:

→ [`agent/reference/binance-us/API-REFERENCE-v1.md`](../../agent/reference/binance-us/API-REFERENCE-v1.md)

*(File to be created as part of Backend Box v1 / broker-mechanics foundation work.)*

Study that document's structure before writing any new reference doc:
- Source cited at the top with URL and snapshot date
- Verbatim quotes for every formula and definition
- `NOT SPECIFIED IN SOURCE` for every gap
- `OUR INTERPRETATION` block clearly fenced and separated from the source
- Self-audit section listing every place the author was tempted to fill from memory
- Verification checklist of facts still requiring live-source confirmation

---

## Adding a New Reference Doc

Use [`_TEMPLATE.md`](./_TEMPLATE.md). Fill every section. Do not skip the self-audit or the
verification checklist — those sections exist precisely to surface the gaps that would
otherwise become silent bugs.

File under the most specific category. If the topic spans two categories, place it in the
primary one and cross-link.

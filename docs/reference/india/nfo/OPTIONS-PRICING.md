# NFO Options Pricing / Greeks — `derived/greeks` and `derived/synthetic_future`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | NSE F&O (NFO) options pricing, greeks-in-₹, and synthetic future (Track E / 4b / 4d) |
| **Primary source** | Station capability matrix (`agent/src/data/matrix.rs`, read-only) · [Kotak Neo REST quotes + scrip master](../kotak-neo/REST.md) · NSE product URLs attempted 2026-08-27 (body not extractable) · [Binance Global options mechanics](../../crypto/binance-global/options/MECHANICS.md) (venue contrast only) |
| **Snapshot date** | 2026-08-27 |
| **Source version** | Matrix as in-repo 2026-08-27 · REST.md snapshot 2026-08-27 · Binance options MECHANICS.md snapshot 2026-07-02 · NSE static product pages fetched 2026-08-27 (SPA shell; contract-spec body empty) · legacy `nifty_50_FO.htm` HTTP 404 |
| **Staleness warning** | Re-verify against an NSE circular / NCL rulebook PDF (not a JS shell) and any Kotak doc that *names a pricing model* before any greek or synthetic-future *number* is computed. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> The following facts are needed to show greeks in ₹ or a synthetic-future *hero number* but are **NOT SPECIFIED IN SOURCE**:
>
> - Named options model for NFO (including whether Black-76, Black–Scholes, or another) with a citable primary (NSE circular, exchange rulebook, Hull **with page**, or Kotak docs that specify the model).
> - Model inputs: day-count (365 vs 365.25 vs trading days), risk-free rate source, dividend/borrow, volatility convention, and how IV is taken from LTP (if at all).
> - NFO contract mechanics required to scale greeks into **₹**: lot size, multiplier, premium units, index vs stock settlement (cash vs physical), exercise style — not captured as verbatim body from NSE on this snapshot.
> - Terminal-payoff **numeric bound** for a short call (the word “Unlimited” is not a number).
>
> Do **not** implement Black-76 (or any pricing formula) from memory. Do **not** fill fixture Δ, Γ, Θ. Do **not** register `derived/iv_smile`, `derived/drawdown_ladder`, or max pain. Shipping fixture greeks is a lie-by-shipping.

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Agent matrix | `agent/src/data/matrix.rs` `known_physics` (read-only; not edited) | 2026-08-27 | `derived` / `greeks` → `BoundedSnapshot`; `derived` / `synthetic_future` → `BoundedSnapshot`. No `iv_smile`, `drawdown_ladder`, or max-pain id. |
| Kotak REST | [`../kotak-neo/REST.md`](../kotak-neo/REST.md) | 2026-08-27 | `nse_fo` appears in sample `filesPaths`; v1 refuses F&O CSV. Quotes `quote_type` includes `oi`. No pricing model, no greeks. |
| B6 sheet | [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) | 2026-08-27 | F&O segments refused in v1. No options model. |
| NSE Nifty 50 F&O product page | https://www.nseindia.com/static/products-services/equity-derivatives-nifty50 | 2026-08-27 | HTTP 200 title “NIFTY 50 F&O”. Contract-spec paragraphs **not present** in static HTML (SPA). **Not used as a formula source.** |
| NSE individual-securities F&O product page | https://www.nseindia.com/static/products-services/equity-derivatives-individual-securities | 2026-08-27 | HTTP 200. Same: no extractable contract-spec body. **Not used as a formula source.** |
| Legacy NSE Nifty FO HTML | https://www.nseindia.com/products/content/derivatives/equities/nifty_50_FO.htm | 2026-08-27 | HTTP **404**. Cannot quote. |
| NCL settlement-mechanism page | https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism | 2026-08-27 | Title “Settlement Mechanism”; `<div class="body"></div>` empty in fetch. **Not used as a formula source.** |
| Binance Global options mechanics | [`../../crypto/binance-global/options/MECHANICS.md`](../../crypto/binance-global/options/MECHANICS.md) | 2026-07-02 | **Different venue** (`eapi.binance.com`, USDT). Contrast only. Do not copy formulas onto NFO. |
| Binance Global options REST | [`../../crypto/binance-global/options/REST.md`](../../crypto/binance-global/options/REST.md) | 2026-07-02 | `GET /eapi/v1/…` — not NFO. |
| Hull | — | — | **Not on this snapshot.** No page citation → no formula. |
| Kotak docs naming Black-76 / Black–Scholes / greeks | REST.md + Trade API guide + SDK Quotes/Scrip_Master as cited from REST.md | 2026-08-27 | **No named options pricing model.** |

---

## Concepts

---

### Identity — `derived/greeks` / `BoundedSnapshot`

**Source:** `agent/src/data/matrix.rs` `known_physics`

**Verbatim definition / formula:**

```
(Family::Derived, "greeks") => BoundedSnapshot
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `derived` | capability family |
| ID | `greeks` | capability id |
| Physics | `BoundedSnapshot` | physics |
| Δ Γ Θ (and other greek) formulas | **NOT SPECIFIED IN SOURCE** | — |
| Units (₹ vs per-point vs per-lot) | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Which greeks are in the snapshot (Δ, Γ, Θ, ν, ρ, …).
- Sign convention (buyer vs writer).
- Per-unit vs per-lot vs rupee delta.

> **OUR INTERPRETATION**
>
> - The **id is registered**. That is not a license to compute or fixture numbers.
> - A `BoundedSnapshot` with empty/unavailable greeks and a reason is honest. A snapshot stuffed with ΔΓΘ from a fixture or from memory is the design’s largest lie-by-shipping.
> - Do not edit the matrix in this track.

---

### Identity — `derived/synthetic_future` / `BoundedSnapshot`

**Source:** `agent/src/data/matrix.rs` `known_physics`

**Verbatim definition / formula:**

```
(Family::Derived, "synthetic_future") => BoundedSnapshot
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `derived` | capability family |
| ID | `synthetic_future` | capability id |
| Physics | `BoundedSnapshot` | physics |
| Put-call / synthetic-future formula | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Exact identity (which strikes, which expiry, futures vs spot, costs of carry).
- Units and rounding.

> **OUR INTERPRETATION**
>
> - **Citation-only on provenance, never a hero number.** If a later primary source states a synthetic-future identity, the number may appear in provenance text with that citation — not as the headline figure.
> - Do not implement put-call parity from memory. Do not edit the matrix.

---

### Horizon σ move is not a registered capability id

**Source:** `agent/src/data/matrix.rs` `known_physics` (closed list of derived ids in this snapshot); product fence for this track

**Verbatim definition / formula:**

Derived ids present in `known_physics` on this snapshot:

```
(Family::Derived, "ohlcv")
(Family::Derived, "greeks")
(Family::Derived, "synthetic_future")
```

There is no `derived/iv_smile`, no `derived/drawdown_ladder`, no max-pain id, and no id for a horizon volatility move.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Horizon σ move | **Not a registered capability id** | — |
| `derived/iv_smile` | **Not a registered capability id** | — |
| `derived/drawdown_ladder` | **Not a registered capability id** | — |
| max pain | **Not a registered capability id** | — |
| IV × √t scaling formula | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Which IV (implied from LTP, from a vendor, from NSE) and which `t` (calendar, trading, year-fraction convention).

> **OUR INTERPRETATION**
>
> - Station may later *compose* IV with a time factor in UI. That composition is **not** a capability to register as `derived/iv_smile`, `derived/drawdown_ladder`, max pain, or a horizon-σ id.
> - Do not write √t / 365 / 365.25 as if sourced. Day-count remains a BLOCKER.

---

### Greeks in ₹ need NFO mechanics + a named model on provenance

**Source:** BLOCKER; NSE fetches 2026-08-27 (no extractable spec body); Kotak REST.md (no model); Hull not cited

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

No NSE circular, NCL rulebook paragraph, Hull page, or Kotak document in the inventory names the model Station must use, nor the lot/multiplier/premium-unit chain that turns a model greek into rupees.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Named model | **NOT SPECIFIED IN SOURCE** | — |
| Lot size / multiplier | **NOT SPECIFIED IN SOURCE** (NFO scrip-master schema is out of this doc; v1 Kotak refuses `nse_fo` CSV — REST.md) | — |
| Premium unit | **NOT SPECIFIED IN SOURCE** | — |
| Provenance field for model name | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Black-76 vs Black–Scholes vs exchange theoretical price vs “use LTP, skip model”.
- Index option vs stock option differences that change ₹ scaling.
- Rounding of rupee greeks.

> **OUR INTERPRETATION**
>
> - A rupee greek is two citations: **NFO mechanics** (lot, multiplier, units, settlement) **and** a **named model** on provenance.
> - Until both exist in a reference doc with verbatim source text, leave the path unimplemented. No fixture ΔΓΘ.

---

### Synthetic future — citation-only, never a hero number

**Source:** Same as identity `derived/synthetic_future`; no Hull page; no NSE extract

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Hero / headline synthetic future | Forbidden until a cited identity exists | product fence |
| Provenance citation | Required if a number is ever shown | product fence |

**Gaps (NOT SPECIFIED IN SOURCE):**

- The identity itself (previous concept).

> **OUR INTERPRETATION**
>
> - Repeating the fence so it cannot be “interpreted away”: **never** put synthetic future in the hero slot. Provenance-only if a later source supplies a formula with page or circular number.

---

### Terminal payoff — “Unlimited” stays a word until a hedge bounds it

**Source:** No NFO payoff circular in inventory; Binance MECHANICS.md is a different venue (not copied)

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

This snapshot does not contain an NSE/NCL verbatim payoff formula for NFO calls or puts, nor a cited hedge that replaces an unbounded short-call loss with a number.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Short call max loss | **NOT SPECIFIED IN SOURCE** as a number | — |
| Word “Unlimited” | Product label until a cited hedge bound exists | word, not a float |
| Hedge bound / haircut | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Exchange payoff at expiry (cash-settled index vs physically settled stock, if and when a circular is snapshotted).
- Any broker or clearing hedge that caps displayed loss.

> **OUR INTERPRETATION**
>
> - UI may show the **word** “Unlimited” for an unbounded short-call *shape*. That word is not `f64::INFINITY` and not a fake cap.
> - A numeric bound requires a **cited hedge**. Do not invent `* 0.29` (see [`../kotak-neo/MARGIN-CALCULATOR.md`](../kotak-neo/MARGIN-CALCULATOR.md)).
> - Do not copy Binance MECHANICS.md’s “Loss potentially unbounded (for calls)” as NFO law — different venue, and that file itself mixes confirmed endpoints with unconfirmed exercise notes.

---

### Venue contrast — Binance Global options are not NFO

**Source:** [`../../crypto/binance-global/options/MECHANICS.md`](../../crypto/binance-global/options/MECHANICS.md); [`../../crypto/binance-global/options/REST.md`](../../crypto/binance-global/options/REST.md)

**Verbatim definition / formula:**

MECHANICS.md header:

```
Exchange: Binance Global only (`eapi.binance.com`) — NOT available on Binance.US
TradeAutopsy status: NOT BUILDING — reference only
```

REST.md base:

```
https://eapi.binance.com
```

MECHANICS.md states Binance options are described there as European-style and USDT-settled, with `GET /eapi/v1/marginAccount` for writers. That file’s P&L section marks the exact formula **NOT SPECIFIED IN SOURCE in this thread**.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Venue | Binance Global European options | crypto options |
| NFO | NSE F&O segment (Kotak `nse_fo` in REST.md sample filenames) | India listed derivatives |
| Shared formula | **None cited.** Do not copy. | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Any mapping of Binance `eapi` fields onto NFO contracts.

> **OUR INTERPRETATION**
>
> - Read MECHANICS.md only to remember **crypto options are a different venue**.
> - Do not copy its structural P&L bullets, theta wording, or margin notes into NFO calculators or fixtures.

---

### Named model, day-count, rate, and IV-from-LTP (all blank)

**Source:** Full inventory. NSE product HTML had no spec body. Hull not opened. Kotak REST/Quotes do not name a model.

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Black-76 | **NOT SPECIFIED IN SOURCE** (no circular/rulebook/Hull page/Kotak model name on this snapshot) | — |
| Black–Scholes | **NOT SPECIFIED IN SOURCE** as Station’s NFO model | — |
| Day-count 365 vs 365.25 vs other | **NOT SPECIFIED IN SOURCE** | — |
| Risk-free rate | **NOT SPECIFIED IN SOURCE** | — |
| IV inverted from LTP | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Every input a pricer would need. Left unimplemented on purpose.

> **OUR INTERPRETATION**
>
> - Search snippets and training data that “NSE uses Black–Scholes for base price of new contracts” were **not** captured as verbatim primary text (legacy HTML 404; SPA body empty). They are **not** cited here and must not be coded.
> - Do not write Black-76 from memory. Do not pick 365 vs 365.25. Do not pick a repo-rate. Do not invert IV from LTP.

---

## Verification Checklist

Facts that still need confirming against the live source or a test environment before code is written against them.

- [ ] Snapshot an NSE circular or NCL PDF that states exercise style and settlement (cash vs physical) for index vs stock options — verbatim, not SPA chrome.
- [ ] Snapshot a primary that **names** the model for any theoretical price or greek Station would show, with revision date.
- [ ] If using Hull, record **edition and page** before any formula enters a reference doc.
- [ ] Confirm Kotak still publishes no greeks/model on Quotes / scrip master.
- [ ] Confirm matrix still has `derived/greeks` and `derived/synthetic_future` only (no smile / ladder / max pain ids).
- [ ] Do **not** check off “greeks in ₹ unblocked” until mechanics + named model are both in a reference doc.
- [ ] Do **not** ship fixture ΔΓΘ.

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Write Black-76 (forward price, discounted Black) from memory | No NSE circular / rulebook body / Hull page / Kotak model name on this snapshot | **Left blank.** No Black-76 implementation |
| Treat web-search “Black–Scholes for NSE base price” as a cite | Live NSE spec HTML not extractable; old URL 404 | Not quoted; **NOT SPECIFIED IN SOURCE** |
| Pick 365 vs 365.25 vs 252 | Nothing in inventory | Left blank |
| Pick a risk-free rate (T-bill, MIBOR, zero) | Nothing in inventory | Left blank |
| Invert IV from option LTP and underlying | Kotak Quotes.md documents `ltp` as a quote slice, not an IV solver | Left blank; no IV-from-LTP |
| Fill fixture ΔΓΘ so the UI looks alive | Matrix registers the **id**, not numbers | Forbidden lie-by-shipping; dark/unavailable |
| Register `derived/iv_smile` or max pain | Not in `known_physics` | Do not register |
| Put synthetic future in the hero metric | Identity exists; formula does not | Citation-only; never hero |
| Replace “Unlimited” with a haircut (`* 0.29`) | No hedge formula in inventory | Word stays; haircut forbidden |
| Copy Binance European/USDT/theta notes onto NFO | MECHANICS.md is `eapi.binance.com`, NOT BUILDING | Contrast only; no formula copy |
| Infer cash vs physical settlement from memory | NSE/NCL body not captured | Left **NOT SPECIFIED IN SOURCE** |

No memory fills for pricing formulas, day-count, rates, IV inversion, or rupee scaling. Gaps stay `NOT SPECIFIED IN SOURCE`.

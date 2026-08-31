# NFO Options Pricing / Greeks — `derived/greeks` and `derived/synthetic_future`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | NSE F&O (NFO) options pricing, greeks-in-₹, and synthetic future (Track E / 4b / 4d) |
| **Primary source** | Station capability matrix (`agent/src/data/matrix.rs`, read-only) · [Kotak Neo REST quotes + scrip master](../kotak-neo/REST.md) · [NSE Equity Derivatives contract specifications](https://www.nseindia.com/static/products-services/equity-derivatives-contract-specifications) (body extractable 2026-08-31) · [NCL settlement mechanism](https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism) (body extractable 2026-08-31) · NSE FAOP PDFs attempted 2026-08-31 (Akamai 503) · [Binance Global options mechanics](../../crypto/binance-global/options/MECHANICS.md) (venue contrast only) |
| **Snapshot date** | 2026-08-31 (S5 Slice 1 research; prior 2026-08-27 rows kept) |
| **Source version** | Matrix as in-repo 2026-08-31 · REST.md snapshot 2026-08-27 · NSE contract specs page footer **Updated on: 11/08/2026** · NCL settlement page **Updated On: 27/09/2023** · Binance options MECHANICS.md snapshot 2026-07-02 · nsearchives FAOP67690.pdf / FAOP73928.pdf HTTP **503** this fetch |
| **Staleness warning** | Re-verify nsearchives FAOP PDFs when Akamai is not 503. Exercise/settlement are now cited from NCL HTML. A named **live-trader** greeks model is still required before any Δ/Γ/Θ number. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER** (S5 Slice 1 — fail-closed 2026-08-31)
>
> Exercise style and cash settlement are now cited from NCL (below). The following facts are still **NOT SPECIFIED IN SOURCE** for a **live-trader** greek or synthetic-future *hero number*:
>
> - Named options model for **trader Δ/Γ/Θ** (Black-76 vs Black–Scholes vs other) from an **NSE/NCL PDF body** this snapshot. `nsearchives.nseindia.com/content/circulars/FAOP67690.pdf` and `FAOP73928.pdf` returned **HTTP 503** (Akamai). Third-party HTML copies that name Black-Scholes for **theoretical base price of new / untraded contracts** are **not** this lock and are **not** a licence to show trader greeks.
> - Model inputs: day-count (365 vs 365.25 vs trading days), dividend/borrow, volatility convention, and how IV is taken from LTP (if at all). Kotak Quotes `quote_type` still has no `iv` / greeks.
> - Strike **scale** and expiry **calendar conversion** on the Kotak FO master (`dStrikePrice;` / `lExpiryDate `) — still unspecified; do not ÷100; do not OpenAlgo offset.
> - NFO mechanics to scale greeks into **₹**: multiplier / premium units / per-lot vs per-unit. Lot size is locked per FO row (`lLotSize`/`iLotSize`); that is not a greek formula.
> - Terminal-payoff **numeric bound** for a short call (the word “Unlimited” is not a number).
> - NSE SPAN is **not** trader greeks.
>
> Do **not** implement Black-76 or Black–Scholes from memory or from a 503'd circular. Do **not** fill fixture Δ, Γ, Θ. Do **not** register `derived/iv_smile`, `derived/drawdown_ladder`, or max pain. G0 continues: `extract_greeks` stays `pricing_model_unspecified` / `mark_not_this_slice`. Binance `GET /eapi/v1/mark` stays dark until this lock is real.

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
| NCL settlement-mechanism page | https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism | 2026-08-27 | Title “Settlement Mechanism”; `<div class="body"></div>` empty. **Superseded for body by 2026-08-31 fetch.** |
| NCL settlement-mechanism page | https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism | **2026-08-31** | Body extractable. Footer **Updated On: 27/09/2023**. European + cash settlement (see concept). **No greeks formulas.** |
| NSE Equity Derivatives contract specifications | https://www.nseindia.com/static/products-services/equity-derivatives-contract-specifications | **2026-08-31** | Body extractable (was SPA-empty 2026-08-27). Footer **Updated on: 11/08/2026**. Descriptor `OPTIDX` / CE/PE / `DD-MMM-YYYY`. Price bands “based on its **delta value**” — operating range, **not** a trader-greek snapshot. **No Black-Scholes. No day-count. No strike-scale for Kotak CSV.** |
| NSE FAOP67690 PDF | https://nsearchives.nseindia.com/content/circulars/FAOP67690.pdf | **2026-08-31** | HEAD **HTTP 503** (Akamai). Indexed elsewhere as “Base Price for Future and Option Contracts – Update” (24 Apr 2025). **Not quoted. Not a formula source this snapshot.** |
| NSE FAOP73928 PDF | https://nsearchives.nseindia.com/content/circulars/FAOP73928.pdf | **2026-08-31** | HEAD **HTTP 503**. Indexed as F&O Consolidated Circular 51/2026 (28 Apr 2026). **Not quoted.** |
| Third-party copies of NSE base-price circulars | ricago / teamlease / kite.trade forum PDFs | 2026-08-31 | Name Black-Scholes for **theoretical base price**. **Not NSE/NCL primary.** Do **not** copy formulas. Even if the official PDF later matches, that use is **base price**, not live trader ΔΓΘ. |
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

### Greeks in ₹ need NFO mechanics + a named **trader** model on provenance

**Source:** BLOCKER; NCL 2026-08-31 (exercise/settlement only); NSE contract specs 2026-08-31 (no pricer); FAOP PDFs 503; Kotak REST.md (no model); Hull not cited

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE (trader Δ Γ Θ / ₹ scaling).
```

NCL names European + cash. NSE specs name a **price-band** delta. Neither names a Station greeks snapshot.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Named trader model | **NOT SPECIFIED IN SOURCE** (official PDF body this snapshot) | — |
| Lot size / multiplier | Lot from FO row is locked elsewhere; multiplier/premium-unit chain for **greeks in ₹** **NOT SPECIFIED** | — |
| Provenance field for model name | **NOT SPECIFIED IN SOURCE** | — |

> **OUR INTERPRETATION**
>
> - A rupee greek is two citations: **NFO mechanics** (lot, multiplier, units) **and** a **named trader model** on provenance.
> - Until both exist in a reference doc with verbatim **NSE/NCL** source text, leave the path unimplemented. No fixture ΔΓΘ.
> - Two modules: `greeks_nfo.rs` (`pricing_model_unspecified`) and `greeks_binance_options.rs` (`mark_not_this_slice`). Never one `calculate()` with `if CRYPTO`.

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

**Source:** NCL settlement (cash, European) 2026-08-31; no numeric payoff formula; Binance MECHANICS.md is a different venue (not copied)

**Verbatim definition / formula:**

NCL names cash settlement and automatic exercise. It does **not** write a rupee max-loss number for a short call.

```
NOT SPECIFIED IN SOURCE (numeric bound).
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Short call max loss | **NOT SPECIFIED IN SOURCE** as a number | — |
| Word “Unlimited” | Product label until a cited hedge bound exists | word, not a float |
| Hedge bound / haircut | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Exchange payoff *identity* in rupees at expiry (beyond “cash settled”).
- Any broker or clearing hedge that caps displayed loss.

> **OUR INTERPRETATION**
>
> - UI may show the **word** “Unlimited” for an unbounded short-call *shape*. That word is not `f64::INFINITY` and not a fake cap.
> - A numeric bound requires a **cited hedge**. Do not invent `* 0.29` (see [`../kotak-neo/MARGIN-CALCULATOR.md`](../kotak-neo/MARGIN-CALCULATOR.md)).
> - Do not copy Binance MECHANICS.md’s “Loss potentially unbounded (for calls)” as NFO law — different venue.

---

### NCL — European exercise, cash settlement (not trader greeks)

**Source:** https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism · fetched **2026-08-31** · page footer **Updated On: 27/09/2023**

**Verbatim definition / formula:**

```
For index options contracts and options contracts on individual securities, exercise style is European style. Final Exercise is Automatic on expiry of the option contracts.

Exercise settlement is cash settled by debiting/ crediting of the clearing accounts of the relevant Clearing Members with the respective Clearing Bank.

Final settlement loss/ profit amount for option contracts on Index is debited/ credited to the relevant CMs clearing bank account on T+1 day (T = expiry day).

Final settlement loss/ profit amount for option contracts on Individual Securities is debited/ credited to the relevant CMs clearing bank account on T+1 day (T = expiry day).
```

Also on the same page (futures theoretical, **not** option greeks):

```
F = S * e rt
F = theoretical futures price
S = value of the underlying index
r = rate of interest (MIBOR)
t = time to expiration
Rate of interest may be the relevant MIBOR rate or such other rate as may be specified.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Exercise style | European; automatic on expiry | index **and** individual securities |
| Settlement | cash settled | T+1 (T = expiry day) |
| MIBOR | named for **futures** theoretical daily settlement `F = S * e rt` | not a greek |
| Day-count for `t` | **NOT SPECIFIED IN SOURCE** | — |
| Δ Γ Θ | **not on this page** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Option pricing model, greeks, IV, strike scale, Kotak expiry integer conversion.

> **OUR INTERPRETATION**
>
> - Exercise/settlement are specified. That does **not** light `derived/greeks`.
> - Do not treat futures `F = S * e^rt` as an options pricer.
> - Do not treat “individual securities … cash settled” as permission to invent physical-delivery greeks.

---

### NSE contract specs — delta is a price-band input, not a Station greek

**Source:** https://www.nseindia.com/static/products-services/equity-derivatives-contract-specifications · fetched **2026-08-31** · footer **Updated on: 11/08/2026**

**Verbatim definition / formula:**

```
Instrument | OPTIDX | OPTSTK
Option Type | CE / PE
Expiry Date | DD-MMM-YYYY
Price Bands | A contract specific price range based on its delta value is computed and updated on a daily basis
```

(Index Options and Options on Individual Securities columns.)

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Descriptor expiry | `DD-MMM-YYYY` | security descriptor, **not** Kotak `lExpiryDate ` integer conversion |
| Delta | used to compute **price range** / operating band, updated daily | exchange control, not `derived/greeks` |
| Black-Scholes / Black-76 | **not on this page** | — |
| Kotak `dStrikePrice;` scale | **not on this page** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- How NSE computes that delta. Whether it is N(d1). Day-count. Whether Station may display it.

> **OUR INTERPRETATION**
>
> - “Delta value” here is the exchange’s operating-range input. It is **not** a cited Station BoundedSnapshot of trader Δ.
> - `DD-MMM-YYYY` does not convert `1474554600`. Store `expiry_raw` still.

---

### NSE FAOP PDFs — 503 this snapshot; base-price copies are not trader greeks

**Source:** nsearchives HEAD **2026-08-31**; third-party indexes naming NSE/FAOP/67690 (24 Apr 2025) and NSE/FAOP/73928 (28 Apr 2026)

**Verbatim definition / formula:**

```
HTTP 503 (Akamai) on
https://nsearchives.nseindia.com/content/circulars/FAOP67690.pdf
https://nsearchives.nseindia.com/content/circulars/FAOP73928.pdf
```

No official PDF body this fetch. Third-party pages paraphrase Black-Scholes **theoretical base price** (first day / untraded). That paraphrase is **not** quoted here.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Official circular body | **NOT SPECIFIED IN SOURCE** this snapshot (503) | — |
| Live-trader ΔΓΘ from that circular | **NOT SPECIFIED** even in third-party indexes (they say **base price**) | — |

> **OUR INTERPRETATION**
>
> - Fail-closed. Do not implement from ricago / teamlease / kite.trade.
> - If a later nsearchives body only names base price for new listings, G0 **still** continues (plan fail-closed row).

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

### Named model, day-count, rate, and IV-from-LTP (trader greeks still blank)

**Source:** Full inventory 2026-08-31. NCL names MIBOR only for futures theoretical settlement. NSE specs have no model. FAOP PDFs 503. Hull not opened. Kotak REST/Quotes do not name a model.

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

- [x] Snapshot NCL HTML that states European exercise and cash settlement (index **and** individual securities) — fetched 2026-08-31. **Not** a greeks unblocking.
- [ ] Snapshot an **NSE/NCL PDF body** (not 503, not third-party HTML) that names the model Station would use for **trader** theoretical price or greeks, with revision date.
- [ ] If using Hull, record **edition and page** before any formula enters a reference doc — founder must name it; do not silently switch.
- [ ] Confirm Kotak still publishes no greeks/model on Quotes / scrip master.
- [x] Confirm matrix still has `derived/greeks` and `derived/synthetic_future` only (no smile / ladder / max pain ids). Two book modules; both dark.
- [ ] Do **not** check off “greeks in ₹ unblocked” until mechanics + named **trader** model are both in this doc from official PDF/HTML body.
- [x] Do **not** ship fixture ΔΓΘ. Tests still `data: None`.
- [ ] Official strike scale + expiry calendar conversion (still raw cells).

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Write Black-76 (forward price, discounted Black) from memory | No official FAOP PDF body this snapshot (503). NCL names futures `F = S * e rt` only. | **Left blank.** No Black-76 implementation |
| Treat web-search / ricago / teamlease “Black–Scholes for NSE base price” as a cite | nsearchives FAOP67690.pdf **503**. Those copies are not the primary. Even they name **base price**, not trader greeks. | Not quoted; **NOT SPECIFIED IN SOURCE** for `derived/greeks` |
| Pick 365 vs 365.25 vs 252 | NCL `t = time to expiration` with no day-count | Left blank |
| Pick a risk-free rate (T-bill, MIBOR, zero) for **options greeks** | NCL names MIBOR for **futures** theoretical settlement only | Left blank for greeks; do not copy futures `r` onto options Δ |
| Invert IV from option LTP and underlying | Kotak Quotes.md documents `ltp` as a quote slice, not an IV solver | Left blank; no IV-from-LTP |
| Fill fixture ΔΓΘ so the UI looks alive | Matrix registers the **id**, not numbers | Forbidden lie-by-shipping; dark/unavailable |
| Treat NSE “price range based on its delta value” as Station Δ | Contract specs: operating band, updated daily. No formula. | Not a `derived/greeks` payload |
| Register `derived/iv_smile` or max pain | Not in `known_physics` | Do not register |
| Put synthetic future in the hero metric | Identity exists; formula does not | Citation-only; never hero |
| Replace “Unlimited” with a haircut (`* 0.29`) | No hedge formula in inventory | Word stays; haircut forbidden |
| Copy Binance European/USDT/theta notes onto NFO | MECHANICS.md is `eapi.binance.com` | Contrast only; NCL now names European/cash for **NFO** separately |
| Infer cash vs physical settlement from memory | NCL 2026-08-31: **cash settled** for index **and** individual securities | Cited; still not a greek |
| Light Binance `GET /eapi/v1/mark` because NFO research moved | Plan: same ship window, not earlier. Lock still dark. | `mark_not_this_slice`; path not allowlisted |
| One `calculate()` with `if CRYPTO` | Book law: two modules or zero | Dispatcher by `book_id` only |

No memory fills for pricing formulas, day-count, option greeks rate, IV inversion, or rupee scaling. Gaps stay `NOT SPECIFIED IN SOURCE`. G0 continues.

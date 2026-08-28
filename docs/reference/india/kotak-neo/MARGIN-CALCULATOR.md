# Kotak Neo Margin Calculator — `account/margin_estimate`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo margin *calculator* vs order `check-margin` / RMS `limits` (Track D / 3C.2) |
| **Primary source** | [Kotak Neo Trade API guide](https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/) · [API migration guide PDF](https://www.kotakneo.com/uploads/API_Migration_guide_03_12_2025_accfccef45.pdf) · [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) `docs/Margin_Required.md`, `docs/Limits.md`, `settings.py` `PROD_URL`, `margin_api.py`, `limits_api.py` |
| **Snapshot date** | 2026-08-27 |
| **Source version** | Trade API guide page “Updated: 22 May 2026, 5:16 PM IST” · migration PDF filename `API_Migration_guide_03_12_2025` · SDK package v2.0.0 / git `main` as fetched 2026-08-27 (same tree REST.md cites as `8cee5bda63bd9334f8501bb23b7f1d2945f93397`) |
| **Staleness warning** | Re-verify against the live Trade API guide, migration PDF, and SDK `PROD_URL` / `Margin_Required.md` / `Limits.md` before treating any path as a calculator. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> The following facts are needed to light `account/margin_estimate` but are **NOT SPECIFIED IN SOURCE**:
>
> - **Margin calculator HTTP host / path / method / auth** as a what-if SPAN/exposure estimator (the entire calculator fetch path).
> - **SPAN vs exposure:** definitions, formulas, which legs they apply to, and how they combine into a total.
> - **Multi-leg / spread** request shape and benefit rules for a calculator (or for `check-margin`).
> - **Caching / freshness:** what a timestamp means, TTL, and whether a cached estimate may be shown.
>
> Product rule: until a reference doc cites **host/path/auth**, **SPAN vs exposure**, *and* **multi-leg behavior**, the margin row is **permanently unavailable with reason**. This document is that reference. Those three are not jointly specified. Feature code must stay a **dark envelope**. Do not implement SPAN. Do not invent a `* 0.29` (or any) hedge haircut. Do not add `extract_margin_estimate` in this track.

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Station REST quotes + scrip master | [`REST.md`](./REST.md) | 2026-08-27 | No calculator path. Documented paths table: file-paths, quotes, cash CSV, history none, `/quick/quotes` unspecified. |
| B6 sheet | [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) | 2026-08-27 | Row 2 names “holdings/positions/limits” as words in fetch path. No calculator host/path/auth. No SPAN vs exposure. No multi-leg. F&O refused in v1. |
| Trade API guide | https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/ | 2026-08-27 | Marketing “margin validation”. Named endpoints in this fetch: file-paths and `{BASE_URL}/quick/order/rule/ms/place` only. |
| Migration guide PDF | https://www.kotakneo.com/uploads/API_Migration_guide_03_12_2025_accfccef45.pdf | 2026-08-27 | Maps “Check Margin” and “Limits” to `{{baseUrl}}/quick/user/check-margin` and `…/limits`. Not labelled a calculator. No SPAN/exposure/multi-leg prose. |
| SDK `PROD_URL` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/settings.py | 2026-08-27 | `"margin": "quick/user/check-margin"`, `"limits": "quick/user/limits"` |
| SDK `Margin_Required.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Margin_Required.md | 2026-08-27 | `client.margin_required(…)` — single instrument/qty. Return type `object`. |
| SDK `margin_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/margin_api.py | 2026-08-27 | **POST**, `Sid`/`Auth`, query `sId`, form body one `tok` |
| SDK `Limits.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Limits.md | 2026-08-27 | `client.limits()` — RMS limits of demat account. Sample JSON field *names* include `SpanMarginPrsnt`, `ExposureMarginPrsnt`. No formulas. |
| SDK `limits_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/limits_api.py | 2026-08-27 | **POST**, `Sid`/`Auth`, query `sId` |
| SDK `neo_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/neo_api.py | 2026-08-27 | `margin_required` / `limits` wrappers require `edit_token` and `edit_sid` |
| Session + trades | [`equities/kotak-neo/SESSION-AND-TRADES.md`](../../equities/kotak-neo/SESSION-AND-TRADES.md) | 2026-07-26 | Mint + trade book. Not a calculator. |
| Agent matrix | `agent/src/data/matrix.rs` (read-only; not edited) | 2026-08-27 | Identity already registered: `account` / `margin_estimate` → `BoundedSnapshot` |

---

## Concepts

---

### Calculator HTTP host / path / method / auth

**Source:** [`REST.md`](./REST.md) documented-paths table; B6 sheet rows 2–3 and M1–M8; Trade API guide fetch 2026-08-27

**Verbatim definition / formula:**

REST.md “Documented paths (this snapshot)” does not list a margin calculator. Its last row:

```
| `/quick/quotes` | **NOT SPECIFIED IN SOURCE** | — | — | Do not guess |
```

B6 row 2 (fetch path) names session REST including the word `limits`. It does not give method, host, or auth for a calculator:

```
REST after session: GET {BASE_URL}/quick/user/trades (trade book / fills), /quick/user/orders, holdings/positions/limits.
```

Trade API guide (fetched 2026-08-27) named endpoints:

```
To understand how to fetch live market data with Kotak Neo API, use the endpoint:
{BASE_URL}/script-details/1.0/masterscrip/file-paths.

Send a POST request to {BASE_URL}/quick/order/rule/ms/place.
```

The same page’s overview sentence (not an endpoint table):

```
…while also offering access to real-time market data and margin validation.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Calculator host | **NOT SPECIFIED IN SOURCE** | — |
| Calculator path | **NOT SPECIFIED IN SOURCE** | — |
| Calculator method | **NOT SPECIFIED IN SOURCE** | — |
| Calculator auth | **NOT SPECIFIED IN SOURCE** | — |
| “margin validation” | Trade API guide overview prose only | marketing sentence |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Any dedicated calculator hostname (as opposed to session `baseUrl` used for orders).
- Any path whose documented purpose is SPAN/exposure what-if for a proposed book.
- Method, headers, query, and body for such a calculator.
- Whether “margin validation” in the guide means `check-margin`, `limits`, UI-only, or something else.

> **OUR INTERPRETATION**
>
> - The **calculator fetch path is unspecified**. REST.md and B6 do not cite one. The Trade API guide does not name a calculator URL.
> - Adjacent order-RMS and account-limits calls (next two concepts) are **not** this calculator. Mapping them onto `account/margin_estimate` would ship a number the product rule forbids.
> - Station identity for a future row, if ever lit: `account` / `margin_estimate`, physics `BoundedSnapshot` (already on the agent matrix). Until the three-part citation exists, the row stays **unavailable with reason** (dark envelope). This track does not add `extract_margin_estimate`.

---

### Adjacent — Check Margin (`margin_required` / `check-margin`) is not a calculator

**Source:** Migration guide PDF “Others / Check Margin”; `PROD_URL["margin"]`; `docs/Margin_Required.md`; `margin_api.py`; `NeoAPI.margin_required`

**Verbatim definition / formula:**

Migration PDF (extracted 2026-08-27):

```
Others      Check         /Orders/2.0/.../     {{baseUrl}}/quick/use     Use baseUrl. Drop
            Margin        check-margin         r/check-margin            ‘Authorization’ in
                                                                         headers.
```

SDK prod path (`settings.py`):

```
"margin": "quick/user/check-margin"
```

`Margin_Required.md` title and wrapper:

```
Get required margin details

client.margin_required(exchange_segment = "", price = "", order_type= "", product = "", quantity = "", instrument_token = "",
                       transaction_type = "")
```

SDK README line (same repo, cited from search of this tree; `NeoAPI.margin_required` docstring):

```
Calculates the margin required for a given trade using the NEO API.
```

HTTP (`margin_api.py`):

```
POST {get_url_details("margin")}
Query: sId={serverId}
Headers: Sid={edit_sid}, Auth={edit_token}, Content-Type=application/x-www-form-urlencoded
Body: exSeg, prc, prcTp, prod, qty, tok, trnsTp, trgPrc, brkName, brnchId, slAbsOrTks, slVal, sqrOffAbsOrTks, sqrOffVal, trailSL, tSLTks
```

`NeoAPI.margin_required` requires `edit_token` and `edit_sid`; otherwise:

```
Complete the 2fa process before accessing this application
```

`Margin_Required.md` parameter table `order_type`: `L`, `MKT`, `SL`, `SL-M` only. One `instrument_token` / `quantity`. Sample `data` keys: `avlCash`, `totMrgnUsd`, `mrgnUsd`, `ordMrgn`, `rmsVldtd`, `reqdMrgn`, `avlMrgn`, `insufFund`, `stat`, `stCode`. Return type: **object**.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Path | `quick/user/check-margin` (`PROD_URL`); `{{baseUrl}}/quick/user/check-margin` (migration PDF) | path |
| Method | `POST` (`margin_api.py`) | HTTP |
| Host | Session `baseUrl` via `get_url_details` — not a second documented calculator host | URL |
| `Sid` / `Auth` | `edit_sid` / `edit_token` (`margin_api.py`) | headers |
| `sId` | `configuration.serverId` | query |
| `tok` | `instrument_token` — “pSymbol in ScripMaster files” (`Margin_Required.md`) | string |
| `qty` | “Quantity of the order” | string |
| Sample `totMrgnUsd` / `mrgnUsd` | Keys in sample JSON only | **units NOT SPECIFIED IN SOURCE** (names contain `Usd`; B6 currency is INR) |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether this call is a SPAN calculator, an OMS RMS pre-trade check, or both.
- Field semantics and units for `reqdMrgn` / `ordMrgn` / `totMrgnUsd` / `mrgnUsd`.
- SPAN vs exposure split on this response (those keys are absent from the sample).
- Multi-leg body (no array of legs; `order_type` table omits `SP` / `2L` / `3L` even though `settings.py` `order_type` maps those for *place order*).
- Timestamp / cache headers on the response.

> **OUR INTERPRETATION**
>
> - Cite `check-margin` as **order margin required for one ticket**, not as the missing calculator.
> - Host/path/auth for *this* POST are specified in the SDK. That does **not** satisfy the product rule, which also requires SPAN vs exposure *and* multi-leg behavior.
> - Do not treat this POST as `account/margin_estimate`. Place/modify/cancel remain not-data (B6 M8 / REST.md).

---

### Adjacent — Limits snapshot names SPAN and exposure; it is not a calculator

**Source:** `docs/Limits.md`; `limits_api.py`; `PROD_URL["limits"]`; migration PDF “Limits”; B6 row 2 word `limits`

**Verbatim definition / formula:**

`Limits.md`:

```
Get RMS Limits details of your demat account

client.limits()
```

HTTP (`limits_api.py`):

```
POST {get_url_details("limits")}
Query: sId={serverId}
Headers: Sid={edit_sid}, Auth={edit_token}, Content-Type=application/x-www-form-urlencoded
Body: seg, exch, prod
```

Sample JSON keys (names only; `Limits.md` sample): `SpanMarginPrsnt`, `ExposureMarginPrsnt`, `NfospreadBenefit`, `TimeStamp` (sample value `"1737540175823"`), plus many other `*Prsnt` keys. No prose definitions. Return type: **object**.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Path | `quick/user/limits` | path |
| Method | `POST` | HTTP |
| `SpanMarginPrsnt` | Sample key only — **definition NOT SPECIFIED IN SOURCE** | unknown |
| `ExposureMarginPrsnt` | Sample key only — **definition NOT SPECIFIED IN SOURCE** | unknown |
| `NfospreadBenefit` | Sample key only — **definition NOT SPECIFIED IN SOURCE** | unknown |
| `TimeStamp` | Sample key and numeric string — **unit/timezone NOT SPECIFIED IN SOURCE** | unknown |

**Gaps (NOT SPECIFIED IN SOURCE):**

- How `SpanMarginPrsnt` and `ExposureMarginPrsnt` are computed, whether they are INR, and how they combine.
- Whether `NfospreadBenefit` is multi-leg SPAN offset.
- Meaning of `TimeStamp` (epoch ms vs other).
- Whether `limits` may be shown as a what-if estimate for a *proposed* order.

> **OUR INTERPRETATION**
>
> - Field *names* on an account RMS snapshot are not SPAN/exposure mechanics and not a calculator.
> - Do not parse `SpanMarginPrsnt` / `ExposureMarginPrsnt` into `account/margin_estimate`.
> - Do not implement SPAN from these names.

---

### SPAN vs exposure

**Source:** `Limits.md` sample keys; `Margin_Required.md` sample; REST.md; B6; Trade API guide

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

The only primary-source *strings* found in this inventory are sample JSON keys `SpanMarginPrsnt` and `ExposureMarginPrsnt` on `Limits.md` (previous concept). No formula, no circular, no “SPAN plus exposure equals …” sentence appears in REST.md, B6, the Trade API guide fetch, `Margin_Required.md`, or the migration PDF extract.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| SPAN | **NOT SPECIFIED IN SOURCE** (key name only on Limits sample) | — |
| Exposure | **NOT SPECIFIED IN SOURCE** (key name only on Limits sample) | — |
| Combined initial margin | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Clearing-corporation SPAN file, scenario set, or look-ahead.
- Exposure percentage, underlying, and product (NRML vs MIS).
- Premium add-on, delivery margin, additional/ELM, and how they sit relative to SPAN/exposure.
- Whether Kotak’s RMS equals NCL SPAN.

> **OUR INTERPRETATION**
>
> - Do not implement SPAN. Do not fill exposure as a percentage from memory.
> - Until a primary source defines both components and the combination rule, the product rule’s SPAN-vs-exposure citation is unmet.

---

### Multi-leg / spread

**Source:** `Margin_Required.md` parameters; `settings.py` `order_type`; `Limits.md` sample `NfospreadBenefit`

**Verbatim definition / formula:**

`Margin_Required.md` `order_type` column:

```
L - Limit MKT - Market SL - Stop loss limit SL-M - Stop loss market
```

`settings.py` `order_type` also maps (for other APIs, not documented on `Margin_Required.md`):

```
"Spread": "SP", "SP": "SP", "2L": "2L", "Two Leg": "2L", "3L": "3L", "Three leg": "3L"
```

`Limits.md` sample includes key `NfospreadBenefit` with no definition.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Multi-leg calculator request | **NOT SPECIFIED IN SOURCE** | — |
| `NfospreadBenefit` | Sample key only | unknown |
| `2L` / `3L` / `SP` on `margin_required` | **NOT SPECIFIED IN SOURCE** (absent from `Margin_Required.md` table) | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Body shape for two- or three-leg margin (list of `tok`/`qty`/`trnsTp`, net vs gross).
- Which spreads receive benefit, calendar vs same-expiry, index vs stock.
- Whether `check-margin` accepts `prcTp` `2L`/`3L`/`SP`.

> **OUR INTERPRETATION**
>
> - Place-order spread enums in `settings.py` are not a multi-leg *calculator* spec.
> - Product rule’s multi-leg citation is unmet. No invented spread haircut.

---

### Caching / timestamp

**Source:** `Limits.md` sample `TimeStamp`; `Margin_Required.md` sample (no time field)

**Verbatim definition / formula:**

`Limits.md` sample:

```
"TimeStamp": "1737540175823"
```

`Margin_Required.md` sample `data` object has no time key.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Calculator cache TTL | **NOT SPECIFIED IN SOURCE** | — |
| `TimeStamp` on limits | Sample string only — **unit NOT SPECIFIED IN SOURCE** | unknown |
| `check-margin` as-of time | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether Station may cache an estimate, for how long, and under which session.
- Clock (exchange vs broker vs device) and timezone.
- Staleness rule for UI.

> **OUR INTERPRETATION**
>
> - Do not assume epoch milliseconds from the sample’s digit length.
> - A dark envelope does not need a cache policy; a lit row would.

---

### Invented hedge haircut (`* 0.29` or any) is forbidden

**Source:** Full inventory above (REST.md, B6, Trade API guide, migration PDF, `Margin_Required.md`, `Limits.md`)

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

No `0.29`, “29%”, hedge ratio, or haircut formula appears in the sources listed in Source Inventory.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Hedge haircut | **NOT SPECIFIED IN SOURCE** | — |
| `0.29` | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Any numeric short-option or spread haircut Kotak or NCL wants Station to apply locally.

> **OUR INTERPRETATION**
>
> - **Forbidden:** `* 0.29`, “29% of SPAN”, or any invented hedge haircut in Station, Notch, fixtures, or comments.
> - A bound on short-option payoff is a *citation* problem (see [`india/nfo/OPTIONS-PRICING.md`](../nfo/OPTIONS-PRICING.md)), not a constant to invent here.

---

### Identity — `account/margin_estimate` / `BoundedSnapshot` (dark envelope)

**Source:** `agent/src/data/matrix.rs` `known_physics` (read-only); product rule in this document’s BLOCKER

**Verbatim definition / formula:**

Matrix pair already registered (do not edit the matrix in this track):

```
(Family::Account, "margin_estimate") => BoundedSnapshot
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `account` | capability family |
| ID | `margin_estimate` | capability id |
| Physics | `BoundedSnapshot` | physics |
| Availability | Unspecified calculator + unspecified SPAN/exposure + unspecified multi-leg → **unavailable with reason** | product rule |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Payload schema for a lit `margin_estimate` snapshot.
- Reason-code string enum for the dark row (leave to the agent that owns the stub, if any).

> **OUR INTERPRETATION**
>
> - Identity is already on the matrix so the *name* exists. This reference **does not** light the row.
> - Permanently unavailable with reason until a later snapshot of a primary source cites host/path/auth **and** SPAN vs exposure **and** multi-leg.
> - Another agent may add an `extract_margin_estimate` stub that returns unavailable. This track does not.

---

## Verification Checklist

Facts that still need confirming against the live source or a test environment before code is written against them.

- [ ] Confirm the Trade API guide still names no calculator URL (re-fetch).
- [ ] Confirm migration PDF still maps Check Margin → `{{baseUrl}}/quick/user/check-margin` and Limits → `{{baseUrl}}/quick/user/limits` without SPAN/multi-leg prose.
- [ ] Confirm `Margin_Required.md` still has no SPAN/exposure fields and no multi-leg parameter table.
- [ ] Confirm `Limits.md` still documents `SpanMarginPrsnt` / `ExposureMarginPrsnt` as unexplained sample keys only.
- [ ] Do **not** check off “calculator path specified” unless a primary source adds host, path, method, and auth *for a calculator*.
- [ ] Do **not** ship `account/margin_estimate` as a number while this BLOCKER stands.

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Treat SDK `check-margin` as the missing calculator so the row can light | Migration/SDK: “Check Margin” / “margin required for a given trade”; one `tok`/`qty`; no SPAN split | Documented as adjacent RMS check; calculator path remains **NOT SPECIFIED IN SOURCE** |
| Implement SPAN from `SpanMarginPrsnt` | Limits.md: sample key only | No SPAN implementation; definition gap kept |
| Fill exposure as a % of notional from clearing folklore | No exposure formula in inventory | Left **NOT SPECIFIED IN SOURCE** |
| Allow `2L`/`3L` on `margin_required` because `settings.py` maps them | Those maps are not on `Margin_Required.md` | Multi-leg calculator behavior **NOT SPECIFIED IN SOURCE** |
| Assume `TimeStamp` is Unix ms | Sample is a numeric string with no unit | Unit left unspecified |
| Assume `totMrgnUsd` is USD | Key name only; B6 says INR; no unit sentence | Units **NOT SPECIFIED IN SOURCE** |
| Invent `* 0.29` hedge haircut so short options look bounded | No 0.29 in any cited source | Forbidden; recorded as unspecified and banned |
| Write `extract_margin_estimate` “just as a stub” | Product rule: dark envelope until three-part citation | No code in this track |
| Copy Binance `GET /eapi/v1/marginAccount` | Different venue | Not used |

No memory fills for calculator host/path/auth, SPAN math, multi-leg benefit, or haircuts. Gaps stay `NOT SPECIFIED IN SOURCE`.

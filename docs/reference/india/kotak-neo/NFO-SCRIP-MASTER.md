# Kotak Neo NFO / F&O scrip master — named book `kotak-nse-nfo`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo NSE F&O (NFO) scrip-master CSV — column names, token, lot, tick, strike cell, expiry cell for book `kotak-nse-nfo` |
| **Primary source** | Lock [`/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md`](/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md) (unsigned GET **2026-08-28 IST**) · live CSV `https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2026-08-28/transformed/nse_fo.csv` · [Kotak-Neo/Kotak-neo-api-v2 `docs/Scrip_Master.md`](https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md) · Station [REST.md](./REST.md) |
| **Snapshot date** | 2026-08-28 |
| **Source version** | Lock fetch 2026-08-28 IST, HTTP 200 · Trade API guide “Updated: 22 May 2026” · SDK `Scrip_Master.md` sample date `2025-01-22` (that dated URL returned **403** this fetch) |
| **Staleness warning** | Re-verify the live dated `nse_fo.csv` header against the lock before changing extract field names. Strike **scale**, expiry **calendar conversion**, CNC on `nse_fo`, and a named greeks model remain unspecified — do not implement those paths from memory. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER** *(column **names** closed 2026-08-28 — remaining facts still NOT SPECIFIED IN SOURCE)*
>
> Live unsigned GET of `transformed/nse_fo.csv` (lock 2026-08-28 IST) names the 79 columns below. `pToken` is **absent**. The following are still **NOT SPECIFIED IN SOURCE** and must stay unimplemented:
>
> - Official **strike scale** (`dStrikePrice;` sample `2.1e+06` vs `21000` in `pTrdSymbol`). Do **not** ÷100. Do **not** invent **57500**. Store the raw cell.
> - **Expiry integer → calendar** (`lExpiryDate ` / `pExpiryDate` sample `1474554600`). Offset / epoch meaning unspecified. Store the raw integer. Do not convert.
> - Which lot column wins if `lLotSize` ≠ `iLotSize` (this sample they match at **65**). Skip the row. Do not guess.
> - **CNC on `nse_fo`**. Do not enable. Do not invent a refuse.
> - Named NFO **greeks / pricing model** (Black-76 or otherwise). See [`../nfo/OPTIONS-PRICING.md`](../nfo/OPTIONS-PRICING.md).
>
> Cash book `kotak-nse-bse-cash` still **refuses FO**. Do not copy cash `pSymbolName`-as-token onto this book. Do not allowlist this FO URL on the cash book.

---

## Source Inventory

List every file, page, or paper you read to produce this document.

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| NFO lock (charge / session / scrip-header SoT) | `/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md` | 2026-08-28 | Named book `kotak-nse-nfo`. Unsigned GET 2026-08-28 IST. Header + sample row verbatim. |
| Live NFO scrip CSV | `https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2026-08-28/transformed/nse_fo.csv` | 2026-08-28 IST | Unsigned GET, HTTP 200. Filename `transformed/nse_fo.csv` (not `-v1`). Do **not** commit CSV bytes. |
| Same date `transformed-v1/nse_fo-v1.csv` | same host / date / `transformed-v1/` | 2026-08-28 IST | HTTP **403**. Not the live FO file. |
| SDK sample dated `2025-01-22` FO CSV | SDK `Scrip_Master.md` sample path | 2026-08-28 IST | HTTP **403**. |
| SDK `docs/Scrip_Master.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md | 2026-08-28 | Sample `filesPaths` lists `nse_fo.csv` (and BFO/CDS/MCX). Listing ≠ this book’s allowlist: **`nse_fo` only**. Return type `object`. No official column table. |
| Trade API guide | https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/ | 2026-08-28 | Downloadable CSV files; file-paths GET. No CSV schema. |
| SDK `settings.py` `exchange_segment` | Kotak-neo-api-v2 | 2026-08-27 | `"NFO": "nse_fo"`. |
| Station REST cash columns | [`REST.md`](./REST.md) | 2026-08-27 | Cash `nse_cm-v1.csv` / `bse_cm-v1.csv`: token = `pSymbol`, ticker = `pSymbolName`. **Cash-only.** Do not copy `pSymbolName`-as-token onto FO. |
| `kotak_scrip_master.rs` header | `agent/src/kotak_scrip_master.rs` (not edited this slice) | 2026-08-27 | Cash parser. Cash book still refuses FO CSV. |
| OPTIONS-PRICING.md | [`../nfo/OPTIONS-PRICING.md`](../nfo/OPTIONS-PRICING.md) | 2026-08-27 | Greeks model unspecified. No Black-76. |
| OpenAlgo Kotak NFO mapper | https://github.com/marketcalls/openalgo/blob/main/broker/kotak/database/master_contract_db.py | 2026-08-27 | Third-party: `dStrikePrice/100`, `lExpiryDate+315513000`. **Not** official. Do not copy. |
| B6 sheet | [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) | 2026-08-28 | Cash row 1 / row 12 / M1 still refuse FO **on the cash book**. Named NFO book is a different book. |

This slice does **not** commit `nse_fo.csv` bytes into the repo.

---

## Concepts

*Add one sub-section per concept, formula, or definition. Copy the block below as many times
as needed. Never merge two concepts into one block.*

---

### Live FO CSV URL and 79-column header (column names specified)

**Source:** Lock `/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md` “Live FO scrip-master header”; unsigned GET 2026-08-28 IST, HTTP 200.

**Verbatim definition / formula:**

URL:

```
https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2026-08-28/transformed/nse_fo.csv
```

Header row VERBATIM (79 columns; `pToken` absent):

```
pSymbol,pGroup,pExchSeg,pInstType,pSymbolName,pTrdSymbol,pOptionType,pScripRefKey,pISIN,pAssetCode,pSubGroup,pCombinedSymbol,pDesc,pAmcCode,pContractId,dTickSize ,lLotSize,lExpiryDate ,lMultiplier ,lPrecision,dStrikePrice;,pExchange,pInstName,pExpiryDate,pIssueDate,pMaturityDate,pListingDate,pNoDelStartDate,pNoDelEndDate,pBookClsStartDate,pBookClsEndDate,pRecordDate,pCreditRating,pReAdminDate,pExpulsionDate,pLocalUpdateTime,pDeliveryUnits,pPriceUnits,pLastTradingDate,pTenderPeridEndDate,pTenderPeridStartDate,pSellVarMargin,pBuyVarMargin,pInstrumentInfo,pRemarksText,pSegment,pNav,pNavDate,pMfAmt,pSipSecurity,pFaceValue,pTrdUnits,pExerciseStartDate,pExerciseEndDate,pElmMargin,pVarMargin,pTotProposedLimitValue,pScripBasePrice,pSettlementType,pCurrectionTime,iPermittedToTrade,iBoardLotQty ,iMaxOrderSize ,iLotSize,dOpenInterest ,dHighPriceRange ,dLowPriceRange ,dPriceNum   ,dGenDen,dGenNum,dPriceQuatation ,dIssuerate ,dPriceDen,dWarningQty ,dIssueCapital ,dExposureMargin ,dMinRedemptionQty ,lFreezeQty,CASEligible
```

One sample data row VERBATIM (first data row this GET):

```
56526,XX,nse_fo,OPTIDX,NIFTY,NIFTY2692221000PE,PE,NIFTY22SEP2621000.00PE,,26000,,,,,,5,65,1474554600,-1,2,2.1e+06,NSE,OPTIDX,1474554600,1471564800,1474554600,1471564800,0,0,0,0,0,,0,0,1472316328,,,,,,,,,,FO,,,,,,,1474502400,1474554600,,,,15.0000,Cash,,1,1,1801.00,65,0,2015,5,1,1,1,0,0,1,0,1e+12,1,0,1801,false
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Token | `pSymbol` (`pToken` **absent**) | sample `56526` |
| Segment | `pExchSeg` | `nse_fo` only on this book |
| Inst type | `pInstType` | sample `OPTIDX` |
| Option type | `pOptionType` | sample `PE` |
| Underlying ticker | `pSymbolName` | sample `NIFTY` — **not** the token |
| Trading symbol | `pTrdSymbol` | sample `NIFTY2692221000PE` |
| Lot | `lLotSize` **and** `iLotSize` | sample both **65** |
| Tick | `dTickSize ` (trailing space in the live header) | sample `5` |
| Strike column name | `dStrikePrice;` (semicolon in the name) | sample `2.1e+06` |
| Expiry | `lExpiryDate ` / `pExpiryDate` | sample `1474554600` |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Strike scale (see strike concept).
- Expiry calendar conversion (see expiry concept).
- Which lot column wins when they differ (see lot concept).

---

> **OUR INTERPRETATION**
>
> - Column **names** for this book are specified by the lock’s live header. A later CSV parser must match these names, including trailing spaces and the semicolon on `dStrikePrice;`.
> - Tradable id is `nse_fo|{pSymbol}` → sample `nse_fo|56526`. Do not use `pSymbolName` (`NIFTY`) as the token.
> - This URL is **not** allowlisted on `kotak-nse-bse-cash`. Cash still refuses FO.
> - Do not commit the full CSV. Dated fixture above is one contract, not a global NIFTY lot.

---

### Token is `pSymbol`, not cash `pSymbolName`

**Source:** Lock table “Role / Header name(s)”; sample row `56526,…,NIFTY,NIFTY2692221000PE,…`

**Verbatim definition / formula:**

```
Token | pSymbol (pToken absent) | 56526
Ticker / tradingsymbol | pSymbolName / pTrdSymbol | NIFTY / NIFTY2692221000PE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| FO token | `pSymbol` | numeric string in sample `56526` |
| FO `pToken` | **absent** on the live header | — |
| FO `pSymbolName` | underlying ticker (`NIFTY`) | string — **not** token |
| Cash token (other book) | live cash `pSymbol` numeric; cash ticker `pSymbolName` | REST.md 2026-08-27 — **do not copy as FO token** |

**Gaps (NOT SPECIFIED IN SOURCE):**

- None for the token **column name** on this snapshot. Scale of other fields remains blocked.

---

> **OUR INTERPRETATION**
>
> - Cash `pSymbolName`-as-ticker must not be copied as the NFO instrument token. NFO token = `pSymbol`.
> - `extract_contracts` `instrument_id` is `nse_fo|{token}` from `pSymbol`.

---

### Segment is `pExchSeg` = `nse_fo` only

**Source:** Lock “Venue + asset class”; sample `pExchSeg=nse_fo`

**Verbatim definition / formula:**

```
Segments allowed: nse_fo only
Segments refused: nse_cm, bse_cm (cash — other book), bse_fo, cde_fo, mcx_fo, MCX, CDS, FX
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `nse_fo` | This book’s only segment | segment id |
| `nse_cm` / `bse_cm` | Cash book `kotak-nse-bse-cash` | other book |
| `bse_fo` / `cde_fo` / `mcx_fo` | Refused on this book | segment id |

**Gaps (NOT SPECIFIED IN SOURCE):**

- None for the allowlist. CNC validity on `nse_fo` is still unspecified (see CNC concept).

---

> **OUR INTERPRETATION**
>
> - `extract_contracts` for this book sets `segment` to `nse_fo`. Cash / spot / options books are not this extract (`Unavailable`, `data: None`).
> - Cash book continues to refuse FO CSV even though this named book exists.

---

### Lot is `lLotSize` and `iLotSize` (sample 65); disagreement skips the row

**Source:** Lock lot row + golden “qty>1 OPTIONS lot fixture”; sample both columns **65**.

**Verbatim definition / formula:**

```
Lot | lLotSize and iLotSize | both 65
Do not invent 25/50/75. Do not ship fo_mktlots.csv.
Dated fixture below is one contract, not a global NIFTY/BANKNIFTY lot.
```

Lock remaining BLOCKER:

```
Which lot column wins if lLotSize ≠ iLotSize (this fixture they match).
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `lLotSize` | Live header column | sample **65** |
| `iLotSize` | Live header column | sample **65** |
| Winner if they differ | **NOT SPECIFIED IN SOURCE** | skip row — do not guess |
| BANKNIFTY / other contract lot | Must come from **that** row | not a constant |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Which column wins when `lLotSize` ≠ `iLotSize`.

---

> **OUR INTERPRETATION**
>
> - Lot is a **JSON number** on the contract row (sample `65`, not `"65"`). It is a field, not a capability id.
> - A CSV parser (not this extract’s injected-row path) that sees disagreeing lot columns **skips the row**. Do not pick one. Do not average. Do not invent 50.
> - Empty NFO store → envelope `Unavailable` (not `Empty`-as-success, not last:0).

---

### Tick is `dTickSize ` (trailing space in the live header)

**Source:** Lock tick row; header token `dTickSize ` with trailing space.

**Verbatim definition / formula:**

```
Tick | dTickSize  (trailing space is in the live header) | 5
Do not invent a tick.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `dTickSize ` | Header name includes trailing space | sample `5` |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Unit prose (points vs paise) beyond the raw cell.

---

> **OUR INTERPRETATION**
>
> - Parser must not strip the header name into `dTickSize` unless a cited source says the wire is equivalent. This extract does not emit tick (lot + identity only). Do not invent a tick in code.

---

### Strike column `dStrikePrice;` — store raw; scale NOT SPECIFIED

**Source:** Lock strike row; sample `2.1e+06` vs `21000` in `pTrdSymbol`.

**Verbatim definition / formula:**

```
Strike | dStrikePrice; (semicolon in the name) | 2.1e+06 vs 21000 in pTrdSymbol — looks scaled; official scale NOT SPECIFIED IN SOURCE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Column name | `dStrikePrice;` | semicolon is part of the name |
| Sample cell | `2.1e+06` | raw |
| `pTrdSymbol` substring | `21000` in `NIFTY2692221000PE` | not an official scale factor |
| Official ÷100 (or any factor) | **NOT SPECIFIED IN SOURCE** | — |
| Ghost strike 57500 | **not in this sample** | forbidden to invent |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official scale. OpenAlgo `dStrikePrice / 100` is third-party, not this source.

---

> **OUR INTERPRETATION**
>
> - Store `strike_raw` as the cell string (`2.1e+06`). Do **not** ÷100. Do **not** write 21000 or **57500** as a scaled strike.
> - No strike grid on `extract_contracts`. Chain / Pre-trade ladders stay dark until scale is specified.

---

### Expiry `lExpiryDate ` / `pExpiryDate` — store raw integer; calendar NOT SPECIFIED

**Source:** Lock expiry row; sample `1474554600`.

**Verbatim definition / formula:**

```
Expiry | lExpiryDate  / pExpiryDate | 1474554600 (10-digit integer). Calendar conversion / epoch offset NOT SPECIFIED IN SOURCE
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `lExpiryDate ` | Trailing space in the live header | sample `1474554600` |
| `pExpiryDate` | Same sample integer | `1474554600` |
| Calendar / epoch offset | **NOT SPECIFIED IN SOURCE** | — |
| OpenAlgo `+ 315513000` | Third-party | not this source |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether the integer is Unix seconds, an offset epoch, or another clock. Do not convert to a calendar date.

---

> **OUR INTERPRETATION**
>
> - Store `expiry_raw` as the integer string (`1474554600`). Do not apply OpenAlgo’s 10-year offset. Do not format `29-SEP-26` from memory.

---

### CNC on `nse_fo` remains unspecified

**Source:** Lock products NOT SPECIFIED: “CNC on `nse_fo`”.

**Verbatim definition / formula:**

```
CNC on nse_fo — no fetched page says “CNC is invalid on F&O”. README nse_fo expected list is NRML, MIS, BO (omits CNC; does not say invalid). Do not invent a CNC refuse. Do not enable CNC on this book until a page says it is valid.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Products allowed | **NRML**; **MIS** scoped | lock |
| CNC on `nse_fo` | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Validity of CNC on this segment.

---

> **OUR INTERPRETATION**
>
> - This extract does not enable or refuse CNC. Leave that path unimplemented. Cash CNC stays on the cash book.

---

### Greeks / pricing model remains unspecified

**Source:** [`../nfo/OPTIONS-PRICING.md`](../nfo/OPTIONS-PRICING.md) BLOCKER; lock “Do not claim Greeks”; Quotes.md `quote_type` has no greeks.

**Verbatim definition / formula:**

```
Named options model for NFO … NOT SPECIFIED IN SOURCE
Do not implement Black-76 (or any pricing formula) from memory. Do not fill fixture Δ, Γ, Θ.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Pricing model | **NOT SPECIFIED IN SOURCE** | — |
| `derived/greeks` | Matrix BoundedSnapshot; extract stays a hole until a named model exists | identity |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Model name, day-count, rate source, IV convention, rupee scaling.

---

> **OUR INTERPRETATION**
>
> - Lit NFO contract rows do **not** light greeks. `extract_greeks(Some("kotak-nse-nfo"), …)` with lit chain + lit contracts is still `Unavailable` + `pricing_model_unspecified`, `data: None`. Binance options book is a **different** module (`mark_not_this_slice`).
> - Provenance `model` on contracts stays `"raw"` (CSV), not Black-76.
> - S5 Slice 1 (2026-08-31): NCL names European + cash; that does **not** delete this hole. Strike scale and expiry conversion remain unspecified.

---

### Cash book still refuses FO

**Source:** Lock “Cash book `kotak-nse-bse-cash` stays SHIPPING and still refuses FO”; B6 M1; `kotak_scrip_master.rs` cash-only parse.

**Verbatim definition / formula:**

```
Do not allowlist this URL on book kotak-nse-bse-cash.
Do not copy kotak-nse-bse-cash sessions, lots, or CNC+MIS onto this book.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Cash book | `kotak-nse-bse-cash` | still refuses `nse_fo` / `*_fo.csv` |
| NFO book | `kotak-nse-nfo` | this extract |

**Gaps (NOT SPECIFIED IN SOURCE):**

- None for the cash refuse.

---

> **OUR INTERPRETATION**
>
> - `extract_contracts(Some("kotak-nse-bse-cash"))` is `Unavailable`, `data: None` (cash is not NFO contracts). Do not parse FO CSV in the cash parser this slice.

---

### `extract_contracts` identity is `reference/derivative_contracts/bounded_snapshot`

**Source:** `agent/src/data/matrix.rs` `known_physics`; `agent/src/data/contracts.rs`

**Verbatim definition / formula:**

```
Family::Reference, capability derivative_contracts, Physics::BoundedSnapshot
```

Not `market/option_chain`. Not `market/order_book`. Not Market family.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `reference` | identity |
| Capability | `derivative_contracts` | identity |
| Physics | `bounded_snapshot` | identity |
| Lit | `InputHonesty::Lit` when `data.is_some()` | honesty.rs — **no** `HonestyStatus::Success` |
| Empty NFO store | `Unavailable` | not Empty-as-success |
| `canonical` / `persist_canonical` | false | envelope |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Strike grid, scaled strike, converted expiry (blocked above). AppState / CSV store wiring is out of this extract slice.

---

> **OUR INTERPRETATION**
>
> - Hole / unknown book / `None` / cash / spot / options → `status: Unavailable`, `data: None`.
> - `kotak-nse-nfo` with no rows → `Unavailable` (empty store is a hole until the parent wires a store).
> - Non-empty injected rows → `data: Some({ identity, contract_count, rows })` with `lot` as a JSON **number**. Honesty lit = `data.is_some()`. Do not add a fifth honesty variant.
> - Provenance `adapter_id` = `kotak_neo` when lit; `model` stays `"raw"`.

---

### File-paths sample lists more FO files; this book takes `nse_fo` only

**Source:** SDK `Scrip_Master.md` sample `filesPaths` (fetched 2026-08-28); lock “Listing ≠ this book’s allowlist”.

**Verbatim definition / formula:**

SDK sample (abbreviated):

```json
"filesPaths": [
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/cde_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/mcx_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_fo.csv"
]
```

Live FO filename this lock: `transformed/nse_fo.csv` (not `-v1`). Dated `2025-01-22` and `transformed-v1/nse_fo-v1.csv` returned **403** on 2026-08-28.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Live FO file | `…/2026-08-28/transformed/nse_fo.csv` | URL |
| `bse_fo` / `cde_fo` / `mcx_fo` | Listed in SDK sample | **refused** on this book |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether production `filesPaths` always includes every sample FO file.

---

> **OUR INTERPRETATION**
>
> - Listing a URL is not permission to fetch BFO/CDS/MCX. This book is `nse_fo` only.
> - Cash book still must not allowlist `_fo.csv`.

---

### Third-party OpenAlgo NFO mapping is not official scale or expiry conversion

**Source:** OpenAlgo `process_kotak_nfo_csv` (fetched 2026-08-27). Official lock: scale and offset **NOT SPECIFIED**.

**Verbatim definition / formula:**

```
tokensymbols["strike"] = df["dStrikePrice"] / 100
df["lExpiryDate"] = df["lExpiryDate"] + 315513000
tokensymbols["lotsize"] = df["lLotSize"]
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| OpenAlgo ÷100 | Third-party | **not** this lock |
| OpenAlgo `+315513000` | Third-party | **not** this lock |
| Official meaning | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether OpenAlgo’s offset/scale is correct on 2026-08-28 lapi files.

---

> **OUR INTERPRETATION**
>
> - Station stores raw strike and raw expiry. Do not copy OpenAlgo’s arithmetic.
> - OpenAlgo’s `lLotSize` mapping agrees with the lock **when both lot columns match**; disagreement is still unspecified → skip row.

---

### `market/option_chain` is not this extract; ghost strikes are forbidden

**Source:** `matrix.rs`; ADR 0002; lock “Strike grid stays dark”.

**Verbatim definition / formula:**

```
(Family::Market, "option_chain") => BoundedSnapshot
(Family::Reference, "derivative_contracts") => BoundedSnapshot
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| This extract | `reference/derivative_contracts` | bounded snapshot of master rows |
| Chain | `market/option_chain` | different identity; not this slice |
| Ghost 57500 | not in the lock sample | forbidden |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Strike scale, so a query “BANKNIFTY 57500 PE …” cannot be matched without guessing.

---

> **OUR INTERPRETATION**
>
> - Do not implement optionchain, glance/extract_chain, or a strike grid here.
> - Empty/unavailable is correct for chain while scale is blocked. A fixture 57500 ladder is cheating.

---

## Verification Checklist

Facts that still need confirming against the live source or a test environment before
code is written against them.

- [x] Live `nse_fo.csv` header row (79 columns, `pToken` absent) cited from unsigned GET 2026-08-28 IST (lock)
- [x] Token column = `pSymbol` (sample 56526); not cash `pSymbolName`-as-token
- [x] Segment = `pExchSeg` = `nse_fo` only
- [x] Lot columns `lLotSize` and `iLotSize` (sample both 65)
- [x] Tick header `dTickSize ` (trailing space)
- [x] Strike column name `dStrikePrice;` (semicolon); sample cell stored raw
- [x] Expiry columns `lExpiryDate ` / `pExpiryDate` sample `1474554600` stored raw
- [x] Live filename `transformed/nse_fo.csv` (not `-v1`; `-v1` was 403)
- [x] Cash book `kotak-nse-bse-cash` still refuses FO
- [ ] Official strike **scale** (do not ÷100; do not invent 57500)
- [ ] Expiry integer → calendar / epoch offset
- [ ] Which lot column wins if `lLotSize` ≠ `iLotSize` (skip row until specified)
- [ ] CNC validity on `nse_fo`
- [ ] Named greeks / pricing model (no Black-76)
- [ ] AppState / CSV store wiring for production `extract_contracts(Some("kotak-nse-nfo"))` (parent slice)
- [ ] FO REST quotes JSON LTP key (cash observed `ltp`; FO body not re-hit this lock)

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill
a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| ÷100 strike because OpenAlgo / “looks scaled” (`2.1e+06` vs `21000`) | Lock: official scale **NOT SPECIFIED IN SOURCE**. Store raw. | `strike_raw` = cell string. No ÷100. No 21000. No **57500**. |
| Convert `1474554600` with `+315513000` or Unix UTC | Lock: calendar / epoch offset **NOT SPECIFIED**. | `expiry_raw` = integer string. No conversion. |
| Pick `lLotSize` over `iLotSize` (or the reverse) when they differ | Lock: which wins is **NOT SPECIFIED**. Sample they match at 65. | Skip the row. Do not guess. Fixture lot **65** is this dated contract only. |
| Add `HonestyStatus::Success` for a lit master | `honesty.rs`: four variants; lit is `InputHonesty::Lit` when `data.is_some()`. | No fifth variant. Empty store → `Unavailable`, not Empty-as-success. |
| Copy cash `pSymbolName` as FO token | Cash REST.md: ticker = `pSymbolName`. FO lock: token = `pSymbol` (`56526`); `pSymbolName` is `NIFTY`. | Do not copy cash token mapping. |
| Allowlist FO CSV on `kotak-nse-bse-cash` | Lock: cash still refuses FO. Do not allowlist this URL on cash. | Cash extract path stays `Unavailable`. |
| Invent tick 0.05 / lot 50 / BANKNIFTY 15 | Lock: lot/tick from the row; do not invent 25/50/75. | Lot from fixture row 65 only. No `fo_mktlots.csv`. |
| Implement Black-76 because contracts are lit | OPTIONS-PRICING.md + lock: no named model. | Greeks stay `pricing_model_unspecified`. |
| Enable CNC on `nse_fo` because “F&O is NRML” | Lock: CNC on `nse_fo` **NOT SPECIFIED**. Do not enable; do not invent refuse. | Leave unimplemented. |
| Paint a strike grid / optionchain from the master | Lock: strike grid stays dark until scale + expiry conversion are specified. | No grid. No 57500. No glance/extract_chain this slice. |
| Use SDK `2025-01-22` or `nse_fo-v1.csv` as live | This fetch: those URLs **403**. Live file is dated `2026-08-28` `transformed/nse_fo.csv`. | Cite the lock URL only. |

No memory fills for strike scale, expiry conversion, lot-column winner, CNC, or greeks. Column **names** are cited from the lock’s 2026-08-28 header. Gaps stay `NOT SPECIFIED IN SOURCE`.

# Kotak Neo NFO / F&O scrip master — v1 refused

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo NSE F&O (NFO) scrip-master CSV — expiry list, option-symbol resolve, lot size as a contract field |
| **Primary source** | [Kotak Neo Trade API guide](https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/) · [Kotak-Neo/Kotak-neo-api-v2 `docs/Scrip_Master.md`](https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md) · Station [REST.md](./REST.md) · B6 [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) |
| **Snapshot date** | 2026-08-27 |
| **Source version** | Trade API guide page “Updated: 22 May 2026, 5:16 PM IST” · SDK package v2.0.0 / git `main` @ `8cee5bda63bd9334f8501bb23b7f1d2945f93397` · B6 SIGNED (cash only v1) |
| **Staleness warning** | Re-verify against live `Scrip_Master.md`, the Trade API guide, and a cited live FO header row **before** any extract is written. This doc does **not** lift the v1 F&O refuse. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> This slice is research only. Station v1 / B6 **refuses** F&O (`nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo`) even when `filesPaths` lists them. The following facts are needed for a **later-phase** NFO extract and are **NOT SPECIFIED IN SOURCE** (official Kotak SDK + Trade API guide):
>
> - Official NFO / `nse_fo.csv` column names, types, and units (the SDK return type is `object`; the sample is URL paths only).
> - Which FO column is the instrument token (`pSymbol` vs `pToken` vs other). Cash live files use numeric `pSymbol` and have **no** `pToken` — that observation is cash-only ([REST.md](./REST.md)).
> - Which FO column is lot size (`lLotSize` vs `iLotSize` vs other) and its unit (shares vs lots vs scaled integer).
> - Which FO column is strike and its scale (raw vs ÷100). Header punctuation such as `dStrikePrice;` is unspecified officially.
> - Which FO column is expiry (`pExpiryDate` vs `lExpiryDate` vs `pScripRefKey`) and whether the value is a calendar date, an epoch, or an epoch that needs an offset.
> - BANKNIFTY (or any index/stock) lot size as a number.
> - Weekly (or monthly) expiry weekday for BANKNIFTY / NIFTY / FINNIFTY / SENSEX.
> - Whether live `filesPaths` always includes `nse_fo.csv` / `nse_fo-v1.csv` (founder 2026-08-27 file-paths log extracted **two cash URLs**; FO presence in that payload is unspecified).
> - How to map the query “BANKNIFTY 57500 PE 29 Sep” onto a single tradable id from an official schema.
>
> Do **not** implement NFO extracts, do **not** allowlist `nse_fo`, and do **not** download `nse_fo.csv` into the repo until these are resolved from a primary source **and** B6 row 12 is explicitly re-signed. A Pre-trade strike grid that invents 57500 (or any ghost strike) while the master is refused is forbidden.

---

## Source Inventory

List every file, page, or paper you read to produce this document.

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Trade API guide | https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/ | 2026-08-27 | “downloadable CSV files containing all of the tradeable instruments”; path `{BASE_URL}/script-details/1.0/masterscrip/file-paths`. No CSV schema. |
| SDK `docs/Scrip_Master.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md | 2026-08-27 | Sample `filesPaths` includes `nse_fo.csv`, `bse_fo.csv`, `cde_fo.csv`, `mcx_fo.csv`. Return type `object`. No column list. |
| SDK `scrip_master_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/scrip_master_api.py | 2026-08-27 | GET file-paths; optional `exchange_segment` substring filter; **does not** GET CSV bytes. |
| SDK `settings.py` `exchange_segment` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/settings.py | 2026-08-27 | `"NFO": "nse_fo"`, `"nfo": "nse_fo"`. Allowed values include `nse_fo` / `bse_fo` / `cde_fo` / `mcx_fo`. |
| SDK README quotes | Kotak-neo-api-v2 README (quoted in [REST.md](./REST.md)) | 2026-08-26 | `exchange_segment` for quotes includes `nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo`. |
| Station REST scrip master | [`REST.md`](./REST.md) | 2026-08-27 | Cash CSV columns observed on `nse_cm-v1.csv` / `bse_cm-v1.csv`. F&O URLs refused. |
| B6 sheet | [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) | 2026-08-27 | Row 1 cash only; row 12 refuse F&O; M1 refuse FO files even when listed. |
| `kotak_scrip_master.rs` header | `agent/src/kotak_scrip_master.rs` (read-only this slice) | 2026-08-27 | Cash: token = `pSymbol`, ticker = `pSymbolName`. “F&O CSVs stay refused.” |
| Station Data matrix | `agent/src/data/matrix.rs` `known_physics` (read-only this slice) | 2026-08-27 | `reference/expiry`, `reference/option_symbol` are BoundedSnapshot. No `lot_size` capability id. |
| Station operations catalog | `agent/src/data/operations.rs` (read-only this slice) | 2026-08-27 | OpenAlgo nouns `expiry` → `reference/expiry`; `optionsymbol` → `reference/option_symbol`. |
| `kotak_neo.s1k.v1` manifest | `agent/src/data/source_manifest.rs` (read-only this slice) | 2026-08-27 | Implemented: quotes, instruments, tradebook, depth. Coverage `nse_cm` / `bse_cm`. Comment: “no optionchain.” |
| Glance chain hole | `agent/src/data/glance.rs` (read-only this slice) | 2026-08-27 | `extract_chain` / `extract_open_interest` return `unavailable` with `data: null`. |
| OpenAlgo Kotak master (third-party) | https://github.com/marketcalls/openalgo/blob/main/broker/kotak/database/master_contract_db.py | 2026-08-27 | `process_kotak_nfo_csv` — **not** official Kotak schema. Maps FO files; Station v1 must not copy this into an extract. |
| GitHub issue (user report, not schema) | https://github.com/Kotak-Neo/Kotak-neo-api-v2/issues/67 | 2026-08-27 | User report: `pexpirydate` / `lexpirydate` vs `pscriprefkey` inconsistency on options. **Not** official docs. |
| GitHub issue (user report, not schema) | https://github.com/Kotak-Neo/kotak-neo-api/issues/104 | 2026-08-27 | v1-repo user paste of an `nse_fo.csv` header row. **Not** official docs. Not re-fetched as CSV bytes. |

This slice did **not** download `nse_fo.csv` / `nse_fo-v1.csv` bytes into the repo.

---

## Concepts

*Add one sub-section per concept, formula, or definition. Copy the block below as many times
as needed. Never merge two concepts into one block.*

---

### File-paths payload lists F&O CSV URLs

**Source:** SDK [`docs/Scrip_Master.md`](https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md) sample response (fetched 2026-08-27); Trade API guide “How to Fetch Live Market Data with Kotak Neo API” (updated 22 May 2026, fetched 2026-08-27)

**Verbatim definition / formula:**

```
The request retrieves downloadable CSV files containing all of the tradeable instruments.
```

SDK sample (abbreviated to the F&O paths; cash paths also present — see REST.md):

```json
"filesPaths": [
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/cde_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/mcx_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv",
    "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_fo.csv"
]
```

`scrip_master_api.py` (git `8cee5bda`): GET file-paths; if `exchange_segment` is passed, keep the first `filesPaths` entry whose URL contains that segment substring. It does **not** download the CSV.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `filesPaths` | List of CSV URLs in the SDK sample | string[] |
| `nse_fo.csv` | Filename in the SDK sample | filename |
| `bse_fo.csv` | Filename in the SDK sample | filename |
| `cde_fo.csv` | Filename in the SDK sample | filename |
| `mcx_fo.csv` | Filename in the SDK sample | filename |
| `baseFolder` | `https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod` (sample) | URL |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether production `filesPaths` always includes every sample FO file.
- Whether live FO uses `transformed/nse_fo.csv` (SDK sample) or `transformed-v1/nse_fo-v1.csv` (cash live pattern). Official sample is `transformed/nse_fo.csv` only.
- Official CSV header row for any FO file.

---

> **OUR INTERPRETATION**
>
> - Listing a URL is not permission to fetch it in Station v1. B6 row 12 / M1 refuse F&O files even when this sample lists them.
> - A later-phase extract would start from the same file-paths GET already documented in REST.md, then take the `nse_fo` (NFO) URL — **only after** B6 is re-signed and the FO schema blocker above is closed.
> - This slice does not fetch those bytes.

---

### Station v1 / B6 refuses F&O segments and files

**Source:** B6 [`docs/research/sheets/kotak_neo.md`](../../../research/sheets/kotak_neo.md) rows 1, 12, M1 (SIGNED); [REST.md](./REST.md) scrip-master interpretation; `kotak_scrip_master.rs` header comment “F&O CSVs stay refused.”

**Verbatim definition / formula:**

B6 row 1:

```
Asset class(es) when connected: equities cash v1 — segments nse_cm / bse_cm only. API also supports F&O — refuse F&O in v1
```

B6 row 12:

```
Refuse list v1: F&O segments (nse_fo, …); …
```

B6 M1:

```
Cash v1: use nse_cm / bse_cm only. Refuse F&O files (nse_fo, bse_fo, cde_fo, mcx_fo, …) even when listed in the sample filesPaths.
```

`kotak_scrip_master.rs` header (2026-08-27):

```
Live cash CSV token is pSymbol, ticker pSymbolName (no pToken). F&O CSVs stay refused.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `nse_fo` | Refused v1 segment (B6 row 12; also an SDK `exchange_segment` value) | segment id |
| `bse_fo` | Refused v1 segment | segment id |
| `cde_fo` | Refused v1 segment | segment id |
| `mcx_fo` | Refused v1 segment | segment id |
| `nse_cm` / `bse_cm` | Cash v1 allowlist | segment id |

**Gaps (NOT SPECIFIED IN SOURCE):**

- No later-phase date in B6 for lifting the F&O refuse.
- Whether a future lift is NFO-only (`nse_fo`) or also BFO / CDS / MCX.

---

> **OUR INTERPRETATION**
>
> - This document **does not** lift the refuse. Research only.
> - Fills stay cash-only (`GET {baseUrl}/quick/user/trades`). Quotes/depth/scrip master in `kotak_neo.s1k.v1` stay `nse_cm` / `bse_cm`.
> - Do not allowlist `nse_fo` / `nse_fo-v1.csv` / `*_fo.csv` in host policy. Do not extend `kotak_scrip_master.rs` to parse FO rows.

---

### Segment alias: NFO → `nse_fo`

**Source:** SDK `settings.py` `exchange_segment` map (git `8cee5bda`, fetched 2026-08-27)

**Verbatim definition / formula:**

```
exchange_segment = {
    ...
    "NFO": "nse_fo", "nse_fo": "nse_fo", "nfo": "nse_fo",
    "BFO": "bse_fo", "bse_fo": "bse_fo", "bfo": "bse_fo",
    "CDS": "cde_fo", "cde_fo": "cde_fo", "cds": "cde_fo",
    ...
    "MCX": "mcx_fo", "mcx": "mcx_fo", "mcx_fo": "mcx_fo"
}
```

`exchange_segment_allowed_values` includes both `NFO` / `nfo` and `nse_fo`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `NFO` | Maps to `nse_fo` | alias |
| `nse_fo` | Canonical Kotak segment id for NSE F&O | segment id |
| `BFO` | Maps to `bse_fo` | alias |
| `CDS` | Maps to `cde_fo` | alias |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official prose definition of “NFO” vs “NSE-FO” vs `nse_fo` outside this map.
- Whether OpenAlgo’s stored exchange string `"NFO"` (third-party) is the Station TickBook identity. Station cash identity is `{segment}|{token}` with Kotak segment ids (`nse_cm|…`). FO identity shape is unspecified until an extract exists.

---

> **OUR INTERPRETATION**
>
> - In Kotak wire terms, NFO **is** `nse_fo`. A future extract’s tradable id should use the Kotak segment id (`nse_fo|…`), not a Zerodha-style `NFO:` prefix, unless a primary source says otherwise.
> - v1 still refuses that segment.

---

### Official F&O CSV schema is unspecified

**Source:** SDK `Scrip_Master.md` “Return type: **object**”; sample JSON is `filesPaths` + `baseFolder` only. Trade API guide names the file-paths endpoint, not columns.

**Verbatim definition / formula:**

```
### Return type

**object**
```

No official table of FO (or cash) CSV headers appears in `Scrip_Master.md`. Cash headers in REST.md come from a **live unsigned GET** of `nse_cm-v1.csv` / `bse_cm-v1.csv` (2026-08-27), not from the SDK doc. This slice does **not** repeat that ritual for `nse_fo.csv`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| CSV bytes at `filesPaths` | Downloadable instrument catalog (guide) | CSV |
| FO header row | **NOT SPECIFIED IN SOURCE** (official) | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Every FO column name, including token, ticker, expiry, strike, option type, lot size, instrument type.
- Whether FO files share the 80 cash headers observed 2026-08-27.
- Whether FO `pSymbol` is numeric token (as cash live) or a text trading symbol (as the cash **fixture**).
- Whether FO files include `pToken`.
- Strike scale, expiry representation, lot-size unit.

---

> **OUR INTERPRETATION**
>
> - A future extract must parse columns from a **cited live FO header row**, the same way cash used the 2026-08-27 GET — after B6 lifts the refuse. Until then, leave the code path unimplemented.
> - Third-party parsers (OpenAlgo, GitHub issues) are not a substitute for that citation. See the OpenAlgo concept below.
> - Do not copy cash fixture `pToken` + text-`pSymbol` onto FO files from memory.

---

### Cash token/ticker columns do not license FO columns

**Source:** [REST.md](./REST.md) “Live cash CSV columns”; `kotak_scrip_master.rs` header

**Verbatim definition / formula:**

REST.md (2026-08-27):

```
Live cash CSV columns | Station unsigned GET 2026-08-27 of nse_cm-v1.csv / bse_cm-v1.csv
(80 headers, no pToken). Token = pSymbol (numeric). Ticker = pSymbolName.
Name = pDesc. Segment = pExchSeg.
Official SDK does not document this schema.
```

`kotak_scrip_master.rs` header:

```
Live cash CSV token is pSymbol, ticker pSymbolName (no pToken). F&O CSVs stay refused.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Cash `pSymbol` | Numeric instrument token (live lapi 2026-08-27) | integer (observed cash) |
| Cash `pSymbolName` | Ticker (live lapi 2026-08-27) | string (observed cash) |
| Cash `pToken` | Absent on live cash files; present on **test fixture** only | — |
| FO `pSymbol` / `pToken` | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- FO token column.
- FO ticker / trading-symbol column (`pTrdSymbol` vs `pSymbolName` vs `pScripRefKey`).
- FO `pInstType` values as an official enum (Station cash parser refuses rows whose `pInstType` is `OPTIDX` / `FUTIDX` / … — that list is a v1 cash filter, not an official FO schema).

---

> **OUR INTERPRETATION**
>
> - Cash observation stays cash. Do not assume FO files use the same token/ticker split.
> - The cash parser’s refused `pInstType` list (`FUTIDX`, `OPTIDX`, …) is a **v1 skip rule** for FO rows that might appear in a cash file, not documentation of the FO CSV.

---

### Identity `reference/expiry` (BoundedSnapshot)

**Source:** `agent/src/data/matrix.rs` `known_physics`; `agent/src/data/operations.rs` (`expiry` → `reference` / `expiry` / `BoundedSnapshot`); ADR 0002 (OpenAlgo Data nouns as the read vocabulary)

**Verbatim definition / formula:**

```
(Family::Reference, "expiry") => Some(&[Physics::BoundedSnapshot])
```

Operation catalog: OpenAlgo noun `expiry` binds to capability id `expiry`, family `reference`, physics `BoundedSnapshot`.

`kotak_neo.s1k.v1` implemented ops: `quotes`, `instruments`, `tradebook`, `depth`. **`expiry` is not claimed.** Coverage venues: `nse_cm`, `bse_cm`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `reference` | identity family |
| Capability id | `expiry` | snake_case id |
| Physics | `BoundedSnapshot` | matrix physics |
| Kotak v1 extract | **does not exist** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official Kotak field to distinct-list expiries for an underlying (see FO schema blocker).
- Sort order, timezone, and whether the list is trading-day dates vs expiry-session timestamps.
- Weekly vs monthly vs quarterly as Kotak-documented partitions.

---

> **OUR INTERPRETATION**
>
> - `reference/expiry` is already on the product matrix. It is **not** a later invented capability. What is missing is a Kotak **extract**.
> - Until an NFO master extract exists, the honest envelope is **empty / unavailable**, not a hardcoded Thursday list and not a fixture.
> - A future extract must return the **complete** expiry list present on the cited master for that underlying, or empty/unavailable. Partial “we know the near week” lists are a lie.

---

### Identity `reference/option_symbol` (BoundedSnapshot)

**Source:** `agent/src/data/matrix.rs` `known_physics`; `agent/src/data/operations.rs` (`optionsymbol` → `reference` / `option_symbol` / `BoundedSnapshot`)

**Verbatim definition / formula:**

```
(Family::Reference, "option_symbol") => Some(&[Physics::BoundedSnapshot])
```

Operation catalog: OpenAlgo noun `optionsymbol` binds to capability id `option_symbol`.

`kotak_neo.s1k.v1` does **not** claim `optionsymbol`. Glance `extract_chain` is a **different** identity (`market/option_chain`) and already returns `unavailable`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Family | `reference` | identity family |
| Capability id | `option_symbol` | snake_case id |
| Physics | `BoundedSnapshot` | matrix physics |
| Resolve query (product) | Underlying + strike + CE/PE + expiry → tradable id | — |
| Kotak v1 extract | **does not exist** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official Kotak composition of a tradable option id from those four parts.
- Strike matching (57500 vs 5750000 vs `dStrikePrice` scale).
- Put/call column (`pOptionType` `PE`/`CE` vs other).
- Date matching (“29 Sep” vs `29-SEP-26` vs epoch).

---

> **OUR INTERPRETATION**
>
> - Worked example the extract must handle **without guessing a row**: BANKNIFTY 57500 PE 29 Sep → the tradable Kotak id from the master **or** empty/unavailable.
> - Empty is correct when the master is refused, the row is absent, or the schema is still unspecified. A fixture token for 57500 is cheating.
> - `reference/option_symbol` is a **single-contract resolve**. It is not `market/option_chain` (a bounded cross-section of many strikes). Do not implement chain by synthesizing strikes around 57500.

---

### Lot size is a field on the contract, not a capability id

**Source:** B6 row 8; `agent/src/data/matrix.rs` `known_physics` (no `lot_size` id); cash `KotakInstrument` in `kotak_scrip_master.rs` (token, ticker, name, segment — **no lot field**)

**Verbatim definition / formula:**

B6 row 8:

```
Calculation factors: qty (shares) × price INR; product CNC (delivery) vs MIS (intraday square-off);
session calendar NSE/BSE; CalcProfile equities_inr_cash; lot/scrip multipliers from scrip master when needed
```

`known_physics` lists `expiry` and `option_symbol` under `Family::Reference`. It does **not** list `lot_size` / `lotsize` as a capability id.

Cash `KotakInstrument` fields: `instrument_token`, `trading_symbol`, `name`, `segment`. No lot size.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Lot / scrip multiplier | “from scrip master when needed” (B6 row 8) | field on a master row |
| Capability id `lot_size` | **Not on the matrix** | — |
| Official FO lot column | **NOT SPECIFIED IN SOURCE** | — |
| BANKNIFTY lot quantity | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official column name and unit for FO lot size.
- Any numeric lot for BANKNIFTY, NIFTY, or stock options.
- Whether lot size can differ by expiry of the same underlying.

---

> **OUR INTERPRETATION**
>
> - Lot size is **data on the contract row**, obtained from the cited master (or unavailable). It is not `obtain(lot_size)` and not a hardcoded 15 / 25 / 30.
> - Cash v1 does not even store lot size on `KotakInstrument`. An NFO extract that needs lot must add a sourced field — after the FO schema is cited — not a constant.
> - Do not ship Pre-trade quantity math that assumes BANKNIFTY lot = 30 (or any other memory value).

---

### Extracts do not exist today — honest envelopes

**Source:** `kotak_neo.s1k.v1` manifest; `glance.rs` chain/OI hole; `kotak_scrip_master.rs` cash-only parse; ADR 0002 “typed unsupported/unavailable”

**Verbatim definition / formula:**

`source_manifest.rs`:

```
/// Kotak S1k: quotes (REST latest_state) + REST depth bounded_snapshot + scrip master
/// + session tradebook. No history, no HSM `isDepth`, no optionchain.
```

Implemented: `quotes`, `instruments`, `tradebook`, `depth`. Coverage: `nse_cm`, `bse_cm`.

`glance.rs`:

```
//! S3 holes: options chain / OI widgets stay unavailable until those extracts exist.
```

`extract_chain` / `extract_open_interest`: `status: Unavailable`, `data: None`.

ADR 0002 selection step 5: `typed unsupported/unavailable`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `reference/expiry` extract | **does not exist** | — |
| `reference/option_symbol` extract | **does not exist** | — |
| NFO `instrument_master` extract | **does not exist** (cash master only) | — |
| `market/option_chain` extract | exists as an **unavailable hole** (`glance.rs`) | envelope |
| Lot size on cash `KotakInstrument` | **not stored** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Wire JSON for a future expiry / option_symbol envelope (follow existing extract envelopes when that slice is signed; do not invent a new success shape here).

---

> **OUR INTERPRETATION**
>
> Honest envelopes for a future NFO extract (and for today’s UI, which must not pretend the extract exists):
>
> | Ask | Honest result while master is refused / unimplemented |
> | --- | --- |
> | Expiry list for BANKNIFTY | complete list from master **or** empty / unavailable — never a guessed Thursday calendar |
> | BANKNIFTY 57500 PE 29 Sep | tradable id from master **or** empty / unavailable — never a fixture token |
> | Lot size for that contract | value from the master row **or** unavailable — never a hardcoded 30 |
>
> Dark state is **no contracts**, not a demo chain. Empty is truth.

---

### `market/option_chain` is not `reference/option_symbol`; ghost strikes are forbidden

**Source:** `matrix.rs` (`option_chain` = market BoundedSnapshot; `option_symbol` = reference BoundedSnapshot); ADR 0002 “Option chain is not order-book depth”; `glance.rs` unavailable chain

**Verbatim definition / formula:**

```
(Family::Market, "option_chain") => Some(&[Physics::BoundedSnapshot])
(Family::Reference, "option_symbol") => Some(&[Physics::BoundedSnapshot])
```

ADR 0002:

```
Option chain is not order-book depth. REST option chain is a bounded market
cross-section; ordered depth requires snapshot-plus-contiguous-deltas.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `market/option_chain` | BoundedSnapshot cross-section of contracts | market identity |
| `reference/option_symbol` | Single-contract resolve | reference identity |
| Ghost strike | Strike shown without a master row | forbidden (interpretation) |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official Kotak “option chain” REST (Quotes.md `quote_type` has `oi` / `scrip_details`; it is **not** documented as a chain table). Do not invent `/optionchain`.

---

> **OUR INTERPRETATION**
>
> - A chain table is a projection **over master rows** (plus quotes/OI if those extracts exist). Strikes that are not on the master are ghosts.
> - While NFO master is v1-refused, a chain UI that paints 57500 (or any ATM ladder) is cheating. Show empty / unavailable.
> - Do not “fill in” weekly strikes from a remembered NSE grid. Do not use a fixture list of 57500 as live state.

---

### Third-party OpenAlgo NFO mapping is not official Kotak schema

**Source:** OpenAlgo `broker/kotak/database/master_contract_db.py` `process_kotak_nfo_csv` (fetched 2026-08-27). REST.md already cites this file for **cash** columns. Official `Scrip_Master.md` still has no FO schema.

**Verbatim definition / formula:**

OpenAlgo `process_kotak_nfo_csv` (third-party; not Kotak docs):

```
tokensymbols["token"] = df["pSymbol"]
tokensymbols["name"] = df["pSymbolName"]
df["lExpiryDate"] = df["lExpiryDate"] + 315513000
tokensymbols["expiry"] = pd.to_datetime(df["lExpiryDate"], unit="s")
tokensymbols["expiry"] = tokensymbols["expiry"].dt.strftime("%d-%b-%y").str.upper()
tokensymbols["strike"] = df["dStrikePrice"] / 100
tokensymbols["lotsize"] = df["lLotSize"]
tokensymbols["brsymbol"] = df["pTrdSymbol"]
tokensymbols["brexchange"] = df["pExchSeg"]
tokensymbols["exchange"] = "NFO"
tokensymbols["instrumenttype"] = df["pOptionType"].str.replace("XX", "FUT")
```

OpenAlgo FO fallback URL (same file `fallback_urls`): `…/transformed/nse_fo.csv` (not `-v1`). Cash fallback in the same dict uses `transformed-v1/{nse,bse}_cm-v1.csv`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| OpenAlgo `pSymbol` (NFO) | Mapped to `token` in OpenAlgo | third-party mapping |
| OpenAlgo `lExpiryDate` | Unix seconds **plus** `315513000` then formatted | third-party offset |
| OpenAlgo `dStrikePrice` | Divided by 100 | third-party scale |
| OpenAlgo `lLotSize` | Mapped to `lotsize` | third-party mapping |
| Official meaning of those columns | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether OpenAlgo’s +315513000 offset is correct, still required, or harmful on current lapi files (user issues #67 / #104 report expiry-field bugs; they are not official errata).
- Whether `iLotSize` (seen in a **user** header paste, issue #104) is the field to use instead of `lLotSize`.
- Whether live FO files use `dStrikePrice` or `dStrikePrice;` (OpenAlgo cash NSE path reads `dStrikePrice;`; NFO path strips `;` from headers first).

---

> **OUR INTERPRETATION**
>
> - Cite OpenAlgo here the same way REST.md cites it for cash: a **working third-party parser**, not a primary schema.
> - A future Station extract must not copy the 10-year epoch offset, the ÷100 strike, or `lLotSize` until a **live FO header + sample row** is cited in this library (and B6 lifts the refuse).
> - Do not treat GitHub issues as column authority. They are verification warnings: expiry fields have been reported inconsistent.

---

## Verification Checklist

Facts that still need confirming against the live source or a test environment before
code is written against them.

- [ ] B6 row 12 / M1 still refuse F&O — this research pass must **not** flip them (confirm on the sheet)
- [ ] Official Kotak doc or a cited live `nse_fo` / `nse_fo-v1.csv` **header row** names token, ticker, expiry, strike, option type, lot size
- [ ] FO token column on live lapi matches or differs from cash `pSymbol` (do not assume; do not commit CSV bytes to git)
- [ ] FO lot-size column and unit; BANKNIFTY lot is **that field’s value**, not a constant
- [ ] Expiry column and representation (date vs epoch vs offset); weekday of weekly expiry remains unspecified until the master says so
- [ ] Strike column and scale so “57500” can match a row without guessing
- [ ] Live `filesPaths` includes `nse_fo` and which filename (`nse_fo.csv` vs `nse_fo-v1.csv`)
- [ ] `reference/expiry` extract returns complete list or empty/unavailable — never a fixture week
- [ ] `reference/option_symbol` for BANKNIFTY 57500 PE 29 Sep returns tradable id or empty — never a hardcoded token
- [ ] Chain / Pre-trade strike grid is empty while NFO master is refused (no ghost 57500 ladder)
- [ ] v1 host allowlist still drops `nse_fo` / `bse_fo` / `cde_fo` / `mcx_fo` / `*-fo.csv`

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill
a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| BANKNIFTY lot = 30 (or 15 / 25) | Official Kotak docs do not state a lot quantity. B6 says lot comes from scrip master when needed. OpenAlgo maps `lLotSize` but does not publish BANKNIFTY’s value. | **NOT SPECIFIED IN SOURCE.** Lot is a field on the row or unavailable. |
| Weekly expiry is Thursday | No Kotak or NSE rulebook is cited in this pass. SDK/guide do not mention weekday. | **NOT SPECIFIED IN SOURCE.** Do not hardcode Thursday. |
| FO files use cash live columns (`pSymbol` token, `pSymbolName` ticker, no `pToken`) | REST.md observation is `nse_cm-v1.csv` / `bse_cm-v1.csv` only. `kotak_scrip_master.rs`: F&O CSVs stay refused. | Do not extend cash columns to FO. Token column on FO is **NOT SPECIFIED**. |
| FO files use cash **fixture** `pToken` + text `pSymbol` | Fixture is for cash tests. Live cash has no `pToken`. | Self-audit only. FO `pToken` vs `pSymbol` remains unspecified. |
| Copy OpenAlgo `process_kotak_nfo_csv` (`lExpiryDate+315513000`, `dStrikePrice/100`, `lLotSize`) | OpenAlgo is third-party code. Official `Scrip_Master.md` has no columns. | Recorded as third-party mapping. Not implementable schema. |
| Treat GitHub issue #104 header dump as official schema | User paste on v1-repo issue; includes both `lLotSize` and `iLotSize`. Not `Scrip_Master.md`. | Cited as user report only. Do not implement from it. |
| Treat issue #67 (`pexpirydate` off by 10 years) as the official expiry rule | User report on v2 repo, open as of fetch. | Gap / verification item. Not a formula to code. |
| Invent NSE weekly strike interval (100 / 50) to build a 57500 ladder | No primary source in this inventory. | Forbidden ghost strikes. Empty chain while refused. |
| `reference/lot_size` as a capability | Matrix has `expiry` and `option_symbol`, not `lot_size`. B6 row 8: multiplier from master. | Lot size is a **field**, not a capability id. |
| Lift v1 refuse because `filesPaths` lists `nse_fo.csv` | B6 M1: refuse even when listed. REST.md: F&O stay refused. | Refuse unchanged. This doc does not allowlist `nse_fo`. |
| Assume live FO filename is `nse_fo-v1.csv` because cash is `nse_cm-v1.csv` | SDK sample: `transformed/nse_fo.csv`. OpenAlgo FO fallback: same, not `-v1`. Cash live: `transformed-v1/nse_cm-v1.csv`. | FO live filename **NOT SPECIFIED**. Refuse patterns in REST.md already cover both. |
| Guess “29 Sep” parses as 29-Sep-2026 | Year and calendar convention unspecified. | Option-symbol resolve stays unimplemented; empty/unavailable. |
| Use glance `extract_chain` success path for Pre-trade | `glance.rs` returns `Unavailable` / `data: None`. | Keep dark state. Do not add a fixture chain in this slice. |
| Download `nse_fo.csv` to “just see headers” | Task forbids fetching FO CSV bytes into the repo. Official schema still wouldn’t be the SDK doc. | Not fetched. Header remains a blocker. |

No memory fills for lot size, expiry weekday, or FO columns. Official gaps stay `NOT SPECIFIED IN SOURCE`. v1 refuse is unchanged. No Rust/Swift was edited in this slice.

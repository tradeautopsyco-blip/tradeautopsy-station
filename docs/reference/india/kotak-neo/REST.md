# Kotak Neo Trade API — REST quotes and scrip master

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo REST scrip-master file-paths + REST quotes (`quote_type`) |
| **Primary source** | [Kotak Neo Trade API guide](https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/) · [historical data FAQ](https://www.kotakneo.com/support/how-do-i-get-historical-data/) · [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) |
| **Snapshot date** | 2026-08-27 |
| **Source version** | Trade API guide page “Updated: 22 May 2026, 5:16 PM IST” · SDK package v2.0.0 / git `main` @ `8cee5bda63bd9334f8501bb23b7f1d2945f93397` |
| **Staleness warning** | Re-verify against the live FAQ, Trade API guide, and SDK `settings.py` / `quotes_neo_symbol_api.py` / `scrip_master_api.py` before changing hosts, methods, or paths. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> The following facts are needed for Slice A host allowlist / live fetch but are **NOT SPECIFIED IN SOURCE**:
>
> - Numeric rate-limit window for quotes or file-paths (HTTP 429 exists; counts and shared envelope with `/quick/user/trades` are unspecified).
> - Whether REST quotes, when called without TOTP/MPIN, use a fixed host or the session `baseUrl` (`get_url_details` always prefixes `self.base_url`).
> - Whether a live cash CSV GET on `lapi.kotaksecurities.com` requires `Sid` / `Auth` if a future GET returns 401/403 (Station unsigned GET 2026-08-27 returned 200).
> - Whether production gateways require `Sid` / `Auth` / `sId` on REST quotes despite the SDK sending only `Authorization` = consumer key.
>
> Do not invent klines, OHLCV history, or `/quick/quotes`. **S2:** Kotak `obtain(history)` remains unsupported (FAQ 2026-08-26). Binance.com klines live on slug `binance_com` — [`docs/reference/crypto/binance-global/spot/REST.md`](../../crypto/binance-global/spot/REST.md).

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Historical market data FAQ | https://www.kotakneo.com/support/how-do-i-get-historical-data/ | 2026-08-26 | “Historical data is unavailable at the moment.” |
| Trade API guide | https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/ | 2026-08-26 | Live market data + `{BASE_URL}/script-details/1.0/masterscrip/file-paths` |
| SDK `docs/Quotes.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Quotes.md | 2026-08-26 | `quote_type` enum; HTTP status table |
| SDK `docs/Scrip_Master.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Scrip_Master.md | 2026-08-26 | Sample `filesPaths` including `nse_cm.csv` / `bse_cm.csv` |
| OpenAlgo Kotak master contract | https://github.com/marketcalls/openalgo/blob/main/broker/kotak/database/master_contract_db.py | 2026-08-27 | Unsigned lapi GET; `fallback_urls` live cash `transformed-v1/{nse,bse}_cm-v1.csv` |
| OpenAlgo Kotak quotes | https://github.com/marketcalls/openalgo/blob/main/broker/kotak/api/data.py | 2026-08-27 | Same `/quotes/neosymbol` GET; `response[0]` is a list row with `ltp` / `exchange` / `exchange_token` |
| SDK README | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/README.md | 2026-08-26 | Quotes without completing login; `quote_type` list |
| `settings.py` `PROD_URL` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/settings.py | 2026-08-26 | Path constants |
| `quotes_neo_symbol_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/quotes_neo_symbol_api.py | 2026-08-26 | **GET** + headers |
| `scrip_master_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/scrip_master_api.py | 2026-08-26 | **GET** + headers |
| `neo_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/neo_api.py | 2026-08-26 | `quotes()` vs `scrip_master()` 2FA gates |
| `neo_utility.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/neo_utility.py | 2026-08-26 | `get_url_details` joins `base_url` + path |
| `urls.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/urls.py | 2026-08-26 | Login `BASE_URL`; listed trading hosts |
| B6 sheet | `docs/research/sheets/kotak_neo.md` | 2026-08-26 | Cash `nse_cm` / `bse_cm` only; Phase 1 fills unchanged |
| Session + trades | `docs/reference/equities/kotak-neo/SESSION-AND-TRADES.md` | 2026-07-26 | Mint + `GET …/quick/user/trades` |

First-party GitHub search on 2026-08-26 found **Kotak-Neo/Kotak-neo-api-v2** only (no separate `kotak-neo-api` v1 repo under that org).

---

## Concepts

---

### Historical market candles are unavailable

**Source:** [How do I get historical data in Neo Trade API?](https://www.kotakneo.com/support/how-do-i-get-historical-data/) (fetched 2026-08-26)

**Verbatim definition / formula:**

```
Historical data is unavailable at the moment.

This feature is not allowed for this platform.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Historical data | Unavailable / not allowed for this platform | FAQ prose |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether “historical data” means market candles only, or also multi-day trade books (B6 already treats trade book as session/day — see SESSION-AND-TRADES).
- Any future enablement date.

> **OUR INTERPRETATION**
>
> - Do **not** add a `history` / `historical_series` / klines capability for `kotak_neo`.
> - REST `quote_type=ohlc` is a quote slice (see below), not a candle history API.

---

### Scrip master file-paths (instrument catalog)

**Source:** Trade API guide “How to Fetch Live Market Data with Kotak Neo API” (updated 22 May 2026, fetched 2026-08-26); `PROD_URL["scrip_master"]`; `scrip_master_api.py`; `docs/Scrip_Master.md`

**Verbatim definition / formula:**

Official guide (2026-08-26 fetch):

```
To understand how to fetch live market data with Kotak Neo API, use the endpoint:
{BASE_URL}/script-details/1.0/masterscrip/file-paths.

The request retrieves downloadable CSV files containing all of the tradeable instruments.
Equity, ETF, and index live quotes are also available to developers…
```

SDK prod path constant (`settings.py` `PROD_URL`, git `8cee5bda`):

```
"scrip_master": "script-details/1.0/masterscrip/file-paths"
```

SDK HTTP (`scrip_master_api.py`):

```
GET {get_url_details("scrip_master")}
Headers: Authorization={consumer_key}, Content-Type=application/x-www-form-urlencoded
```

`get_url_details` (`neo_utility.py`): `{get_domain()}/{PROD_URL[api_info]}` where `get_domain()` (non-init) is session `self.base_url`.

`NeoAPI.scrip_master()` requires `edit_token` **and** `edit_sid` (TOTP + MPIN complete); otherwise returns `"Complete the 2fa process before accessing this application"`.

Sample JSON (`Scrip_Master.md`):

```json
{
    "filesPaths": [
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_cm.csv",
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/cde_fo.csv",
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/mcx_fo.csv",
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv",
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv",
        "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_fo.csv"
    ],
    "baseFolder": "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod"
}
```

Live cash filenames (fetched 2026-08-27):

- SDK sample filenames: `…/transformed/{nse,bse}_cm.csv` (`Scrip_Master.md` sample above).
- Live cash files observed in OpenAlgo's unsigned lapi GET / PowerShell-tested CDN fallback (`broker/kotak/database/master_contract_db.py` `fallback_urls`, fetched 2026-08-27) on host `lapi.kotaksecurities.com`:

```
NSE_CM: https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/{today}/transformed-v1/nse_cm-v1.csv
BSE_CM: https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/{today}/transformed-v1/bse_cm-v1.csv
```

OpenAlgo downloads those URLs with an unsigned HTTP GET (no `Sid` / `Auth` on the CSV request).

Founder Station log 2026-08-27: file-paths GET 200, JSON keys `["data"]`, two cash URLs extracted, then `path_not_allowlisted` (not `host_blocked`) because the matcher required a `/nse_cm.csv` / `/bse_cm.csv` suffix. Live URLs used the `transformed-v1/{nse,bse}_cm-v1.csv` filenames.

CSV GET remains unsigned `AuthMode::Public` on `lapi.kotaksecurities.com` only (same ritual as OpenAlgo public CDN GET). Do **not** add hosts. `Sid` / `Auth` on CSV stay **NOT SPECIFIED IN SOURCE** until a real GET returns 401/403. F&O (`nse_fo.csv`, `nse_fo-v1.csv`, `*_fo.csv`, `*-fo.csv`) stay refused.

`scrip_master_api.py` reads `scrip_report.json()["data"]` then optionally filters `filesPaths` by `exchange_segment` substring. It does **not** GET the CSV bytes.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `{BASE_URL}` | Session trading base after login (guide: save `BASE_URL` from authenticate/validate) | URL |
| Path | `script-details/1.0/masterscrip/file-paths` (guide + `PROD_URL`) | path |
| Method | `GET` (`scrip_master_api.py`) | HTTP |
| `Authorization` | `configuration.consumer_key` | header |
| `filesPaths` | List of CSV URLs in SDK sample | string[] |
| `nse_cm` / `bse_cm` | Cash segments in sample filenames and SDK `exchange_segment` map | segment id |
| `{nse,bse}_cm.csv` | SDK sample cash filenames under `…/transformed/` | filename |
| `{nse,bse}_cm-v1.csv` | OpenAlgo `fallback_urls` live cash filenames under `…/transformed-v1/` (fetched 2026-08-27) | filename |
| Live cash CSV columns | Station unsigned GET 2026-08-27 of `nse_cm-v1.csv` / `bse_cm-v1.csv` (80 headers, no `pToken`). Token = `pSymbol` (numeric). Ticker = `pSymbolName`. Name = `pDesc`. Segment = `pExchSeg`. Matches OpenAlgo `process_kotak_nse_csv` (`pSymbol`→token, `pSymbolName`→symbol, `pDesc`→name). Official SDK does not document this schema. | columns |
| `nse_fo` / `bse_fo` / `cde_fo` / `mcx_fo` | Also present in the sample `filesPaths` | segment id (v1 refuse) |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether `Sid` / `Auth` / `neo-fin-key` / query `sId` are required on the file-paths GET (SDK does not send them on this call).
- Whether `Sid` / `Auth` are required on the cash CSV GET if a future GET returns 401/403 (Station unsigned GET 2026-08-27 of `nse_cm-v1.csv` / `bse_cm-v1.csv` returned 200).
- When `scrip_master_napi` (`Files/1.0/masterscrip/v2/file-paths`) is used — `scrip_master_api.py` calls `get_url_details("scrip_master")`, not the napi key.
- UAT constant `scrip/1.0/masterscrip/file-paths` vs prod `script-details/…` — prod/guide path is the one cited here.

> **OUR INTERPRETATION**
>
> - Station v1 / s1k instrument master: **cash `nse_cm` and `bse_cm` only**. Allow SDK sample `{nse,bse}_cm.csv` and live `{nse,bse}_cm-v1.csv`. Refuse F&O URLs (`nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo`, `nse_fo-v1.csv`, …) even if the file-paths payload lists them (B6). F&O CSV schema and identity mapping (`reference/expiry`, `reference/option_symbol`, lot size as a contract field): [`NFO-SCRIP-MASTER.md`](./NFO-SCRIP-MASTER.md) — **v1 refuse unchanged**.
> - File-paths GET auth mode is **PrivateRead session attach**, not `AuthMode::Public`. Wrapper requires 2FA; HTTP uses the application consumer key. This is not Binance unsigned public and not HMAC.
> - Cash CSV GET on `lapi.kotaksecurities.com` is unsigned **Public** (OpenAlgo public CDN GET). Do not attach `Sid` / `Auth`. Do not add hosts.
> - Live cash CSV (observed 2026-08-27): `pSymbol` is the instrument token, `pSymbolName` is the ticker, `pDesc` is the name. Fixture `pToken`+text-`pSymbol` remains valid for tests.
> - Phase 1 fills path (`GET {baseUrl}/quick/user/trades`) is unchanged.

---

### REST quotes (`quotes` / `quotes_neo_symbol`)

**Source:** README “Get quote details”; `docs/Quotes.md`; `quotes_neo_symbol_api.py`; `PROD_URL["quotes_neo_symbol"]`; `NeoAPI.quotes()`

**Verbatim definition / formula:**

README (fetched 2026-08-26):

```
# quote_type: Expected values are `all`, `depth`, `ohlc`, `ltp`, `oi`, `52w`, `circuit_limits`, `scrip_details`
    # By default, `quote_type` is set as `all`, which means you will get the complete data.
    # Quotes API can be accessed by access token without completing login.
# instrument_tokens: This is a list of dictionaries.
    # instrument_token: The instrument token of the stock
    # exchange_segment: Expected values are nse_cm, bse_cm, nse_fo, bse_fo, cde_fo, mcx_fo
```

`Quotes.md` parameters table: `quote_type` = `all, depth, ohlc, ltp, oi, 52w, circuit_limits, scrip_details` (default `all`). Example uses `quote_type = "all"`. `Quotes.md` also says “Quotes API can be accessed by access token without completing login” and “Only need to initialize session and generate session token”.

SDK HTTP (`quotes_neo_symbol_api.py`):

```
GET {get_url_details("quotes_neo_symbol").format(neo_symbols=urlencoded, quote_type=quote_type)}
neo_symbols = "{exchange_segment}|{instrument_token}" joined by comma, then urllib.parse.quote
Headers: Authorization={consumer_key}, Content-Type=application/x-www-form-urlencoded
Default quote_type if empty: "all"
```

Prod path constant (`settings.py`):

```
"quotes_neo_symbol": "/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}"
```

`get_url_details` concatenates `domain_info += '/' + PROD_URL.get(api_info)`. Combined with the leading `/` on this constant, the joined string is `{baseUrl}//script-details/1.0/quotes/neosymbol/…` unless a gateway normalizes it.

`NeoAPI.quotes()` does **not** check `edit_token` / `edit_sid` (unlike `scrip_master` and `subscribe`).

HTTP statuses in `Quotes.md`: 200, 400, 403 “Invalid session, please re-login to continue”, 429 “Too many requests to the API”, 500, 502, 503, 504.

**Exact HTTP method + path:**

| Item | Source value |
| ---- | ------------ |
| Method | **GET** (`quotes_neo_symbol_api.py`) |
| Path template | `/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}` (`PROD_URL["quotes_neo_symbol"]`) |
| Host | Session `baseUrl` via `get_domain()` — **not** a second documented public host in this module |

Alternate constant **not called** by `QuotesAPI.get_quotes`:

```
"quotes_neo_symbol_napi": "apim/quotes/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}"
```

Do **not** treat `/quick/quotes` as specified. It does not appear in `PROD_URL` or `quotes_neo_symbol_api.py`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `quote_type` | `all`, `depth`, `ohlc`, `ltp`, `oi`, `52w`, `circuit_limits`, `scrip_details` | string |
| `market_depth` | **Not a `quote_type` in SDK Quotes.md / README** | — |
| `instrument_token` | Token of the stock (README) | string |
| `exchange_segment` | `nse_cm`, `bse_cm`, `nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo` (README) | string |
| Auth on this GET | `Authorization` = consumer key only | header |

**Live quotes body (observed 2026-08-27):** Quotes.md return type is still `object` (schema **NOT SPECIFIED IN SOURCE**). Founder `GET …/quotes/neosymbol/nse_cm|11536/all` HTTP 200 had **root JSON array** (`root_keys=["<array>"]`) — not the v1 `{ "message": [ … ] }` fixture. OpenAlgo `broker/kotak/api/data.py` (same path; used for cash CSV column mapping) reads `response[0]`: `ltp`, `exchange`, `exchange_token`, `ohlc`, `depth`. Station maps those to TickBook cash identity `{exchange}|{exchange_token}` when `exchange` is `nse_cm` / `bse_cm`. v1 `last_traded_price` / `instrument_token` / `exchange_segment` remain valid.

**Live FO quotes body (observed 2026-08-31 IST — FO is not cash):** founder probe, logged-in session, host `e21.kotaksecurities.com`, headers `Authorization` + `Content-Type` + `Auth` + `Sid` + `Accept`. Three GETs, one token (`nse_fo|56526` = `NIFTY2692221000PE`, a live row in that day's `nse_fo` master — not fixture `12345`), all **HTTP 200**:

| `quote_type` | URL path | Root | First-row keys (sorted) |
|---|---|---|---|
| `all` | `…/quotes/neosymbol/nse_fo%7C56526/all` | `root_keys=["<array>"]` len 1 | `avg_cost`, `change`, `depth`, `display_symbol`, `exchange`, `exchange_token`, `high_price_range`, `last_traded_quantity`, `last_volume`, `low_price_range`, `lstup_time`, `ltp`, `ohlc`, `open_int`, `per_change`, `total_buy`, `total_sell`, `year_high`, `year_low` |
| `ltp` | `…/quotes/neosymbol/nse_fo%7C56526/ltp` | `root_keys=["<array>"]` len 1 | `display_symbol`, `exchange`, `exchange_token`, `ltp` |
| `oi` | `…/quotes/neosymbol/nse_fo%7C56526/oi` | `root_keys=["<array>"]` len 1 | `display_symbol`, `exchange`, `exchange_token`, `oi_high`, `oi_las`, `oi_low` |

Redacted example row (`all`) — names and JSON types only, no values:

```json
{"avg_cost": "<string>", "change": "<string>", "depth": "<object>", "display_symbol": "<string>", "exchange": "<string>", "exchange_token": "<string>", "high_price_range": "<string>", "last_traded_quantity": "<string>", "last_volume": "<string>", "low_price_range": "<string>", "lstup_time": "<string>", "ltp": "<string>", "ohlc": "<object>", "open_int": "<string>", "per_change": "<string>", "total_buy": "<string>", "total_sell": "<string>", "year_high": "<string>", "year_low": "<string>"}
```

- **Last on FO is `ltp`**, a JSON **string** that parses as f64 — the same key name cash reports, now observed on FO in its own right rather than assumed from cash. `quote_type=ltp` is a genuinely thinner slice (4 keys) carrying **the same** `ltp` key.
- **`last_traded_price`, `last`, `lastPrice`, `LTP` are ABSENT** from the live FO body. The v1 fixture `quotes_neosymbol_nfo.json` (`last_traded_price`) is hand-authored and is **not** an observation. `last_traded_quantity` / `last_volume` do exist but are quantity/volume, **not** price.
- **Identity on FO is `exchange` + `exchange_token`** — observed `"nse_fo"` / `"56526"`, matching the cash/OpenAlgo shape, **not** v1 `exchange_segment` / `instrument_token`. A third key `display_symbol` (`"NIFTY2692221000PE"`) is present on all three slices.
- **Open interest is named, and the spelling differs by slice.** `all` carries **`open_int`**; `oi` carries **`oi_las`**, **`oi_high`**, **`oi_low`** and **no** `open_int`. All are strings that parse as f64. **`oi_las` is verbatim** — truncated, no trailing `t`; do not silently read it as `oi_last`. What `oi_las` measures, and whether it equals `open_int` numerically, are **NOT SPECIFIED IN SOURCE** on the 2026-08-31 pass (values were not compared on that pass).

**Live FO OI slice compare (observed 2026-09-01 IST — same session, same day):** founder probe, logged-in session, token `nse_fo|68407` (`NIFTY26SEPFUT`). Two GETs on the same contract:

| `quote_type` | URL path | Field | Observed value |
|---|---|---|---|
| `all` | `…/quotes/neosymbol/nse_fo%7C68407/all` | `open_int` | `"16271450"` |
| `oi` | `…/quotes/neosymbol/nse_fo%7C68407/oi` | `oi_las` | `"0"` |
| `oi` | same | `oi_high` | `"0"` |
| `oi` | same | `oi_low` | `"0"` |

**UNEQUAL** — Station publishes `open_interest` from `open_int` on `all` only. The `oi` slice's `oi_las` / `oi_high` / `oi_low` are supplementary `oi_session_*` fields from `quote_type=oi`; they must **not** overwrite `open_interest` when the two slices disagree.
- **`ltp` and `open_int` can both be `"0"`, and that is a real reading.** Dogfood 2026-09-01 IST with the market shut, same session and path, two contracts: `nse_fo|68407` (`NIFTY26SEPFUT`) returned `ltp="24245.0000"` / `open_int="15778945"`, while `nse_fo|56526` (`NIFTY2692221000PE`) returned `"0"` for both. A liquid contract carrying real numbers at the same moment rules out a closed-market placeholder. So `ltp="0"` is not a price (last stays unusable — Station refuses `last <= 0`) and `open_int="0"` is a publishable reading of genuinely no open positions. Neither zero means "not fetched".
- **A `depth` key is present as an object on FO `all`.** Its inner level shape was **not** inspected on that pass — see the dedicated FO depth row below for `quote_type=depth`.

**Live FO depth body (observed 2026-09-01 IST — market shut, levels may be zero):** founder capture, logged-in session, host from session `baseUrl`, headers `Authorization` + `Content-Type` + `Auth` + `Sid` + `Accept`. One GET, token `nse_fo|56526` (`NIFTY2692221000PE`, live row in that day's `nse_fo` master):

| `quote_type` | URL path | Root | First-row keys (sorted) | Depth field | Level keys (`buy`/`sell` rows) |
|---|---|---|---|---|---|
| `depth` | `…/quotes/neosymbol/nse_fo%7C56526/depth` | `root_keys=["<array>"]` len 1 | `depth`, `display_symbol`, `exchange`, `exchange_token` | **`depth`** (JSON object) | **`price`**, **`quantity`**, **`orders`** (JSON strings) |

Redacted example row (`depth`) — names and JSON types only, no values:

```json
{"display_symbol": "<string>", "exchange": "<string>", "exchange_token": "<string>", "depth": {"buy": [{"price": "<string>", "quantity": "<string>", "orders": "<string>"}], "sell": [{"price": "<string>", "quantity": "<string>", "orders": "<string>"}]}}
```

- **Root is a JSON array**, same envelope family as cash `nse_cm|11536` and FO `all` — not the v1 `{ "message": [ … ] }` fixture.
- **Depth field name is `depth`**, nested object with sides **`buy`** / **`sell`** (not `bids`/`asks`).
- **Level keys are `price`, `quantity`, `orders`** — the same three names cash depth uses on `buy`/`sell`.
- **Identity on FO depth is `exchange` + `exchange_token`** — observed `"nse_fo"` / `"56526"`, not v1 `exchange_segment` / `instrument_token`.
- **Zero levels are a real reading when the market is shut.** This capture returned `"0"` for every `price`/`quantity`/`orders` cell on both sides. That is unusable as a ladder (Station refuses `price <= 0` / `quantity <= 0`), not a missing field — see the `ltp`/`open_int` zero note above on the same contract/day.

**Gaps (NOT SPECIFIED IN SOURCE):**

- JSON body schema for REST quotes (Quotes.md return type is `object`).
- Whether `ohlc` is one session bar vs any other window — field names for live stock feed `op`/`h`/`lo`/`c` are defined on the **websocket** doc, not on Quotes.md.
- Numeric 429 budget; whether it is shared with trade book.
- Host when `base_url` is unset (quotes without login).
- Use of `quotes_neo_symbol_napi`.
- `Sid` / `Auth` / `sId` on this GET.

> **OUR INTERPRETATION**
>
> - REST quotes are **latest_state** (LTP and optional session OHLC as `quote_type` slices), **not** `historical_series`.
> - Product cash coverage remains `nse_cm` / `bse_cm` only.
> - Not `AuthMode::Public` (Binance unsigned). Consumer key and/or session is **session attach**. Slice A should keep PrivateRead for Kotak quotes even though the SDK omits Sid/Auth on this GET.
> - Quotes/subscribe are for a **later s1k** slice. v1 poll remains trade book only.
> - Slice D: `quote_type=depth` is stored as `market/order_book/bounded_snapshot` (not `ordered_state`; HSM `isDepth` is out of scope).
> - **`nse_fo` depth body observed 2026-09-01** on `GET …/quotes/neosymbol/nse_fo|56526/depth`: root JSON array; field **`depth`**; sides **`buy`/`sell`**; level keys **`price`/`quantity`/`orders`**. Identity **`exchange`/`exchange_token`**. Station maps FO depth into DepthBook slot `{kotak-nse-nfo}\0{nse_fo|token}`. Zero levels when the market is shut are unusable, not a schema gap.

---

### Auth is not Binance public and not HMAC

**Source:** Quotes.md / README (access token without completing login); `scrip_master` 2FA gate; SESSION-AND-TRADES (Sid/Auth/`sId` on trades); `neo_utility.py` (no HMAC signature helper)

**Verbatim definition / formula:**

```
Quotes API can be accessed by access token without completing login.
```

Trade book (already documented): `GET {baseUrl}/quick/user/trades?sId={hsServerId}` with headers `Sid`, `Auth`.

**Gaps (NOT SPECIFIED IN SOURCE):**

- Official name of the consumer key vs “access token” vs `edit_token`.
- Whether quotes REST and file-paths share one rate bucket with trades.

> **OUR INTERPRETATION**
>
> - Kotak market-data REST in Station is **PrivateRead**, not Public, not HMAC.
> - Do not send Binance-style `signature` query params.

---

## Documented paths (this snapshot)

| Capability | Method | Path | Auth in SDK HTTP | Notes |
| ---------- | ------ | ---- | ---------------- | ----- |
| Scrip master file-paths | **GET** | `{BASE_URL}/script-details/1.0/masterscrip/file-paths` | `Authorization` = consumer key | Wrapper requires 2FA. Official guide 2026-05-22 / fetched 2026-08-26 |
| REST quotes | **GET** | `{baseUrl}/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}` | `Authorization` = consumer key | `quote_type` enum above. Leading `/` in `PROD_URL` may double-slash |
| CSV bytes at `filesPaths` | **GET** | `/wso2-scripmaster/v1/prod/{date}/transformed/{nse,bse}_cm.csv` (SDK sample) and `…/transformed-v1/{nse,bse}_cm-v1.csv` (live) | Unsigned Public on `lapi.kotaksecurities.com` (OpenAlgo public CDN GET, fetched 2026-08-27). Official SDK does not download. `Sid`/`Auth` **NOT SPECIFIED** | Host already allowlisted. F&O refused. No new hosts. |
| Historical candles / klines | **none** | — | — | FAQ 2026-08-26: unavailable. **S2:** `obtain(history)` remains unsupported |
| `/quick/quotes` | **NOT SPECIFIED IN SOURCE** | — | — | Do not guess |

---

## Verification Checklist

- [ ] Live `GET {baseUrl}/script-details/1.0/masterscrip/file-paths` with founder session returns 200 and `filesPaths` (founder 2026-08-27: 200, keys=`["data"]`, two cash URLs)
- [ ] Cash-only filter keeps `nse_cm.csv` / `bse_cm.csv` and live `nse_cm-v1.csv` / `bse_cm-v1.csv`; drops `nse_fo.csv` / `nse_fo-v1.csv` / `*_fo.csv`
- [ ] Unsigned GET of cash CSV on already-allowlisted `lapi.kotaksecurities.com` is path-allowlisted; `Sid`/`Auth` attach is refused; non-lapi host is `host_blocked`
- [ ] Live `GET …/quotes/neosymbol/{seg}|{token}/{quote_type}` with consumer key (and, if required, session headers) returns 200
- [x] Founder 2026-08-27: quotes 200 root is a JSON array; parser accepts OpenAlgo `ltp` / `exchange` / `exchange_token` (not only v1 `message[]`)
- [ ] Confirm whether Sid/Auth are required on quotes despite SDK omission
- [ ] Confirm whether a live unsigned cash CSV GET returns 200 or 401/403 (`Sid`/`Auth` still NOT SPECIFIED)
- [ ] Confirm quotes host when `baseUrl` is empty
- [ ] Confirm 429 numeric limits and sharing with trade book

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Invent `/quick/quotes` or `/api/v3/klines` | File-paths path is in the official guide; quotes path is `script-details/1.0/quotes/neosymbol/…` in SDK `PROD_URL`; FAQ says historical data unavailable | Documented those only |
| Treat `quote_type` `ohlc` as candle history | Quotes.md lists `ohlc` as a quote slice; FAQ forbids historical data | `ohlc` ≠ `historical_series` |
| Call `quote_type` `market_depth` | SDK says `depth` | Recorded `depth`; `market_depth` not in source |
| Mark quotes `AuthMode::Public` like Binance ticker | README: access token without login; 403 invalid session; scrip_master requires 2FA | PrivateRead / session attach |
| Allowlist `lapi.kotaksecurities.com` as proven required | Sample `filesPaths` host + OpenAlgo unsigned GET / founder 2026-08-27 file-paths URLs on lapi | Host already allowlisted; do **not** add hosts. Live filenames `transformed-v1/{nse,bse}_cm-v1.csv` now cited. |
| Require `/nse_cm.csv` suffix only | Founder 2026-08-27: two cash URLs then `path_not_allowlisted` (not `host_blocked`) | Matcher also allows `{nse,bse}_cm-v1.csv` |
| Attach `Sid`/`Auth` on cash CSV GET | Official SDK does not download; OpenAlgo GET is unsigned; no 401/403 observation | Unsigned Public; `Sid`/`Auth` remain NOT SPECIFIED |
| Use napi quotes/master paths | Constants exist; `QuotesAPI` / `ScripMasterAPI` use non-napi keys | Recorded as unused by those modules |
| Treat live quotes as v1 `{ "message": [ last_traded_price, instrument_token ] }` only | Founder 2026-08-27: 200 root `["<array>"]`; OpenAlgo `data.py` uses `response[0].ltp` / `exchange` / `exchange_token` | Parser accepts both envelopes; cash only |

No memory fills for method/path. Gaps stay `NOT SPECIFIED IN SOURCE`.

---

## Private account reads (PR A5 · observed 2026-09-01)

Session: `prod.kotak_neo.00000000-0000-4000-8000-000000000003` · fixtures under `agent/fixtures/kotak/`.

| Endpoint | Method | Root | Row key | Identity / notes |
| -------- | ------ | ---- | ------- | ---------------- |
| `/quick/user/orders` | **GET** | `{stat,stCode,data[]}` | `data[]` | `exSeg`, `prod`, `trdSym`, `nOrdNo`, `trnsTp`, `qty`, `prc`, `unFldSz`, `ordSt`. Open obtain rows: `unFldSz > 0` only. |
| `/quick/user/positions` | **GET** | `{stat,stCode,data[]}` | `data[]` | `exSeg`, `prod`, `trdSym`, `flBuyQty`/`flSellQty`, `cfBuyQty`/`cfSellQty`. Net qty = (fl+cf buy) − (fl+cf sell). |
| `/portfolio/v1/holdings` | **GET** | `{data[]}` only | `data[]` | `exchangeSegment`, `symbol`, `quantity`, `sellableQuantity`, `averagePrice`, `mktValue`, `instrumentType`. Cash book only (`nse_cm`/`bse_cm`). |
| `/quick/user/limits` | **POST** (SDK) | — | — | Live probe **HTTP 500** empty body (2026-09-01). Allowlisted; not wired to obtain until a success body is captured. |
| `/quick/user/check-margin` | **POST** (SDK) | — | — | Live probe **HTTP 500** empty body (2026-09-01). RMS ticket check, not `account/margin_estimate` calculator. |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Success JSON for `limits` / `check-margin` on this tenant gateway.
- Whether holdings GET requires `sId` query (SDK `portfolio_holdings_api.py` omits it; Station adds `sId` via session blob and GET succeeded).

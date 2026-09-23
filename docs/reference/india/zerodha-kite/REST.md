# Zerodha Kite Connect — REST API (v3)

---

## Header Block

| Field | Value |
| ----- | ----- |
| **Topic** | Kite Connect 3 REST — base transport, session/token exchange, read paths, errors, rate limits, refused execution set |
| **Primary source** | [Kite Connect 3 documentation](https://kite.trade/docs/connect/v3/) |
| **Snapshot date** | 2026-09-23 IST (pages re-fetched via HTTPS for this reference) |
| **Source version** | Kite Connect **v3** (`X-Kite-Version: 3`, login `v=3`) |
| **Staleness warning** | Re-verify every path, limit, and JSON field against the live docs before implementation. |
| **Author** | TradeAutopsy Station (P2-Z4 REFERENCE lane) |
| **B6 sheet** | `issues/brokers/sheets/zerodha_kite.md` — `SIGNED` 2026-09-23 |

---

> ⚠️ **BLOCKER**
>
> Facts below are **NOT SPECIFIED IN SOURCE** on the official pages fetched 2026-09-23 IST:
>
> - Pagination / page size for `GET /orders` and `GET /trades` (docs describe a single day list; no `limit` / cursor parameters named).
> - IP-ban or weight-budget semantics beyond the per-second rate table and HTTP `429` (no Binance-style weight system documented).
> - Settlement currency, fee-asset currency, and per-fill commission fields on order/trade JSON (no `commission` / currency keys in published response schemas).
> - Venue market open/close times (trading-session clock vs credential clock).
> - Sandbox, testnet, or paper-trading host (production URLs only on fetched pages).
>
> Do not invent values for the above at adapter build time; resolve at Z9 lock or live probe with citation.

---

## Source Inventory

| Source | URL | Date accessed (IST) | Notes |
| ------ | --- | ------------------- | ----- |
| Introduction | https://kite.trade/docs/connect/v3/ | 2026-09-23 | Form-encoded inputs; JSON responses; `api_key` + `api_secret`; redirect URL |
| User (login, token, profile, margins, logout) | https://kite.trade/docs/connect/v3/user/ | 2026-09-23 | Session exchange, signing, TTL |
| Exceptions and errors | https://kite.trade/docs/connect/v3/exceptions/ | 2026-09-23 | HTTP codes, exception names, rate table |
| Orders | https://kite.trade/docs/connect/v3/orders/ | 2026-09-23 | Order/trade book, glossary, write verbs |
| Portfolio | https://kite.trade/docs/connect/v3/portfolio/ | 2026-09-23 | Holdings, positions, conversion, authorise |
| Market quotes and instruments | https://kite.trade/docs/connect/v3/market-quotes/ | 2026-09-23 | CSV dump, quote limits |
| Historical candle data | https://kite.trade/docs/connect/v3/historical/ | 2026-09-23 | Candles (not user trade ledger) |
| GTT orders | https://kite.trade/docs/connect/v3/gtt/ | 2026-09-23 | Execution — **refused v1** (B6 row 12) |
| WebSocket streaming | https://kite.trade/docs/connect/v3/websocket/ | 2026-09-23 | `wss://ws.kite.trade` — documented; **refused as v1 sync path** (B6 row 12) |

OpenAlgo `broker/zerodha` was used only as a research lead on the B6 sheet; **this file cites official kite.trade pages only** (no OpenAlgo code or snippets).

---

## Concepts

---

### REST transport and hosts

**Source:** [Introduction](https://kite.trade/docs/connect/v3/) · fetch 2026-09-23 IST — **verify at URL**

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| REST base host | `https://api.kite.trade` | URL-dated |
| Login entry (browser redirect) | `https://kite.zerodha.com/connect/login?v=3&api_key=…` | URL-dated |
| Request body encoding | Form-encoded parameters | URL-dated |
| Success/error envelope | JSON (`status`, `data` / error fields); responses may be Gzipped | URL-dated |
| Required version header | `X-Kite-Version: 3` on API calls (shown on user/orders/portfolio examples) | URL-dated |
| CORS | Endpoints are **not** cross-site request enabled — not callable directly from browsers | URL-dated |
| Testnet / sandbox | **NOT SPECIFIED IN SOURCE** on fetched v3 pages | — |

**Gaps (NOT SPECIFIED IN SOURCE):** alternate REST base URLs; regional failover hosts.

> **OUR INTERPRETATION**
>
> - Station v1 treats production only (matches B6 row 23).
> - Kill allowlist hosts (`api.kite.trade`, `kite.zerodha.com`, `ws.kite.trade`) are documented on B6 row 22 — not expanded here.

---

### Session lifecycle and token exchange

**Source:** [User — Login flow / Authentication and token exchange](https://kite.trade/docs/connect/v3/user/) · fetch 2026-09-23 IST — **verify at URL**

**Flow (official summary):**

1. Navigate to Kite login with `api_key` (optional `redirect_params` query string on login URL).
2. After login, redirect URL receives `request_token` as a query parameter.
3. `POST /session/token` with form fields `api_key`, `request_token`, `checksum`.
4. `checksum` = SHA-256 hash of concatenation **`api_key` + `request_token` + `api_secret`** (no separators documented).
5. Response `data` includes `access_token` and profile fields (`exchanges`, `products`, `order_types`, etc.).

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Token exchange path | `POST https://api.kite.trade/session/token` | URL-dated |
| `request_token` lifetime | “only a few minutes”; exchange immediately | URL-dated |
| `access_token` expiry | Expires **6 AM on the next day** unless invalidated (regulatory requirement) | URL-dated |
| Invalidate session | `DELETE /session/token` with query `api_key` and `access_token` | URL-dated |
| Secret handling | `api_secret` must not be embedded in mobile/client apps; `access_token` must not be exposed publicly | URL-dated |

**Token exchange — request form fields**

| Field | Source definition | Provenance |
| ----- | ----------------- | ---------- |
| `api_key` | Public API key | URL-dated |
| `request_token` | One-time token from login redirect | URL-dated |
| `checksum` | SHA-256(`api_key` + `request_token` + `api_secret`) | URL-dated |

**Token exchange — response `data` attributes (published)**

| Field | Type (docs) | Provenance |
| ----- | ----------- | ---------- |
| `user_id` | string | URL-dated |
| `user_name` | string | URL-dated |
| `user_shortname` | string | URL-dated |
| `email` | string | URL-dated |
| `user_type` | string (e.g. `individual`) | URL-dated |
| `broker` | string (e.g. `ZERODHA`) | URL-dated |
| `exchanges` | string[] | URL-dated |
| `products` | string[] | URL-dated |
| `order_types` | string[] | URL-dated |
| `api_key` | string | URL-dated |
| `access_token` | string | URL-dated |
| `public_token` | string | URL-dated |
| `enctoken` | string | URL-dated |
| `refresh_token` | string (empty in example; “only available to certain approved platforms”) | URL-dated |
| `login_time` | string | URL-dated |
| `meta.demat_consent` | map | URL-dated |
| `avatar_url` | string | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** exact timezone label for “6 AM” (IST implied by product context but not spelled on token attribute line alone).

> **OUR INTERPRETATION**
>
> - Proposed auth scheme name on B6: `KiteChecksumSession` (Z5 locks) — not a venue term.
> - Loopback redirect registrability is on B6 row 10 (`127.0.0.1`); apps page not duplicated here.

---

### Signing authenticated requests

**Source:** [User — Signing requests](https://kite.trade/docs/connect/v3/user/) · fetch 2026-09-23 IST — **verify at URL**

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Scheme | HTTP `Authorization` header | URL-dated |
| Format | `Authorization: token api_key:access_token` | URL-dated |
| Per-request HMAC | **Not used** after token mint (checksum only at `/session/token`) | URL-dated (signing section) |

**Gaps (NOT SPECIFIED IN SOURCE):** whether `POST /session/token` requires `Authorization` (examples show only `X-Kite-Version` + form body).

---

### User profile and margins (read)

**Source:** [User](https://kite.trade/docs/connect/v3/user/) · fetch 2026-09-23 IST — **verify at URL**

| Method | Path | Purpose | Provenance |
| ------ | ---- | ------- | ---------- |
| GET | `/user/profile` | Profile without tokens | URL-dated |
| GET | `/user/margins/:segment` | Funds and margins | URL-dated |
| DELETE | `/session/token` | Logout / invalidate access token | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Margin segments | `:segment` = `equity` or `commodity` | URL-dated |
| Profile fields | Same entitlement arrays as token exchange (`exchanges`, `products`, `order_types`, …) | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** full nested margin JSON key list (docs publish a large example object; adapter should parse from live response at Z6, not memorize every `utilised.*` key here).

---

### Orders and trades (read paths)

**Source:** [Orders](https://kite.trade/docs/connect/v3/orders/) · fetch 2026-09-23 IST — **verify at URL**

| Method | Path | Purpose | Provenance |
| ------ | ---- | ------- | ---------- |
| GET | `/orders` | All orders for the **day** (open, pending, executed) | URL-dated |
| GET | `/orders/:order_id` | History of one order | URL-dated |
| GET | `/trades` | All trades for the **day** (all executed orders) | URL-dated |
| GET | `/orders/:order_id/trades` | Trades for one order | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Order book retention | “transient as it only lives for a day in the system” | URL-dated |
| Trade scope | “all trades generated by all executed orders for the day” | URL-dated |
| Pagination | **NOT SPECIFIED IN SOURCE** (no documented `limit` / cursor) | — |
| Timestamp format (examples) | `YYYY-MM-DD HH:MM:SS` on trades (`fill_timestamp`, `exchange_timestamp`) | URL-dated |

**Order row — published response attributes (selection)**

| Field | Type (docs) | Provenance |
| ----- | ----------- | ---------- |
| `order_id` | string | URL-dated |
| `parent_order_id` | string | URL-dated |
| `exchange_order_id` | null, string | URL-dated |
| `modified` | bool | URL-dated |
| `placed_by` | string | URL-dated |
| `variety` | string | URL-dated |
| `status` | string | URL-dated |
| `tradingsymbol` | string | URL-dated |
| `exchange` | string | URL-dated |
| `instrument_token` | string | URL-dated |
| `transaction_type` | string (`BUY` / `SELL`) | URL-dated |
| `order_type` | string | URL-dated |
| `product` | string | URL-dated |
| `validity` | string | URL-dated |
| `price` | float64 | URL-dated |
| `quantity` | int64 | URL-dated |
| `trigger_price` | float64 | URL-dated |
| `average_price` | float64 | URL-dated |
| `pending_quantity` | int64 | URL-dated |
| `filled_quantity` | int64 | URL-dated |
| `disclosed_quantity` | int64 | URL-dated |
| `order_timestamp` | string | URL-dated |
| `exchange_timestamp` | string | URL-dated |
| `exchange_update_timestamp` | string | URL-dated |
| `status_message` | null, string | URL-dated |
| `status_message_raw` | null, string | URL-dated |
| `cancelled_quantity` | int64 | URL-dated |
| `tag` | null, string | URL-dated |
| `guid` | string | URL-dated |

**Trade row — published response attributes**

| Field | Type (docs) | Provenance |
| ----- | ----------- | ---------- |
| `trade_id` | string | URL-dated |
| `order_id` | string | URL-dated |
| `exchange_order_id` | null, string | URL-dated |
| `tradingsymbol` | string | URL-dated |
| `exchange` | string | URL-dated |
| `instrument_token` | string | URL-dated |
| `transaction_type` | string | URL-dated |
| `product` | string | URL-dated |
| `average_price` | float64 | URL-dated |
| `filled` | int64 (attribute table) | URL-dated |
| `fill_timestamp` | string | URL-dated |
| `order_timestamp` | string | URL-dated |
| `exchange_timestamp` | string | URL-dated |

**Doc inconsistency (verify at URL):** trade **examples** on the orders page use JSON key `quantity` while the attribute table names `filled`. Live responses must be checked before mapping fill size.

**Gaps (NOT SPECIFIED IN SOURCE):** commission, fees, taxes, or currency on order/trade objects.

> **OUR INTERPRETATION**
>
> - v1 fill path for Station: prefer `GET /trades` poll (B6 row 2); day-book honesty (B6 row 5).
> - Order/trade `product` and `variety` values must be filtered for NSE/BSE cash CNC+MIS at book lock (B6 rows 3, 12, 21).

---

### Portfolio (read paths)

**Source:** [Portfolio](https://kite.trade/docs/connect/v3/portfolio/) · fetch 2026-09-23 IST — **verify at URL**

| Method | Path | Purpose | Provenance |
| ------ | ---- | ------- | ---------- |
| GET | `/portfolio/holdings` | Long-term equity holdings | URL-dated |
| GET | `/portfolio/positions` | Short-term positions | URL-dated |
| GET | `/portfolio/holdings/auctions` | Auction list | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Positions shape | Object with arrays `net` and `day` | URL-dated |
| Holdings example fields | Includes `tradingsymbol`, `exchange`, `instrument_token`, `isin`, `product`, `quantity`, `average_price`, `last_price`, `close_price`, `pnl`, `authorised_quantity`, … | URL-dated |
| Positions example fields | Includes `multiplier`, `average_price`, `pnl`, `m2m`, `unrealised`, `realised`, `buy_quantity`, `sell_quantity`, … | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** currency field on holdings/positions JSON.

---

### Instruments and market quotes (read)

**Source:** [Market quotes and instruments](https://kite.trade/docs/connect/v3/market-quotes/) · fetch 2026-09-23 IST — **verify at URL**

| Method | Path | Purpose | Provenance |
| ------ | ---- | ------- | ---------- |
| GET | `/instruments` | Gzipped CSV of all tradable instruments | URL-dated |
| GET | `/instruments/:exchange` | CSV for one exchange | URL-dated |
| GET | `/quote` | Full market quotes (up to instrument limit) | URL-dated |
| GET | `/quote/ohlc` | OHLC snapshot quotes | URL-dated |
| GET | `/quote/ltp` | LTP snapshot quotes | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| CSV refresh | Dump generated once daily; `last_price` not real-time | URL-dated |
| Recommended refresh time | “ideally at around **08:30 AM**” | URL-dated |
| CSV columns (published) | `instrument_token`, `exchange_token`, `tradingsymbol`, `name`, `last_price`, `expiry`, `strike`, `tick_size`, `lot_size`, `instrument_type`, `segment`, `exchange` | URL-dated |
| Max instruments per call | `/quote` **500**; `/quote/ohlc` **1000**; `/quote/ltp` **1000** | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** query parameter names for multi-instrument quote requests (documented in prose/examples on page — verify at URL before coding).

---

### Historical candles (read; not trade history)

**Source:** [Historical candle data](https://kite.trade/docs/connect/v3/historical/) · fetch 2026-09-23 IST — **verify at URL**

| Method | Path | Purpose | Provenance |
| ------ | ---- | ------- | ---------- |
| GET | `/instruments/historical/:instrument_token/:interval` | Archived OHLCV candles | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Intervals (published) | `minute`, `day`, `3minute`, `5minute`, `10minute`, `15minute`, `30minute`, `60minute` | URL-dated |
| Query params | `from`, `to` as `yyyy-mm-dd hh:mm:ss`; optional `continuous` (`0`/`1`); optional `oi` (`0`/`1`) | URL-dated |
| Candle tuple | `[timestamp, open, high, low, close, volume]` (+ OI when requested) | URL-dated |

> **OUR INTERPRETATION**
>
> - This API is **market candles**, not a substitute for `GET /trades` (B6 row 5).
> - Rate limit bucket: **3 req/s** (exceptions page) — verify at URL.

---

### Errors, exceptions, and rate limits

**Source:** [Exceptions and errors](https://kite.trade/docs/connect/v3/exceptions/) · fetch 2026-09-23 IST — **verify at URL**

**Error JSON shape (example):** `status: error`, `message`, `error_type` (e.g. `GeneralException`).

**Exception names (published)**

| `error_type` | Meaning (short) | Provenance |
| ------------ | --------------- | ---------- |
| `TokenException` | Preceded by **403** — session expired/invalid; clear session and re-login | URL-dated |
| `UserException` | User account errors | URL-dated |
| `OrderException` | Order placement/fetch failures | URL-dated |
| `InputException` | Missing/bad parameters | URL-dated |
| `MarginException` | Insufficient funds | URL-dated |
| `HoldingException` | Insufficient holdings for sell | URL-dated |
| `NetworkException` | API ↔ OMS communication failure | URL-dated |
| `DataException` | Internal OMS response parse failure | URL-dated |
| `GeneralException` | Unclassified | URL-dated |

**HTTP status codes (published)**

| Code | Meaning | Provenance |
| ---- | ------- | ---------- |
| 400 | Missing or bad request parameters or values | URL-dated |
| 403 | Session expired or invalid; must relogin | URL-dated |
| 404 | Resource not found | URL-dated |
| 405 | Method not allowed on endpoint | URL-dated |
| 410 | Resource gone permanently | URL-dated |
| 429 | Too many requests (rate limiting) | URL-dated |
| 500 | Unexpected error | URL-dated |
| 502 | OMS down | URL-dated |
| 503 | Service unavailable | URL-dated |
| 504 | Gateway timeout | URL-dated |

**API rate limits (published table)**

| Endpoint bucket | Limit | Provenance |
| --------------- | ----- | ---------- |
| Quote | **1** req/s | URL-dated |
| Historical candle | **3** req/s | URL-dated |
| Order placement | **10** req/s | URL-dated |
| All other endpoints | **10** req/s | URL-dated |

**Additional order-path limits (notes on same page)**

| Limit | Value | Provenance |
| ----- | ----- | ---------- |
| Orders per minute | **400** | URL-dated |
| Orders per second | **10** | URL-dated |
| Orders per day per user/API key | **≤ 5000** (across segments/varieties) | URL-dated |
| Modifications per order | **≤ 25**; then cancel and re-place | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** burst windows beyond stated notes; trades/orders list page size; global IP throttle beyond `429`.

---

### Refused set — v1 execution and sync (B6 row 12)

**Source:** B6 `zerodha_kite` row 12 + official endpoint lists above · fetch 2026-09-23 IST

Station **must not** call these for v1 (documented on kite.trade; refused by product gate):

| Method | Path | Official purpose | Refusal reason |
| ------ | ---- | ---------------- | -------------- |
| POST | `/orders/:variety` | Place order | Execution |
| PUT | `/orders/:variety/:order_id` | Modify order | Execution |
| DELETE | `/orders/:variety/:order_id` | Cancel order | Execution |
| POST | `/gtt/triggers` | Place GTT | Execution |
| GET | `/gtt/triggers` | List GTTs | Execution / sync |
| GET | `/gtt/triggers/:id` | Get GTT | Execution / sync |
| PUT | `/gtt/triggers/:id` | Modify GTT | Execution |
| DELETE | `/gtt/triggers/:id` | Delete GTT | Execution |
| PUT | `/portfolio/positions` | Position conversion (product change) | Execution |
| POST | `/portfolio/holdings/authorise` | Start holdings authorisation | Execution + redirect to `kite.zerodha.com/connect/portfolio/authorise/...` |

**Also refused as v1 fetch/sync paths (B6 row 12):**

| Mechanism | Detail | Provenance |
| --------- | ------ | ---------- |
| WebSocket streaming | `wss://ws.kite.trade?api_key=…&access_token=…` (≤3000 instruments/connection, ≤3 connections/key on WS page) | URL-dated WS doc; refused v1 |
| Postbacks / webhooks | Inbound order updates | B6 row 12 |
| Zerodha Console | Back-office web app — not Kite Connect | B6 row 0 |

Read-only endpoints in this reference remain allowed for a future signed adapter; v1 product scope still limits **book** to NSE/BSE cash CNC+MIS (B6 rows 1, 21).

---

## Documented paths (this snapshot)

| Capability | Method | Path | Auth | Notes | Provenance |
| ---------- | ------ | ---- | ---- | ----- | ---------- |
| Token exchange | POST | `/session/token` | Form: `api_key`, `request_token`, `checksum` | Mint `access_token` | URL-dated |
| Logout | DELETE | `/session/token?api_key=…&access_token=…` | Query params | Invalidates session | URL-dated |
| Profile | GET | `/user/profile` | `Authorization: token …` | No tokens in response | URL-dated |
| Margins | GET | `/user/margins/:segment` | `segment` = `equity` \| `commodity` | URL-dated |
| Day orders | GET | `/orders` | Bearer token | Day-only book | URL-dated |
| Order history | GET | `/orders/:order_id` | Bearer token | URL-dated |
| Day trades | GET | `/trades` | Bearer token | Primary fill poll | URL-dated |
| Order trades | GET | `/orders/:order_id/trades` | Bearer token | URL-dated |
| Holdings | GET | `/portfolio/holdings` | Bearer token | URL-dated |
| Positions | GET | `/portfolio/positions` | Bearer token | `net` + `day` | URL-dated |
| Auctions | GET | `/portfolio/holdings/auctions` | Bearer token | URL-dated |
| Instruments CSV | GET | `/instruments`, `/instruments/:exchange` | Bearer token | Gzipped CSV | URL-dated |
| Quotes | GET | `/quote`, `/quote/ohlc`, `/quote/ltp` | Bearer token | 500/1000/1000 limits | URL-dated |
| Historical candles | GET | `/instruments/historical/:instrument_token/:interval` | Bearer token | Not user trade history | URL-dated |

---

## Verification Checklist

- [ ] Live `POST /session/token` with test app keys returns `access_token` and entitlement arrays (verify at URL: user page)
- [ ] Live `GET /trades` returns day list; confirm fill-size key is `quantity` vs `filled` (verify at URL: orders page examples vs attribute table)
- [ ] Live `403` + `TokenException` after forced logout (`DELETE /session/token`) — reconnect flow (verify at URL: exceptions page)
- [ ] Rate limit: quote calls throttled at 1 req/s; generic reads at 10 req/s (verify at URL: exceptions page)
- [ ] `GET /instruments` gzip CSV parses with published column headers (verify at URL: market-quotes page)
- [ ] Refused verbs (`POST /orders/regular`, `POST /gtt/triggers`, `PUT /portfolio/positions`) stay unwired in v1 (B6 row 12)

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Copy OpenAlgo Zerodha client code | B6 forbids oracle-only evidence | Official kite.trade pages only in this file |
| Assume HMAC per request | Signing section specifies `token api_key:access_token` only | Documented checksum-at-mint model |
| Treat historical API as fill history | Historical page describes OHLC candles; orders page defines day trades | Split concepts; candles ≠ ledger |
| Wire GTT or order POST for “testing” | GTT and order write paths are on official docs but B6 row 12 refuses v1 | Listed under refused set |
| Invent pagination on `/trades` | Orders page describes full day list; no limit param | NOT SPECIFIED IN SOURCE |

No memory fills for numeric rate limits or paths. All limits and paths above were read from fetched kite.trade v3 pages on **2026-09-23 IST**.

---

## Fact ledger (for gate reporting)

| Class | Count |
| ----- | ----- |
| **URL-dated facts** | **150** (`URL-dated` markers in this file — paths, limits, codes, field names tied to kite.trade v3 pages fetched 2026-09-23 IST; **verify at URL** before build) |
| **NOT SPECIFIED IN SOURCE** | **13** (`NOT SPECIFIED IN SOURCE` markers — gaps explicitly not on fetched official pages) |

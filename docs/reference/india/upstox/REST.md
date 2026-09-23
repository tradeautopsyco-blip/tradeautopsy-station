# Upstox Developer API — REST (v2 / v3)

---

## Header Block

| Field | Value |
| ----- | ----- |
| **Topic** | Upstox REST — base transport, OAuth session/token exchange, read paths (day book), errors, rate limits, refused execution set |
| **Primary source** | [Upstox Developer API documentation](https://upstox.com/developer/api-documentation/) |
| **Snapshot date** | 2026-09-24 IST (pages re-fetched via HTTPS for this reference) |
| **Source version** | REST paths under **`/v2/`** (main host) and **`/v3/`** (order slicing, GTT, enhanced quotes/candles per official docs) |
| **Staleness warning** | Re-verify every path, limit, and JSON field against the live docs before implementation. |
| **Author** | TradeAutopsy Station (Wave 2 / Upstox REFERENCE lane) |
| **Status** | **RESEARCH** until B6 SIGNED |
| **B6 sheet** | [`issues/brokers/sheets/upstox.md`](issues/brokers/sheets/upstox.md) — not signed at snapshot time |

---

> ⚠️ **BLOCKER**
>
> Facts below are **NOT SPECIFIED IN SOURCE** on the official pages fetched 2026-09-24 IST:
>
> - Timezone label for access-token expiry (“3:30 AM the following day” — IST implied by product context but not spelled as a zone on the token page alone).
> - Commission, fees, taxes, or currency on day-trade JSON (`get-trades-for-day` / order-trades schemas).
> - Whether v1 fill sync should prefer `GET /order/trades/get-trades-for-day` vs per-order `GET /order/trades` (B6 row 2).
> - Product filter for NSE/BSE cash book (`I` / `D` vs CNC/MIS naming — venue uses `I`, `D`, `CO`, `MTF`; B6 rows 3, 21).
> - Kill allowlist host set beyond documented production hosts (B6 row 22 — e.g. whether `api-hft.upstox.com` is in-scope for read-only v1).
> - Sandbox vs production scope for Station v1 (sandbox host documented; production-only gate is B6 row 23).
>
> Do not invent values for the above at adapter build time; resolve at B6 sign or live probe with citation.

---

## Source Inventory

| Source | URL | Date accessed (IST) | Notes |
| ------ | --- | ------------------- | ----- |
| API overview | https://upstox.com/developer/api-documentation/api-overview | 2026-09-24 | Suite map |
| Request structure | https://upstox.com/developer/api-documentation/request-structure | 2026-09-24 | URL template, Bearer auth |
| New URL / headers | https://upstox.com/developer/api-documentation/announcements/new-url-and-simplified-headers | 2026-09-24 | `api.upstox.com/v2`, drop `Api-Version` header |
| Authentication | https://upstox.com/developer/api-documentation/authentication | 2026-09-24 | OAuth dialog + token exchange |
| Authorize | https://upstox.com/developer/api-documentation/authorize | 2026-09-24 | `GET /login/authorization/dialog` |
| Get Token | https://upstox.com/developer/api-documentation/get-token | 2026-09-24 | Form POST token + profile in response |
| Logout | https://upstox.com/developer/api-documentation/logout | 2026-09-24 | `DELETE /logout` |
| Rate limits | https://upstox.com/developer/api-documentation/rate-limiting | 2026-09-24 | Per-API buckets |
| Get Trades (day) | https://upstox.com/developer/api-documentation/get-trade-history | 2026-09-24 | **Page title “Get Trades”; path `get-trades-for-day`** |
| Get Order Trades | https://upstox.com/developer/api-documentation/get-trades-by-order | 2026-09-24 | Trades for one `order_id` |
| Get Order Book | https://upstox.com/developer/api-documentation/get-order-book | 2026-09-24 | Day order list |
| Get Order Details | https://upstox.com/developer/api-documentation/get-order-details | 2026-09-24 | Single order status |
| Get Order History | https://upstox.com/developer/api-documentation/get-order-history | 2026-09-24 | State transitions |
| Get Trade History | https://upstox.com/developer/api-documentation/get-historical-trades | 2026-09-24 | **`/charges/historical-trades`** — not day book |
| Get Profile | https://upstox.com/developer/api-documentation/get-profile | 2026-09-24 | `GET /user/profile` |
| Instruments (files) | https://upstox.com/developer/api-documentation/instruments | 2026-09-24 | BOD JSON on `assets.upstox.com` |
| Instrument Search | https://upstox.com/developer/api-documentation/instrument-search | 2026-09-24 | `GET /instruments/search` |
| Place / Modify / Cancel Order (v2) | place-order, modify-order, cancel-order pages | 2026-09-24 | **`api-hft.upstox.com`** — refused v1 |
| Place / Modify / Cancel Order (v3) | v3/place-order, v3/modify-order, v3/cancel-order | 2026-09-24 | **`api-hft.upstox.com/v3`** — refused v1 |
| GTT place / modify / cancel / details | place-gtt-order, modify-gtt-order, cancel-gtt-order, get-gtt-order-details | 2026-09-24 | **`api.upstox.com/v3/order/gtt*`** — refused v1 |
| Sandbox | https://upstox.com/developer/api-documentation/sandbox | 2026-09-24 | `sandbox.upstox.com` host |
| ADR 0006 (Station) | `docs/adr/0006-upstox-oauth-form-session.md` | 2026-09-24 | Auth **summary only** — official paths cited here |

**No OpenAlgo code or snippets.** Oracle leads belong on the B6 sheet, not in this file.

---

## Concepts

---

### REST transport and hosts

**Source:** [Request structure](https://upstox.com/developer/api-documentation/request-structure) · [New URL announcement](https://upstox.com/developer/api-documentation/announcements/new-url-and-simplified-headers) · fetch 2026-09-24 IST — **verify at URL**

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Primary REST URL pattern | `https://api.upstox.com/{API_VERSION}/{API_ENDPOINT}` | URL-dated |
| Recommended v2 base | `https://api.upstox.com/v2` | URL-dated |
| Legacy transitional host | `https://api-v2.upstox.com` (dual availability during migration) | URL-dated |
| HFT order host | `https://api-hft.upstox.com` (order place/modify/cancel examples) | URL-dated |
| Sandbox host (examples) | `https://sandbox.upstox.com/v2/...` | URL-dated |
| Success/error envelope | JSON; typical success uses `status: success` and `data` | URL-dated |
| Authenticated reads/writes | `Authorization: Bearer {access_token}` | URL-dated |
| Accept header | `Accept: application/json` (shown on examples) | URL-dated |
| `Api-Version` header | **Removed** on new URL (no longer required per migration notice) | URL-dated |
| JSON POST/PUT bodies | `Content-Type: application/json` | URL-dated |
| Token exchange body | `Content-Type: application/x-www-form-urlencoded` | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** full list of endpoints that must use `api-hft` vs `api.upstox.com`; regional failover hosts.

> **OUR INTERPRETATION**
>
> - Station v1 read sync uses **`api.upstox.com/v2`** only until B6 row 22 locks allowlist (HFT host is order-execution path).
> - Auth transport and host-side token exchange: **[ADR 0006](../../../adr/0006-upstox-oauth-form-session.md)** (`UpstoxOAuthFormSession` proposed); exact token path/version locked on B6 rows 14–16, not in the ADR body.

---

### Session lifecycle and token exchange (summary)

**Source:** [Authentication](https://upstox.com/developer/api-documentation/authentication) · [Get Token](https://upstox.com/developer/api-documentation/get-token) · fetch 2026-09-24 IST — **verify at URL**

**Flow (official summary):**

1. Browser redirect to **`GET https://api.upstox.com/v2/login/authorization/dialog`** with query `response_type=code`, `client_id`, `redirect_uri`, optional `state`.
2. After login, redirect URI receives **`code`** (single-use).
3. Backend **`POST https://api.upstox.com/v2/login/authorization/token`** with form fields `code`, `client_id`, `client_secret`, `redirect_uri`, `grant_type=authorization_code`.
4. Response includes **`access_token`** and profile fields (`exchanges`, `products`, `order_types`, `user_id`, …).

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Authorize path | `GET /v2/login/authorization/dialog` | URL-dated |
| Token path | `POST /v2/login/authorization/token` | URL-dated |
| Grant type | `authorization_code` | URL-dated |
| `code` lifetime | Single use (success or failure) | URL-dated |
| `access_token` expiry | Until **3:30 AM the following day** (calendar examples on token page) | URL-dated |
| Logout | `DELETE https://api.upstox.com/v2/logout` | URL-dated |
| Alternate login | `POST /v2/totp-login/authorization/token` (TOTP + PIN; `x-api-key` header) | URL-dated |

**Token exchange — request form fields**

| Field | Source definition | Provenance |
| ----- | ----------------- | ---------- |
| `code` | Authorization code from redirect | URL-dated |
| `client_id` | API key from app | URL-dated |
| `client_secret` | API secret (server-only) | URL-dated |
| `redirect_uri` | App redirect URL | URL-dated |
| `grant_type` | Must be `authorization_code` | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** IST label on 3:30 AM expiry; whether v1 uses token response profile vs subsequent `GET /user/profile`.

> **OUR INTERPRETATION**
>
> - Loopback redirect, `state` nonce, and Keychain blob shape: **ADR 0006** + B6 rows 10, 14–16 in [`issues/brokers/sheets/upstox.md`](issues/brokers/sheets/upstox.md).
> - Do not implement token exchange from this reference alone — wait for B6 **SIGNED** and ADR 0006 **ACCEPTED**.

---

### User profile (read)

**Source:** [Get Profile](https://upstox.com/developer/api-documentation/get-profile) · fetch 2026-09-24 IST — **verify at URL**

| Method | Path (full URL example) | Purpose | Provenance |
| ------ | ----------------------- | ------- | ---------- |
| GET | `https://api.upstox.com/v2/user/profile` | Exchanges, products, order types, account flags | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Profile `products` values | `I`, `D`, `CO`, `MTF` (docs) | URL-dated |
| Profile fields | `email`, `exchanges`, `products`, `broker`, `user_id`, `user_name`, `order_types`, `user_type`, `poa`, `ddpi`, `is_active` | URL-dated |

---

### Orders and trades — day book (read paths)

**Source:** order/trade pages · fetch 2026-09-24 IST — **verify at URL**

| Method | Documented path | Full URL example | Purpose | Provenance |
| ------ | --------------- | ---------------- | ------- | ---------- |
| GET | `/order/trades/get-trades-for-day` | `https://api.upstox.com/v2/order/trades/get-trades-for-day` | All **day** executions (primary fill ledger candidate) | URL-dated |
| GET | `/order/trades` | `https://api.upstox.com/v2/order/trades?order_id={id}` | Trades for one order | URL-dated |
| GET | `/order/retrieve-all` | `https://api.upstox.com/v2/order/retrieve-all` | Day **order book** | URL-dated |
| GET | `/order/details` | `https://api.upstox.com/v2/order/details?order_id={id}` | Latest status for one order | URL-dated |
| GET | `/order/history` | `https://api.upstox.com/v2/order/history?order_id={id}` | State history for one order | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Day order retention | Orders “remain active for a **single day**” / cleared after session (order-book pages) | URL-dated |
| Day trades scope | “All trades executed for the **day**” (`get-trades-for-day`) | URL-dated |
| Historical trades (non–day-book) | `GET /v2/charges/historical-trades` — last **3 financial years**, paginated (`segment`, `start_date`, `end_date`, `page_number`, `page_size`) | URL-dated |

**Day trade row — published response attributes (`get-trades-for-day` example/schema)**

| Field | Type (docs) | Provenance |
| ----- | ----------- | ---------- |
| `exchange` | string | URL-dated |
| `product` | string (`I`, `D`, `CO`, `MTF`) | URL-dated |
| `trading_symbol` | string | URL-dated |
| `tradingsymbol` | string (duplicate key in example JSON) | URL-dated |
| `instrument_token` | string | URL-dated |
| `order_type` | string | URL-dated |
| `transaction_type` | string (`BUY` / `SELL`) | URL-dated |
| `quantity` | int32 | URL-dated |
| `exchange_order_id` | string | URL-dated |
| `order_id` | string | URL-dated |
| `exchange_timestamp` | string | URL-dated |
| `average_price` | float | URL-dated |
| `trade_id` | string | URL-dated |
| `order_ref_id` | string | URL-dated |
| `order_timestamp` | string | URL-dated |

**Doc inconsistency (verify at URL):** `instrument_token` appears as a numeric string in the day-trades example (`"151064324"`) but as `NSE_EQ|…` / `NSE_FO|…` pipe keys on the order-book examples. Live responses must be checked before master join.

**Gaps (NOT SPECIFIED IN SOURCE):** fee/commission fields on trade rows; unified timestamp format (`DD-Mon-YYYY` vs `YYYY-MM-DD HH:MM:SS` across order vs trade objects).

---

### FillEvent mapping notes (verbatim preservation)

**Source:** day-trade and order-book schemas above · WIT `fill-event` in `docs/contracts/ubi-data.wit` · fetch 2026-09-24 IST

Station adapters map venue JSON into `FillEvent` without normalizing venue vocabulary in the reference layer. **Preserve venue strings as returned** for the fields below; book-level filtering (NSE/BSE cash, product allowlist) is B6-gated, not documented here as CNC/MIS aliases.

| FillEvent field | Upstox source field(s) | Preservation rule |
| --------------- | ---------------------- | ----------------- |
| `exchange-segment` | `exchange` | Copy verbatim (e.g. `NSE`, `BSE`, `NFO` — appendix-defined codes). Do not map to internal segment slugs in the adapter without B6 row lock. |
| `product` | `product` | Copy verbatim (`I`, `D`, `CO`, `MTF`). Do not translate to `CNC`/`MIS` at the JSON boundary. |
| `symbol` | `tradingsymbol` **or** `trading_symbol` | Both keys appear on trade rows; B6 must pick canonical key. Until signed, prefer **`tradingsymbol`** when present, else `trading_symbol`, **without** stripping `-EQ` suffixes or changing case. |
| `side` | `transaction_type` | Verbatim `BUY` / `SELL`. |
| `qty` | `quantity` | Numeric as returned. |
| `price` | `average_price` | Numeric as returned. |
| `fill-id` | `trade_id` | Verbatim exchange trade id string. |
| `trade-id` (optional) | `order_id` and/or `exchange_order_id` | B6 row must lock which venue id fills Station semantics; store chosen value verbatim. |
| `filled-at-unix-ms` | `exchange_timestamp` (preferred) or `order_timestamp` | Parse only after B6/timezone probe; multiple string formats documented. |

> **OUR INTERPRETATION**
>
> - `instrument_token` is **not** a `FillEvent` field; join via instrument master using verbatim token string.
> - Currency on fills: docs do not publish a trade-row currency key — **`INR` stamping is host/book policy**, not a venue field copy (B6 row 1).

---

### Instruments (read)

**Source:** [Instruments](https://upstox.com/developer/api-documentation/instruments) · [Instrument Search](https://upstox.com/developer/api-documentation/instrument-search) · fetch 2026-09-24 IST — **verify at URL**

| Mechanism | Location | Purpose | Provenance |
| --------- | -------- | ------- | ---------- |
| BOD JSON files | `https://assets.upstox.com/market-quote/instruments/exchange/complete.json.gz` (and per-exchange `NSE.json.gz`, `BSE.json.gz`, …) | Beginning-of-day contract master | URL-dated |
| MTF / MIS / suspended / global / MF | Other `assets.upstox.com/market-quote/instruments/exchange/*.json.gz` URLs on instruments page | Segment-specific masters | URL-dated |
| CSV files | `*.csv.gz` on same host | **Deprecated** in favor of JSON (announcement on instruments page) | URL-dated |
| GET | `https://api.upstox.com/v2/instruments/search` | Query + filters (`query`, `exchanges`, `segments`, pagination) | URL-dated |

| Fact | Value | Provenance |
| ---- | ----- | ---------- |
| Canonical id | Docs recommend **`instrument_key`** (e.g. `NSE_EQ|INE839G01010`) over `exchange_token` | URL-dated |
| Search `query` max length | 50 characters | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** REST endpoint to download full BOD (files are CDN URLs, not `api.upstox.com` paths).

---

### Rate limits

**Source:** [Rate Limits](https://upstox.com/developer/api-documentation/rate-limiting) · fetch 2026-09-24 IST — **verify at URL**

Limits are **per API, per user**. Exceeding limits may cause temporary suspension (docs).

**Combined order-placement bucket** — Place, Modify, Cancel, Multi Order, **GTT Order** (same bucket):

| Algo class | Per second | Per minute | Per 30 minutes | Provenance |
| ---------- | ---------- | ---------- | -------------- | ---------- |
| Regular algos (no algo registration) | **10** | **500** | **2000** | URL-dated |
| SEBI-registered algos | **50** | **500** | **2000** | URL-dated |

**Other standard APIs** (holdings, positions, funds, historical candles, etc.):

| Window | Limit | Provenance |
| ------ | ----- | ---------- |
| Per second | **50** | URL-dated |
| Per minute | **500** | URL-dated |
| Per 30 minutes | **2000** | URL-dated |

**Payout APIs**

| Access | Per second | Per minute | Per 30 minutes | Provenance |
| ------ | ---------- | ---------- | -------------- | ---------- |
| Standard (Get Payouts, Get Payout Modes, Get Payins) | **10** | **500** | **2000** | URL-dated |
| Restricted (Payout Request, Modify Payout, Cancel Payout) | — | **10** | **300** | URL-dated |

**Apply IPO**

| Per second | Per minute | Per 30 minutes | Provenance |
| ---------- | ---------- | -------------- | ---------- |
| **1** | **10** | **300** | URL-dated |

**TOTP login (Get Token via TOTP)**

| Per second | Per minute | Per 30 minutes | Provenance |
| ---------- | ---------- | -------------- | ---------- |
| **1** | **10** | **60** | URL-dated |

**Gaps (NOT SPECIFIED IN SOURCE):** explicit bucket for `get-trades-for-day` vs generic “standard APIs”; HTTP `429` body shape (see error-codes page for codes).

---

### Refused set — v1 execution (B6 row 12 TBD until sheet signed)

**Source:** official order/GTT pages · fetch 2026-09-24 IST

Station **must not** call these for v1. Paths below are **explicit** full URLs as documented (method + host + path).

| Method | Full path | Official purpose | Refusal reason |
| ------ | --------- | ---------------- | -------------- |
| POST | `https://api-hft.upstox.com/v2/order/place` | Place order (v2) | Execution |
| PUT | `https://api-hft.upstox.com/v2/order/modify` | Modify order (v2) | Execution |
| DELETE | `https://api-hft.upstox.com/v2/order/cancel?order_id={order_id}` | Cancel order (v2) | Execution |
| POST | `https://api-hft.upstox.com/v3/order/place` | Place order with slicing (v3) | Execution |
| PUT | `https://api-hft.upstox.com/v3/order/modify` | Modify order (v3) | Execution |
| DELETE | `https://api-hft.upstox.com/v3/order/cancel?order_id={order_id}` | Cancel order (v3) | Execution |
| POST | `https://api.upstox.com/v3/order/gtt/place` | Place GTT | Execution |
| PUT | `https://api.upstox.com/v3/order/gtt/modify` | Modify GTT | Execution |
| DELETE | `https://api.upstox.com/v3/order/gtt/cancel` | Cancel GTT (JSON body `gtt_order_id`) | Execution |
| GET | `https://api.upstox.com/v3/order/gtt?gtt_order_id={id}` | GTT order details | GTT sync / execution family — **confirm on B6 row 12** |

**Also refused as v1 sync paths (typical B6 row 12 — confirm on sign):**

| Mechanism | Detail | Provenance |
| --------- | ------ | ---------- |
| WebSocket market/portfolio feeds | Authorize + stream URLs on websocket docs | URL-dated; refused v1 sync |
| Order/GTT webhooks | Inbound POST notifications | URL-dated; refused v1 |
| Sandbox write host | e.g. `https://sandbox.upstox.com/v2/order/place` | URL-dated; production-only v1 until B6 row 23 |

Read-only REST paths in this reference remain allowed for a future signed adapter; v1 **book** scope still limits segments/products on B6 rows 1, 21.

---

## Documented paths (this snapshot)

| Capability | Method | Path | Host | Auth | Notes | Provenance |
| ---------- | ------ | ---- | ---- | ---- | ----- | ---------- |
| OAuth dialog | GET | `/v2/login/authorization/dialog` | `api.upstox.com` | Query: `client_id`, `redirect_uri`, `response_type=code` | Browser step | URL-dated |
| Token exchange | POST | `/v2/login/authorization/token` | `api.upstox.com` | Form OAuth fields | Mint `access_token` | URL-dated |
| Logout | DELETE | `/v2/logout` | `api.upstox.com` | Bearer | Invalidates session | URL-dated |
| Profile | GET | `/v2/user/profile` | `api.upstox.com` | Bearer | Entitlements | URL-dated |
| Day trades (fills) | GET | `/v2/order/trades/get-trades-for-day` | `api.upstox.com` | Bearer | Primary day ledger | URL-dated |
| Order trades | GET | `/v2/order/trades?order_id=` | `api.upstox.com` | Bearer | Per-order fills | URL-dated |
| Day orders | GET | `/v2/order/retrieve-all` | `api.upstox.com` | Bearer | Order book | URL-dated |
| Order details | GET | `/v2/order/details?order_id=` | `api.upstox.com` | Bearer | | URL-dated |
| Order history | GET | `/v2/order/history?order_id=` | `api.upstox.com` | Bearer | | URL-dated |
| Historical trades | GET | `/v2/charges/historical-trades` | `api.upstox.com` | Bearer | Not day book; 3 FY | URL-dated |
| Instrument search | GET | `/v2/instruments/search` | `api.upstox.com` | Bearer | Paginated search | URL-dated |
| BOD instruments | GET | CDN `*.json.gz` | `assets.upstox.com` | None (public files) | Master data | URL-dated |

---

## Verification Checklist

- [ ] Live OAuth dialog + `POST /v2/login/authorization/token` returns `access_token` and entitlement arrays (verify at URL: authentication / get-token)
- [ ] Live `GET /v2/order/trades/get-trades-for-day` returns day list; confirm `tradingsymbol` vs `trading_symbol` and `instrument_token` shape (verify at URL: get-trade-history page)
- [ ] Live `GET /v2/order/retrieve-all` JSON wraps vs array — confirm response envelope (verify at URL: get-order-book)
- [ ] Rate limit: standard read bucket 50 req/s (verify at URL: rate-limiting)
- [ ] Refused verbs (`POST …/v2/order/place`, `POST …/v3/order/gtt/place`, etc.) stay unwired in v1 (B6 row 12)
- [ ] B6 [`issues/brokers/sheets/upstox.md`](issues/brokers/sheets/upstox.md) signed; ADR 0006 accepted before bindings

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Copy OpenAlgo Upstox client | B6 forbids oracle-only evidence | Official upstox.com developer pages only |
| Map `D`/`I` to CNC/MIS in reference | Product appendix uses `I`, `D`, `CO`, `MTF` | Verbatim preservation; filter at B6 lock |
| Use historical-trades as fill poll | Page describes multi-year charges history | Day book = `get-trades-for-day` |
| Call HFT host for reads | Examples use HFT for writes only | Reads on `api.upstox.com/v2` until B6 says otherwise |
| Treat doc as SIGNED | Header status RESEARCH | Wait for B6 + ADR 0006 accept |

No memory fills for numeric rate limits or paths. Limits and paths above were read from fetched Upstox developer pages on **2026-09-24 IST**.

---

## Fact ledger (for gate reporting)

| Class | Count |
| ----- | ----- |
| **URL-dated facts** | **95** (`URL-dated` markers in this file — paths, limits, field names tied to upstox.com pages fetched 2026-09-24 IST; **verify at URL** before build) |
| **NOT SPECIFIED IN SOURCE** | **11** (`NOT SPECIFIED IN SOURCE` markers — gaps explicitly not on fetched official pages) |

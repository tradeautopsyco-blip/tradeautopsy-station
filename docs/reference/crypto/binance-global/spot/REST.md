# Binance Global — Spot REST API

**Exchange:** Binance Global (`api.binance.com`) — **not** Binance.US (`api.binance.us`)  
**Source:** `rest-api.md`, `introduction.md` — developers.binance.com  
**Schema:** `schema.yaml` (13,311 lines), `schema__2_.yaml` (duplicate, same)  
**Snapshot date:** 2026-07-02  
**Klines re-fetched:** 2026-08-26 from [binance-spot-api-docs `rest-api.md`](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) (Kline/Candlestick + General API Information + Request Security) · [Market Data Only FAQ](https://github.com/binance/binance-spot-api-docs/blob/master/faqs/market_data_only.md) · [errors.md](https://github.com/binance/binance-spot-api-docs/blob/master/errors.md)  
**Depth weight table re-fetched:** 2026-08-30 from [binance-spot-api-docs `rest-api.md` Order book](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) (`GET /api/v3/depth`). Default `limit` is 100; maximum 5000.  
**Reviewed:** Klines / `historical_series` section yes (2026-08-26). Depth weight table yes (2026-08-30). Remainder of this file: no.

---

## Base Endpoints

- `https://api.binance.com` (primary)
- `https://api-gcp.binance.com`
- `https://api1.binance.com` through `https://api4.binance.com` (better perf, less stable)
- Market data only: `https://data-api.binance.vision`

Responses: JSON by default. SBE available (see SBE FAQ).  
Data order: chronological (oldest first) unless noted.  
Timestamps: milliseconds by default. Add header `X-MBX-TIME-UNIT:MICROSECOND` for microseconds.

---

## Authentication

Methods supported: HMAC-SHA256, RSA (PKCS#8), Ed25519

`SIGNED` endpoints require `timestamp` + `signature` in query string or body.  
`signature = HMAC_SHA256(secretKey, queryString + requestBody)` for HMAC.  
API key via `X-MBX-APIKEY` header.

`timestamp` param: milliseconds or microseconds.  
`recvWindow`: defaults 5000ms, max 60000ms.

---

## Rate Limits

Response header: `X-MBX-USED-WEIGHT-{intervalNum}{intervalLetter}`

| Type | Interval | Limit |
|---|---|---|
| `REQUEST_WEIGHT` | 1 minute | 6000 |
| `ORDERS` | 1 second | 10 |
| `RAW_REQUESTS` | 5 minutes | 61000 |

HTTP 429 = rate limit hit. HTTP 418 = IP banned.

---

## Security Types

| Type | Description |
|---|---|
| `NONE` | Public, no auth |
| `TRADE` | API key + signature |
| `USER_DATA` | API key + signature |
| `USER_STREAM` | API key only |
| `MARKET_DATA` | API key only |

---

## Key Endpoints (TradeAutopsy relevant)

### Exchange Info
```
GET /api/v3/exchangeInfo    Weight: 20 (no params) / 4 (with symbol)
Security: NONE
```
Returns: `timezone`, `serverTime`, `rateLimits[]`, `exchangeFilters[]`, `symbols[]`  
Each symbol includes: filters, `orderTypes`, `permissions`, `status`

### Account Info
```
GET /api/v3/account    Weight: 20
Security: USER_DATA
```
Returns: `makerCommission`, `takerCommission`, `buyerCommission`, `sellerCommission`, `canTrade`, `canWithdraw`, `canDeposit`, `brokered`, `requireSelfTradePrevention`, `preventSor`, `updateTime`, `accountType`, `balances[]`, `permissions[]`, `uid`

### My Trades
```
GET /api/v3/myTrades    Weight: 20
Security: USER_DATA
```
Params: `symbol` (required), `orderId`, `startTime`, `endTime`, `fromId`, `limit` (default 500, max 1000)

Returns per trade: `symbol`, `id`, `orderId`, `orderListId`, `price`, `qty`, `quoteQty`, `commission`, `commissionAsset`, `time`, `isBuyer`, `isMaker`, `isBestMatch`

### Open Orders
```
GET /api/v3/openOrders    Weight: 6 (symbol) / 40 (no symbol)
Security: USER_DATA
```
Params: `symbol` (optional)

### All Orders
```
GET /api/v3/allOrders    Weight: 20
Security: USER_DATA
```
Params: `symbol` (required), `orderId`, `startTime`, `endTime`, `limit` (default 500, max 1000)

### Order Status
```
GET /api/v3/order    Weight: 4
Security: USER_DATA
```
Params: `symbol` + (`orderId` or `origClientOrderId`)

### Ticker (24hr)
```
GET /api/v3/ticker/24hr    Weight: 80 (no symbol) / 2 (1 symbol) / varies (list)
Security: NONE
```

### Current Price
```
GET /api/v3/ticker/price    Weight: 4 (no symbol) / 2 (1 symbol)
Security: NONE
```

### Order Book
```
GET /api/v3/depth
Security: NONE
```
Params: `symbol` (mandatory), `limit` (INT, optional; **default 100**, maximum 5000). If `limit > 5000`, only 5000 entries are returned.

**Weight** — adjusted by `limit`. Source: [binance-spot-api-docs `rest-api.md` Order book](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) fetched 2026-08-30.

| Limit     | Weight |
|-----------|--------|
| 1–100     | 5      |
| 101–500   | 25     |
| 501–1000  | 50     |
| 1001–5000 | 250    |

### Recent Trades
```
GET /api/v3/trades    Weight: 25
Security: NONE
```

### Klines (Candlesticks) — S2 market `ohlcv` / `historical_series`

**S2 docs lock (2026-08-26).** Live fetch, host-path allowlist of this path, sqlite coverage store, and Notch History rewrite are **out of this slice**. B6: [`docs/research/sheets/binance_com.md`](../../../../research/sheets/binance_com.md). Kotak `obtain(history)` stays unsupported on a **different slug** — do not invent Kotak klines.

> ⚠️ **NOT SPECIFIED IN SOURCE (do not invent for the S2 quality layer)**
>
> - How far back REST klines are retained (listing date, N years, per-interval TTL): **not in** `rest-api.md` Kline/Candlestick, General API Information, Market Data Only FAQ, or `errors.md` (fetched 2026-08-26). Do **not** emit `insufficient_retention` from this source set.
> - Max hours between `startTime` and `endTime` on **this** path. General error `-1127 MORE_THAN_XX_HOURS` exists in `errors.md` but is **not** attached to `GET /api/v3/klines` in the Kline/Candlestick section.
> - Whether S2 must allowlist `data-api.binance.vision`, `api-gcp.binance.com`, or `api1`–`api4.binance.com` in addition to `api.binance.com`.
> - Stitch / Yahoo / `binance-public-data` bulk dump as a Binance.com REST capability.

```
GET /api/v3/klines
Weight: 2
Security: NONE (unsigned / public)
```

**Host (COM):** `https://api.binance.com` — General API Information (fetched 2026-08-26): “The following base endpoints are available… **https://api.binance.com**”. Same section lists `api-gcp`, `api1`–`api4`, and says public market-data-only traffic may use **`https://data-api.binance.vision`**. Market Data Only FAQ (fetched 2026-08-26) lists `GET /api/v3/klines` on that vision host and states those URLs “do not require any authentication (i.e. The API key is not necessary)”. **Not** `api.binance.us`.

**Security NONE:** Source heading is `Kline/Candlestick data` with **no** `(TRADE)` / `(USER_DATA)` suffix. Request Security (fetched 2026-08-26): “If unspecified, the security type is `NONE`.” Table: `NONE` = “Public market data”. Do **not** attach HMAC `api_key` / `api_secret`, `signature`, or `X-MBX-APIKEY`. That attach is `PrivateCredentialOnPublicCall`, not USER_DATA.

**Mandatory params:** `symbol` (STRING), `interval` (ENUM). **Optional:** `startTime` (LONG), `endTime` (LONG), `timeZone` (STRING, default `0` / UTC), `limit` (INT, default **500**, maximum **1000**).

**Intervals (case-sensitive), verbatim from source:**

| Interval | `interval` value |
| -------- | ---------------- |
| seconds | `1s` |
| minutes | `1m`, `3m`, `5m`, `15m`, `30m` |
| hours | `1h`, `2h`, `4h`, `6h`, `8h`, `12h` |
| days | `1d`, `3d` |
| weeks | `1w` |
| months | `1M` |

Unsupported `interval` → later S2 code ineligible **`unsupported_interval`**. Source error `-1120 BAD_INTERVAL` / “Invalid interval.” (`errors.md`, fetched 2026-08-26) is the exchange reject; it does not define extra intervals.

**Range / limit:** Documented ceiling is **`limit` max 1000** per call (default 500). Requesting beyond that documented max is **`unsupported_range`**. A wall-clock span longer than 1000 bars is a **pagination** problem under that limit, not a documented retention horizon.

**Pagination (General API Information, fetched 2026-08-26; klines notes repeat the empty-range case):**

- Klines are uniquely identified by their open time.
- Data is returned in chronological order unless noted otherwise.
- Without `startTime` or `endTime`: most recent items up to the limit (klines section: “If `startTime` and `endTime` are not sent, the most recent klines are returned.”).
- With `startTime`: oldest items from `startTime` up to the limit.
- With `endTime`: most recent items up to `endTime` and the limit.
- With both: behaves like `startTime` but does not exceed `endTime`.
- `timeZone`: hours/minutes (e.g. `-1:00`, `05:45`) or hours only (`0`, `8`, `4`); accepted range strictly `[-12:00 to +14:00]` inclusive. If provided, **kline intervals** are interpreted in that timezone; **`startTime` / `endTime` are always UTC**.

**Retention / how far back:** **NOT SPECIFIED IN SOURCE.** Do not invent listing-date coverage, a year count, or `insufficient_retention` from this snapshot.

**Response array indexes** (one kline = one inner array; source comments, fetched 2026-08-26):

| Index | Source comment | Example in source |
| ----- | -------------- | ----------------- |
| 0 | Kline open time | `1499040000000` |
| 1 | Open price | `"0.01634790"` |
| 2 | High price | `"0.80000000"` |
| 3 | Low price | `"0.01575800"` |
| 4 | Close price | `"0.01577100"` |
| 5 | Volume | `"148976.11427815"` |
| 6 | Kline Close time | `1499644799999` |
| 7 | Quote asset volume | `"2434.19055334"` |
| 8 | Number of trades | `308` |
| 9 | Taker buy base asset volume | `"1756.87402397"` |
| 10 | Taker buy quote asset volume | `"28.46694368"` |
| 11 | Unused field, ignore. | `"0"` |

**Data Source:** Database. Timestamps: milliseconds by default (General API Information).

**Not this capability:**

- Account `GET /api/v3/myTrades` (USER_DATA) — B6 row 5 trade history, not market OHLCV.
- `GET /api/v3/uiKlines` — same params/weight; source: “modified kline data, optimized for presentation of candlestick charts.” Not the S2 `historical_series` path unless a later slice cites it.
- Place / modify / cancel (`POST /api/v3/order`, …) — not data.
- Kotak candles — unsupported on `kotak_neo`.

### Ping / Time
```
GET /api/v3/ping    Weight: 1
GET /api/v3/time    Weight: 1
```

---

## Notes

General API Information (re-confirmed 2026-08-26; applies unless an endpoint notes otherwise — klines uses these plus its own empty-range note):

- Without `startTime`/`endTime`: returns most recent items up to limit
- With `startTime`: returns oldest items from startTime
- With `endTime`: returns most recent up to endTime
- With both: behaves like startTime, does not exceed endTime

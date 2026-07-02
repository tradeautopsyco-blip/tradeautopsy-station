# Binance Global — Spot REST API

**Exchange:** Binance Global (api.binance.com)  
**Source:** `rest-api.md`, `introduction.md` — developers.binance.com  
**Schema:** `schema.yaml` (13,311 lines), `schema__2_.yaml` (duplicate, same)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

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
GET /api/v3/depth    Weight: 5–250 depending on limit
Security: NONE
```
Params: `symbol`, `limit` (default 100, max 5000)

### Recent Trades
```
GET /api/v3/trades    Weight: 25
Security: NONE
```

### Klines (Candlesticks)
```
GET /api/v3/klines    Weight: 2
Security: NONE
```
Params: `symbol`, `interval`, `startTime`, `endTime`, `timeZone`, `limit` (default 500, max 1000)

Intervals: `1s`, `1m`, `3m`, `5m`, `15m`, `30m`, `1h`, `2h`, `4h`, `6h`, `8h`, `12h`, `1d`, `3d`, `1w`, `1M`

### Ping / Time
```
GET /api/v3/ping    Weight: 1
GET /api/v3/time    Weight: 1
```

---

## Notes

- Without `startTime`/`endTime`: returns most recent items up to limit
- With `startTime`: returns oldest items from startTime
- With `endTime`: returns most recent up to endTime
- With both: behaves like startTime, does not exceed endTime

# Binance.US — Spot REST API

**Exchange:** Binance.US (separate entity from Binance Global)  
**Base URL:** `https://api.binance.us`  
**Source:** GitHub `binance-us/binance-us-api-docs` (9 files)  
**Snapshot date:** 2023-09-06  
**Reviewed:** No  
**TradeAutopsy use:** Read-only sync (fills, balances, open orders). No order placement.

---

## Authentication

Signing method: HMAC-SHA256  
`SIGNED` endpoints require `timestamp` + `signature` parameters.  
`signature = HMAC_SHA256(secretKey, queryString + requestBody)`  
API key passed via `X-MBX-APIKEY` header.

`recvWindow` defaults to 5000ms, max 60000ms.

---

## Rate Limits

Response headers: `X-MBX-USED-WEIGHT-{intervalNum}{intervalLetter}`

Rate limit types (from source enums):
- `REQUEST_WEIGHT` — 1200/min (NOT SPECIFIED IN SOURCE as exact limit; verify live)
- `ORDERS` — NOT SPECIFIED IN SOURCE for Binance.US specifically
- `RAW_REQUESTS` — NOT SPECIFIED IN SOURCE for Binance.US specifically

HTTP 429 = rate limit hit. HTTP 418 = IP banned after repeated 429s.

> ⚠️ Global Binance limits are 6000 REQUEST_WEIGHT/min and 61000 RAW_REQUESTS/5min. Binance.US limits are NOT CONFIRMED to match these. Verify against live `GET /api/v3/exchangeInfo`.

---

## Endpoints Used by TradeAutopsy

### Account Info
```
GET /api/v3/account
Security: USER_DATA (signed)
Weight: 20
```
Returns: `makerCommission`, `takerCommission`, `buyerCommission`, `sellerCommission`, `canTrade`, `canWithdraw`, `canDeposit`, `updateTime`, `accountType`, `balances[]`, `permissions[]`

**TradeAutopsy relevance:** `canWithdraw` flag at account level is the only confirmed withdraw-permission signal. Key-level withdraw-permission endpoint does NOT appear in source docs — see MANIFEST.md Blocker 1.

---

### My Trades (Fill History)
```
GET /api/v3/myTrades
Security: USER_DATA (signed)
Weight: 20
```
Parameters: `symbol` (required), `orderId`, `startTime`, `endTime`, `fromId`, `limit` (default 500, max 1000)

Returns per trade: `symbol`, `id`, `orderId`, `orderListId`, `price`, `qty`, `quoteQty`, `commission`, `commissionAsset`, `time`, `isBuyer`, `isMaker`, `isBestMatch`

**Retention window:** NOT SPECIFIED IN SOURCE as a hard limit for Binance.US. The only "90 days" language in source refers to archived zero-fill orders (`ORDER_ARCHIVED` error `-2026`), not trade history. See MANIFEST.md Blocker 2.

---

### Open Orders
```
GET /api/v3/openOrders
Security: USER_DATA (signed)
Weight: 40 (no symbol) / 6 (with symbol)
```
Parameters: `symbol` (optional)

Returns array of order objects: `symbol`, `orderId`, `orderListId`, `clientOrderId`, `price`, `origQty`, `executedQty`, `cummulativeQuoteQty`, `status`, `timeInForce`, `type`, `side`, `stopPrice`, `icebergQty`, `time`, `updateTime`, `isWorking`, `origQuoteOrderQty`

---

### All Orders (Historical)
```
GET /api/v3/allOrders
Security: USER_DATA (signed)
Weight: 20
```
Parameters: `symbol` (required), `orderId`, `startTime`, `endTime`, `limit` (default 500, max 1000)

---

### Exchange Info
```
GET /api/v3/exchangeInfo
Security: NONE
Weight: 20
```
Returns rate limits, symbol filters, trading rules. Use to confirm live rate limits (see Blocker 3).

---

### Ping / Time
```
GET /api/v3/ping       Weight: 1
GET /api/v3/time       Weight: 1
```

---

## Error Codes (Relevant Subset)

| Code | Name | Meaning |
|---|---|---|
| `-1021` | INVALID_TIMESTAMP | timestamp outside recvWindow |
| `-1022` | INVALID_SIGNATURE | signature mismatch |
| `-2014` | BAD_API_KEY_FMT | malformed API key |
| `-2015` | REJECTED_MBX_KEY | invalid key / IP / permissions |
| `-2026` | ORDER_ARCHIVED | order older than 90 days (archived zero-fill orders only — NOT trade history) |

---

## TradeAutopsy Constraints (Hard Rules)

1. **Read-only.** No `POST`, `PUT`, `DELETE` endpoints called. Ever.
2. **No withdrawals.** `canWithdraw` must be false or blocked at credential-save time.
3. **Keychain-backed credentials.** API key + secret never in plaintext after auth.
4. **Behavioral opt-out.** User can disable sync without deleting credentials.
5. **Redaction boundary.** API key displayed truncated in UI. Secret never displayed.

# Binance Global — Portfolio Margin REST API

**Exchange:** Binance Global  
**Product:** Portfolio Margin (`papi.binance.com`)  
**Source:** General Info doc (document index 20 in session), Public API Definitions (index 23/24)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Base Endpoint

`https://papi.binance.com`

Data: ascending order (oldest first). All times in UTC milliseconds. Java data types.

---

## HTTP Return Codes

| Code | Meaning |
|---|---|
| `4XX` | Malformed request — sender's issue |
| `403` | WAF limit violated |
| `429` | Rate limit exceeded |
| `418` | IP auto-banned after repeated 429s |
| `503 "Unknown error..."` | Request received, no response before timeout — **execution status UNKNOWN, may have succeeded** |
| `503 "Service Unavailable."` | 100% failure, retry with backoff |
| `503 / -1008 "Request throttled..."` | 100% failure — system overload. Close-position and reduce-only orders exempt |

---

## Rate Limits

IP limit: **6000/min**  
Order limit: **1200/min**

Header: `X-MBX-ORDER-COUNT-{intervalNum}{intervalLetter}` on order responses.  
IP bans: 2 minutes to 3 days, scaling for repeat offenders.

---

## Authentication

Method: HMAC-SHA256 or RSA (PKCS#8)  
Signed params: query string + body concatenated, signature appended last.  
`recvWindow` default 5000ms, max 60000ms.

---

## Security Types

| Type | Requirement |
|---|---|
| `NONE` | Public |
| `TRADE` | API key + signature |
| `USER_DATA` | API key + signature |
| `USER_STREAM` | API key + signature |

---

## Endpoint Prefixes

| Scope | Prefix |
|---|---|
| UM (USDⓈ-M Futures) | `/papi/v1/um/...` |
| CM (COIN-M Futures) | `/papi/v1/cm/...` |
| Margin | `/papi/v1/margin/...` |
| Portfolio account | `/papi/v1/...` or `/sapi/v1/portfolio/...` |

---

## Key Endpoints (partial — full list in schema)

### Account
- `GET /papi/v1/um/account` — UM account info
- `GET /papi/v2/um/account` — v2: only symbols with positions/open orders
- `GET /papi/v1/cm/account` — CM account info
- `GET /papi/v1/um/positionRisk` — UM positions
- `GET /papi/v1/cm/positionRisk` — CM positions
- `GET /papi/v1/balance` — account balances

### UM Order Management
- `POST /papi/v1/um/order` — place UM order
- `PUT /papi/v1/um/order` — modify UM order
- `DELETE /papi/v1/um/order` — cancel UM order
- `GET /papi/v1/um/order` — query UM order
- `GET /papi/v1/um/openOrders` — open UM orders
- `GET /papi/v1/um/allOrders` — all UM orders

### CM Order Management
- `POST /papi/v1/cm/order`
- `PUT /papi/v1/cm/order`
- `DELETE /papi/v1/cm/order`

### Margin Order Management
- `POST /papi/v1/margin/order`
- `DELETE /papi/v1/margin/order`
- `GET /papi/v1/margin/order`
- `GET /papi/v1/margin/openOrders`
- `GET /papi/v1/margin/allOrders`
- `GET /papi/v1/margin/myTrades`

### Algo Orders (from 2026-04-28)
- `POST /papi/v1/um/algo/order`
- `DELETE /papi/v1/um/algo/order`
- `GET /papi/v1/um/algo/openAlgoOrders`
- `GET /papi/v1/um/algo/allAlgoOrders`

### Margin Call Level
- `POST /sapi/v1/portfolio/margin-call-level`
- `GET /sapi/v1/portfolio/margin-call-level`
- `DELETE /sapi/v1/portfolio/margin-call-level`

### Connectivity
- `POST /papi/v1/ping`

---

## Terminology (from source)

- `Margin` = Cross Margin
- `UM` = USD-M Futures
- `CM` = Coin-M Futures

# OKX Global — Spot REST API (`okx_com`)

**Venue:** OKX global · slug **`okx_com`**  
**Host:** `https://www.okx.com` (**not** `us.okx.com`, **not** `eea.okx.com`)  
**Source:** [OKX API v5 documentation](https://www.okx.com/docs-v5/en/)  
**Snapshot date:** 2026-09-24 IST

---

## Base URL

- Production REST: `https://www.okx.com`
- Use the **`www`** host to avoid cross-domain redirects that may strip auth headers (official note in API overview).

**Refused for Station live:** `us.okx.com`, `eea.okx.com`, demo/sandbox header `x-simulated-trading: 1`.

---

## Authentication (private REST)

Headers on signed requests:

| Header | Description |
|--------|-------------|
| `OK-ACCESS-KEY` | API key |
| `OK-ACCESS-SIGN` | Base64 HMAC-SHA256 signature |
| `OK-ACCESS-TIMESTAMP` | ISO-8601 UTC with milliseconds |
| `OK-ACCESS-PASSPHRASE` | Passphrase set at key creation |

Signature preimage:

```text
timestamp + METHOD + requestPath + body
```

- `requestPath` is path + query (e.g. `/api/v5/trade/fills?instType=SPOT&limit=100`).
- GET `body` is empty.

Envelope: `{ "code": "0", "msg": "", "data": … }` — non-zero `code` is an application error.

---

## Rate limits

Per-endpoint limits; throttling returns **`code` 50011** (“Rate limit reached…”). See [Rate Limits](https://www.okx.com/docs-v5/en/#overview-rate-limits).

---

## TradeAutopsy v1 endpoints

### Transaction details (fills — last 3 days)

```
GET /api/v5/trade/fills
```

| Param | Notes |
|-------|--------|
| `instType` | `SPOT` for first book |
| `instId` | Optional, e.g. `BTC-USDT` |
| `after` / `before` | Pagination (bill id) |
| `limit` | Default 100, max 100 |

Fill fields used: `instId`, `tradeId`, `ordId`, `side`, `fillPx`, `fillSz`, `fee`, `feeCcy`, `ts`, `billId`.

### Account balance (bootstrap / Connect probe)

```
GET /api/v5/account/balance
```

### Public instruments (filters)

```
GET /api/v5/public/instruments?instType=SPOT
```

Per-symbol **`tickSz`**, **`lotSz`**, **`minSz`**, **`quoteCcy`**, **`baseCcy`** — persist at runtime; do not invent tick/lot from precision fields alone.

### Candles (optional history obtain)

```
GET /api/v5/market/candles?instId=&bar=&limit=
```

---

## Quote filter (B6 row 24)

| Quote on `instId` | Desk handling |
|-------------------|---------------|
| USDT, USDC, USD | `crypto_spot_usd` WAC (`round_trip_engine`) |
| EUR | Flag `quote_not_usd`; separate honesty — **no silent USD reuse** |
| Other (`BTC`, `ETH`, …) | `currency` from suffix; flag `quote_unhandled` when honesty fails |

Fees: `fee` + `feeCcy` from fill row — never hardcoded.

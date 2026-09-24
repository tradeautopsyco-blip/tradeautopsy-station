# Bybit — Spot REST API (V5)

**Exchange:** Bybit global  
**Host:** `https://api.bybit.com` (production)  
**Refused live hosts:** `api-testnet.bybit.com`, `api-demo.bybit.com`  
**Status:** RESEARCH (Phase 4 scaffold — cite before integrator ships)  
**Snapshot date:** 2026-09-24 IST  
**Primary docs:** https://bybit-exchange.github.io/docs/v5/guide

---

## Authentication (HMAC)

Private GET requests:

```text
payload = timestamp + api_key + recv_window + queryString
signature = HMAC_SHA256(secret, payload) as lowercase hex
```

Headers:

| Header | Value |
|--------|--------|
| `X-BAPI-API-KEY` | API key |
| `X-BAPI-TIMESTAMP` | UTC ms |
| `X-BAPI-RECV-WINDOW` | default `5000` |
| `X-BAPI-SIGN` | signature |

Response envelope: `{ retCode, retMsg, result, time }` — require `retCode == 0`.

---

## Rate limits (trade reads)

| Method | Path | Limit (UTA2.0 Pro) |
|--------|------|---------------------|
| GET | `/v5/execution/list` | 50/s |
| GET | `/v5/order/history` | 50/s |
| GET | `/v5/account/wallet-balance` | (account tier — pace conservatively) |

Source: https://bybit-exchange.github.io/docs/v5/rate-limit

---

## TradeAutopsy endpoints

### Execution list (fills)

```
GET /v5/execution/list
Auth: signed GET
```

Query (spot book):

| Param | Required | Notes |
|-------|----------|-------|
| `category` | yes | `spot` |
| `symbol` | optional | e.g. `BTCUSDT` |
| `startTime` / `endTime` | optional | ms; **7-day window rules** (see below) |
| `limit` | optional | 1–100, default 50 |
| `cursor` | optional | pagination |

Time rules (official execution doc):

- Neither time → last **7 days**
- Only `startTime` → `[startTime, startTime + 7d]`
- Only `endTime` → `[endTime - 7d, endTime]`
- Both → `endTime - startTime ≤ 7 days`

List item fields (mapping): `execId`, `symbol`, `side` (`Buy`/`Sell`), `execQty`, `execPrice`, `execTime` (ms string), `execFee`, `feeCurrency`, `orderId`.

Sorted by `execTime` descending.

### Wallet balance (bootstrap)

```
GET /v5/account/wallet-balance?accountType=UNIFIED
Auth: signed GET
```

Use `result.list[].coin[]` — `coin`, `walletBalance`, `locked`.

### Instruments (filters)

```
GET /v5/market/instruments-info?category=spot&symbol=BTCUSDT
Auth: public
```

Persist per symbol:

| Filter | Fields |
|--------|--------|
| `lotSizeFilter` | `minOrderQty`, `maxOrderQty` / `maxLimitOrderQty`, `minOrderAmt` |
| `priceFilter` | **`tickSize`** |

Do **not** use `basePrecision` / `quotePrecision` as tick.

Spot: pagination `limit`/`cursor` **not supported** on instruments-info (all symbols in one response category).

### Klines (history obtain)

```
GET /v5/market/kline?category=spot&symbol=BTCUSDT&interval=1&limit=200
Auth: public
```

Intervals: `1`, `3`, `5`, `15`, `30`, `60`, `120`, `240`, `360`, `720`, `D`, `W`, `M`.

---

## Quote-filter (Station desk)

Book `bybit-com-spot` / profile `crypto_spot_usd`: accept spot symbols ending in **USDT**, **USDC**, or **USD** only for v1 dogfood. Other quotes → refuse or honesty flag.

---

## Oracle cross-check

NautilusTrader pin v2.0.0rc5 (`docs/research/nautilus-trader-citation.md`): prod host `api.bybit.com`; leads `GET /v5/execution/list`, `/v5/account/wallet-balance`, `/v5/market/instruments-info`. Cite only — not a dependency.

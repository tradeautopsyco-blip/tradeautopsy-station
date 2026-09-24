# Kraken — Spot REST API

**Exchange:** Kraken spot (`api.kraken.com`) — **not** Kraken Futures (`futures.kraken.com`)  
**Source:** [Kraken REST docs](https://docs.kraken.com/rest/) · [Authentication guide](https://docs.kraken.com/exchange/guides/rest/authentication) · OpenAPI `spot-rest.yaml`  
**Snapshot date:** 2026-09-24 IST  
**Book:** `kraken-com-spot` · B6 `/Users/bishnu/issues/brokers/sheets/kraken.md` · ADR 0017

---

## Base endpoint

- `https://api.kraken.com`
- Path prefix: `/0/public/*` (unsigned GET) and `/0/private/*` (signed POST)

**Refused on book `kraken-com-spot`:** `futures.kraken.com`, `demo-futures.kraken.com`

---

## Authentication (private POST)

| Header / field | Value |
|----------------|--------|
| `API-Key` | Public API key |
| `API-Sign` | Base64(HMAC-SHA512(base64_decode(secret), path + SHA256(nonce + POST body))) |
| `Content-Type` | `application/x-www-form-urlencoded` |
| `nonce` | Required in POST body; always-increasing unsigned 64-bit integer |

POST body shape: `nonce=<n>&key=value&…` (nonce must be first parameter in the signed string).

Station attaches signing in `agent/src/ubi/kraken_session.rs` (host only; Wasm never sees secret).

---

## TradeAutopsy-relevant endpoints

### Trade history (fills lead)

```
POST /0/private/TradesHistory
Security: private (signed POST)
```

Body parameters (optional unless noted):

- `nonce` (required) — in body, host-generated
- `type` — `all`, `any`, `closed`, `limit`, `stop`, `trade`, `margin`
- `trades` — bool, include order txids in response
- `start`, `end` — unix **seconds**
- `ofs` — result offset for pagination

Response: `{ "error": [], "result": { "trades": { "<id>": { … } }, "count": N } } }`

Trade fields used for FillEvent mapping: `pair`, `type` (buy/sell), `vol`, `price`, `fee`, `time` (seconds float), `trade_id`, `ordertxid`.

### Account balance (bootstrap)

```
POST /0/private/Balance
Security: private (signed POST)
```

### Public market data (later obtain)

```
GET /0/public/AssetPairs
GET /0/public/OHLC
GET /0/public/Ticker
GET /0/public/Depth
GET /0/public/Time
```

---

## Desk quote-filter (B6 row 7)

USD desk v1 (`crypto_spot_usd`): allow pair altnames ending in **USDT**, **USDC**, **USD**, or **EUR** only. Refuse BTC/XBT-quoted pairs on this book without an explicit lock amendment.

---

## Rate limits

Spot REST uses a decay counter (dynamic). Errors return in JSON `error[]` (e.g. rate limit, invalid nonce). Station treats invalid nonce / permission errors as reconnect candidates at dogfood — not Kill.

# Binance Global — Futures USDⓈ-M WebSocket

**Exchange:** Binance Global  
**Product:** Futures USDⓈ-M  
**Source:** `Connect.md`, `Important-WebSocket-Change-Notice.md`, `Live-Subscribing-Unsubscribing-to-streams.md`, `user-data-streams.md`, `websocket-api-general-info.md`  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## ⚠️ Breaking URL Change (effective 2026-04-23)

Legacy `wss://fstream.binance.com/ws` and `/stream` permanently decommissioned.

**New base URL structure:**
```
wss://fstream.binance.com/public    ← high-frequency public data (order book, book ticker)
wss://fstream.binance.com/market    ← regular market data (aggTrade, markPrice, klines, ticker)
wss://fstream.binance.com/private   ← user data (listenKey-based)
```

**Access modes:**
- `ws` mode: `/ws/<streamName>`  
  Example: `wss://fstream.binance.com/market/ws/btcusdt@aggTrade`
- `stream` mode: `/stream?streams=<name1>/<name2>`  
  Example: `wss://fstream.binance.com/market/stream?streams=bnbusdt@aggTrade/btcusdt@markPrice`

**Private (user data):**
- `wss://fstream.binance.com/private/ws?listenKey=<key>&events=ORDER_TRADE_UPDATE/ACCOUNT_UPDATE`
- `wss://fstream.binance.com/private/stream?listenKey=<key1>&events=ORDER_TRADE_UPDATE&listenKey=<key2>&events=ACCOUNT_UPDATE`

**Stream routing by category:**

| Endpoint | Streams |
|---|---|
| `/public` | `<symbol>@bookTicker`, `!bookTicker`, `<symbol>@depth<levels>`, `<symbol>@depth` |
| `/market` | `<symbol>@aggTrade`, `<symbol>@markPrice`, `<symbol>@kline_<interval>`, `<symbol>@ticker`, `<symbol>@forceOrder`, etc. |
| `/private` | listenKey-based user data |

> Connections to legacy unrouted path (`/ws/`) now only receive `/public` data. `/market` and `/private` streams silently stop pushing.

---

## Connection Lifecycle

- Single connection valid 24 hours
- Server ping every 3 minutes. If no pong within 10 minutes → disconnected
- Unsolicited pong allowed (keeps connection alive if sent within 15 min intervals)
- Max 10 incoming messages/sec
- Max 1024 streams per connection
- Combined stream events: `{"stream":"<name>","data":<payload>}`

---

## Live Subscribe/Unsubscribe

```json
{ "method": "SUBSCRIBE", "params": ["btcusdt@aggTrade", "btcusdt@depth"], "id": 1 }
{ "method": "UNSUBSCRIBE", "params": ["btcusdt@depth"], "id": 312 }
{ "method": "LIST_SUBSCRIPTIONS", "id": 3 }
{ "method": "SET_PROPERTY", "params": ["combined", true], "id": 5 }
```

---

## WebSocket API

**Base:** `wss://ws-fapi.binance.com/ws-fapi/v1`  
**Testnet:** `wss://testnet.binancefuture.com/ws-fapi/v1` (updated 2024-01-24)

Rate limits shared with REST API. WS handshake = 5 weight.

Signature payload: all request params except signature, sorted alphabetically, joined with `&`.

Auth: `session.logon` / `session.status` / `session.logout` — Ed25519 only.

---

## User Data Stream

**REST base:** `https://fapi.binance.com`  
**WS base:** `wss://fstream.binance.com/private`  
**Access:** `/ws/<listenKey>`

listenKey validity: 60 minutes. `PUT` to extend, `DELETE` to close.

**Message ordering guarantee:** Same event type on single connection: strictly ordered by both `T` (matching engine time) and `E` (event generation time). Use `E` for cross-event-type ordering.

**Events:**

### MARGIN_CALL
Pushed when position risk ratio too high. Risk guidance only.

### ACCOUNT_UPDATE
Balance or position changed. Includes reason type `m`:
`DEPOSIT`, `WITHDRAW`, `ORDER`, `FUNDING_FEE`, `WITHDRAW_REJECT`, `ADJUSTMENT`, `INSURANCE_CLEAR`, `ADMIN_DEPOSIT`, `ADMIN_WITHDRAW`, `MARGIN_TRANSFER`, `MARGIN_TYPE_CHANGE`, `ASSET_TRANSFER`, `OPTIONS_PREMIUM_FEE`, `OPTIONS_SETTLE_PROFIT`, `AUTO_EXCHANGE`, `COIN_SWAP_DEPOSIT`, `COIN_SWAP_WITHDRAW`

`bc` = balance change excluding PnL and commission.

FUNDING FEE in crossed position: pushes only balance `B`, no position `P`.  
FUNDING FEE in isolated position: pushes balance `B` + relative position `P`.

### ORDER_TRADE_UPDATE
New order or status change.

Execution types: `NEW`, `CANCELED`, `CALCULATED` (liquidation), `EXPIRED`, `TRADE`, `AMENDMENT`

Liquidation: `c` = "autoclose-XXX", `X` = "NEW"  
ADL: `c` = "adl_autoclose", `X` = "NEW"

Expiry reasons (field `er`): 0=None, 1=STP, 2=IOC partial, 3=IOC STP partial, 4=knocked out by RO, 5=account liquidated, 6=GTE unsatisfied, 7=symbol delisted, 8=initial expired after stop trigger, 9=MARKET partial fill

### ACCOUNT_CONFIG_UPDATE
Leverage or multi-assets margin mode change. Contains `ac` (symbol config) or `ai` (account config).

### TRADE_LITE
Fast trade stream. Only TRADE execution type. Fewer fields, lower latency than `ORDER_TRADE_UPDATE`.

### CONDITIONAL_ORDER_TRIGGER_REJECT
Deprecated effective 2025-12-15. Rejection reasons now in `ALGO_UPDATE` event.

### STRATEGY_UPDATE / GRID_UPDATE (GRID_UPDATE deprecated)
Strategy create/cancel/expire updates.

### ALGO_UPDATE
Algo order status change.

Algo statuses: `NEW`, `CANCELED`, `TRIGGERING`, `TRIGGERED`, `FINISHED`, `REJECTED`, `EXPIRED`

### listenKeyExpired
listenKey expired. No more events until new listenKey used.

---

## Order Book Reconstruction

1. Open stream: `wss://fstream.binance.com/public/stream?streams=btcusdt@depth`
2. Buffer events. For same price, latest update wins.
3. GET snapshot: `https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=1000`
4. Drop events where `u < snapshot.lastUpdateId`
5. First valid event: `U <= lastUpdateId AND u >= lastUpdateId`
6. Each new event: `pu` must equal previous event's `u`, else restart from step 3
7. Data is **absolute quantity** per price level. Quantity = 0 → remove level.

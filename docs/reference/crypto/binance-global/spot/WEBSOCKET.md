# Binance Global — Spot WebSocket

**Exchange:** Binance Global  
**Source:** `web-socket-streams.md`, `web-socket-api.md`, `user-data-stream.md`, `sbe-market-data-streams.md`  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Market Streams

**Base endpoints:**
- `wss://stream.binance.com:9443` or `:443`
- Market-data-only (no user stream): `wss://data-stream.binance.vision`

**Access modes:**
- Raw stream: `/ws/<streamName>`
- Combined: `/stream?streams=<name1>/<name2>` → wrapped as `{"stream":"<name>","data":<payload>}`

**All symbol names lowercase.**

**Connection lifetime:** 24 hours max. `serverShutdown` event sent before disconnect — reconnect immediately.

**Ping/pong:** Server sends ping every 20s. Must pong within 60s. Send pong with copy of ping payload. Unsolicited pong allowed (payload must be empty).

**WebSocket limits:**
- 5 incoming messages/sec (PING, PONG, JSON control)
- Max 1024 streams per connection
- 300 connection attempts per 5min per IP

**Microsecond timestamps:** add `timeUnit=MICROSECOND` to URL param (default is milliseconds).

---

## Live Subscribe/Unsubscribe

```json
{ "method": "SUBSCRIBE", "params": ["btcusdt@aggTrade"], "id": 1 }
{ "method": "UNSUBSCRIBE", "params": ["btcusdt@depth"], "id": 312 }
{ "method": "LIST_SUBSCRIPTIONS", "id": 3 }
{ "method": "SET_PROPERTY", "params": ["combined", true], "id": 5 }
{ "method": "GET_PROPERTY", "params": ["combined"], "id": 2 }
```
`null` result = success for non-query requests.

---

## WebSocket API

**Base:** `wss://ws-api.binance.com:443/ws-api/v3` (alt: `:9443`)  
**Testnet:** `wss://ws-api.testnet.binance.vision/ws-api/v3`

Request format (JSON text frame, one per frame):
```json
{ "id": "<uuid or int>", "method": "order.place", "params": { ... } }
```

Response format:
```json
{ "id": "<same>", "status": 200, "result": { ... }, "rateLimits": [ ... ] }
{ "id": "<same>", "status": 400, "error": { "code": -1102, "msg": "..." }, "rateLimits": [ ... ] }
```

**Rate limits:** shared with REST API. WS handshake = 5 weight. Ping/pong limit: 5/sec.

**Auth methods:** HMAC-SHA256, RSA, Ed25519.

**Session auth:**
- `session.logon` — authenticate connection (Ed25519 only for WS API session auth)
- `session.status` — query current auth
- `session.logout` — forget key

---

## User Data Stream

**Subscribe via WebSocket API** with API Key.

**Events:**

### outboundAccountPosition
Pushed when account balance changes. Contains changed assets only.

### balanceUpdate
Pushed on deposits, withdrawals, or inter-account transfers.

### executionReport (Order Update)
Pushed on every order state change.

Key conditional fields (appear only when applicable):
- `d` / `D` — trailing delta / trailing time (trailing stop orders)
- `j` / `J` — strategyId / strategyType
- `v` / `A` / `B` / `u` / `U` / `Cs` / `pl` / `pL` / `pY` — STP-related fields
- `W` — working time
- `b` / `a` / `k` / `uS` — SOR allocation fields
- `gP` / `gOT` / `gOV` / `gp` — pegged order fields

Average price = `Z` / `z`.

**Order reject reasons:**
- `INSUFFICIENT_BALANCES`
- `STOP_PRICE_WOULD_TRIGGER_IMMEDIATELY`
- `WOULD_MATCH_IMMEDIATELY`
- `OCO_BAD_PRICES`

### listStatus
Pushed alongside executionReport for order list orders.

### eventStreamTerminated
Pushed on listenKey expiry, session.logout, or userDataStream.unsubscribe.

### externalLockUpdate
Pushed when spot wallet balance locked/unlocked by external system (e.g. margin collateral).

---

## SBE Market Data Streams

**Base:** `wss://stream-sbe.binance.com` or `:9443`

**Auth:** Ed25519 API key required in `X-MBX-APIKEY` header. No signature needed for public data.

**Encoding:** Binary frames for market data. JSON text frames for subscriptions and `serverShutdown`.

**Connection lifetime:** 24 hours. Server ping every 20s, pong within 60s.

**WebSocket limits:** 5 requests/sec, max 1024 streams, 300 connection attempts per 5min per IP.

**Available streams:**
- `<symbol>@trade` — raw trades, real-time
- `<symbol>@bestBidAsk` — best bid/ask with **auto-culling** (outdated events dropped under load)
- `<symbol>@depth` — incremental order book updates, 25ms
- `<symbol>@depth20` — top 20 levels snapshot, 50ms

---

## Order Book Reconstruction (WebSocket Market Streams)

1. Open stream to `wss://stream.binance.com:9443/ws/<symbol>@depth`
2. Buffer events, note `U` of first event
3. GET depth snapshot from `https://api.binance.com/api/v3/depth?symbol=<SYMBOL>&limit=5000`
4. If snapshot `lastUpdateId` < buffered event `U` → go to step 3
5. Discard buffered events where `u <= lastUpdateId`
6. First valid event: `lastUpdateId` in `[U, u]` range
7. Set local book to snapshot, apply buffered events, then continue

**Update procedure per event:**
- If `u < local book update ID` → ignore
- If `U > local book update ID + 1` → missed events, restart
- Normally: `U` of next event = `u + 1` of previous
- For each bid/ask price level: set new quantity. Quantity = 0 → remove level.

> Note: Snapshot limit is 5000 levels per side. Levels outside snapshot range only become visible when they change.

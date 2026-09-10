# Binance Global — Options REST API (European)

**Exchange:** Binance Global  
**Product:** European Options (`eapi.binance.com`)  
**Source:** `schema__5_.yaml` (5,957 lines), changelog in session  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

Schema title: "Options REST API — Access market data, manage accounts, and trade Binance Options."

---

## Base Endpoint

`https://eapi.binance.com`

---

## WebSocket Base

`wss://nbstream.binance.com/eoptions`

Ping every 3 minutes. Pong timeout: 10 minutes.

---

## Slice 2 — chain snapshot + open interest

Station may rebuild a **BoundedSnapshot** chain from `GET /eapi/v1/exchangeInfo` `optionSymbols[]` (same (`underlying`, `expiryDate`) as the declared mixed-case contract). Last overlay from TickBook `lastPrice` remains optional. Open interest is **LatestState** from `GET /eapi/v1/openInterest?underlyingAsset=&expiration=` → `sumOpenInterest` (string).

There is **no** `/eapi/v1/optionChain`. Do not stuff ticker rows as a chain. `/eapi/v1/depth` and `/eapi/v1/mark` were fenced in slice 2 and are locked in **Slice 3** below.

Primary sources (accessed 2026-08-29):

- Official CLI exchange-information / open-interest
- Official OpenAPI-generated SDK `OpenInterestResponseInner` (`symbol`, `sumOpenInterest`, `sumOpenInterestUsd`, `timestamp`)
- Live public `GET https://eapi.binance.com/eapi/v1/exchangeInfo` and `GET /eapi/v1/openInterest?underlyingAsset=BTC&expiration=260925` (field names only)

`GET /eapi/v1/userTrades` is **Slice 4** (not Slice 2). PnL owner remains **none**. `POST /eapi/v1/order` is a mutation.


| Item | Lock |
|------|------|
| Host | `eapi.binance.com` (`https://eapi.binance.com`) |
| Method + path | `GET /eapi/v1/ticker` — 24hr ticker price change statistics |
| Last field | `lastPrice` (string) |
| Symbol | hyphenated contract id, e.g. `BTC-200730-9000-C`. **Do not** `to_ascii_lowercase` (not a spot `BTCUSDT` pair). |
| Auth | public (`security: []`) |
| Response | JSON array of objects (`symbol`, `lastPrice`, …). Optional `?symbol=` filters one contract. |

Primary sources (accessed 2026-08-28):

- Binance developers catalog: `GET /eapi/v1/ticker` — [24hr Ticker Price Change Statistics](https://developers.binance.com/docs/derivatives/options-trading/market-data/24hr-Ticker-Price-Change-Statistics)
- Binance CLI example: `binance-cli derivatives-options ticker24hr-price-change-statistics --symbol BTC-200730-9000-C`
- OpenAPI (`binance-european-options-openapi.yml`): path `/eapi/v1/ticker` GET, property `lastPrice` type string

WS last on `<symbol>@ticker`: stream exists (`T` = transaction time). The **last price field on the WS payload is NOT SPECIFIED IN SOURCE in this lock** — do not map spot `@trade` `p` or invent `c`. Slice 1 last is REST `lastPrice`.

No default subscribe symbol in this lock. Do not auto-dial a contract unless an env/lock names one later.

`GET /eapi/v1/userTrades` is **Slice 4** (not Slice 2). PnL owner remains **none**. `POST /eapi/v1/order` is a mutation.

---

## Slice 3 — venue-published greeks (`/eapi/v1/mark`) + order book (`/eapi/v1/depth`)

Both endpoints are **public** (`Security: None`) on the official Market Data page. Station
**copies** what the venue published on `mark`; it does not price. That is the
`GreeksSource::VenuePublished` half of the two-books law — the NFO half stays
`ModelComputed` and is governed by [`OPTIONS-PRICING.md`](../../../india/nfo/OPTIONS-PRICING.md),
not by this file.

Primary sources (accessed **2026-08-31**):

- Official docs: [Option Mark Price](https://developers.binance.com/docs/derivatives/options-trading/market-data/Option-Mark-Price) — method, path, weight, security, verbatim field table + response example.
- Live public `GET https://eapi.binance.com/eapi/v1/mark?symbol=BTC-260925-100000-C` → HTTP 200, response header `x-mbx-used-weight-1m: 5` (weight confirmed empirically).
- Live public `GET https://eapi.binance.com/eapi/v1/mark` (no `symbol`) → JSON array of **1736** elements.

### `GET /eapi/v1/mark`

| Item | Lock |
|------|------|
| Host | `eapi.binance.com` |
| Method + path | `GET /eapi/v1/mark` |
| Auth | **public** (`Security: None`) |
| Weight | **5 IP** (docs; confirmed by `x-mbx-used-weight-1m: 5`) |
| Request param | `symbol` — **optional per docs, required by this lock** (see below) |
| Response | JSON **array** of objects |

Response fields, verbatim from the official table. **Every value is a `string`.**

| Field | Type | Official description |
|-------|------|----------------------|
| `symbol` | string | symbol |
| `markPrice` | string | Mark price |
| `bidIV` | string | Implied volatility Buy |
| `askIV` | string | Implied volatility Sell |
| `markIV` | string | Implied volatility mark |
| `delta` | string | delta |
| `theta` | string | theta |
| `gamma` | string | gamma |
| `vega` | string | vega |
| `highPriceLimit` | string | Current highest buy price |
| `lowPriceLimit` | string | Current lowest sell price |
| `riskFreeInterest` | string | risk free rate |

Official response example (verbatim):

```json
[ { "symbol": "BTC-200730-9000-C", "markPrice": "1343.2883", "bidIV": "1.40000077", "askIV": "1.50000153", "markIV": "1.45000000", "delta": "0.55937056", "theta": "3739.82509871", "gamma": "0.00010969", "vega": "978.58874732", "highPriceLimit": "1618.241", "lowPriceLimit": "1068.3356", "riskFreeInterest": "0.1" } ]
```

**Four greeks, not five.** Binance publishes `delta` / `theta` / `gamma` / `vega`. There is
**no `rho`** on this payload. A rho chip on this book is NOT SPECIFIED IN SOURCE — leave it
dark; do not compute one to fill the grid.

**`bidIV` can be `-1.0`.** Observed live 2026-08-31 on `BTC-260925-145000-C`:
`"bidIV":"-1.0"` with `"askIV":"0.74930251"`. A negative implied volatility is not a
volatility — it is a no-bid sentinel, and the official field table does not mention it.
**Refuse any IV `<= 0` rather than rendering it.** This is the one place this endpoint will
hand you a number that looks real and is not.

**Units and model are NOT SPECIFIED IN SOURCE.** The field table says "delta", "theta",
"vega" with no per-day / per-1%-vol convention, no day-count, and no named pricing model.
Station therefore copies these as **the strings the venue published**, attributes them
`VenuePublished { path: "/eapi/v1/mark", adapter: "binance_com" }`, and does **not**
rescale, re-derive, or relabel them. Do not assume they share vollib's conventions.
Do not carry a Binance convention onto NFO.

**`riskFreeInterest` is the venue's rate, for this book only.** Observed `0.0444` / `0.0454`
(2026-08-31). It describes Binance's own USDT-settled pricing. It is **not** an Indian
risk-free rate and must never be reused on the NFO book.

**Always send `symbol`.** The parameter is optional per the docs, but the unfiltered call
returned a 1736-element array in one response at weight 5. Station dials one contract.

### `GET /eapi/v1/depth`

| Item | Lock |
|------|------|
| Method + path | `GET /eapi/v1/depth` |
| Auth | **public** (`Security: None`) |
| Weight | **1** (limit 5/10/20/50) · **5** (100) · **10** (500) · **20** (1000) — IP |
| Request params | `symbol` (**mandatory**), `limit` (optional, max 1000) |

| Field | Type | Official description |
|-------|------|----------------------|
| `bids` | array[] | Bid orders. Each entry is `[price, quantity]`. |
| `asks` | array[] | Ask orders. Each entry is `[price, quantity]`. |
| `T` | integer (int64) | transaction time |
| `lastUpdateId` | integer (int64) | update id |

```json
{ "bids": [ [ "1000.000", "0.1000" ] ], "asks": [ [ "1900.000", "0.1000" ] ], "T": 1762780909676, "lastUpdateId": 361 }
```

**Levels are 2-element arrays, not objects.** Kotak's cash ladder uses
`{"price":…,"quantity":…,"orders":…}`; this one is positional `[price, quantity]` with **no
order count**. The Kotak depth parser must not be pointed at this body. An `orders` field on
an eapi level is NOT SPECIFIED IN SOURCE.

Identity is `market/order_book/bounded_snapshot`, the same as cash depth — it is a bounded
snapshot with a named `lastUpdateId`, not an ordered state.

`GET /eapi/v1/userTrades` is **Slice 4** (not Slice 3). PnL owner remains **none**. `POST /eapi/v1/order` is a mutation.

---

## Key Endpoints (from changelog)

```
GET    /eapi/v1/ticker              24hr ticker (lastPrice) — slice 0 last lock
GET    /eapi/v1/exchangeInfo        Exchange info, symbols, filters, rate limits
GET    /eapi/v1/account             Account info (includes riskLevel field)
GET    /eapi/v1/marginAccount       Margin account info (includes riskLevel)
POST   /eapi/v1/order               Place order
PUT    /eapi/v1/order               Modify order
DELETE /eapi/v1/order               Cancel order
POST   /eapi/v1/batchOrders         Place batch orders
PUT    /eapi/v1/batchOrders         Modify batch orders
DELETE /eapi/v1/batchOrders         Cancel batch orders
GET    /eapi/v1/order               Query order status
GET    /eapi/v1/openOrders          Open orders
GET    /eapi/v1/historyOrders       Historical orders (last 5 days)
GET    /eapi/v1/userTrades          Trade history
GET    /eapi/v1/depth               Order book (includes updateId field `u`)
GET    /eapi/v1/openInterest        Open interest by underlying + expiration
GET    /eapi/v1/exerciseHistory     Exercise history
GET    /eapi/v1/commission          User commission rate (added 2026-01-07)
GET    /eapi/v1/income/asyn         Download ID for transaction history
GET    /eapi/v1/income/asyn/id      Transaction history download link
```

### Auto-Cancel (Kill Switch)
```
POST   /eapi/v1/countdownCancelAll            Set auto-cancel config
GET    /eapi/v1/countdownCancelAll            Get auto-cancel config
POST   /eapi/v1/countdownCancelAllHeartBeat   Heartbeat to reset countdown
```

### Block Trades (from 2024-11-01)
```
POST   /eapi/v1/block/order/create
PUT    /eapi/v1/block/order/create
DELETE /eapi/v1/block/order/create
GET    /eapi/v1/block/order/orders
POST   /eapi/v1/block/order/execute
GET    /eapi/v1/block/order/execute
GET    /eapi/v1/block/user-trades
GET    /eapi/v1/blockTrades                   Recent block trades (from 2024-12-17)
```

---

## STP (from 2026-03-11)

Modes: `EXPIRE_MAKER`, `EXPIRE_TAKER`, `EXPIRE_BOTH`  
New order status: `EXPIRED_IN_MATCH`  
Field `selfTradePreventionMode` on all order endpoints.  
WebSocket field `V` in `ORDER_TRADE_UPDATE`.

---

## WebSocket Streams

```
<symbol>@trade               Trades
<underlyingAsset>@trade      All trades by underlying
<symbol>@depth<levels>       Order book depth (includes u and pu fields from 2022-12-13)
<symbol>@ticker              Ticker (includes T: transaction time)
<underlyingAsset>@ticker@<expirationDate>  Ticker by expiry
<underlyingAsset>@markPrice  Mark price
<underlyingAsset>@openInterest@<expirationDate>  Open interest
option_pair                  Option pair stream
```

## User Data Stream Events

```
ORDER_TRADE_UPDATE    Order state change (field V = STP mode from 2026-03-11)
RISK_LEVEL_CHANGE     Account risk level change (from 2023-08-29)
```

---

## TradeAutopsy Relevance

Slice 2: public last + `optionSymbols` chain snapshot + `openInterest` `sumOpenInterest` on `binance-com-options`. Spot BTCUSDT last stays on `binance-com-spot`. No `/eapi/v1/optionChain`, no eapi orders.

Slice 4: **private tradebook** on `binance-com-options` — `GET /eapi/v1/userTrades?symbol=`
(PrivateRead, HMAC) → `id` / `symbol` / `price` / `qty` / `side` / `time` only. Host fetch
plants AccountBook fills on the options book; **not** merge_poll. PnL owner remains **none**.

Slice 3: **venue-published greeks** on `binance-com-options` — `GET /eapi/v1/mark?symbol=`
(public, weight 5 IP) → `delta` / `gamma` / `theta` / `vega` copied as the venue's own
strings, served on `GET /api/station/greeks` and `obtain operation=optiongreeks`. The
`optiongreeks` binding is the **only** binding carrying `display: true`; every other one
stays `research_fetch_only`. Four greeks, no rho. No IV `<= 0`. `/eapi/v1/depth` is
allowlisted by the same lock but its parser is **not** this slice.

#70 / session series: public `GET /eapi/v1/klines?symbol=<mixed-case>&interval=<ENUM>`
on `eapi.binance.com` (weight 1, no HMAC). Official HTML Response Example names object
keys `open` / `high` / `low` / `close` / `volume` / `amount` / `interval` / `tradeCount`
/ `takerVolume` / `takerAmount` / `openTime` / `closeTime`. Live GET returns a 12-slot
array; the lock maps those names onto the live slots. Obtain noun `history` on
`binance-com-options`. Empty `[]` is unavailable, not a zero candle. Interval not on
the ENUM is unsupported — Station does **not** resample (`OHLCV-RESAMPLE.md`). Never
`/api/v3/klines` on a dated contract.

Options positions as behavioral risk remain a later slice (see MECHANICS.md).

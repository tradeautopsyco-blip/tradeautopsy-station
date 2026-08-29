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

There is **no** `/eapi/v1/optionChain`. Do not stuff ticker rows as a chain. Do not dial `/eapi/v1/depth` or `/eapi/v1/mark` this slice.

Primary sources (accessed 2026-08-29):

- Official CLI exchange-information / open-interest
- Official OpenAPI-generated SDK `OpenInterestResponseInner` (`symbol`, `sumOpenInterest`, `sumOpenInterestUsd`, `timestamp`)
- Live public `GET https://eapi.binance.com/eapi/v1/exchangeInfo` and `GET /eapi/v1/openInterest?underlyingAsset=BTC&expiration=260925` (field names only)

`GET /eapi/v1/userTrades` stays off this slice. `POST /eapi/v1/order` is a mutation.


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

`GET /eapi/v1/userTrades` stays off this slice. `POST /eapi/v1/order` is a mutation.

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

Slice 2: public last + `optionSymbols` chain snapshot + `openInterest` `sumOpenInterest` on `binance-com-options`. Spot BTCUSDT last stays on `binance-com-spot`. No `/eapi/v1/optionChain`, no userTrades, no eapi orders.

Options positions as behavioral risk remain a later slice (see MECHANICS.md).

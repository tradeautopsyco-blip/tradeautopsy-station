# Binance Global — Futures USDⓈ-M Changelog Notes

**Exchange:** Binance Global  
**Source:** Combined changelog (document index 19 in session)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

> Only changes that affect TradeAutopsy assumptions. Not a full dump.

---

## 2026-06-30 — COIN-M/USDⓈ-M Architecture Integration (complete)

Cross-reference: `futures-coinm/CHANGELOG-NOTES.md`

- `POST /fapi/v1/positionSide/dual` now auto-syncs CM `dualSidePosition`. If CM has open positions/orders, UM mode change rejected with `-4531`.

## 2026-04-23 — Legacy WS URLs decommissioned

`wss://fstream.binance.com/ws` and `/stream` no longer serve `/market` or `/private` streams. See `WEBSOCKET.md`.

## 2026-04-06 — forceOrders query window

`GET /fapi/v1/forceOrders` — only last 90 days queryable.

## 2026-03-19 — historicalTrades window shrunk

`GET /fapi/v1/historicalTrades` — data availability reduced from 3 months to **1 month**.

## 2025-12-09 — Conditional orders migrated to Algo Service

`STOP_MARKET`, `TAKE_PROFIT_MARKET`, `STOP`, `TAKE_PROFIT`, `TRAILING_STOP_MARKET` via `/fapi/v1/order` now return `-4120 STOP_ORDER_SWITCH_ALGO`. Use `/fapi/v1/algoOrder` instead.

## 2025-12-15 — CONDITIONAL_ORDER_TRIGGER_REJECT deprecated

Rejection reasons now pushed in `ALGO_UPDATE` event.

## 2025-10-23 — OPPONENT_10 / OPPONENT_20 priceMatch temporarily removed

Affects place/amend on `/fapi/v1/order`, `/fapi/v1/batchOrders`, `/fapi/v1/order` (PUT).

## 2025-07-02 — WS stream max connections increased

Single connection max streams: 200 → **1024**.

## 2024-10-30 — userTrades window

`GET /fapi/v1/userTrades` — only last 6 months queryable.

## 2024-10-17 — aggTrades window

`GET /fapi/v1/aggTrades` — supports querying up to 1 year ago.

## 2024-10-16 — positionMargin/history window

`GET /fapi/v1/positionMargin/history` — supports querying up to 30 days ago.

## 2024-09-03 — TRADE_LITE event added

Fast user data stream with lower latency, TRADE execution type only.

## 2024-04-09 — GTC 1-year validity

GTC orders auto-canceled after 1 year. Applies to all order types including reduceOnly, except part-filled or strategy/copy-trade orders.

## 2024-01-11 — Self-Trade Prevention released

STP modes: `NONE`, `EXPIRE_TAKER`, `EXPIRE_BOTH`, `EXPIRE_MAKER`.

## 2023-07-27 — allOrders / order query window

`GET /fapi/v1/order` and `GET /fapi/v1/allOrders` — only last 3 months queryable.

## 2023-05-05 — Order modification added

`PUT /fapi/v1/order` and `PUT /fapi/v1/batchOrders` for limit order modify. `GET /fapi/v1/orderAmendment` for history.

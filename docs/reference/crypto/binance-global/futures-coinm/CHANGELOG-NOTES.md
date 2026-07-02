# Binance Global — Futures COIN-M Changelog Notes

**Exchange:** Binance Global  
**Product:** Futures COIN-M (`dapi.binance.com`)  
**Source:** Combined changelog (document index 19 in session)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

> Only changes that affect TradeAutopsy assumptions.

---

## ⚠️ CRITICAL — COIN-M / USDⓈ-M Architecture Integration

**Announced:** 2026-06-10  
**Effective:** Progressively from 2026-06-24, fully by 2026-06-30

COIN-M Futures is being integrated into the USDⓈ-M unified architecture. Account-level behaviors, endpoints, and streams now shared.

**Breaking changes:**

### Account level
- `dualSidePosition` unified across UM and CM. Changing one syncs the other. If CM has open positions/orders when UM mode changes → rejected with `-4531`.
- `POST /dapi/v1/positionSide/dual` and `POST /papi/v1/cm/positionSide/dual`: CM dualSidePosition must match UM. Changing to same value rejected.

### Deprecated endpoint
- `GET /dapi/v1/pmAccountInfo` — no longer in use. Use `GET /fapi/v1/pmAccountInfo`.

### Stream changes
- `<pair>@indexPrice`: field renamed from `"i"` to `"s"` in response payload
- `<pair>@indexPrice@1s` (1000ms variant) **removed**. Only `<pair>@indexPrice` available now, at 1000ms speed.

### countdownCancelAll suspended
- `POST /dapi/v1/countdownCancelAll` suspended 2026-06-29 09:00 UTC for CM migration maintenance. Will be restored after CM resumes.

Cross-reference: `futures-usdm/CHANGELOG-NOTES.md`

---

## 2026-04-10 — dualSidePosition consistency enforced

CM `dualSidePosition` must stay consistent with UM going forward.

## 2026-04-06 — forceOrders query window

`GET /dapi/v1/forceOrders` — only last 90 days queryable.

## 2026-03-19 — historicalTrades window shrunk

`GET /dapi/v1/historicalTrades` — 3 months → **1 month**.

## 2025-02-25 — WebSocket API launched

`wss://ws-dapi.binance.com/ws-dapi/v1` — placing/canceling orders over WS. Same filters and rate limits as REST.

## 2024-10-30 — userTrades window

`GET /dapi/v1/userTrades` — only last 6 months queryable.

## 2024-10-11 — STP and Price Match added

STP modes: `NONE`, `EXPIRE_TAKER`, `EXPIRE_BOTH`, `EXPIRE_MAKER`  
Price match modes: `NONE` through `QUEUE_20`

## 2024-10-08 — allOrders / userTrades query constraints

`GET /dapi/v1/allOrders` and `GET /dapi/v1/userTrades`: most recent 7 days by default, query period < 7 days.  
`GET /dapi/v1/order` and `GET /dapi/v1/allOrders`: only last 3 months.

## 2024-04-09 — GTC 1-year validity

GTC orders auto-canceled after 1 year.

## 2023-08-31 — Ping frequency changed

Server ping: 5 min → **3 min**. Pong timeout: 15 min → **10 min**.

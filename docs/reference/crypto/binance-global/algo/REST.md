# Binance Global — Algo Trading REST API

**Exchange:** Binance Global  
**Source:** `schema__9_.yaml` (1,806 lines), changelog in session  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

Schema title: "Algo Trading REST API — Programmatic access to Binance's execution algorithms for creating and managing Spot and Futures algo orders."

Rate limits guide: `products/algo/general-info#limits`  
Security guide: `products/algo/general-info#request-security`

---

## What Algo Orders Are

Institutional-grade execution algorithms (TWAP, VP) for executing large or illiquid orders. Not the same as conditional stop/take-profit orders. Separate product, separate endpoints.

Contact: `trading@binance.com`

---

## Endpoint Prefixes

| Scope | Prefix |
|---|---|
| Spot Algo | `/sapi/v1/algo/spot/` |
| Futures Algo | `/sapi/v1/algo/futures/` |

---

## Spot Algo Endpoints (from 2023-04-18)

```
POST   /sapi/v1/algo/spot/newOrderTwap      Place TWAP order
DELETE /sapi/v1/algo/spot/order             Cancel algo order
GET    /sapi/v1/algo/spot/openOrders        Open algo orders
GET    /sapi/v1/algo/spot/historicalOrders  Historical algo orders
GET    /sapi/v1/algo/spot/subOrders         Sub-orders for a given algoId
```

## Futures Algo Endpoints

```
POST   /sapi/v1/algo/futures/newOrderVp         VP order (from 2022-04-13)
POST   /sapi/v1/algo/futures/newOrderTwap       TWAP order (from 2022-04-27)
DELETE /sapi/v1/algo/futures/order              Cancel
GET    /sapi/v1/algo/futures/openOrders         Open orders
GET    /sapi/v1/algo/futures/historicalOrders   Historical orders
GET    /sapi/v1/algo/futures/subOrders          Sub-orders for algoId
```

## Strategies

**TWAP (Time-Weighted Average Price)** — splits large order over time interval to reduce market impact.

**VP (Volume Participation)** — executes as a percentage of market volume.

---

## Signing Note (effective 2026-01-15)

Percent-encode payloads **before** computing signatures. Requests not following this order rejected with `-1022 INVALID_SIGNATURE`.

# Binance Global — Futures USDⓈ-M Enums & Filters

**Exchange:** Binance Global  
**Product:** Futures USDⓈ-M (`fapi.binance.com`)  
**Source:** `common-definition.md` (USDⓈ-M section), `schema__3_.yaml`  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Symbol Type
`FUTURE`

## Contract Types
`PERPETUAL`, `CURRENT_MONTH`, `NEXT_MONTH`, `CURRENT_QUARTER`, `NEXT_QUARTER`, `PERPETUAL_DELIVERING`

## Contract Status
`PENDING_TRADING`, `TRADING`, `PRE_DELIVERING`, `DELIVERING`, `DELIVERED`, `PRE_SETTLE`, `SETTLING`, `CLOSE`

## Order Status
`NEW`, `PARTIALLY_FILLED`, `FILLED`, `CANCELED`, `REJECTED`, `EXPIRED`, `EXPIRED_IN_MATCH`

## Order Types
`LIMIT`, `MARKET`, `STOP`, `STOP_MARKET`, `TAKE_PROFIT`, `TAKE_PROFIT_MARKET`, `TRAILING_STOP_MARKET`

> ⚠️ As of 2025-12-09, conditional order types (`STOP_MARKET`, `TAKE_PROFIT_MARKET`, `STOP`, `TAKE_PROFIT`, `TRAILING_STOP_MARKET`) migrated to Algo Service. Placing via `/fapi/v1/order` returns error `-4120 STOP_ORDER_SWITCH_ALGO`. Use `/fapi/v1/algoOrder` instead.

## Order Side
`BUY`, `SELL`

## Position Side (Hedge Mode)
`BOTH`, `LONG`, `SHORT`

## Time in Force
| Value | Description |
|---|---|
| `GTC` | Good Till Cancel (validity: 1 year from placement) |
| `IOC` | Immediate or Cancel |
| `FOK` | Fill or Kill |
| `GTX` | Good Till Crossing (Post Only) |
| `GTD` | Good Till Date |
| `RPI` | Retail Price Improvement (post-only, matched only from APP/Web) |

## Working Type (stop trigger price)
`MARK_PRICE`, `CONTRACT_PRICE`

## Response Type
`ACK`, `RESULT`

## Price Match
`NONE`, `OPPONENT`, `OPPONENT_5`, `OPPONENT_10`, `OPPONENT_20`, `QUEUE`, `QUEUE_5`, `QUEUE_10`, `QUEUE_20`

> ⚠️ `OPPONENT_10` and `OPPONENT_20` temporarily removed from place/amend flows effective 2025-10-23.

## STP Modes
`EXPIRE_TAKER`, `EXPIRE_BOTH`, `EXPIRE_MAKER`

## Rate Limits
```json
{ "rateLimitType": "REQUEST_WEIGHT", "interval": "MINUTE", "intervalNum": 1, "limit": 2400 }
{ "rateLimitType": "ORDERS", "interval": "MINUTE", "intervalNum": 1, "limit": 1200 }
```

## Kline Intervals
`1s`, `1m`, `3m`, `5m`, `15m`, `30m`, `1h`, `2h`, `4h`, `6h`, `8h`, `12h`, `1d`, `3d`, `1w`, `1M`

---

## Symbol Filters

### PRICE_FILTER
- `minPrice`, `maxPrice`, `tickSize`
- `price >= minPrice`, `price <= maxPrice`, `(price - minPrice) % tickSize == 0`

### LOT_SIZE
- `minQty`, `maxQty`, `stepSize`
- `quantity >= minQty`, `quantity <= maxQty`, `(quantity - minQty) % stepSize == 0`

### MARKET_LOT_SIZE
- Same structure, applies to MARKET orders

### MAX_NUM_ORDERS
- `limit` — max open orders per symbol (algo + normal)

### MAX_NUM_ALGO_ORDERS
- `limit` — max open algo orders per symbol

### PERCENT_PRICE
- `multiplierUp`, `multiplierDown`, `multiplierDecimal`
- BUY: `price <= markPrice * multiplierUp`
- SELL: `price >= markPrice * multiplierDown`

### MIN_NOTIONAL
- `notional` — minimum `price * quantity` (MARKET uses mark price)

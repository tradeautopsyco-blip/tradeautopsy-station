# Binance.US — Spot Enums & Filters

**Exchange:** Binance.US  
**Source:** GitHub `binance-us/binance-us-api-docs`  
**Snapshot date:** 2023-09-06  
**Reviewed:** No

---

## Order Status

| Status | Description |
|---|---|
| `NEW` | Accepted by engine |
| `PARTIALLY_FILLED` | Part filled |
| `FILLED` | Completed |
| `CANCELED` | Canceled by user |
| `PENDING_CANCEL` | Currently unused |
| `REJECTED` | Not accepted, not processed |
| `EXPIRED` | Canceled per order type rules or by exchange |

## Order Types

`LIMIT`, `MARKET`, `STOP_LOSS`, `STOP_LOSS_LIMIT`, `TAKE_PROFIT`, `TAKE_PROFIT_LIMIT`, `LIMIT_MAKER`

## Order Side

`BUY`, `SELL`

## Time in Force

| Value | Meaning |
|---|---|
| `GTC` | Good Till Canceled |
| `IOC` | Immediate Or Cancel |
| `FOK` | Fill or Kill |

## Rate Limit Types

| Type | Interval |
|---|---|
| `REQUEST_WEIGHT` | Per minute |
| `ORDERS` | Per second / per day |
| `RAW_REQUESTS` | Per 5 minutes |

## Symbol Filters (from source)

### PRICE_FILTER
- `minPrice`, `maxPrice`, `tickSize`
- `price >= minPrice`, `price <= maxPrice`, `price % tickSize == 0`

### LOT_SIZE
- `minQty`, `maxQty`, `stepSize`
- `quantity >= minQty`, `quantity <= maxQty`, `quantity % stepSize == 0`

### MIN_NOTIONAL
- `minNotional` — minimum `price * quantity`
- `applyToMarket` — whether applies to MARKET orders

### ICEBERG_PARTS
- `limit` — max parts for iceberg order (`CEIL(qty / icebergQty)`)

### MARKET_LOT_SIZE
- Same as LOT_SIZE but for MARKET orders

### MAX_NUM_ORDERS
- `maxNumOrders` — max open orders per symbol

### MAX_NUM_ALGO_ORDERS
- `maxNumAlgoOrders` — max open algo orders (STOP_LOSS, STOP_LOSS_LIMIT, TAKE_PROFIT, TAKE_PROFIT_LIMIT) per symbol

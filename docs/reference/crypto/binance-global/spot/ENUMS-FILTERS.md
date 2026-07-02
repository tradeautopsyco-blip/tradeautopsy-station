# Binance Global — Spot Enums & Filters

**Exchange:** Binance Global (api.binance.com)  
**Source:** `enums.md`, `filters.md` — developers.binance.com markdown export  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Symbol Status
`TRADING`, `END_OF_DAY`, `HALT`, `BREAK`

## Account & Symbol Permissions
`SPOT`, `MARGIN`, `LEVERAGED`, `TRD_GRP_002` through `TRD_GRP_025`

## Order Status

| Status | Description |
|---|---|
| `NEW` | Accepted by engine |
| `PENDING_NEW` | Pending until working order of order list is fully filled |
| `PARTIALLY_FILLED` | Part filled |
| `FILLED` | Completed |
| `CANCELED` | Canceled by user |
| `PENDING_CANCEL` | Currently unused |
| `REJECTED` | Not accepted, not processed |
| `EXPIRED` | Canceled per order type rules or by exchange |
| `EXPIRED_IN_MATCH` | Expired due to STP |

## Order List Status (listStatusType)
`RESPONSE`, `EXEC_STARTED`, `UPDATED`, `ALL_DONE`

## Order List Order Status (listOrderStatus)
`EXECUTING`, `ALL_DONE`, `REJECT`

## ContingencyType
`OCO`, `OTO`

## AllocationType
`SOR`

## Order Types
`LIMIT`, `MARKET`, `STOP_LOSS`, `STOP_LOSS_LIMIT`, `TAKE_PROFIT`, `TAKE_PROFIT_LIMIT`, `LIMIT_MAKER`

## Order Response Type (newOrderRespType)
`ACK`, `RESULT`, `FULL`

## Order Side
`BUY`, `SELL`

## Time in Force

| Value | Description |
|---|---|
| `GTC` | Good Till Canceled |
| `IOC` | Immediate Or Cancel |
| `FOK` | Fill or Kill |

## STP Modes
`NONE`, `EXPIRE_MAKER`, `EXPIRE_TAKER`, `EXPIRE_BOTH`, `DECREMENT`, `TRANSFER`

## Rate Limit Types

```json
{ "rateLimitType": "REQUEST_WEIGHT", "interval": "MINUTE", "intervalNum": 1, "limit": 6000 }
{ "rateLimitType": "ORDERS", "interval": "SECOND", "intervalNum": 1, "limit": 10 }
{ "rateLimitType": "RAW_REQUESTS", "interval": "MINUTE", "intervalNum": 5, "limit": 61000 }
```

## Execution Types
`NEW`, `CANCELED`, `REPLACED`, `REJECTED`, `TRADE`, `EXPIRED`, `TRADE_PREVENTION`

## Expiry Reasons
`NONE`, `REJECTED`, `EXCHANGE_CANCELED`, `OCO_TRIGGER`, `OTO_PHASE_ONE_EXPIRED`, `UNFILLED_IOC_QUANTITY_EXPIRED`, `UNFILLED_FOK_ORDER_EXPIRED`, `INSUFFICIENT_LIQUIDITY`, `EXECUTION_RULE_PRICE_RANGE_EXCEEDED`

---

## Symbol Filters

### PRICE_FILTER
- `minPrice`, `maxPrice`, `tickSize`
- Pass: `price >= minPrice`, `price <= maxPrice`, `price % tickSize == 0`

### PERCENT_PRICE
- Valid price range based on average of previous trades
- Pass: `price <= avg * multiplierUp`, `price >= avg * multiplierDown`
- `avgPriceMins`: 0 = use last price

### PERCENT_PRICE_BY_SIDE
- Separate BUY/SELL multipliers: `bidMultiplierUp/Down`, `askMultiplierUp/Down`

### LOT_SIZE
- `minQty`, `maxQty`, `stepSize`
- Pass: `quantity >= minQty`, `quantity <= maxQty`, `quantity % stepSize == 0`

### MIN_NOTIONAL
- `minNotional` = min `price * quantity`
- `applyToMarket`: applies to MARKET orders using VWAP

### NOTIONAL
- `minNotional`, `maxNotional`, `applyMinToMarket`, `applyMaxToMarket`

### ICEBERG_PARTS
- `limit` = max parts (`CEIL(qty / icebergQty)`)

### MARKET_LOT_SIZE
- Same structure as LOT_SIZE but for MARKET orders only

### MAX_NUM_ORDERS
- `maxNumOrders` — max open orders per symbol (algo + normal)

### MAX_NUM_ALGO_ORDERS
- `maxNumAlgoOrders` — max open algo orders (STOP_LOSS, STOP_LOSS_LIMIT, TAKE_PROFIT, TAKE_PROFIT_LIMIT)

### MAX_NUM_ICEBERG_ORDERS
- `maxNumIcebergOrders`

### MAX_POSITION
- `maxPosition` = max (free balance + locked balance + sum of open BUY qty)

### TRAILING_DELTA
- `minTrailingAboveDelta`, `maxTrailingAboveDelta`, `minTrailingBelowDelta`, `maxTrailingBelowDelta`

### MAX_NUM_ORDER_AMENDS
- `maxNumOrderAmends` — max times a single order can be amended; `-2038` if exceeded

### MAX_NUM_ORDER_LISTS
- `maxNumOrderLists` — max open order lists per symbol

## Exchange Filters
- `EXCHANGE_MAX_NUM_ORDERS` — `maxNumOrders`: 1000
- `EXCHANGE_MAX_NUM_ALGO_ORDERS` — `maxNumAlgoOrders`: 200
- `EXCHANGE_MAX_NUM_ICEBERG_ORDERS` — `maxNumIcebergOrders`: 10000
- `EXCHANGE_MAX_NUM_ORDER_LISTS` — `maxNumOrderLists`: 20

## Asset Filters
- `MAX_ASSET` — `asset`, `limit` — max quantity of an asset in a single order (base or quote)

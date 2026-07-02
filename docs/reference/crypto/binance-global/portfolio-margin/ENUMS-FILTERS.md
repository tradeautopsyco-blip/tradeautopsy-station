# Binance Global — Portfolio Margin Enums & Filters

**Exchange:** Binance Global  
**Source:** Public API Definitions (document index 23/24 in session)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Order Side
`BUY`, `SELL`

## Position Side (Futures)
`BOTH`, `LONG`, `SHORT`

## Time in Force
`GTC`, `IOC`, `FOK`, `GTX` (Post Only)

## Stop-Limit Time in Force
`GTC`, `IOC`, `FOK`

## Side Effect Type (Margin)
`NO_SIDE_EFFECT`, `MARGIN_BUY`, `AUTO_REPAY`

## Price Match
`NONE`, `OPPONENT`, `OPPONENT_5`, `OPPONENT_10`, `OPPONENT_20`, `QUEUE`, `QUEUE_5`, `QUEUE_10`, `QUEUE_20`

## STP Modes
`NONE`, `EXPIRE_TAKER`, `EXPIRE_BOTH`, `EXPIRE_MAKER`

## Response Type
`ACK`, `RESULT`

## Order Types (spot/margin)
`LIMIT`, `MARKET`

## Conditional Order Types
`STOP`, `STOP_MARKET`, `LIMIT_MAKER`, `TAKE_PROFIT`, `TAKE_PROFIT_MARKET`, `TRAILING_STOP_MARKET`

## Working Type (Futures conditional)
`MARK_PRICE`

## Order Status
`NEW`, `CANCELED`, `REJECTED`, `PARTIALLY_FILLED`, `FILLED`, `EXPIRED`, `EXPIRED_IN_MATCH`

## Conditional Order Status
`NEW`, `CANCELED`, `TRIGGERED`, `FINISHED`, `EXPIRED`

## Futures Contract Type
`PERPETUAL`, `CURRENT_MONTH`, `NEXT_MONTH`, `CURRENT_QUARTER`, `NEXT_QUARTER`, `PERPETUAL_DELIVERING`

## Contract Status
`PENDING_TRADING`, `TRADING`, `PRE_DELIVERING`, `DELIVERING`, `DELIVERED`, `PRE_SETTLE`, `SETTLING`, `CLOSE`

## Rate Limits
```json
{ "rateLimitType": "REQUEST_WEIGHT", "interval": "MINUTE", "intervalNum": 1, "limit": 2400 }
{ "rateLimitType": "ORDERS", "interval": "MINUTE", "intervalNum": 1, "limit": 1200 }
```

---

## Symbol Filters

### PRICE_FILTER
- Sell: `price >= minPrice`. Buy: `price <= maxPrice`. `(price - minPrice) % tickSize == 0`

### LOT_SIZE
- `quantity >= minQty`, `quantity <= maxQty`, `(quantity - minQty) % stepSize == 0`

### PERCENT_PRICE
- Futures: based on mark price. Cross Margin: based on weighted average (`avgPriceMins` minutes).
- BUY: `price <= ref * multiplierUp`. SELL: `price >= ref * multiplierDown`

### MIN_NOTIONAL
- Futures: uses mark price. Cross Margin: uses average price over `avgPriceMins`.

### MARKET_LOT_SIZE / MAX_NUM_ORDERS / MAX_NUM_ALGO_ORDERS
- Same structure as USDⓈ-M Futures filters.

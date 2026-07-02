# Binance Global — Portfolio Margin WebSocket / User Data Streams

**Exchange:** Binance Global  
**Source:** User Data Streams doc (document index 21 in session)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Connection

**REST base:** `https://papi.binance.com`  
**WS base:** `wss://fstream.binance.com/pm`  
**Access:** `/ws/<listenKey>`

Example: `wss://fstream.binance.com/pm/ws/pqia91ma19a5s61cv6a81va65sdf19v8a65a1a5s61cv6a81va65sdf19v8a65a1`

listenKey validity: 60 minutes. `PUT` to extend. `DELETE` to close.  
Single connection valid 24 hours.

**Message ordering:** Same event type on same connection strictly ordered by `T` (matching engine) and `E` (event generation). Use `E` for cross-event-type ordering.

---

## Events

### ACCOUNT_UPDATE
Balance or position changed. `m` = reason type (same set as USDⓈ-M).  
`bc` = balance change excluding PnL and commission.  
Update speed: 50ms.

FUNDING FEE cross position: balance `B` only.  
FUNDING FEE isolated position: balance `B` + position `P`.

### ORDER_TRADE_UPDATE
Futures order state change.

Order types: `MARKET`, `LIMIT`, `LIQUIDATION`  
Execution types: `NEW`, `CANCELED`, `CALCULATED` (liquidation), `EXPIRED`, `TRADE`  
Liquidation: `c` = "autoclose-XXX" | ADL: `c` = "adl_autoclose"

### ACCOUNT_CONFIG_UPDATE
Leverage change. Payload contains `ac` (symbol config: `s`=symbol, `l`=leverage).

### CONDITIONAL_ORDER_TRADE_UPDATE
Conditional order state change.

Conditional types: `STOP`, `TAKE_PROFIT`, `STOP_MARKET`, `TAKE_PROFIT_MARKET`, `TRAILING_STOP_MARKET`  
Statuses: `NEW`, `CANCELED`, `EXPIRED`, `TRIGGERED`, `FINISHED`

### balanceUpdate
Margin balance update.

### executionReport (Margin Order Update)
Margin order state change.

Execution types: `NEW`, `CANCELED`, `REJECTED`, `TRADE`, `EXPIRED`, `TRADE_PREVENTION`

### outboundAccountPosition
Account balance changed. Contains assets possibly changed by the triggering event.

### liabilityChange
Margin liability update (borrow, repayment, interest).

### openOrderLoss
Cross margin order margin stream.

### RISK_LEVEL_CHANGE
Position risk ratio too high. Types: `MARGIN_CALL`, `REDUCE_ONLY`, `FORCE_LIQUIDATION`. Risk guidance only.

### ALGO_UPDATE
Algo order status change.

Statuses: `NEW`, `CANCELED`, `TRIGGERING`, `TRIGGERED`, `FINISHED`, `REJECTED`, `EXPIRED`

### listenKeyExpired
listenKey expired. No more events until new listenKey.

### PM_PRO_ACCOUNT_UPDATE (Portfolio Margin Pro)
Pushes account asset status every 5 seconds. Added 2026-04-17.

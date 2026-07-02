# Binance Global — Margin WebSocket / Risk Data Stream

**Exchange:** Binance Global  
**Product:** Margin Trading  
**Source:** Risk Data Stream doc, Trade Data Stream doc (in `combined_appendix__1_.md`)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

---

## Risk Data Stream (Cross Margin only)

**REST base:** `https://api.binance.com`  
**WS base:** `wss://margin-stream.binance.com`  
**Access:** `/ws/<listenKey>` or `/stream?streams=<listenKey>`

> Only supports **Cross Margin Accounts**. Isolated margin not supported here.

listenKey validity: 60 minutes. `PUT` to extend. `DELETE` to close.  
Multiple listenKeys / streams per connection supported.  
Single connection valid 24 hours.

---

## Events

### MARGIN_LEVEL_STATUS_CHANGE
Pushed when margin call triggers.

### USER_LIABILITY_CHANGE
Pushed on:
- Borrowing
- Repayment
- Interest calculation

---

## Trade Data Stream (deprecated)

**REST base:** `https://api.binance.com`

Listed as deprecated in source. Do not build new features against this stream.

---

## Note on Margin User Data

Margin account order execution events (`executionReport`, `outboundAccountPosition`, `balanceUpdate`) flow through the standard Spot User Data Stream, not through the Risk Data Stream. The Risk Data Stream is Margin-specific risk monitoring only.

# Binance Global — Wallet REST API

**Exchange:** Binance Global  
**Source:** `schema__7_.yaml` (6,024 lines)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No

Schema title: "Wallet REST API — Query balances, manage assets, and perform wallet operations via the Binance Wallet API."

Rate limits guide: `products/wallet/general-info#limits`  
Security guide: `products/wallet/general-info#request-security`

---

## Scope

Wallet endpoints cover: balance queries, deposit/withdrawal history, asset management, dust transfer, asset conversion records, API key permission info.

Base path: `/sapi/v1/` (various sub-paths)

---

## TradeAutopsy relevance

Potential read-only use cases:
- Deposit/withdrawal history for behavioral context (large fund movements correlated with trading behavior)
- Asset balance snapshots across wallet types (spot, funding, etc.)

**No wallet actions performed by TradeAutopsy.** Read-only only.

Full endpoint list requires reviewing `schema__7_.yaml` directly — 6,024 lines not fully extracted here. File available in uploads for Cursor to process when Wallet feature slice is started.

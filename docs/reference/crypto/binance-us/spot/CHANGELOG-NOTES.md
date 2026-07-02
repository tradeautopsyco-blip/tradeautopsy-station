# Binance.US — Spot Changelog Notes

**Exchange:** Binance.US  
**Source:** GitHub `binance-us/binance-us-api-docs` CHANGELOG  
**Snapshot date:** 2023-09-06  
**Reviewed:** No

> Only entries relevant to TradeAutopsy assumptions. Not a full changelog dump.

---

## ⚠️ Snapshot staleness warning

Source snapshot is dated **2023-09-06**. Live `docs.binance.us` shows changelog entries through at least Oct 2025, including WAPI→SAPI migration. **This file reflects 2023 state only.** Before building any feature against Binance.US, verify live.

Known changes likely to have occurred since snapshot:
- WAPI endpoints deprecated in favor of SAPI (confirmed from live docs.binance.us reference)
- Rate limit values may have changed
- Any new mandatory params or field changes since 2023

---

## Relevant historical entries (pre-snapshot)

No specific dated entries extracted from the 2023-09-06 snapshot that affect TradeAutopsy read-only behavior. The snapshot covers stable core SPOT endpoints (`/api/v3/`).

---

## Open questions requiring live verification

1. Does a key-level withdraw-permission check endpoint exist on Binance.US? (Not in source.)
2. What is the actual `myTrades` retention window for Binance.US?
3. Are `/api/v3/` endpoints still the canonical paths, or has anything moved?

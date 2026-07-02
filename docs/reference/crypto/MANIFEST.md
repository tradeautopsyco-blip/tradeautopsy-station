# Crypto Reference Manifest

**Last updated:** 2026-07-02  
**Standard:** `CRYPTO-STANDARD.md`

---

## Status Table

| Exchange | Asset Class | Folder | Status | Source Material | Snapshot Date | Blockers |
|---|---|---|---|---|---|---|
| Binance.US | Spot | `binance-us/spot/` | Structured | GitHub `binance-us/binance-us-api-docs` (9 files) | 2023-09-06 | **3 open — see below** |
| Binance Global | Spot | `binance-global/spot/` | Structured | `schema.yaml` (13,311 lines) + markdown exports | 2026-07 | None |
| Binance Global | Margin | `binance-global/margin/` | Structured | `schema__1_.yaml` (9,524 lines) + Risk Data Stream doc | 2026-07 | None |
| Binance Global | Futures USDⓈ-M | `binance-global/futures-usdm/` | Structured | `schema__3_.yaml` (13,621 lines) + markdown docs | 2026-07 | None |
| Binance Global | Futures COIN-M | `binance-global/futures-coinm/` | Structured | `schema__4_.yaml` (9,915 lines) + markdown docs | 2026-07 | Architecture merger with USDⓈ-M, effective 2026-06-30 |
| Binance Global | Options | `binance-global/options/` | Structured | `schema__5_.yaml` (5,957 lines) | 2026-07 | None |
| Binance Global | Wallet | `binance-global/wallet/` | Structured | `schema__7_.yaml` (6,024 lines) | 2026-07 | None |
| Binance Global | Convert | `binance-global/convert/` | Structured | `schema__8_.yaml` (1,051 lines) | 2026-07 | None |
| Binance Global | Portfolio Margin | `binance-global/portfolio-margin/` | Structured | General Info (`papi.binance.com`), User Data Streams, Error Codes, Public API Definitions | 2026-07 | None |
| Binance Global | Algo | `binance-global/algo/` | Structured | `schema__9_.yaml` (1,806 lines) — Spot Algo + Futures Algo unified | 2026-07 | None |

## Status Legend

| Status | Meaning |
|---|---|
| **Raw material in hand** | Source collected, not yet written into standard structure |
| **Structured** | Written into REST/WEBSOCKET/ENUMS/ERRORS/CHANGELOG files per Rule 5 |
| **Reviewed** | Self-audited per Rule 3, snapshot age checked per Rule 4, build-ready |

---

## Binance.US Spot — Open Blockers

These block Backend Box v1 release. Need live verification against docs.binance.us — not a full re-scrape, just these three points.

**Blocker 1** — No key-level withdraw-permission endpoint found  
Only `canWithdraw` flag exists at account level (`GET /api/v3/account`). Backend Box's hard release gate ("block save if withdraw permission detected on key") cannot be built as designed until confirmed live. May not exist on Binance.US at all.

**Blocker 2** — 90-day `myTrades` backfill unconfirmed  
The only "90 days" language in the 2023-09-06 source refers to archived zero-fill orders (`-2026 ORDER_ARCHIVED`), not fill/trade history retention. The PRD's 90-day backfill assumption may be phantom.

**Blocker 3** — Snapshot dated 2023-09-06  
Stale by ~3 years. `docs.binance.us` shows changelog entries through Oct 2025 at minimum (WAPI→SAPI migration). Endpoints may have moved.

---

## Futures COIN-M — Architecture Merger Note

COIN-M Futures (`dapi.binance.com`) is being integrated into the USDⓈ-M architecture. Key changes effective 2026-06-30:

- `GET /dapi/v1/pmAccountInfo` deprecated — use `GET /fapi/v1/pmAccountInfo`
- `dualSidePosition` now unified across COIN-M and USDⓈ-M — changing one syncs the other
- `<pair>@indexPrice@1s` stream variant removed
- `pair` field in `<pair>@indexPrice` payload renamed from `"i"` to `"s"`

Any code reading COIN-M account info or dual-side position must be updated. Cross-reference: `futures-usdm/CHANGELOG-NOTES.md`.

---

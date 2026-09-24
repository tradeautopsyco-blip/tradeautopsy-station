# Dogfood — `zerodha_kite` · book `zerodha-nse-nfo`

**Status:** OPEN (Tier II — founder Kite account required)  
**Lock:** `issues/compliance/locks/zerodha-nse-nfo.md`  
**Owner:** `agent/src/nfo_realized_pnl.rs` (shared with `kotak-nse-nfo`)

## Tier II checklist (step 6)

1. Connect Station with book `zerodha-nse-nfo` (catalog Planned until this closes).
2. `GET /trades` day book + `GET /instruments/NFO` lot stamp (no invented lot).
3. Golden: qty=2 round-trip, lot from instruments row; realized INR hero matches desk law.
4. Cash book `zerodha-nse-bse-cash` must not retain NFO fills after split.

## Accountless (steps 1–5)

Component tests `ubi_zerodha_kite_nfo_component` + wasm32-wasip2 release build.

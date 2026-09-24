# Dogfood — `upstox` · book `upstox-nse-nfo`

**Status:** OPEN (Tier II — founder Upstox account required)  
**Lock:** `issues/compliance/locks/upstox-nse-nfo.md`  
**Owner:** `agent/src/nfo_realized_pnl.rs`

## Tier II checklist (step 6)

1. Connect Station with book `upstox-nse-nfo` (catalog Planned until this closes).
2. Day trade-book + `assets.upstox.com` **NFO.json.gz** lot stamp (no invented lot).
3. Golden: qty=2 round-trip; realized INR matches desk law.
4. Cash book must not retain NFO fills after split.

## Accountless (steps 1–5)

`cargo test --test ubi_upstox_nfo_component` + host fence `upstox_nfo_book_fence_splits_bod_gzip_from_cash`.

**Signed:** —  
**Date (IST):** —

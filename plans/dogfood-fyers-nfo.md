# Dogfood — `fyers` · book `fyers-nse-nfo`

**Status:** OPEN (Tier II — founder Fyers account required)  
**Lock:** `issues/compliance/locks/fyers-nse-nfo.md`  
**Owner:** `agent/src/nfo_realized_pnl.rs`

## Tier II checklist (step 6)

1. Connect Station with book `fyers-nse-nfo` (catalog Planned until this closes).
2. Tradebook + `public.fyers.in` **NSE_FO.csv** lot stamp.
3. Golden: qty=2 round-trip; realized INR matches desk law.
4. Cash book must not retain NFO fills after split.

## Accountless (steps 1–5)

`cargo test --test ubi_fyers_nfo_component` + host fence `fyers_nfo_book_fence_splits_sym_master_from_cash`.

**Signed:** —  
**Date (IST):** —

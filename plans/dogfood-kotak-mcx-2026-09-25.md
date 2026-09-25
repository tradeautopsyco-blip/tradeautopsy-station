# Dogfood — `kotak_neo` MCX (`kotak-mcx-future`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (2026-09-25); Tier II step 6 open  
**Lock:** `issues/compliance/locks/kotak-mcx-future.md`  
**Playbook:** `issues/brokers/playbooks/INDIA-MULTI-BOOK-STACK.md`  
**Ladder:** ADR 0019

## Preconditions (engineering)

- [x] `mcx_fo.csv` scrip lane wired (`KOTAK_MCX_FO_LANE`, `kotak_mcx_scrip_master`)
- [x] Desk core ops: quotes, tradebook, funds, instruments enrichers
- [x] Catalog **Enabled** on `kotak-mcx-future` (Connect beta; ADR 0019 Tier I)
- [ ] Live MCX entitlement on Kotak account
- [ ] Signed drills 1–8 (mirror CDS dogfood plan shape)

## Tier II

Desk-live requires founder-signed step 6 + registry live-slugs update (not done).

# India multi-book stack — active execution

**Status:** ACTIVE  
**Playbook (SoT):** `issues/brokers/playbooks/INDIA-MULTI-BOOK-STACK.md`  
**E2E plan slice:** S1.5 in `issues/brokers/FULL-COVERAGE-E2E-IMPLEMENTATION-PLAN.md`

## Done this slice (2026-09-25)

- Playbook + `agent/src/data/india_book_stack.rs` CI guards (manifest + money matrix + desk core ops declared).
- Kotak `kotak-nse-cds` / `kotak-mcx-future` desk enrichers + private kick parity with NFO.
- **S1.5-2:** `cde_fo` / `mcx_fo` scrip lanes (`spawn_kotak_multi_fo_master_refresh`, per-book masters + cache).

## Next (accuracy)

1. **S1.5-3** — Dogfood CDS then MCX; flip **Planned → Enabled** per book after signed step 6.
2. **Wave 2** — After Kotak signed, one CDS/MCX book per broker only when B6 cites segment.

## Verify

```bash
cd agent && cargo test --lib india_book_stack
cargo test --lib manifest::tests
```

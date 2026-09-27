# FILL_MATCHED loopback inject — sketch only

**Status:** Not implemented. Console lives in `FExEVIL/tradeautopsy`; this Station repo documents the seam only.

## Problem

Harness **journal sink** Debrief works on `pending` declarations (Console upserts `notes.post`). A separate **fidelity** lane wants `status=matched` after fill without a real Binance order. Today `FILL_MATCHED` exists in Console lifecycle but no production caller flips `status` on broker ingest.

## Proposed shape (future)

| Piece | Idea |
| --- | --- |
| Env gate | `BAR_TEST_INJECT_FILL_MATCHED=1` (Console) + loopback-only bind |
| Auth | Same as other daemon routes: Station Caller Bearer from agent; optional `x-daemon-secret` on internal inject only from `127.0.0.1` |
| Route | `POST /api/internal/bar/v1/test/fill-matched` (Console) **or** agent `POST /api/daemon/bar/test/fill-matched` forward |
| Body | `{ "declaration_id": "<uuid>", "symbol", "side", "quantity", "price" }` — no broker HTTP |
| Effect | Apply `FILL_MATCHED` transition + optional synthetic exit row; **never** default in prod |
| Harness | `--wait-closed-sec` + inject flag calls inject before Debrief fidelity poll |

## Out of scope here

- No Console PR in this task
- No agent route until Console contract is frozen
- Harness continues to prove sink without inject (`scripts/notch-live-journal-lib.py`)

See [`plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`](CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md).

# FILL_MATCHED loopback inject — implemented seam

**Status:** Implemented in Station (agent forward + harness). Console contract: [FExEVIL/tradeautopsy#378](https://github.com/FExEVIL/tradeautopsy/pull/378).

## Problem

Harness **journal sink** Debrief works on `pending` declarations (Console upserts `notes.post`). A separate **fidelity** lane wants `status=matched` after fill without a real Binance order.

## Contract (Console — do not reimplement here)

| Piece | Value |
| --- | --- |
| Env gate | `BAR_TEST_INJECT_FILL_MATCHED=1` on Console (404 if unset — **prod must leave unset**) |
| Auth | Station Caller Bearer (same as other bar daemon forwards) |
| Console route | `POST /api/internal/bar/v1/test/fill-matched` |
| Body | `{ "declaration_id", "symbol", "side", "quantity", "price" }` — no broker HTTP |
| Response | `test_only`, `status: "matched"`, `trade_id`, `matched_at_ms`, `fidelity`, optional `trip_cite: { net, currency }` for Journal cite (Wave 0.4) |

## Station (this repo)

| Piece | Value |
| --- | --- |
| Agent route | `POST /api/daemon/bar/test/fill-matched` → forwards to Console internal route |
| Wire | Documented in `station-wire/v1.json` (`test_fill_matched` hops) |
| Harness | `scripts/notch-live-journal.sh` / `notch-live-journal-lib.py` — `--inject-matched` after declare; fails clearly on 404 or non-2xx |
| Journal cite | On `test_only` + `matched`, agent may persist `trip_cite` → `GET /api/daemon/journal/trip-cites` (Wave 0.3 UI). Never flips local declaration status. |
| Poll | Use with `--wait-closed-sec` so scoreboard **Match fidelity** shows matched (not `not_ready`) |

### Example

```bash
./scripts/notch-live-journal.sh \
  --book binance-com-spot --symbol BTCUSDT \
  --inject-matched --wait-closed-sec 30 --keep-declaration
```

Requires Console with `BAR_TEST_INJECT_FILL_MATCHED=1`, Station Debug agent (9137), Gate B signed-in session.

See [`plans/NOTCH-LIVE-JOURNAL-HARNESS.md`](NOTCH-LIVE-JOURNAL-HARNESS.md) and [`plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`](CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md).

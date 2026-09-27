# Notch live journal harness (Phase B)

Founder-facing **live** probe for the Debug agent on loopback (`127.0.0.1:9137`). Complements `scripts/dogfood-suite.sh` with gates, **declare → Working → Debrief journal sink**, tools matrix, and scoreboard.

**Spec:** [`plans/STRONG-TESTING-SYSTEM-2026-09-27.md`](STRONG-TESTING-SYSTEM-2026-09-27.md) §4 Phase B.  
**Investigation:** [`plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`](CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md).

## Run

```bash
chmod +x scripts/notch-live-journal.sh
./scripts/notch-live-journal.sh
```

**Journal sink (no fill required):** Gate B signed-in → declare → Working → **PATCH debrief** → read-back `notes.post` on `GET /api/daemon/bar/declarations`. Works while declaration is still `pending`.

```bash
./scripts/notch-live-journal.sh --book binance-com-spot --symbol BTCUSDT --stance planned
```

Read-only:

```bash
./scripts/notch-live-journal.sh --dry-run
```

Optional **match/fill fidelity** (separate from sink PASS — does not gate PATCH):

```bash
./scripts/notch-live-journal.sh --wait-closed-sec 120 --keep-declaration
```

Outbox accept proof (Mac E2E):

```bash
./scripts/notch-live-journal.sh --prove-capture
```

## Gates

| Gate | Route | Pass |
| --- | --- | --- |
| A | `GET /api/daemon/health` | HTTP 200 |
| B | `GET /api/daemon/auth/station/session` | `signed_in: true` |
| C | `GET /api/daemon/broker/sync-state` | HTTP 200 (report only) |

Exit **2** Gate A · **3** LIE · **4** `--require-debrief` without Debrief PASS.

## Debrief journal sink (default lane)

Console `PATCH /api/bar/v1/post-trade-debrief` upserts `declaration_payload.s1` / list `notes.post` when `declaration_id` is valid. **No closed-trip gate.** `400 invalid_body` = Zod/JSON mismatch (use `BarPostTradeDebriefPayload` shape in harness).

Harness always PATCHes after Working with body from `build_debrief_payload()` (mirrors `notch/BarPostTradeDebriefPayload.swift`).

**PASS:** HTTP success + read-back contains needle `Phase B harness debrief` in `notes.post` (or equivalent serialized post).

**FAIL:** `invalid_body`, other 4xx/5xx, or PATCH ok but read-back missing needle.

**HONEST-BLOCK** is not used for “pending / no fill” on this lane.

## Match fidelity (optional)

`closed_trip_ready()` / `--wait-closed-sec` poll for `matched`+empty post — useful when `FILL_MATCHED` is wired or after real fills. Reported under scoreboard **Match fidelity**; does **not** skip Debrief PATCH.

Future loopback inject sketch: [`plans/FILL-MATCHED-INJECT-SKETCH.md`](FILL-MATCHED-INJECT-SKETCH.md).

## Tradeflow lanes

| Lane | Routes |
| --- | --- |
| Plan | `POST …/bar/declare`, `GET …/declarations?scope=recent` |
| Working | `GET …/bar/live-state`, `POST …/bar/swing-check-in` |
| Debrief | `PATCH …/bar/post-trade-debrief`, read-back declarations |
| Cleanup | `POST …/cancel-declaration` unless `--keep-declaration` |

## Secret / orders

Never log `AGENT_DAEMON_SECRET`. Harness never places broker orders.

**Mac secret discovery:** resolve the listener PID with `lsof -nP -iTCP:9137 -sTCP:LISTEN -t` before plain `lsof -i :9137 -t`. Otherwise Station’s outbound client sockets can sort ahead of the agent and `ps eww` misses `AGENT_DAEMON_SECRET` (Gate A FAIL). Linux uses `ss -ltnp` first (unchanged).

## CI

Not run from `dogfood-suite.sh` by default.

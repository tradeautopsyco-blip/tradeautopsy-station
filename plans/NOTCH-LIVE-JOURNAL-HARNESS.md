# Notch live journal harness (Phase B)

Founder-facing **live** probe for the Debug agent on loopback (`127.0.0.1:9137`). Complements `scripts/dogfood-suite.sh` (CI unit/integration checks) with gates, Plan→Working→Debrief journal sinks, and a tools honesty matrix.

**Spec:** [`plans/STRONG-TESTING-SYSTEM-2026-09-27.md`](STRONG-TESTING-SYSTEM-2026-09-27.md) §4 Phase B.

## Run

```bash
chmod +x scripts/notch-live-journal.sh
./scripts/notch-live-journal.sh
```

Full tradeflow (declare + Working + outbox; Debrief only when a **closed trip** exists on Console):

```bash
./scripts/notch-live-journal.sh --book binance-com-spot --symbol BTCUSDT --stance planned
```

After you **close a real round-trip** on the declared symbol (broker sync — harness never places orders), wait for Console to mark the journal row closed, then:

```bash
./scripts/notch-live-journal.sh \
  --wait-closed-sec 120 \
  --keep-declaration \
  --require-debrief
```

Read-only gates + tools (no declare):

```bash
./scripts/notch-live-journal.sh --dry-run
```

Mac E2E lane (outbox accept proof):

```bash
./scripts/notch-live-journal.sh --prove-capture
```

## Gates (fail fast)

| Gate | Route | Pass |
| --- | --- | --- |
| A | `GET /api/daemon/health` | HTTP 200 |
| B | `GET /api/daemon/auth/station/session` | `signed_in: true`, no `SESSION_PROOF` |
| C | `GET /api/daemon/broker/sync-state` | HTTP 200 (report `syncState`; stale ⇒ tools may be **HONEST-DARK**) |

Exit **2** if Gate A fails · **3** if any tool **LIE** · **4** if `--require-debrief` and Debrief did not PASS.

## Secret

`AGENT_DAEMON_SECRET` from **tradeautopsy-agent** via `ps eww -p <pid>` (macOS) or `/proc/<pid>/environ` (Linux). Never printed.

## Tradeflow lanes

### Plan declare

`POST /api/daemon/bar/declare` — S1 moods (`mood_stress` = calm, `mood_impulse` = confidence, plus frustration/excitement), `stance`, symbol/qty/stops (`BarDeclarationFlowView` / `live_book::declare_from_body`). Assert UUID `declarationId`, list in `GET /api/daemon/bar/declarations?scope=recent`.

### Working

`GET /api/daemon/bar/live-state` pending + `plan_snapshot` / `emotion_in`.  
`POST /api/daemon/bar/swing-check-in` with `thesis_intact`, `declaration_id` (`BarPlanStateView.swingCheckInJSONObject`).

### Debrief (closed trip required)

**Notch body:** `PATCH /api/daemon/bar/post-trade-debrief` with `v: 1`, `declaration_id`, `moment_a_note`, `adherence` (five booleans), `moment_c_context`, `moment_c_note`, `completed_at_ms`, optional `live_note`, `emotion_out`, `stance` — see `notch/BarPostTradeDebriefPayload.swift`.

**Console gate:** debrief applies only when the declaration has a **closed trip** on the journal object (matched row with post empty / trip closed). Without that, Console returns **400 `invalid_body`** — scored **HONEST-BLOCK**, not a harness bug.

Detection: poll `GET /api/daemon/bar/declarations?scope=recent|week` until `closed_trip_ready()` (see harness) or `--wait-closed-sec` elapses.

**PASS:** HTTP success and read-back on the same declaration shows post/debrief fields (e.g. `moment_c_note` needle).

There is **no** loopback HTTP to synthesize a closed trip without broker fills; do not use the harness to place orders.

### Cleanup

Default: `POST /api/daemon/bar/cancel-declaration` with `cancel_reason_chip: scratch` after Debrief attempt.  
`--keep-declaration` skips cancel (use while waiting for a real fill).

## Tools honesty matrix

See prior table in `agent/src/api/mod.rs` (`chain` / `oi` for option_chain / open_interest). Scores include **LIE** and **HONEST-BLOCK** (Debrief only).

## Outbox

`GET /api/daemon/journal/toolbar-capture/outbox/status`.  
`--prove-capture` additionally `POST …/accept` (202 expected).  
`--skip-capture` skips the whole outbox lane.

## Outputs

- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.md`
- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.json`

(gitignored)

## CI

Default `dogfood-suite.sh` does **not** run this harness.

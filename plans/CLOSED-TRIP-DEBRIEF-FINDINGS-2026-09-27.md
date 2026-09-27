# Closed-trip / Debrief journal sink — investigation (2026-09-27 IST)

Goal: fastest path to declare → Working → Debrief → journal (incl. screenshot) **without** a real Binance order.

## Critical correction

**Console `PATCH /api/bar/v1/post-trade-debrief` does NOT gate on a closed trip.**

`invalid_body` (400) means JSON parse fail or Zod `.strict()` schema fail — **not** “no closed trip”.

Evidence:
- `FExEVIL/tradeautopsy` `app/api/bar/v1/post-trade-debrief/route.ts` (main): upserts `declaration_payload.s1.{live,post,adherence}` when `declaration_id` exists; no status/trip check.
- Earlier Mac console-up probe sent **wrong body** (`patch`, `notes`, `outcome_chip`, nested `emotion_out`) → Zod → `invalid_body`. Mis-scored as closed-trip block.
- PR #82 scoreboard skipped PATCH because `closed_trip_ready()` failed (status still `pending`); verdict text assumed closed-trip gate.

## What `closed_trip_ready()` actually checks

`scripts/notch-live-journal-lib.py` (PR #82 / branch `cursor/notch-live-journal-8c31`):

Ready iff post/debrief **not** already saved, status not cancelled/expired/pending, AND any of:
- `tripClosed` / `trip_closed`
- `closedAt` / `closed_at_ms` / `closedAtMs`
- nested `trip|escrowTrip|escrow.status` ∈ closed|complete|filled
- **or** `status` ∈ matched|closed|filled|complete **and** empty `post`

Declarations list items (`serializeListItem`) expose: `status`, `matched_at`, `notes.{pre,live,post}`, `fidelity`, `attachments` — **not** `tripClosed` / `closedAt`. So for real data, ready ≈ **`status === 'matched'` + empty `notes.post`**.

Declare creates **`pending`**. So harness waits forever without a match flip.

## Status `matched` writers (gap)

Lifecycle has `FILL_MATCHED` (`lib/bar/v1/declaration-lifecycle.ts`) but **no production caller** writes `UPDATE … status = 'matched'` on fill ingest.

Observed writers of `status = 'matched'`:
- Impulsive debrief INSERT in `post-trade-debrief` (creates new row with post already filled)

Fill path (`/api/internal/bar/v1/broker-ingest/fill`) → event log + BullMQ protective dispatch + optional fidelity — **does not flip declaration status**.

## Existing “synthesize” APIs?

| API | Purpose | Closed-trip / debrief? |
| --- | --- | --- |
| `PATCH …/bar/post-trade-debrief` | Upsert notes on declaration (+ signal) | **Works on pending**; no trip gate |
| Impulsive debrief (no `declaration_id`, `impulsive`/`reactive` + symbol/side/qty) | Creates matched row with post | Journal proof yes; skips declare→Working chain |
| `POST /api/internal/bar/v1/broker-ingest/fill` | Daemon fill ingest (staging; off if `BAR_INTERNAL_BROKER_FILL_INGEST=0`) | Logs/enqueues; **product enum MIS\|CNC only**; COM spot agent omits product; **does not mark matched** |
| `AGENT_BAR_BROKER_FILL_INGEST=1` + UUID envs | Agent posts real local fills upstream | Still needs real broker fills; not synthetic |
| Notch `BarDebriefArming` | UI arms Debrief on positions 1→0 or explicit exit | Local Notch only; not journal gate |
| toolbar-capture accept/presign/pending | Screenshot escrow | Independent of closed trip |

**No** `AGENT_TEST_INJECT_CLOSED_TRIP`, paper auto-fill, or dogfood “mark trip closed” endpoint found.

## Screenshot / post after debrief

1. Debrief submit (`BarPostTradeView`): optional `attachUnpostedToTicket(declId)` → `capture_ids` in PATCH body → merged into `s1.capture_ids`.
2. Capture pipeline: `POST /api/daemon/journal/toolbar-capture/accept` → `pending_capture_id` → `POST /api/daemon/screenshot/presign` → R2 PUT → `PATCH …/toolbar-capture/pending/:id` with `r2_key`.
3. Week journal `attachments.shots` = count of `pending_captures` with non-empty `r2_key` for `pre_trade_declaration_id`.
4. Harness `--prove-capture` only proves accept 202 / outbox status — not full screenshot→journal unless extended.

## Ranked options (speed vs fidelity)

### 1. FASTEST THIS WEEK — harness-only (recommended)
Drop or bypass `closed_trip_ready` for sink proof. PATCH correct `BarPostTradeDebriefPayload` against open declare `a510a022-…` (or fresh declare). Assert `notes.post` / `moment_c_note` needle on `GET …/declarations`.

- Ship: hours (edit PR #82 harness)
- Fidelity: proves **journal sink** (Console upsert + signal), not UI arming / match / fill path
- CloudAgent: **harness-only** on `tradeautopsy-station` PR #82
- Manual: agent signed-in on `:9137`; keep declaration or re-declare

### 2. Impulsive debrief lane (harness)
PATCH without `declaration_id`, with `impulsive:true` / `stance:reactive` + symbol/side/qty.

- Faster than fills; creates matched journal row
- Low fidelity vs Plan→Working→Debrief on same id

### 3. Console/agent test hook (next)
Loopback-only `AGENT_TEST_INJECT_CLOSED_TRIP` or Console daemon `POST …/test/mark-matched` applying `FILL_MATCHED` + optional synthetic exit — then harness waits `closed_trip_ready`.

- Needs CloudAgent PRs on **both** `tradeautopsy` + optionally station harness
- Days; higher fidelity; production-safe if loopback + daemon secret + env flag

### 4. Real tiny round-trip (not for harness automation)
User closes position; positions poll arms UI; `--wait-closed-sec --require-debrief`. Still blocked until FILL_MATCHED is wired if harness keys off `status=matched`.

### 5. broker-ingest synthetic fills — poor fit this week
Schema cash-locked; doesn’t flip matched; needs connection UUIDs; risk of polluting event log.

## Exact files / APIs

| Layer | Path |
| --- | --- |
| Harness | `scripts/notch-live-journal-lib.py` (`closed_trip_ready`, `build_debrief_payload`, `wait_for_closed_trip`) |
| Notch payload | `notch/BarPostTradeDebriefPayload.swift` |
| Notch UI arming | `notch/BarDebriefArming.swift` |
| Agent forward | `agent/src/api/bar.rs` → Console `/api/bar/v1/post-trade-debrief` |
| Console debrief | `tradeautopsy/app/api/bar/v1/post-trade-debrief/route.ts` |
| Console list | `…/declarations/route.ts` → `notes.post` |
| Capture | `agent/src/api/capture.rs`; Console `…/daemon/journal/toolbar-capture/*` |
| Fill ingest | `…/internal/bar/v1/broker-ingest/fill/route.ts`; `agent/src/bar_fill_ingress.rs` |

## Open declaration

`a510a022-b881-4af5-9288-4fb4ce22cb3c` — still valid candidate for Option 1 if not cancelled/expired.

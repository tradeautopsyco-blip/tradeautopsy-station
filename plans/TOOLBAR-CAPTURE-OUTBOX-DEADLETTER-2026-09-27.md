# Toolbar-capture outbox dead-letter — 2026-09-27

## Symptom

`--prove-capture` POSTs local `POST /api/daemon/journal/toolbar-capture/accept` → **HTTP 202** / `success: true` / `data.status=queued`, outbox `enqueued` 0→1, then **`dead_letter` rises**. Dead-letter reason:

```text
network: error sending request for url (https://www.tradeautopsy.in/api/daemon/journal/toolbar-capture/accept)
```

Observed outbox ids 3–6 (attempts=5). Draft examples include harness probe text (`Harness capture accept probe — safe to delete.`).

## Confirmed mechanism (Station agent)

1. Local accept handler enqueues raw JSON, then `process_now` → `CaptureOutbox::send_attempt` (`agent/src/outbox.rs`).
2. `send_attempt` POSTs to `{TRADEAUTOPSY_SERVER_BASE_URL}/api/daemon/journal/toolbar-capture/accept` with **Station Bearer only** (`authorize_brain`). Wire `x-user-id` / daemon secret stay local (A8).
3. `UpstreamClient` is built with **`.timeout(Duration::from_secs(10))`** (`agent/src/lib.rs`).
4. Live timing (Mac, Debug agent on `:9137`, Gate B signed in):
   - `GET …/bar/declarations` → **200 in ~1.4s** (Console reachable with same client/token).
   - `POST …/journal/toolbar-capture/accept` (local) → **202 in ~10.01s** exactly once per first attempt.
5. Reqwest timeout is classified as `AttemptOutcome::Retry { reason: "network: {err}" }`. After `AGENT_OUTBOX_MAX_ATTEMPTS` (default **5**) → `DEAD_LETTER`.
6. Local HTTP **202** means “queued after failed/incomplete immediate delivery”, **not** Console ACK. Prior harness treated 202 as PASS — false green.

TLS/DNS from the Mac to `https://www.tradeautopsy.in` is fine (HEAD/GET/OPTIONS without auth → **401** in <100ms; cert verifies). Apex `https://tradeautopsy.in` **308**→www; agent env already uses **www**.

## Payload check

`capture_accept_payload()` sends `draftText` + `explicitPending: true` + `idempotencyKey`. Console schema (`toolbarEscrowAcceptBodySchema`) accepts that for pending escrow; **image/R2 bytes are not required** at accept (presign/PATCH is a later phase). Incomplete image is **not** the dead-letter cause.

## Why Gate B / bar declare PASS but capture fails

| Path | Helper | Console route | Live result |
| --- | --- | --- | --- |
| Bar declare / list / debrief | `forward_daemon_json_with_optional_429_retry` (ensure_fresh + 401 refresh) | `/api/bar/v1/*` | **200** in <2s |
| Capture outbox drain | `authorize_brain` + raw `http.post` only (no ensure_fresh / no 401 refresh) | `/api/daemon/journal/toolbar-capture/accept` | **hangs ≥10s** → client timeout → `network:` |

So session JWT works for bar. Capture forward never gets an HTTP status — it times out. That is **path-specific Console latency/hang** (or stall after auth), not “unsigned-in”.

Console route (`app/api/daemon/journal/toolbar-capture/accept/route.ts`) uses `verifyDaemonRequest` (Station Caller JWT) then `executeToolbarEscrowAccept` (Neon insert + signal + BullMQ finalize). `maxDuration = 15`. Bogus Bearer → 401 in ~0.13s; valid Bearer from agent → no response before agent’s 10s cutover.

Historical `acked: 2` rows on this Mac outbox DB look like **Phase 3 test names** (`Phase 3 duplicate path` / `online finalize test`) and are **not** evidence of recent live Console ACK.

## Harness harden (done on Mac working tree)

File: `scripts/notch-live-journal-lib.py` (often untracked local copy).

When `--prove-capture`:

- Snapshot baseline `acked` / `dead_letter` / `enqueued` / `inflight`.
- POST accept; record `accept_seconds`.
- If sync 200 `accepted`/`duplicate` → PASS unless `dead_letter` rose.
- If 202 → poll outbox ~**55s**; PASS only if `acked` increases and `dead_letter` does not.
- FAIL → `report.outbox.prove_capture_verdict` + `errors[]` + **exit code 5**.

Docs: `plans/NOTCH-LIVE-JOURNAL-HARNESS.md`, decision log in `plans/INCOMPLETE-LANES-PLAN-2026-09-27.md`.

## Exact next work (coordinator)

### CloudAgent — Console (`FExEVIL/tradeautopsy`)

1. Reproduce under Station Bearer: why `POST /api/daemon/journal/toolbar-capture/accept` does not return within 10s while `/api/bar/v1/declarations` does.
2. Check Neon `pending_captures` insert, `touchStationPresence`, `ingestSignal`, `enqueueJournalCaptureProcessing` (Redis/BullMQ) for hangs; Vercel runtime logs for that route (MCP logs were 403 from this agent).
3. Confirm middleware vs route auth shapes; ensure success body stays `{ success: true, data: { status: "accepted"|"duplicate", pending_capture_id } }` with **HTTP 200** (agent ACK matcher).

### CloudAgent — Station (`FExEVIL/tradeautopsy-station`) agent Rust (optional but recommended)

In `agent/src/outbox.rs` `send_attempt`:

1. Reuse `forward_daemon_json_with_optional_429_retry` (or call `ensure_fresh_station_access` + 401 refresh) so capture matches bar.
2. Surface timeout distinctly (`timeout:` not `network:`) and consider raising timeout for this route **only after** Console hang is fixed.
3. Do **not** invent a full Console PR from Station; ship agent patch once Console responds.

### Re-verify

```bash
./scripts/notch-live-journal.sh --prove-capture
# expect exit 5 until Console/agent fix; then PASS with ackedΔ≥1 and dead_letter flat
```

No broker orders. Prefer leaving script patches on disk; Station agent PR via CloudAgent when ready.

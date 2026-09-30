# Manual fill → journal — Path A prove · 2026-09-29

Link strategy (accept body) is corrected in `plans/MANUAL-FILL-ACCEPT-LINK-2026-09-29.md`. Do not read a Station `202` or a local fill id as a Console Trades row.

**Lane:** Notch Working (`.armed` / `.livePlan`) or Post debrief · broker miss · no real orders.

**Declaration (reuse or fresh):** `0491f631-78d1-496b-b191-4ade1dacb0df` (PNB Breakout declare — same family as NOTCH-LIVE scoreboard Path A).

## What shipped (Station)

| Check | Detail |
| --- | --- |
| UI | `BarManualFillPanel` — symbol, qty, price, side, time; defaults from active `pendingDeclaration` |
| Local store | `recent_fills` row with `broker=manual` via agent `POST /api/daemon/journal/manual-fill/accept` |
| Journal rail | Same toolbar-capture outbox as live fills → Console `POST …/journal/toolbar-capture/accept` |
| Label | Top-level `journalFillSource: manual` on capture body + JSON line in `draftText` |
| Tests | `agent/tests/manual_fill_journal.rs`, `notch/Tests/NotchTests/ManualFillJournalDraftTests.swift` |

## Prove steps (founder / harness)

1. Gate B signed in on Debug agent (`127.0.0.1:9137`). Harness: `env -u AGENT_DAEMON_SECRET` only if your wire harness uses alternate auth — loopback capture still needs wire v1 from Notch/Station.
2. Open Notch on pending declare `0491f631-78d1-496b-b191-4ade1dacb0df` (Working).
3. **MANUAL FILL** panel → confirm PNB / qty / price / side / time → **Queue to journal**.
4. Expect toast: `Queued … (202 accept path)` or immediate accept when Console up.
5. `GET /api/daemon/journal/toolbar-capture/outbox/status` — new row acked or enqueued (not dead_letter).
6. `GET /api/daemon/toolbar/recent-trades` — row with `"broker":"manual"`, `"symbol":"PNB"`.
7. Screenshots: Working panel filled, outbox status, recent-trades JSON (redact tokens).

## API smoke (wire v1)

```bash
# After agent rebuild; replace wire headers with your signed request helper.
curl -sS -X POST "http://127.0.0.1:9137/api/daemon/journal/manual-fill/accept" \
  -H 'content-type: application/json' \
  -d '{
    "symbol":"PNB","side":"BUY","quantity":100,"price":95.5,
    "filledAtMs":1757000000000,
    "preTradeDeclarationId":"0491f631-78d1-496b-b191-4ade1dacb0df",
    "idempotencyKey":"path-a-manual-fill-prove-1"
  }'
```

Success envelope includes `data.journalFillSource: "manual"` and `data.tradeId` (UUID).

**No broker orders. No secrets in this note.**

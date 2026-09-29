# Manual fill accept link — prove

Declaration used on the pending path: `0491f631-78d1-496b-b191-4ade1dacb0df`

Console `POST /api/daemon/journal/toolbar-capture/accept` writes `pending_captures`. It does not insert a Trades row.

| Founder action | `tradeId` | `explicitPending` | Accept |
| --- | --- | --- | --- |
| Queue with no Console trade picked | `null` | `true` | `200` `accepted` (escrow pending). `preTradeDeclarationId` set when a declare is active. |
| Pick a row under “Link to today’s trade” | that Console `trades.id` | `false` | `200` `accepted` (escrow linked) |
| Outbox has not finished the first delivery | same body as above | same | Station `202` — local enqueue, not a Console accept |
| Blank draft / bad link / non-UUID `tradeId` | — | — | `400` `error.code`: `EMPTY_CONTENT`, `INVALID_LINK_STRATEGY`, `AMBIGUOUS_LINK`, or `VALIDATION_ERROR` |

Station-local fill ids stay on the Mac recent-trades row (`stationFillId` in the draft). They are not sent as `tradeId`.

`REDIS_URL` missing → `503` `JOURNAL_CAPTURE_QUEUE_*` is env on Vercel, not this change.

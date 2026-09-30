# Incomplete lanes plan — 2026-09-27

**Context:** Binance spot journal sink is PASS (declare `75ebed80-…`, Debrief notes, moods). Gaps below are what still blocks “full trade → Journal” honesty and launch Plan polish.

**Preference:** live E2E that proves data lands in Journal over more unit tests.

---

## North star

Prove one book end-to-end:

> Open/Plan declare → Working armed → **close/match** → Debrief (+ optional screenshot) → **Journal shows the full story**

Then repeat per shipping desk (Kotak cash, Binance Options, …).

---

## Track A — Journal completeness (highest leverage)

### A1 · Toolbar capture → Journal (Station harness)
**Why:** Scoreboard outbox counts did not move; no new screenshot attach proved for that declaration.  
**Done when:** Harness (or dogfood) runs accept → outbox pending→acked → Journal row shows capture linked to declare/trade id. Scoreboard section updates.  
**Depends on:** Gate B session; existing toolbar-capture routes.  
**Size:** S–M (harness flag/`--prove-capture` path + scoreboard assert).  
**Owner surface:** Station `scripts/notch-live-journal*` + Console capture APIs already in tree.

### A2 · Closed trip / match fidelity without real Binance fill
**Why:** Debrief notes work on `pending`; Journal still lacks matched/closed actuals.  
**Done when:** Env-gated loopback inject flips declaration to matched (FILL_MATCHED) + optional synthetic exit; harness `--inject-matched` + fidelity poll PASS; never on in prod by default.  
**Depends on:** Console contract in `FExEVIL/tradeautopsy` (sketch: `plans/FILL-MATCHED-INJECT-SKETCH.md`). Station harness only after Console lands.  
**Size:** M–L (Console first, then agent forward optional, then harness).  
**Order:** Spec freeze → Console PR → Station harness flag → scoreboard “Match fidelity” green.

### A3 · Optional real-fill dogfood (L4)
**Why:** Inject proves automation; one real small fill proves venue path.  
**Done when:** Human checklist: Plan → tiny order → Working → close → Debrief → Journal with match rows. Rare.  
**Size:** Human time; no code unless bugs found.

---

## Track B — Plan UI (product)

### B1 · PR #84 broker-scoped asset tabs (draft)
**Why:** Tabs must match supported books per desk.  
**Done when:** Mac `swift test` filters green; Binance shows Spot/Options/USDM/Coin-M only; Kotak Equity/Options only; BROKER chip off asset row; merge + cherry-pick to blip if needed.  
**Size:** S (verify + merge).  
**PR:** https://github.com/FExEVIL/tradeautopsy-station/pull/84

### B2 · Ticket height + Depth bid rows (draft PR)
**Why:** Ticket cramped vs Depth; ~6 bids feel thin.  
**Done when:** Ticket card ≈ Depth vertical weight; bids/asks balanced or more bid rows visible; no clip of qty/TIF.  
**Size:** S UI.  
**PR:** https://github.com/FExEVIL/tradeautopsy-station/pull/86 — Depth 2×2 beside Ticket 2×2; denser scrollable ladder; five mosaic rows.  
**Next:** Mac eyeball Debug Station Plan · Binance Spot · cockpit board.

### B3 · Future asset classes (not now)
Kotak Futures / CDX / MCX (etc.) **only when books ship**. No empty tabs.

---

## Track C — Tools honesty by book

### C1 · Binance Spot (mostly done)
Quote/history/depth WORKING; Options-family tools HONEST-DARK on spot — keep as non-fail.

### C2 · Binance Options book live matrix
**Done when:** On Options + dated contract, chain/OI/greeks/index scored WORKING or honest-dark with correct book id; DualNoBlend holds.  
**Depends on:** Options desk bind + masters.

### C3 · Kotak cash (+ NFO Options later)
Same journal-sink + tools matrix on Kotak slug; scoreboard dated run.

---

## Track D — Testing pyramid (durable)

| Phase | Item | Status |
| --- | --- | --- |
| B | Live journal harness + scoreboard | Mostly done (spot) |
| B+ | Capture assert in harness | = A1 |
| B+ | Match fidelity lane | = A2 |
| C | dogfood-suite contract expansions | Later |
| D | Nightly live job | Later |
| E | Human matrix per book | With A3 / ship |

Settings / Station desk / analysis rail (Match, Patterns, Fidelity, Triage): schedule **after** A1–A2 so Journal truth stays the spine.

---

## Suggested sequence (default)

1. **B1** — Land/verify PR #84 (quick, unblocks honest Plan chrome while testing).  
2. **A1** — Prove capture → Journal on harness.  
3. **A2** — FILL_MATCHED Console + harness (biggest Journal gap).  
4. **B2** — Ticket/Depth layout polish.  
5. **C2/C3** — Options + Kotak live scoreboards.  
6. **D** — Nightly + human matrix once A is green.

Parallel OK: B2 anytime after B1; C2 can start once Options desk is signed in.

---

## Explicit non-goals (this plan)

- Treating `swift test` green as “features work.”
- Showing unsupported asset-class tabs.
- Real broker orders from the automated harness.
- Expanding BrokerCatalog into a fake per-book table (decision B holds until core lane).

---

## Decision log (fill as we go)

| Date | Decision |
| --- | --- |
| 2026-09-27 | Journal sink Debrief does not require closed trip; match fidelity is a separate lane. |
| 2026-09-27 | Asset tabs = supported books only; no CDX/MCX until books exist. |
| 2026-09-27 | Sequence locked: B1 → A1 → A2 → B2. |
| 2026-09-27 | **B1 done:** PR #84 merged FExEVIL (`0bf198c`); cherry-picks on blip (`c171000`/`3755fcb`); Mac Notch tests PASS (`supportedDeclareAssetClassesFilterByDeskSlug`, `reconciledDeclareAssetClassSnapsToDeskDefault`, `switchingExecutionDeskSnapsUnsupportedDeclareClass`). Eyeball checklist still useful when rebuilding Station. |
| 2026-09-27 | **A1 live run:** Gates A/B/C PASS; declare `b1bbc7c7-79ae-4fd1-989c-d6200dae016a`; Debrief PASS; `--prove-capture` accept HTTP 202 success; outbox enqueued 0→1 (acked still 2). Full acked/R2/Journal-row still to confirm after drain. |
| 2026-09-27 | **A1 outbox dead-letter RCA:** Local accept 202 @ ~10.01s = agent `UpstreamClient` timeout on first Console forward; outbox retries → `DEAD_LETTER` reason `network: error sending request for url (https://www.tradeautopsy.in/api/daemon/journal/toolbar-capture/accept)`. Bar `/api/bar/v1/*` still 200 in <2s (same Bearer/client). Payload (`draftText`+`explicitPending`) is complete for escrow — not an image-bytes gap. Harness `--prove-capture` now polls drain and **exit 5** on dead_letter/stuck. Unblock = Console accept hang + optional Station outbox align to `forward_daemon_json_with_optional_429_retry`. See `plans/TOOLBAR-CAPTURE-OUTBOX-DEADLETTER-2026-09-27.md`. |
|  | *Next: CloudAgent Console (why accept hangs under Station Bearer) + Station agent outbox send_attempt harden; then re-run `--prove-capture` before A2.* |
| 2026-09-27 | B2 started — Ticket height + Depth bid rows PR. |
| 2026-09-27 | B2 draft PR #86 opened (agent errored after PR; work landed). |

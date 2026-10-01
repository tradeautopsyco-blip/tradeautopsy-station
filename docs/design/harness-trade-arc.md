# Pre → Working → Post, many trades at once

**Status:** exploration. No new quotes, no invented P&L, no order send.
**Date:** 2026-10-01
**Layout of each screen:** [`harness-open-plan-working.md`](./harness-open-plan-working.md). **Sizing:** [`risk-engine.md`](./risk-engine.md).

The harness is a session of **many** plans and positions. Open, Plan, Working, and Debrief are a list plus the row you have selected. DualNoBlend still means one book’s currency on a strip. It does not mean one ticket.

---

## 1. What is already many, and what is still one

| Already a list | Still one slot |
| --- | --- |
| Console declare is one POST per Confirm (`POST /api/bar/v1/declarations`). Week read is `GET /api/daemon/bar/declarations` (`agent/src/api/bar.rs`). | Local LiveBook keeps a single `notch.pending_declaration`. `LiveBookEvent::Declare` **replaces** that object (`agent/src/live_book.rs` `insert`). A second Confirm on this Mac drops the first local row. |
| Journal N2 is keyed by `declaration_id`: debrief fields and `journal_n2_condition_fire` rows (`agent/src/journal_n2/store.rs`). | `matched_declaration_id` is one string. `undeclared_position` is one object. A second fill that does not match the pending symbol/side **overwrites** it. |
| `GET /api/daemon/positions` returns an array (`agent/src/api/positions.rs`). No `unrealizedPnl`. Rows have no declaration id. | `plan_state` is one string on the book, not on each declaration. Empty string is painted as GREEN (“Plan intact”) in `BarPlanStateView`. |
| Account chrome can list several position rows. `deskTicketByBook` is per book. | Notch `BarLiveStateResponse.pendingDeclaration` is one optional. `workingInvalidationWasBreached` is one bool (`NotchViewModel`). |
| Condition fire POST takes a `declaration_id` (`/api/daemon/journal/condition-fire`). | `CaptureWorkingCondition` writes `condition_at_close` only on the single pending snapshot, and only once. |
| | `BarLiveTradeDeclareCTA` hides “Declare before trade” when **any** pending exists. You cannot start another plan from Working while one is pending. |
| | `BarPostTradeView` is one form. It does not list closed tickets. Cited net is `trips.last` for that symbol (`BarDebriefCitedNet.net`), so two trades in the same name collapse. |
| | Manual fill defaults from that one pending (`BarManualFillPanel`). |

`reconcile_replaces_local_id_not_a_second_arm` is about archive id swap on the same row. It is not a second ticket.

**Remove these single-trade assumptions:** one `pending_declaration`, one `undeclared_position`, one `matched_declaration_id`, one `plan_state` for the whole book, one breach flag, declare-CTA hidden because a pending exists, debrief with no picker, cited net by symbol-only.

**Keep:** one money owner per book, DualNoBlend on the strip, Confirm still fail-closed, Kill still the manual door, no `routeOrder`.

---

## 2. What each stage is for

| Stage | Job | Many trades |
| --- | --- | --- |
| **Pre** (Open + Plan) | Decide the next ticket quickly. Open is the session (rule, clock, overnight count). Plan is the form. | Open lists open tickets and undeclared rows. Plan’s Confirm **appends**. The form clears for another symbol. It does not load the live ticket unless you chose Edit on that row. |
| **Working** | Help the trades that are live: what each one is, whether a condition fired, what to do on the one you selected. | List every pending plan, matched fill, and open inventory row. Detail is the selection. “Plan another” stays on the list. |
| **Post** (Debrief) | After a ticket closes: what the book of record says happened, then whether you did what that ticket’s snapshot said. | List closed tickets for the day (the week declaration list is the archive). The Moments form binds to the selected id. |

Kill stays session-level. It is not a property of one row.

---

## 3. Plan while something is live

Recommended path:

1. Working list shows the live rows. A button “Plan another” opens Plan with an **empty** form.
2. Confirm runs the same `POST /api/daemon/bar/declare` (already one archive row per call).
3. Local apply **pushes** a row into `pending_declarations[]` instead of replacing `pending_declaration`.
4. The new row is selected. The previous row stays in the list with its own snapshot, fills, and fires.
5. Cancel and protective updates take a `declaration_id` and touch only that row. They already receive an id; Cancel already no-ops when the id does not match. The bug is the missing list, not the id check.

Alternative: block a second declare until the first is flat. That matches today’s CTA, and it is the behavior to remove. Two plans on one symbol stay two rows (two ids). Do not merge them because the symbol matches.

DualNoBlend: the Plan strip and the risk preview use the **form’s** `book_id`. A live INR row does not change a USDT form, and the list does not add the two P&Ls into one percent.

---

## 4. Working as a list

Each row: symbol, side, book id, state (`declared` / `partial` / `in position` / `undeclared`), and a condition chip if any fire exists for that id.

Selection loads the detail from [`harness-open-plan-working.md`](./harness-open-plan-working.md): declared-not-filled, or in a trade. The GREEN banner is that row’s `plan_state` only. An empty state is not “Plan intact.”

Inventory rows from `/api/daemon/positions` have no declaration id today. Show them as undeclared until a fill match writes the id. Do not invent a link from symbol alone when two declarations share a symbol.

`composite` “at risk across open positions” can stay a **session** line above the list when `worstCase` is present. It is not the selected row’s stop.

Live capture and match fidelity attach to the selected id. The outbox error stays inside that row’s capture disclosure.

---

## 5. Conditions — users set them, the system tracks what it can

A condition belongs to one `declaration_id`. Fires already do (`rule_id` + `fired_at_ms` in N2). The tracker must not use the single `workingInvalidationWasBreached` flag for every row.

### Authored on Plan (frozen on that declaration)

The declare body already stores stop, target, and invalidation (`BarIntradayDeclarationPayload`, `plan_snapshot_from_s1`).

| Kind | Chip today | What the code actually tracks |
| --- | --- | --- |
| Price invalidation | `BarInvalidationKind.price` | `working_vs_invalidation` compares obtain **last** to `invalidation_price` (buy: last ≤ price is breached). One-shot fire `rule_id` `invalidation_price` (`NotchViewModel.syncWorkingConditionFireEdge`). Unbound last stays dark (`BarWorkingCompare`). |
| Target | `target_price` | `working_vs_target` sets `target_state` `touched` or `intact` on `condition_at_close`. No fire row yet. |
| Stop price on the plan | `stop_loss` | Stored. **Not** compared to last. “SL hit” is a different number from invalidation unless the trader typed the same price. Broker stop is not placed (`place_sl` is cut). |
| Time / behaviour / context | `BarInvalidationKind` | `working_vs_invalidation` returns `waiting`. The line is kept. Nothing may mark them breached from a quote. |

`condition_at_close` freezes last, invalidation state, and target state once, on the single pending snapshot (`CaptureWorkingCondition`). Per row, that freeze still makes sense at session end. It must key off the declaration id, and a second ticket must get its own snapshot.

### Added on Working (same row)

Recommended: the selected row can append a condition of a kind the table already knows (another price level, or a time/behaviour/context line). Append stores it on that declaration’s snapshot and, when the kind is price or target, runs the same compare. Time, behaviour, and context stay lines the trader marks, because no quote can prove them.

Alternative: only Plan may author conditions. Simpler contract. Worse in a live trade, which is when the founder asked to add a condition. Prefer append-on-Working, frozen copy still from Plan.

### First cut the system may mark by itself

Only when the input exists. Otherwise the chip stays `waiting` or `dark`.

| Condition | Honest input | Fire id (existing or new) |
| --- | --- | --- |
| Price invalidation breached | Bound last and a price on **this** id | `invalidation_price` (exists) |
| Target touched | Bound last and `target_price` | New `target_touched`. State already computed, not logged. |
| Plan stop touched | Bound last and this row’s `stop_loss` | New `plan_stop_touched`. Not the broker order. |
| Session end vs those levels | Same last, written once | `condition_at_close` per id (exists on the single snapshot) |

Do not auto-fire: time, behaviour, context, “I scaled in”, emotion, broker SL fill, fees, R:R. Moment B already asks the human those process questions.

Size vs fill: if this id has `filled_qty` and a declared `quantity`, Debrief can show both numbers. That is a fact. It does not check the Moment B toggle for them.

### Where they show

| Surface | |
| --- | --- |
| Working list | Chip per row: intact / breached / touched / waiting / dark. |
| Working detail | Levels for the selection, last with honesty chip when unbound. |
| Debrief, selected closed id | System lines (fires + `condition_at_close`) above Moment B. Moment B stays the human yes / no / unset. A fire does not flip the toggle. |

Console: N2 fires stay local unless a later contract says the week payload must include `condition_fires`. Today `enrich_week_declarations` merges `n2_day_sheet` onto the list Station already gets. Debrief PATCH still goes to `/api/bar/v1/post-trade-debrief` and is stored locally (`upsert_debrief_patch`). Do not invent a second Console condition table from this note. Verify the week item still carries `id` so the picker can use it.

---

## 6. Debrief’s job

The shot (`BarPostTradeView`) opens on an empty manual-fill form, then Moment A’s dash, then Moment B. That reads as data entry. The job is a closed ticket.

**Manual fill** is the broker-miss lane: symbol, qty, price, side, time, optional Console trade id. Copy already says it queues a journal row and sends no order (`BarManualFillPanel`). It is not Moment A. On a multi-trade Debrief it sits behind “Broker missed a fill” and, if opened from a row, defaults from **that** id. The empty form is not the page.

**Moment A — outcome.** Cited Today trip net for **this ticket’s id**, not “last trip with this symbol.” `BarDebriefCitedNet` currently matches symbol and takes `trips.last`. Two RELIANCE tickets would show the same net. Until trips carry a declaration id, show **—** when more than one trip shares the symbol, and show the figure only when the match is unique. NFO and options stay **—** (`display` returns a dash). The optional note stays. Do not compute a second P&L.

**Moment B — process.** Five tri-states against **this** snapshot: stop as declared, size as declared, invalidation respected, exit per plan, no impulsive add. Honest no is allowed. Emotion-out 1–5 is required to continue. System fires sit beside these rows so the trader can disagree with the chip.

**Moment C — one sentence.** Cooling / scratch vs clean win, required note, then save. PATCH is the archive step (`post_trade_debrief_handler`).

Unposted charts stay “not part of Moments A–C” (`unpostedChartsCard`), linked to the selected trade id.

Picker: closed rows from the week list (and any local N2 row). Opening Debrief with no selection shows the list, not Moment A.

---

## 7. Follow-up order

1. LiveBook: `pending_declarations` array. Declare appends. Cancel, protective, fill-match, and condition capture take an id. Tests that a second declare keeps the first id.
2. Notch model: list + `selectedDeclarationId`. Remove the single breach flag. Declare CTA stays while other rows exist.
3. Working list and per-row chips for the conditions in §5 that already have a compare.
4. Debrief picker. Cited net unique-or-dash. Manual fill demoted. Moment B reads the selected snapshot. Fires for that id listed above the toggles.
5. New fire ids `target_touched` and `plan_stop_touched` only as logs of the compares above. No new price source.

Out of this note: blending books, placing stops, inventing last, inventing net, one-ticket Working.

**Later founder queue (not LiveBook):** [`founder-backlog.md`](./founder-backlog.md).

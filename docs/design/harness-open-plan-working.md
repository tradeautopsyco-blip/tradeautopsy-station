# Harness Open / Plan / Working — layout options

**Status:** exploration. This note does not change a formula, invent a quote, or move Confirm’s required fields.
**Date:** 2026-10-01
**Screens:** founder shots of Open, Plan, and Working on Kotak Neo, NSE closed, agent offline, two obtain holes.

The three sidebar items already have one job each in `notch/BarNotchShell.swift`: Open (`morning`), Plan (`pretrade`), Working (`live`). The paint mixes those jobs, so each screen looks like a second copy of the others.

**Multi-trade correction (2026-10-01).** Open, Plan, Working, and Debrief are a list plus a selection, not one live ticket. Section 3’s flat / declared / in-trade split is the detail of the **selected** row. A second Confirm must not replace the first. The product arc, conditions, and the single-slot code to remove are in [`harness-trade-arc.md`](./harness-trade-arc.md). DualNoBlend stays per book: two tickets on one book share that book’s currency; another book’s numbers stay off the strip.

| Screen | Job already named in code | What the shot actually shows |
| --- | --- | --- |
| Open | Session start. `BriefLeftView` in `notch/TabViews/BriefTab.swift`. Start jumps to Plan (`startTradingFromMorningBrief`). | Book card, index zeros, an empty “What matters” card, calm/confidence, a rule, Start trading. |
| Plan | Declare. Cash rail `notch/BarCashCockpitPlanRail.swift`. | Three unavailable tiles, then four emotion scales, then Numbers below the fold. |
| Working | After declare. `notch/BarPlanStateView.swift`. | “Plan intact” with no plan, an at-risk line, a declare button, capture, a 0% match, Kill. |

---

## 1. Open — session start

### Why it feels confused

The top bar already says Kotak Neo and NSE closed. The card under it says **No book** because `openDeskHeader` prints `selectedMarketBookId`, and that string is empty. Those are two different facts.

Nifty, BNF, and VIX show **0.00** when `morningBrief` is nil. `NotchViewModel.niftyValue` (and BNF, VIX) is `morningBrief?.… ?? 0`. That zero is a missing brief, not a print. `BarOpenStartGate.hidesIndexChips` already hides the row on Binance books. On Kotak it stays, and it paints the fallback.

“What matters” is the morning-brief body. With no brief the card is the sentence “No session series yet.” (`BriefLeftView`).

Start gate C1–C5 and K1–K5 write the same `declEmotionalCalm` and `declEmotionalConfidence` Plan asks again. The card’s own copy says Plan still asks frustration, excitement, stance, and the 7-box (`BarPlanGateStrip`).

### Recommended Open

One card, then one rule, then Start.

| Keep | Where it already lives |
| --- | --- |
| Connected desk name (the top-bar broker), session clock, quote currency | Clock is `sessionClockPresentation`. Currency is `formatQuoteCurrency`. Prefer the broker label the top bar already has when `selectedMarketBookId` is empty. |
| Overnight count, and the undeclared line when live-state has one | `positions` and `barLiveState?.undeclaredPosition` |
| Floor line, including “does not fire Kill” | `dailyLossLimit` |
| One non-negotiable sentence | `openNonNegotiable`. Start stays disabled until it is non-empty (`BarOpenStartGate.unlocked` can drop the calm/confidence checks). |
| Start trading → Plan | `startTradingFromMorningBrief` |

| Cut from this screen | Why |
| --- | --- |
| Pre-market Nifty / BNF / VIX | Hide the row unless the morning-brief payload actually contains those numbers. Do not paint `0`. |
| What matters / key metrics / patterns | Those blocks already render only when `morningBrief` exists. The empty card goes away with the brief. When a brief arrives, this is the right screen for it. |
| C1–C5 and K1–K5 | Same two integers as Plan’s state check. Ask them once, on Plan. |

| Move | |
| --- | --- |
| Yesterday’s closed figure | Into the header line when `morningBrief?.sessionPnLKpi` exists. Omit the “Yesterday Today —” line when it does not. |

**Rename:** leave the sidebar word **Open**. Rename the inner “Start gate” card to the rule itself (“One rule for today”). “Start trading” can stay; it only opens Plan.

### Alternative

Leave calm and confidence on Open and remove them from Plan’s four scales. That saves a tap on Plan and splits one check-in across two screens. The shot’s problem is the split. One screen should own the four readings.

A third path — Start always enabled, no rule — drops the D gate the roadmap still lists as open (`plans/station-completion-roadmap-2026-09-17.md`, founder lock 2). The rule is one field. The scales are the duplicate.

---

## 2. Plan — faster declare

### Why Numbers are late

`BarCashCockpitPlanRail` order is: funds row, **State check** (`BarPlanEmotionCheckView`, four scales until all are filled), **Numbers**, setup, invalidation, intent, **Gate** (`BarPlanGateStripView`, five acks plus emotion and exit). The left mosaic (session, depth, funds, last, history) is a second column of holes. The shot’s “unavailable” and “no licensed series” are obtain honesty, not placeholders to replace with a fake chart.

Confirm still needs, via `BarIntradayDeclareValidator.submitReadiness`: symbol, quantity, stop, target, invalidation, four emotions, stance, intent, the gate acks, and protective-SL consent on spot/equity. This note reorders and compacts. It does not delete those gates.

### Recommended sequence

**Now, top of the rail (the trade):**

1. Symbol, side, quantity, entry, stop, target. This is today’s Numbers block, moved above state check.
2. Risk strip beside those prices: planned risk, fees, reward, R:R, proposed size. Same envelope as [`risk-engine.md`](./risk-engine.md). Size and fees stay dashed while the reference doc says the formula is unspecified. The strip uses the numbers just typed. It does not add a new required field.
3. State check as **one row of four chips** once a scale is chosen, and a single compact picker while it is empty. `BarPlanEmotionCheckView` already collapses to `compactSummary` after all four are set. Do that collapse without a paragraph of copy above Numbers.
4. Setup chip, invalidation, one-sentence intent.
5. Gate as one wrapping row of the existing acks (capital, size at the stop, still inside the day, hedge, debrief, protective SL). `BarPlanGateStrip.emptyHint` already names the first empty box under Confirm.
6. Confirm.

**Later, not in the way:**

| Tile | Treatment |
| --- | --- |
| Funds, last, history, depth | One honesty chip in the rail header when status is unavailable (`HonestyChip` already exists on `BarShippingFundsPlanRow`). Expand the mosaic only when obtain is success. |
| Session chart “no session series” | Same chip. Do not keep a large empty well. |
| Style tabs (Intraday / Scalper / Swing) and Cockpit / Hero / Focus | Stay, above the rail. They pick a template. They are not the form. |

### Faster path vs a shorter form

| Path | What you gain | What you give up |
| --- | --- | --- |
| **Reorder and compact (recommended)** | Symbol and stop are first. Emotion and the 7-box stay required, in less vertical space. | First declare is still a full ticket. |
| Drop frustration, excitement, and some acks | Fewer taps. | `submitReadiness` and the declare JSON (`mood_frustration`, `mood_excitement`, `gate_strip`) would change. That is a product cut, not a layout cut. |
| Wizard (numbers → emotion → confirm) | One decision per step. | More taps on the happy path. Worse for a second trade the same day. |

---

## 3. Working — in a trade, or not

### Why the shot looks like three screens

`BarPlanStateView.body` always stacks, in order:

1. `planStateBanner`. Empty `plan_state` is treated as GREEN. Fallback copy is “Plan intact. Nothing to do.” (`fallbackSentence`).
2. Composite line “at risk across open positions” / “% of daily limit” when `composite` is present and there is no plan snapshot (`compositeRiskLine`). A zero budget still prints **0% of daily limit**.
3. “Declare before trade” when there is no pending declaration (`BarLiveTradeDeclareCTA`).
4. `BarLiveCaptureCard` on every visit (voice, screenshot, outbox status).
5. `BarEscrowMatchView` when embedded (the 0% fidelity ring).
6. Kill, when `BarDeskTemplate.allowsVenueProtectivePlace` (`planKillSection`).

So a flat desk still speaks in the voice of an open trade.

### Recommended three states

These are the detail of the **selected** row. The screen also lists every open plan and position (see [`harness-trade-arc.md`](./harness-trade-arc.md)). “Plan another” stays available while a row is live. Paint one detail state for the selection. Do not stack the other states’ chrome on that row.

**Flat** — the list is empty.

| Show | Hide |
| --- | --- |
| One line: “No open trade.” | Plan-intact banner |
| One button: “Plan a trade →” (today’s declare CTA) | At-risk bar |
| | Match / fidelity |
| | Live capture |

Kill stays a text button in the footer, same warning card as `BarPlanKillChrome`. It is not the next action.

**Declared, not filled** — the selected row is a pending declaration with no fill.

| Show | |
| --- | --- |
| Symbol, side, quantity, stop, target from `pendingDeclaration` | |
| “Declared. Not filled.” | |
| Cancel declaration (`BarCancelDeclarationChrome`) | |
| Last vs invalidation only when last obtain is success (`BarWorkingCompare`) | |

Plan-intact / watch / broken only when `plan_state` is actually GREEN, AMBER, or RED. An empty string is not GREEN.

**In a trade** — the selected row is an open position or a fill on that declaration.

| Show | |
| --- | --- |
| The real plan banner, including “Plan intact. Nothing to do.” when the server says GREEN | |
| Last vs stop / invalidation | |
| At-risk line only when `composite.worstCase` is present | |
| Match row (escrow) | |
| Kill footer | |
| Live capture, collapsed under “Add a chart” | The outbox error stays inside that disclosure. It is not the page. |

### Alternative

Keep the current stack and only change copy (“Plan intact” → “No plan yet”). The declare button, the 0% bar, and the match ring would still compete. The three-state split is what makes “what next” one button.

Capture moving to Debrief only is the other split. Working would be cleaner, and a chart grabbed during the trade would need a jump to Debrief. Collapsing it on the in-trade state keeps the shortcut (`requestLiveCaptureScreen` already opens Working) without making it the flat-state headline.

---

## 4. What a follow-up can change without new data

1. Open: stop painting index `0` when the brief is nil. Drop C/K scales from `BriefLeftView`. Leave the rule and Start.
2. Plan rail: Numbers, then the risk strip, then a compact state check, then gate, then Confirm. Unavailable mosaic tiles become one chip.
3. Working: branch the body of `BarPlanStateView` on flat / declared / in-trade. Empty `plan_state` does not use the GREEN fallback.

Tests that already pin the pieces: `BarWave2OpenTests`, `BarWave2PlanFieldTests`, `BarWave2WorkingTests`, `BarPlanKillChromeTests`, `BarLiveTradeDeclareCTA` behavior via plan-state tests. A layout change should keep Start disabled until the rule is set, keep `submitReadiness` failing closed, and keep Kill behind its warning card.

Out of this note: no new quotes, no default Nifty, no sizing formula, no order send.

# Risk engine and position sizing — options

**Status:** partial — `POST /api/daemon/risk/preview` + Plan risk strip (Tradeture layout, honest dashes). `authored_qty` stays null until `POSITION-SIZING.md` cites a formula.
**Date:** 2026-10-01
**Audience:** the next coding agent and the founder.

Station already runs many books. A risk calculator that only knows one instrument will fight that. This note maps what is on disk, compares three ways to build a multi-asset sizer, and shows how a Tradeture-style preview would sit on Notch and Station without inventing a fee schedule or a broker order.

Foundation for any later formula: [`docs/reference/behavioral-science/POSITION-SIZING.md`](../reference/behavioral-science/POSITION-SIZING.md). That file records the gaps. It is not a sizing rule.

Harness layout (Open / Plan / Working), separate from the formula: [`docs/design/harness-open-plan-working.md`](./harness-open-plan-working.md). Multi-trade Pre → Working → Post, including Debrief and live conditions: [`docs/design/harness-trade-arc.md`](./harness-trade-arc.md).

---

## 1. What already exists

### 1.1 Two different “risk” stores

| Store | Where | What it does today |
| --- | --- | --- |
| Desk rules | `notch/DeskRulesStore.swift` (UserDefaults). Station paints them in `station/StationApp/Presentation/DeskRulesPresentation.swift`. Notch reads them in `notch/BarDeskRulesReadout.swift`. | Daily floor, mean loss, max round trips. Display and a Today “remaining” preview. `DeskRulesPresentation.lossLimitsRequest` returns nil. These numbers do not POST loss-limits and do not arm DNS. |
| Console loss-limits | Notch Settings `notch/BarSettingsView.swift` `saveLossLimits()` → agent `GET/POST /api/daemon/bar/profile/loss-limits` → Console `/api/bar/v1/profile/loss-limits` (`agent/src/api/bar.rs`, `station-wire/v1.json`). | Body keys the Notch already sends: `daily_loss_limit`, `weekly_loss_limit`, `margin_utilization_cap_pct`, `acknowledge_bar_features_and_loss_limits`. GET expects `limits.dailyLossLimit`, `weeklyLossLimit`, `marginUtilizationCapPct`. This is the declare-activation profile. It is account-scoped in the payload Station sends. It is not per book. |

`notch/NotchViewModel.swift` `dailyLossLimit` is the Console figure. `DeskRulesStore.dailyFloor` is the local figure. `notch/BarPlanSizer.swift` prints the local floor as “Planned risk (floor)” and always prints proposed size as `—`, with the footnote that no reference doc exists.

`notch/AccountImpact.swift` turns pinned unrealized P&L into a percent of `dailyLossLimit`. It is a chip, not a sizer.

### 1.2 Plan measurement that is already arithmetic

`notch/BarPlanLadder.swift` measures a **typed** quantity:

```text
rung1 = units × (stop − entry) × sign
sign = +1 BUY, −1 SELL
max planned loss = abs(rung1) only when rung1 < 0
```

Tests: `notch/Tests/NotchTests/BarPlanLadderTests.swift`. Missing units, entry, or stop yields nil. A stop on the profitable side does not invent a max loss.

`notch/BarDeclarationFlowView.swift` puts that loss on the declare body as `max_planned_loss_inr` for non-options. Options require a typed max planned loss. Quantity is typed (or `quoteOrderQty` on a quote-size ticket, or the first option leg’s lots). The sizer does not write that quantity.

R:R coloring lives in `notch/BarIntradayDeclareValidator.swift`: `reward / risk` from entry, stop, target, and side. Bands are display (≥ 2 favorable, < 1.5 weak). The completion roadmap still lists “hard gate vs warn + tag” as an open founder lock (`plans/station-completion-roadmap-2026-09-17.md`, “Founder locks still open”, items 3 and 4).

Confirm is blocked when Console live-state sets `declaration_submit_blocked` (`notch/BarNotchModels.swift`). Copy for that block is `notch/BarDeclarationSubmitBlockedPresentation.swift` (kill switch, composite RED, stop-me, protective SL).

### 1.3 Declare, LiveBook, Console archive

Notch builds one JSON object in `notch/BarIntradayDeclarationPayload.swift` and POSTs ` /api/daemon/bar/declare`.

The agent applies it locally first (`agent/src/api/bar.rs` `declare_handler`, `agent/src/live_book.rs` `plan_snapshot_from_s1`), then forwards the same body to `POST /api/bar/v1/declarations` with the Station Caller Bearer (`agent/src/lib.rs` `UpstreamClient`). A Console 5xx keeps the local pending row and returns `archive_error`. A Console 4xx (except 401) cancels the local row.

Snapshot fields already copied: symbol, side, quantity, stop, entry, target, book id, product, invalidation, stance, intent, emotion scales.

Protective stop updates LiveBook and forwards `POST /api/bar/v1/protective`. Plan chrome does not emit `place_sl` (`notch/BarProtectiveSlPlanChrome.swift`, `showsSetSlButton` is false).

### 1.4 Positions, funds, margin, kill

| Surface | Source | Honesty rule already in code |
| --- | --- | --- |
| Fill inventory | `GET /api/daemon/positions` (`agent/src/api/positions.rs`) | Symbol, qty, side. No `unrealizedPnl`. Comment: clients must not treat a missing mark as zero. |
| Account chrome | obtain on the book | `notch/BarAccountChrome.swift` is one book, one currency. Foreign book envelopes are dropped. |
| Funds glance | `GET /api/station/obtain` | `notch/BarPlanSizer` takes `shippingFundsGlance.isLit`. Dark funds dash the size emphasis. Kotak cash funds are RMS limits (`docs/reference/india/kotak-neo/FUNDS-LIMITS.md`), not a margin calculator. |
| Margin estimate | `agent/src/data/margin_estimate.rs` | Always `HonestyStatus::Unavailable`, reason `margin_calculator_unspecified`. |
| Futures leverage | `BarAccountChrome.futuresMargin` | Mode, leverage, liquidation from a position row when the envelope is success. Flat or omitted fields stay nil. A test forbids inventing `50` (`BarDeskTicketTests.flatUsdmMarginIsNoneNotInventedFiftyX`). |
| Kill | `agent/src/api/kill_switch.rs`, `agent/src/kill_policy.rs`, `agent/src/kill_latch.rs` | Manual fire and Console `fog_of_war` / `clear_fog` (`station-wire/v1.json`). Policy row is level, countdown (default 90s), website block. Floor does not write this row. |
| Behavioral verdict | live-state fields `behavioral_score`, `behavioral_verdict` | Notch paints them (`notch/BarBehavioralVerdictPresentation.swift`). The agent crate does not compute them. Wire file records score thresholds `0.25 / 0.35 / 0.45` in `station-wire/v1.json`; no agent match consumes those constants. |

UBI stays read-only. `agent/src/data/host_policy.rs` `is_mutation` treats place / modify / cancel, withdraw, transfer, and `/leverage` as mutations. ADR 0002 forbids those from the data interface.

### 1.5 Console, as Station already speaks to it

This run could not read `tradeautopsyco-blip/Tradeautopsy-console` (GitHub 404 for the available credential). The contract below is what Station sends and what `station-wire/v1.json` records.

| Hop | Path |
| --- | --- |
| Notch → agent | Wire-v1 HMAC on `127.0.0.1:9137`. Machine integrity. `x-user-id` is not Console identity. |
| Agent → Console | `Authorization: Bearer` Station Caller JWT. Base `TRADEAUTOPSY_SERVER_BASE_URL`. |
| Declare archive | `POST /api/bar/v1/declarations` |
| Loss-limits | `GET/POST /api/bar/v1/profile/loss-limits` |
| Live state | `GET /api/bar/v1/live-state` |
| Protective | `POST /api/bar/v1/protective` |
| Kill command | Agent polls `GET /api/daemon/command` for `fog_of_war` / `clear_fog` |

`plans/kill-switch-lifetime-spec.md` names Console files this agent did not re-open: `lib/pre-order/pre-order-guard.ts`, `bar-intervention-route-gate.ts`, `app/api/intelligence/kill-switch/route.ts`. Treat those as a verify-list for a Console owner, not as a Station change.

R4.1, already written in `plans/station-completion-roadmap-2026-09-17.md`: Station computes, the Enforcer holds teeth, Console is the deep archive. S2 in `TRADEAUTOPSY-CONTEXT.md`: one owner per number.

---

## 2. Multi-asset support already in the product

### 2.1 Book, not slug, not a single instrument

ADR 0004 (`docs/adr/0004-ubi-data-taxonomy-book-keyed-catalog.md`) is the taxonomy:

- `asset_class` ∈ fx, equity, commodity, debt, index, cryptocurrency, alternative
- `instrument_class` ∈ spot, swap, future, futures_spread, forward, cfd, bond, option, option_spread, warrant, sports_betting, binary_option
- `is_inverse` is a boolean (Coin-M)
- Host stamps the triple from the **book**. Adapters do not classify.
- Unknown class is rejected.

`agent/src/ubi/catalog.rs` `catalog_books()` is one row per book: slug, axes, quote currency, `calc_profile_id`, `book_id`.

`agent/src/money_matrix.rs` maps **22 SHIPPING books** to one realized-P&L owner each. Same physics share an owner. Options are an explicit none (`agent/src/options_realized_pnl.rs`).

| Calc profile (catalog) | Books (examples) | Realized owner | Quantity the desk already uses |
| --- | --- | --- | --- |
| `equities_inr_cash` | kotak, zerodha, upstox, fyers, dhan, groww `*-nse-bse-cash` | `agent/src/inr_cash_wac.rs` | Shares. Cash product CNC or MIS on the Kotak declare path. |
| `equities_inr_nfo` | those slugs’ `*-nse-nfo` | `agent/src/nfo_realized_pnl.rs` — `(exit − entry) × qty × lot`, INR, segment `nse_fo` | Lots. Lot from the venue instrument row, not a guessed file (`notch` comments and NFO dogfood plans). |
| `fx_cds_inr` | `kotak-nse-cds` | `agent/src/fx_cds_realized_pnl.rs` | Currency contract spec. ADR 0022 is draft; charge drills stay blocked while the lock is draft. |
| `commodity_inr_mcx` | `kotak-mcx-future` | `agent/src/mcx_realized_pnl.rs` | Lot/tick pipeline is ADR 0023. |
| `crypto_spot_usd` | binance, bybit, okx, kraken, coinbase `*-spot` | `agent/src/round_trip_engine.rs` | Base qty, or quote `quoteOrderQty` (`notch/BarDeskTicket.swift` `BarTicketSizeMode`). Step from `VenueLotTick` / exchange filters. |
| `crypto_usdm_usd` | `binance-com-usdm` | `agent/src/usdm_realized_pnl.rs` | Contracts. Ticket type can be conditional (`/fapi/v1/algoOrder` on the ticket intent). Leverage is a readout, not a typed MAX. |
| `crypto_coinm_usd` | `binance-com-coinm` | `agent/src/coinm_realized_pnl.rs` | Inverse flag on the book. |
| `crypto_options_usd` | `binance-com-options` | Explicit none | Dated contract id. Typed max planned loss. Prototype seller path is dark (`plans/harness-remaining.md`). |

`plans/multi-asset-p9-territories-2026-09-24.md` adds territories that are **not** SHIPPING calculators yet: spot FX (`ib-fx-spot`, ADR 0021 draft), CFD, FX perps. A sizer that hard-codes “OKX perpetual” will miss this list and will also miss books that are already shipping.

DualNoBlend is the display law: one book, one quote currency. Switching INR equity and USDT spot does not blend percents (`notch/RiskDeskMode.swift` comment; desk honesty tests).

### 2.2 Where the Plan surface is narrower than the catalog

`notch/BarDeskInstruments.swift` `BarDeclareAssetClass.supported(forDeskSlug:)`:

- Binance desk → Spot, Options, USDM, Coin-M
- Kotak desk → Equity, Options
- Every other slug, including Zerodha and OKX → empty

So the catalog and the money matrix are multi-asset. The declare tabs are two desks. A calculator drawn on the Plan rail can be multi-asset in its **inputs** (book id, profile, contract spec) while the first paint stays on the desks that already declare.

### 2.3 Contract specs that already exist

| Fact | Where it lives | Gap for a pre-trade calculator |
| --- | --- | --- |
| Tick / step | `agent/src/exchange_info.rs` (Binance filters). `agent/src/instruments/store.rs` `lot_size_and_tick` from venue CSV. Notch `VenueLotTick.round` (`notch/BarDeskInstruments.swift`). Quote envelope copies tick and step in `agent/src/api/quote.rs`. | Rounding a proposed qty to step is specified for those filters. A default step is not. |
| Lot multiplier | NFO engine multiplies by venue lot. MCX/CDS have their own owners. | Pre-trade “lot size” on a crypto ticket is base step, not an India lot. The same label must not mean both. |
| Fees on a fill | Fill may carry `fee_amount`. COM spot round-trip folds a fee lookup (`agent/src/round_trip_engine.rs`). India dogfood plans say per-fill charges are lock-specific and often still “charges NOT SPECIFIED” until the lock is SHIPPING (CDS, Groww aggregate `brokerage_and_charges` is funds-level). | There is no pre-trade fee estimator. Post-fill fee is not a schedule. |
| Leverage | Read from a futures position row when present. Setting leverage is a refused mutation (`/leverage`, `margintype`). | A typed “100 MAX” control would be a new product input. Sending it to the venue is outside UBI. |
| Balance | obtain(`funds`) per book, lit or dark. | Not a single number across books. |

---

## 3. Inspirational UX (Tradeture Auto Risk Calculator)

Founder-supplied screenshot (Tradeture ad, `@tradeture`, caption that risk is calculated before the position can be opened). It is a **layout target**, not a formula source and not a broker spec. The 1.1% and the fee figures on that frame are demo pixels.

Fields on that frame, and the Station noun that already exists for each:

| Tradeture control | Station noun today | Fit |
| --- | --- | --- |
| Exchange (OKX) | `book_id` on `BarDeskTicketIntent`, catalog row | OKX spot exists as `okx-com-spot`. Declare tabs for that slug are empty. The control should list **books the desk can declare**, not a free exchange string. |
| Side SHORT / LONG | declare `side` BUY/SELL; futures ticket side | Same idea. Short on cash equity is a product question the cash gate does not answer (CNC/MIS only). |
| RISK % vs FIXED $ | Open founder lock: “1% of obtain(funds) vs a typed rupee box”. Local floor is a typed amount. Console loss-limit is another typed amount. | Both modes can be **inputs**. The screenshot’s 1.1% is not a default. |
| Order type LIMIT | `BarVenueOrderType` on the ticket (MARKET, LIMIT, LIMIT_MAKER, CONDITIONAL) | Ticket intent is stored on the declaration payload. It is not sent as a live order. |
| Balance | obtain funds / account chrome free text | Per book, dashed when dark. |
| Lot size (authored) | Typed qty, step-rounded | This is the **output** the current sizer refuses to invent. |
| Leverage + MAX | `futuresMargin` readout | Lit only when a position row carries it. MAX would be a new control. |
| Risk, Fees, Risk+Fees | Ladder measures risk of a typed qty. Fees have no pre-trade owner. | Risk of typed size can show. Fees stay an honesty chip until a reference names the schedule. |
| Reward−Fees, RR excl fees, RR incl fees | `riskRewardRatio` is price-distance only, no fees | Excl-fees R:R can show from prices. Incl-fees R:R waits on the fee chip. |
| Upper / mid / lower price | target, entry, stop — labels flip with side | The screenshot’s “upper = TP for long, SL for short” matches a side-aware price stack. Station already stores the three prices separately. |
| Open Short | Confirm declare | The screenshot’s button places a position. Station’s button records a plan. See §5. |

Journal & notes and TP levels map to intent / debrief and `target_price`, which the declare body already carries.

---

## 4. Options

### Option A — One preview function, contract spec per book

A pure function in the agent (suggested home `agent/src/risk/`, called by a loopback preview, no broker HTTP).

**Inputs (book-agnostic):** book id, side, budget mode (`risk_percent` or `fixed_money`), budget number, entry, stop, target, optional typed override qty, funds snapshot status.

**Lookups (book-specific, already owned elsewhere):** catalog row (quote currency, asset class, instrument class, inverse, calc profile), contract spec (tick, step, lot or contract multiplier — present or dark), fee schedule status (dark until a reference), leverage snapshot (present or dark).

**Outputs:** an envelope:

- `authored_qty` — null while the sizing reference stays unspecified
- `qty_unit` — shares, base, quote, lots, or contracts, from the profile
- `risk_money` — from the ladder once qty is typed; null when geometry is not a loss
- `fees_money` — null + reason while the schedule is unspecified
- `reward_money`, `rr_ex_fees`, `rr_in_fees` — same honesty
- `gate` — reasons already enforced by `BarIntradayDeclareValidator` can move here later so Station and Notch share them
- `quote_currency`

Notch and the Station window **paint** the envelope. They do not recompute it.

| Tradeoff | |
| --- | --- |
| Matches | ADR 0004 book stamp, S2 one owner, the money matrix (few physics, many books), DualNoBlend |
| Cost | First version returns a lot of nulls. That is the same shape as `extract_margin_estimate`. |
| Risk | Someone fills nulls from the Tradeture screenshot. The reference doc is the block. |

### Option B — Each book adapter exports its own sizer

Wasm `preview_size` per component.

| Tradeoff | |
| --- | --- |
| Appeal | Venue quirks stay next to the adapter. |
| Cost | Twenty-two components, seven physics. Two NFO brokers could drift. The host already stamps class and owns Keychain, funds, and the money matrix. Sizing inside Wasm splits the number S2 wants once. Adapters are read-only and must not see a private formula that depends on Keychain balances. |
| When it would win | A venue publishes an official what-if endpoint (margin or commission) whose host, path, and auth are in a reference doc. That result is an **input** to Option A, not a second sizer. Kotak `check-margin` is documented as adjacent and is **not** the margin calculator (`docs/reference/india/kotak-neo/MARGIN-CALCULATOR.md`). |

### Option C — Swift-only calculator on the Plan rail

Extend `BarPlanSizer` and `BarPlanLadder` in Notch. Station Settings reads the same types.

| Tradeoff | |
| --- | --- |
| Appeal | The measurement already lives in Swift. A UI can land without an agent route. |
| Cost | Tick, step, lot, and funds are agent facts. A second copy in Swift will diverge the moment MCX and CDS specs differ from Binance filters. Console archive would store whatever the client typed, and a later Station window could disagree. |
| When it would win | A throwaway HTML/Swift prototype to choose the layout (the repo already did this for the risk mosaic: `station/prototypes/PROTOTYPE-notch-risk-engine.html`, `plans/harness-remaining.md` Phase P0). Layout prototypes stay prototypes. |

### Option D — Console computes the size

The brain returns authored qty on live-state or loss-limits.

| Tradeoff | |
| --- | --- |
| Appeal | One hosted config UI for limits. |
| Cost | R4.1 and S3 put the desk loop on the Mac. Live-state is already a round trip and was a 401 outage in `plans/NOTCH-FEATURE-AUDIT-2026-09-27.md`. A sizing call that fails closed would block Plan whenever Console is down; a sizing call that fails open would show a size the archive did not accept. |
| Console role that does fit | Keep loss-limits as the activation profile. Store the frozen preview on the declaration (the body already has quantity, prices, `max_planned_loss_inr`, `book_id`). Paint behavioral verdict as it does now. |

---

## 5. Recommended path

**Option A for the engine. Option C only for a layout prototype. Option D’s archive role stays. Option B only as a sourced what-if quote, never as the owner of qty.**

Reasons tied to code:

1. The product is already multi-asset at `book_id` + calc profile + one money owner. A shared preview with a spec table reuses that. A per-instrument function does not.
2. Declare tabs are narrower than the catalog. The engine can accept every SHIPPING book id on day one and return `qty_unit` from the profile. The Notch control shows books `BarDeclareAssetClass.supported` already lists, then grows when those tabs grow.
3. Fees, leverage-as-multiplier, and a default risk percent are unspecified (`POSITION-SIZING.md`). Option A can ship the **panel** with honest dashes. That matches `BarPlanSizer` and the margin hole.
4. The Tradeture “Open Short” button is an order. Station’s matching action is **Confirm**, which archives a declaration. Turning that button into `routeOrder` needs a new ADR. Until then the button label in product copy stays on the plan (“Confirm”), and the preview still shows side, size, risk, and R:R before that confirm. UBI does not gain a place path in this work.
5. Desk floor and Console loss-limits stay two stores until a founder lock merges them. The preview can **display both** and use the local floor as the fixed-money default only when the trader has typed it. It does not POST the floor to loss-limits and does not arm Kill. Wiring floor → DNS stays the later tranche named in the completion roadmap (Wave 3.3).

### Suggested envelope (sketch, not a shipped schema)

```text
POST /api/daemon/risk/preview
{
  book_id, side, budget_mode, budget_value,
  entry, stop, target, override_qty
}

→ {
  quote_currency, qty_unit,
  authored_qty: null | number,
  reason: "position_sizing_unspecified" | "funds_dark" | "no_stop" | "ok",
  risk_money, fees_money, risk_plus_fees,
  reward_minus_fees, rr_ex_fees, rr_in_fees,
  fees_reason: "fee_schedule_unspecified" | null,
  leverage: null | { value, source: "position_row" },
  contract: { tick, step, multiplier, multiplier_reason }
}
```

`authored_qty` stays null until `POSITION-SIZING.md` cites a primary source for the budget→qty rule. `risk_money` for a **typed** qty may call the same identity as `BarPlanLadder` (already tested) because that is measurement of typed numbers, not a new percent-of-capital rule. Put that call in the agent when the preview exists so Swift stops being the only owner. Until the route exists, leave the Swift ladder where it is.

Percent of account uses the book’s quote currency. A missing quote dashes the percent (DualNoBlend). Leverage multiplies notional only after a reference says so for that calc profile. Until then the leverage row shows the position readout or a dash.

### How Notch and Station would show it

One Swift presentation (Notch Plan rail and Station, same type) bound to the preview:

1. Book chip (current declare book).
2. Side.
3. Budget toggle: Risk % | Fixed money. Fixed money pre-fills from `DeskRulesStore.dailyFloor` when that value exists.
4. Balance line from funds glance.
5. Three prices: entry, stop, target, labeled by side the way the screenshot stacks upper/mid/lower.
6. Result column: Risk, Fees, Risk+Fees, Reward−Fees, RR excl, RR incl. Dashed cells keep a one-line reason.
7. Proposed size, step-rounded when step is lit, dashed when the formula is still unspecified.
8. Existing Confirm gate (`submitReadiness`) stays the submit control. Override qty above a future authored size is the gate `plans/harness-remaining.md` already describes; it turns on when authored qty exists.

Station Settings keeps editing desk rules. It does not grow a second calculator. The Today floor preview (`DeskRulesPresentation.remainingPreview`) stays labeled preview.

### How Console configures and receives

| Concern | Where it stays |
| --- | --- |
| Daily / weekly loss and margin-cap acknowledgement | Existing loss-limits route. Still not per book. A per-book cap would be a Console contract change; do not invent the JSON here. |
| Frozen plan | Existing declare body. When a preview exists, add the envelope under `declaration_payload` so the journal stores the numbers the trader saw. Console persists that object; it does not recompute qty. |
| Circuit block | Existing `declaration_submit_blocked` and interventions. |
| Behavioral score | Stay Console-owned. The sizer does not multiply qty by `behavioral_score`. The wire thresholds are not an agent formula today. |
| Kill | Stay manual / `fog_of_war`. |

Console follow-ups to **verify**, not to build from this repo:

- Declare handler accepts `book_id`, `max_planned_loss_inr`, and `declaration_payload` without dropping them.
- Loss-limits remain one profile per caller, unless product later asks for per-book caps.
- Pre-order and kill-switch fail-open notes in the lifetime spec, on the files named above.

---

## 6. Fees, tick, lot, leverage — decision table

| Piece | Recommendation while sources are thin | Alternative | Why the alternative loses for now |
| --- | --- | --- | --- |
| Qty unit | From calc profile (§2.1). | One “lot size” field for every book. | India lot, crypto step, and Coin-M contracts are different multipliers. |
| Tick / step | Round with venue filters when the quote envelope has them (`VenueLotTick`). Missing step → leave the typed number and mark step dark. | Assume 0.001 or 1. | `VenueLotTick` tests say qty 2 with step 0.001 stays 2; a guessed step would change size. |
| Fees | Row visible, value dark, reason `fee_schedule_unspecified`. | Use the last fill’s fee rate, or the Tradeture 1.21. | Last fill is not a schedule. The screenshot is not a source. |
| Fees later | One reference doc per calc profile (or per lock) that names the levy rows. The preview reads that doc’s implementation, still one function. | Per-adapter fee code with no shared output shape. | The panel needs one envelope. |
| Leverage | Show venue readout or dash. Do not multiply. | Typed leverage defaulting to 100 / MAX. | Flat USDM test forbids an invented 50. `/leverage` is a mutation. |
| Risk % | Input the trader types. No default percent. | Default 1% or 1.1%. | Founder lock 3 is open. Reference doc says the percent is unspecified. |
| R:R &lt; 1 | Keep today’s color band. | Hard-block Confirm. | Founder lock 4 is open. |
| Option seller | Measurement dark (prototype law in `plans/harness-remaining.md`). | Premium × qty as if it were a linear future. | Options realized owner is explicit none. Options mechanics doc says the spot/futures risk taxonomy may not map (`docs/reference/crypto/binance-global/options/MECHANICS.md`). |

---

## 7. Sequence for a follow-up agent

Each step is optional until the founder picks it. None of them authorizes a percent or a fee rate from memory.

1. **Layout only.** A throwaway on the existing Plan rail that places the §5 rows and dashes every computed cell. No new formula. Binance spot and Kotak equity only, because those tabs exist.
2. **Preview route, still dashed qty.** Agent envelope in §5. Notch calls it. Tests: funds dark → size dashed; missing stop → no invented stop; `authored_qty` null; margin estimate still unavailable; desk rules still do not POST loss-limits.
3. **Typed-qty measurement moves next to the route.** Same identities as `BarPlanLadderTests`. Swift calls the route for the “At your stop” number so there is one owner.
4. **Contract spec table.** For each SHIPPING calc profile, a row: qty unit, where tick/step/lot are read, fee status `unspecified` or a link to a reference doc. CDS and MCX stay dark on fees while their locks say so.
5. **Formula, only after the reference doc changes.** Implement budget → qty exactly as that doc quotes. Gate: override above authored qty blocks Confirm. Still no place/modify/cancel.
6. **Declare payload.** Attach the preview the trader confirmed. Console stores it.
7. **Widen the book chip** when `supported(forDeskSlug:)` grows. The engine should already know the book id.
8. **Separate, later tranche.** Floor or loss-limit → Kill. Named in the roadmap. Out of the calculator.

Tests to keep green while touching this area: `BarPlanSizerTests`, `BarPlanLadderTests`, `DeskRulesStoreTests`, `BarDeskTicketTests` (no invented 50×), margin estimate unit test, kill-switch apply tests.

---

## 8. Out of this note

- No Rust or Swift feature code in the pass that added this file.
- No change to Console.
- No default risk percent, fee rate, or leverage.
- No `routeOrder`, Set SL send, or flatten.
- Prototype role percents in `station/prototypes/risk-demo-logic.mjs` (0.5% / 1% / 2%, calm ≥ 4 halves) stay prototype. The file says it is not a reference source.

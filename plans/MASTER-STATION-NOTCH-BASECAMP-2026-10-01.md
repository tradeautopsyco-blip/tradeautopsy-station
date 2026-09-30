# Master plan — Station, Notch, Basecamp

**Date:** 2026-10-01  
**Checkout:** `tradeautopsy-station` `main` @ `a8c1a50` (includes `d6327cc` paired-device refresh on 401).  
**Kind:** sequencing only. No product code in this document’s PR.

This file merges three inputs into one build order:

| Input | What it is |
| --- | --- |
| **A** | Notch feature-gap backlog (journal ACK, match fidelity, kill quit-bypass, PLAN JWT, N2, plus mediums) |
| **B** | Station backlog beyond Notch High, plus the suggested spine Journal cite → outbox → notarize → permissions → S8 → sizer → kill policy → Report → India Tier II → Phase Z last |
| **C** | Basecamp list “TradeAutopsy Station” (screenshot 2026-09-30): list 1 Station, then list 2 Notch |

There is no file named `INCOMPLETE-LANES` in this checkout. The “P0 = PLAN JWT + capture ACK” pressure is taken from the founder fold-in that accompanied this plan, and checked against the code below.

---

## How to use this plan

Work waves in order. One follow-up agent session equals one ticket, not a wave.  
`plans/station-completion-roadmap-2026-09-17.md` remains the language lock (Harness / Open / Plan / Working / Debrief, DualNoBlend, Enforcer teeth only). Where that roadmap’s September status disagrees with `main` today, **this file wins**.

Owners: **station** Swift app, **notch** overlay, **agent** Enforcer on port 9137, **console** `FExEVIL/tradeautopsy` (not this repo).

---

## Order decision

Basecamp list 1’s subtitle is the gate: *unblock daily use, prove Station data lands in Console and the database, then risk and kill switch.*  
Basecamp list 2’s subtitle is the next gate: *live conditions and depth first, then the option UI, then pill and delete.*

A strict reading of “finish every Station checkbox, then start Notch” would park live conditions behind Sparkle, the agent button, and the sizer. That fights both subtitles: you cannot prove a declare that 401-cancels, and the Notch list itself says conditions and depth come before chrome.

**Resolved order**

1. **Wave 0 — Prove spine.** JWT failure must leave PLAN up. Capture ACK must be diagnosable. Journal must cite Station trips. Match flip stays a Console dependency, proved with the existing harness.
2. **Wave 1 — Auth leftovers.** Touch ID for the Kotak profile is shipped. The 12-hour TOTP card is new.
3. **Wave 2 — Notch live conditions + CI depth.** First Notch items, on top of a declare that survives.
4. **Wave 3 — Kill latch.** `plans/kill-switch-lifetime-spec.md` Phase 1. After prove, before a kill-and-restart button.
5. **Wave 4 — Margins display + Plan sizer.** Honest funds glance and size. The margin calculator stays dark.
6. **Wave 5 — Notch chrome the subtitle puts last.** Option surfaces, option name, pill, delete-as-cancel.
7. **Wave 6 — Sparkle trust + one-button agent restart.** DNS is an operator step. The button must not reap a latched Enforcer.
8. **Wave 7 — One journal object (N2) and the medium honesty list.** Morning brief route, intervention copy, derived Setups/Process/Book.
9. **Phase Z last.** Seven questions still open. `/api/bar/v1` stays hosted until they are answered.

Station honesty that *is* the prove (Journal cite, Due) sits in Wave 0. Station honesty that is a later product (Report, AlertBus, India Tier II, S8 account chrome) stays parked.

---

## Conflicts

| Conflict | Resolution in this plan |
| --- | --- |
| Phase Z (local truth, cut Console) vs Basecamp “prove data reaches Console and the database” | Prove-to-Console is the current product. Phase Z is last. Do not delete `/api/bar/v1`. See `TRADEAUTOPSY-CONTEXT.md` §5.1 questions 1–7 and roadmap T9. |
| Set SL / “auto risk” vs T5 cut | Protective Set SL send stays cut (`notch/BarProtectiveSlPlanChrome.swift`, `showsSetSlButton: false`). Wave 4 ships honest copy and the sizer. It does not restore `place_sl`. UBI stays read-only (ADR 0002). |
| “Change the Indian option UI to the crypto option UI” vs shipped dual surfaces | Keep both. NFO cockpit is `notch/BarNfoCockpitMosaic.swift` / `notch/BarOptionsDeclareView.swift`. Crypto options are `notch/BarCryptoOptionsDeclareView.swift`. Roadmap lock: no `if crypto` inside NFO chrome. A visual-density borrow is a founder sign-off, not a merge. |
| “Remove the collapsed pill” vs harness-remaining Phase 2 (honest pill) | Pill still exists. Station host (`station/StationApp/FloatingNotchHost.swift` → `NotchLauncher(isHostedByStation: true)`) toggles expand/collapse; `start()` still shows the panel. Currency honesty has tests (`notch/Tests/NotchTests/DeskHonestyPillTests.swift`). Do not delete the pill until the founder signs “Station window is the only chrome.” |
| “Delete the declared trade” vs journal law that cancelled rows stay on the sheet | The action is cancel, which already exists (`POST /api/daemon/bar/cancel-declaration` in `agent/src/api/bar.rs`). Hard delete is out. `plans/station-journal-settings-today.md` Wave J: cancelled-before-fill stays on the sheet. |
| Backlog “outbox has no `ensure_fresh` / 429 retry” vs `main` | Stale. `agent/src/outbox.rs` `send_attempt` calls `ensure_fresh_station_access` and retries 429 and 5xx. `d6327cc` refreshes once on 401 and leaves the row queued. Do not re-add that machinery. Wave 0.2 is “why is this row still `DEAD_LETTER`.” |
| `plans/station-sparkle-update-infrastructure-2026-09-25.md` “Sparkle not integrated” vs `main` | Stale. Client, feed URL, and release workflow exist. Remaining work is notarize-by-default and DNS verification, not a second Sparkle integration. |
| Founder “full Station list then Notch” vs P0 JWT+ACK | Wave 0 *is* Station checklist items 3–4. It is implemented in agent + notch + journal because that is where declare, capture, and cite live. Notch chrome (option UI, pill, delete) waits until Wave 5. |
| Settings daily floor vs Kill | `notch/DeskRulesStore.swift` persists the floor for display (`notch/BarDeskRulesReadout.swift`). Wave S acceptance in the journal plan: these numbers do not fire Kill. Keep that. Floor → DNS is a later named tranche (roadmap Wave 3.3). |
| `account/margin_estimate` vs “auto risk calculator” | Different things. The calculator is an intentional dark hole (`agent/src/data/margin_estimate.rs`, `docs/reference/india/kotak-neo/MARGIN-CALCULATOR.md`). The sizer is `planned_risk / distance_to_invalidation` (roadmap Wave 3.1) and needs a reference doc before any formula beyond the signed prototype. |
| LiveBook “snapshot once” in the September roadmap vs harness-remaining Phase 1 | Phase 1 code is on disk: `LiveBook::apply`, declare apply-first, no 2s poll spine. Founder dogfood of “Confirm stays after refresh” is still open (`plans/harness-remaining.md`). Do not rebuild N1. |

---

## Basecamp map (list C)

Status words: **shipped** (on `main`, with a path), **partial**, **planned** (a plan owns it, code does not), **missing**, **conflict** (do not implement as written).

### 1 — Station

Subtitle: second, after Console. Unblock daily use, prove Station data lands in Console and the database, then risk and kill switch.

| # | Basecamp item | Status | Repo reality | Wave |
| --- | --- | --- | --- | --- |
| 1 | Touch ID instead of password | **shipped** for Kotak profile Edit | `station/StationApp/Broker/Services/KeychainKotakLoginProfileStore.swift` `authenticateDeviceOwner` uses `LAContext` / `.deviceOwnerAuthentication`. `station-wire/KEYCHAIN-ACL.md`: Touch ID is the login-profile Edit gate, and it is the device passcode policy (Touch ID or passcode). It is not WorkOS device login. Device login still shows `user_code` only (AGENTS.md). | 1 — confirm only |
| 2 | TOTP card after 12 hours, on Start | **partial** | Start remint exists: `station/StationApp/Broker/Services/BrokersViewModel.swift` (`presentKotakTotpRemint`, vault missing or Start failed). No 12-hour timer and no timed card. | 1 |
| 3 | Prove Station data reaches Console | **partial** | Declare forwards to Console `POST /api/bar/v1/declarations` (`agent/src/api/bar.rs` `declare_handler`). Quiet refresh on 401 is shipped (`d6327cc`, `agent/src/api/capture.rs` `forward_daemon_json_with_optional_429_retry`). Remaining dark path: a 401 that survives refresh is `is_client_error()` and applies `LiveBookEvent::Cancel` (`bar.rs` ~146). Notch then `clearOptimisticArmedStorage()` (`notch/NotchViewModel.swift` `submitBarDeclaration` ~3106). | 0.1 |
| 4 | Prove sync to the trade section and the database | **partial** | Same spine as 3, plus journal cite and match. Week list forward exists (`declarations_list_handler`, `scope=week`). `citedTrips` is always `[]` (`station/StationApp/Services/Journal/JournalViewModel.swift` `rebuild`). Production `status=matched` writer is a Console gap (`plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`). Test inject exists (`POST /api/daemon/bar/test/fill-matched`). | 0.3, 0.4 |
| 5 | Data-rich Station UI | **planned** | Roadmap Wave 5 “frictionless” and Wave 1 desk honesty. Journal and Today shells exist; they are not the data-rich end state. | 7, after prove |
| 6 | Audit the user flow and remove friction | **planned** | Same Wave 5. No separate audit doc in `plans/`. Do this as dogfood notes on Waves 0–3, not as a standalone rewrite. | alongside 0–3 |
| 7 | Auto risk management and auto risk calculator | **missing** as a calculator; **planned** as a sizer | `extract_margin_estimate()` returns `HonestyStatus::Unavailable` / `margin_calculator_unspecified`. Sizer is roadmap Wave 3 and `plans/harness-remaining.md` Phase 3, blocked on a `docs/reference/` doc and founder sign-off. Settings floor does not author Plan qty. | 4 |
| 8 | Show margins in the Notch and in Station | **partial** | Glance margin chip is unavailable on both desks (`notch/Tests/NotchTests/BarOptionsGlanceStripTests.swift` `marginIsUnavailableOnBothDesks`). NFO copy already says SPAN is the broker’s number (`notch/BarOptionsDeclareView.swift`). Station desk has no funds glance. S8 pulse+ledger is parked (`plans/s8-account-chrome.md`). | 4 |
| 9 | Kill switch management | **partial** | Teeth and overlay exist (`agent/src/api/kill_switch.rs`, `agent/src/dns_block.rs`, `notch/KillSwitchOverlayController.swift`). Quit still kills the Enforcer (`station/StationApp/AgentSupervisor.swift` `shutdown` / `killBundledAgentOrphans`). No `kill_latch.rs`. No `isKillLatched()`. | 3 |
| 10 | Dissect the boring notch and research app update | **partial / near shipped** | Sparkle client: `station/StationApp/SparkleSoftwareUpdateController.swift`. Feed: `SUFeedURL` = `https://updates.tradeautopsy.in/appcast.xml` in `station/StationApp/Info.plist` (0.2.1 / build 5). Release workflow defaults to ad-hoc (`/.github/workflows/release.yml`, `notarize` input off). DNS for that host is unverified (`docs/runbooks/updates-domain-vercel.md`, `updater/README.md`). | 6 |
| 11 | One-button kill + restart agent | **partial** | Auto-restart and crash-loop backoff: `station/StationApp/Models/AgentRestartTracker.swift`, `AgentSupervisor`. Toolbar retry: `station/StationApp/Views/AgentHealthToolbarStatus.swift` → `StationAppCoordinator.retryAgent()`. No product button that stops and starts the agent on purpose. | 6, after Wave 3 |

### 2 — Notch

Subtitle: inside Station, after the Station list. Live conditions and depth first, then the option UI, then pill and delete.

| # | Basecamp item | Status | Repo reality | Wave |
| --- | --- | --- | --- | --- |
| 1 | Live conditions, in the live session and after | **partial** | Working compare exists (`notch/BarWorkingCompare.swift`, `notch/Tests/NotchTests/BarWave2WorkingTests.swift`). Invalidation crossed offers Debrief or Kill (`notch/BarPlanStateView.swift`). No post-session twin and no condition-fire log on the journal object. Per-book clock labels exist (`notch/BookSessionClock.swift`); do not rebuild them. | 2, fire log in 7 |
| 2 | Enable CI depth | **partial** | `DepthBook` + glance (`agent/src/api/glance.rs`). Kotak cash+NFO depth signed on a weekday (`plans/kotak-dogfood-2026-09-18.md`). S3 leftover: NFO and options depth together, COM gap → Unusable, glance 20 rows (roadmap “What is left on the data spine”). | 2 |
| 3 | Change the Indian option UI to the crypto option UI | **conflict** | Dual surfaces are the product. See Conflicts. | 5 — founder sign-off, or drop |
| 4 | Change the option name | **missing** | No rename spec. Founder must name the string. Shipping language lock until then is Harness / Plan (`plans/station-completion-roadmap-2026-09-17.md`). | 5, blocked on the name |
| 5 | Remove the collapsed pill | **conflict** | See Conflicts. Settings can hide the host (`deskRules.hideNotch`, `station/StationApp/StationAppCoordinator.swift`). | 5, blocked on sign-off |
| 6 | Notch action to delete the declared trade | **partial** | Cancel with `cancel_reason_chip` exists (`NotchViewModel` → `/api/daemon/bar/cancel-declaration`). A control that says “delete” and erases the row is **missing** and should stay missing. | 5 — label the existing cancel |

---

## Deduped backlog (A + B + C)

One row per piece of work. Sources that repeat it are listed once.

| ID | Work | Sources | Status on `main` | Wave |
| --- | --- | --- | --- | --- |
| P0-JWT | PLAN stays up when Caller JWT refresh fails | A4, C1.3, fold-in | Refresh shipped; 401 still cancels local declare and clears optimistic armed | 0.1 |
| P0-ACK | Capture outbox reaches ACK, or a named `DEAD_LETTER` reason | A1, B3 | `ensure_fresh` + 429/5xx retry + one 401 refresh shipped. Validation and max-attempts still dead-letter (`agent/src/outbox.rs`, `agent/tests/capture_delivery_phase3.rs`) | 0.2 |
| P0-CITE | Matched net cites a Station trip | A2, B1, C1.4 | `JournalWeek.build` can join trips; `JournalViewModel` passes `[]` | 0.3 |
| P0-DUE | Post-due facet + sidebar Due | B2, Wave J | `JournalFacet` is all/matched/pending/unmatched/impulsive only (`station/StationApp/Presentation/JournalWeek.swift`). Picker matches. Test locks that set (`JournalWeekProjectionTests`). | 0.3 |
| P0-MATCH | `pending` → `matched`, Match/Fidelity not empty | A2, C1.4 | Debrief PATCH works on `pending` (`plans/NOTCH-LIVE-JOURNAL-HARNESS.md`). Production match flip is Console. Test inject is Station-side | 0.4 |
| AUTH-TOUCH | Touch ID for Kotak profile | C1.1 | Shipped. Re-open only if the founder meant Station-wide unlock instead of WorkOS `user_code` | 1 |
| AUTH-TOTP | TOTP card 12h after session mint, on Start | C1.2 | Remint on failure shipped; timer missing | 1 |
| N-COND | Live conditions during session and after | A8, C2.1 | Live compare shipped; after-session twin and fire log missing | 2 / 7 |
| N-DEPTH | CI depth on the desk | C2.2, S3 leftover | DepthBook shipped; combined weekday sighting open | 2 |
| KILL-LATCH | Durable latch, quit does not kill a latched Enforcer | A3, B7, C1.9 | Spec only: `plans/kill-switch-lifetime-spec.md`. No `agent/src/kill_latch.rs` | 3 |
| RISK-SIZE | Plan sizer + R:R from Settings floor and obtain(funds) | A7, B5, C1.7 | Floor display shipped. Sizer blocked on reference doc. Post-Confirm mosaic is HTML only (`plans/harness-remaining.md` Phase P0 done, Phase 3 blocked) | 4 |
| RISK-MARGIN | Margins in Notch and Station | C1.8, B6 | Glance unavailable. S8 account chrome parked | 4 |
| RISK-SL-COPY | Honest “SL not placed” copy; Set SL stays absent | A11 | Button already absent. Copy still reads like a promise (`BarProtectiveSlPlanChrome` “Set SL at ₹… — not placed”) | 4 |
| N-OPT-UI | Indian vs crypto option UI | C2.3 | Conflict. Do not merge | 5 |
| N-OPT-NAME | Option name | C2.4 | Missing. Blocked on the founder’s string | 5 |
| N-PILL | Collapsed pill | C2.5, harness Phase 2 | Conflict. Honesty tests exist. Removal unsigned | 5 |
| N-CANCEL | Declare cancel action on the Notch | C2.6 | Route shipped. “Delete” wording and a dedicated control are the gap | 5 |
| SPARKLE | Notarized updates; `updates.tradeautopsy.in` answers | A10, B4, C1.10 | Client + appcast plumbing shipped. Default DMG is unsigned. DNS unverified | 6 |
| AGENT-BTN | One-button stop and start of the agent | C1.11 | `retryAgent()` only | 6 |
| N2 | One day-sheet object: Plan + Working + Debrief | A5, roadmap 2.x | Notes fields and debrief payload exist. Three surfaces still forget each other (`plans/harness-remaining.md` Phase 4) | 7 |
| OPEN | Morning brief route | A6 | Client calls `GET /api/daemon/morning-brief` (`notch/NotchViewModel.swift` ~4840). No match in `agent/`. 404. Prototype: `station/prototypes/PROTOTYPE-morning-brief.NOTES.md`. Lock is header B + gate D | 7 |
| OVERRIDE | Intervention primary action | A10 | `notch/BarInterventionCardSpec.swift` `manageInWebBar` for kill / enforcement. Web `/dashboard/bar` is culled. Point the accessory at Notch Kill or drop it | 7 |
| DERIVED | Setups / Process / Book | A9, roadmap Wave 4 | Placeholders are already honest empty (`notch/BarNotchShell.swift` `BarPatternsChartView`, `BarFidelityChartView`). Do not paint demo series | 7 |
| PATH-A | Manual fill → journal | A12 | Impulsive debrief payload exists (`notch/BarPostTradeDebriefPayload.swift`). Founder proof on a real fill is open. Harness can PATCH without a fill | 7, founder |
| PARK | Report honesty, AlertBus, brokers marketplace, India Tier II, NFO multi-book money, Health helpline, Sparkle beta channel, guided TCC, pairing multi-Mac, per-book poll audit | B mediums | Planned elsewhere. See “Leave alone.” | after 7 |

---

## Wave 0 — Prove spine

**Goal.** A declare made on this Mac is still on the Notch after a dead access token, the screenshot outbox either ACKs or names why it died, and a matched journal card shows Station trip net. Console and the database are the archive. This is Basecamp 1.3 and 1.4.

**Blockers.** Console must be up for archive proof. Gate B (`signed_in: true`) on the Debug agent. Match flip needs Console `BAR_TEST_INJECT_FILL_MATCHED=1` for the synthetic lane, or a real fill later. Founder dogfood is on a Mac; this cloud checkout cannot click the Notch.

### 0.1 PLAN JWT — first implementation ticket

Quiet refresh is already the law (`d6327cc`). Do not add a second refresh client.

**Defect still open.** `declare_handler` treats every 4xx, including 401 after a failed refresh, as `LiveBookEvent::Cancel`. `forward_daemon_json_with_optional_429_retry` returns that 401 when post-401 refresh is `Revoked` or `Transient` (`agent/src/api/capture.rs` ~185–198). Notch `submitBarDeclaration` on non-2xx calls `clearOptimisticArmedStorage()`. Working then has no local book, and `live_state_handler` falls through to Console, which is also 401. PLAN is dark.

5xx and transport errors already use `archive_kept_response` and keep the local id. 401 should follow that path.

**Files**

- `agent/src/api/bar.rs` — `declare_handler` only. 401 uses `archive_kept_response`. Validation 400 still cancels.
- `agent/tests/bar_forward.rs` — new case: upstream 401 and refresh failure leaves LiveBook pending and responds 200 with `archive_error`.
- `notch/NotchViewModel.swift` — `submitBarDeclaration`: a 2xx body with `archive_error` still records `declarationId`, still fetches live state, and sets a visible “sign in again” line. It does not call `clearOptimisticArmedStorage()`.

**Acceptance**

- Valid refresh: declare still archives to Console (existing behavior, do not regress `station_paired_device` tests).
- Refresh fails: LiveBook still has the declare; `GET /api/daemon/bar/live-state` returns `x-livebook: local`; Notch stays on Working with the sign-in line.
- A malformed declare body still cancels.
- `./scripts/notch-live-journal.sh --dry-run` still parses. Live proof is Gate B then declare (`plans/NOTCH-LIVE-JOURNAL-HARNESS.md`).

**Out of this ticket.** Outbox, Journal cite, kill latch, sizer, morning-brief route, `retry_on_429` for every bar route (declare currently passes `false`; leave it until a 429 is observed).

### 0.2 Capture ACK — diagnose, then a small harden

**Do not** re-implement `ensure_fresh` or 429 backoff. They are in `agent/src/outbox.rs` `send_attempt` (~307, ~380). Presign already retries 429 (`screenshot_presign_handler` passes `retry_on_429: true`).

**Do** read `GET` outbox status (`agent/src/api/outbox_status.rs`) on a signed-in agent and group `dead_letters[].reason`. Known terminal reasons already tested:

- `validation` — no retry loop (`agent/tests/capture_delivery_phase3.rs` `validation_error_goes_dead_letter_without_retry_loop`)
- `station session revoked`
- attempt cap (`max_attempts`)

**Files if a code change falls out of the diagnosis**

- `agent/src/outbox.rs` — only the branch the status snapshot proves is wrong
- `notch/UnpostedCaptureStore.swift` / `notch/UnpostedCaptureTrayView.swift` — show `last_error`, not a silent tray
- Harness: `./scripts/notch-live-journal.sh --prove-capture` (accept 202 / outbox status; it does not prove R2 → journal shots)

**Acceptance.** A toolbar capture with a live Station Bearer ends `ACKED`, or Settings/outbox status shows one of the named reasons above. Founder can tell those apart without reading sqlite.

### 0.3 Journal cite + Due

**Cite.** `JournalViewModel.rebuild` must pass Station trips keyed by `declarationId`, from the Today / inventory owner. `JournalWeek.build` already joins (`station/StationApp/Presentation/JournalWeek.swift` ~272, test around `JournalWeekProjectionTests.swift` line 208). Console must not recompute FIFO onto the card (Wave J law in `plans/station-journal-settings-today.md`).

**Due.** Add facet `due` = matched this week with empty `notes.post`. Paint it in `station/StationApp/Views/Journal/JournalView.swift` picker and a sidebar badge. Update `JournalFacet.allCases` expectation in `station/StationTests/JournalWeekProjectionTests.swift` (it currently locks the five-facet set). Empty post on a still-`pending` row is not Due; Due is the post-trade debt.

**Acceptance.** A card with a cited trip shows that net and currency. DualNoBlend if the trip currency disagrees with the desk. Sidebar Due counts matched rows with empty post. Uncited matched cards show no invented net.

### 0.4 Match fidelity (Console-gated)

Station already proxies `POST /api/daemon/bar/test/fill-matched` → Console `POST /api/internal/bar/v1/test/fill-matched` (`agent/src/api/bar.rs` `test_fill_matched_handler`). Prod must leave `BAR_TEST_INJECT_FILL_MATCHED` unset (`plans/FILL-MATCHED-INJECT-SKETCH.md`).

**This repo’s ticket** is harness proof, not a fake `matched` writer:

```bash
./scripts/notch-live-journal.sh --book binance-com-spot --symbol BTCUSDT \
  --inject-matched --wait-closed-sec 30 --keep-declaration
```

Exit 5 on 404 means the Console flag is off. That is an honest block, not a Station bug.

**Console ticket (other repo).** A production caller for `FILL_MATCHED` on real fill ingest, called out as missing in `plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`. Until that lands, Match/Fidelity stay empty on real trades and the synthetic lane is the proof.

**Acceptance.** With the flag on, declarations list `status` is `matched` and the journal card can cite the trip from 0.3. With the flag off, the harness says 404 and Station does not invent `matched`.

---

## Wave 1 — Auth leftovers

**Goal.** Basecamp 1.1 stays closed. Basecamp 1.2 grows a card.

### 1.1 Touch ID

No code unless the founder says Station itself should unlock with Touch ID in place of the WorkOS device-login `user_code`. That would reopen A8. Default: leave `KeychainKotakLoginProfileStore` as the only biometric gate.

### 1.2 TOTP card at 12 hours, on Start

**Files.** `station/StationApp/Broker/Services/BrokersViewModel.swift` Start path; the connect sheet already has TOTP-only remint (`BrokerConnectSheetView.swift`).

**Behavior.** If the Kotak session mint is older than 12 hours when the trader presses Start, present the existing TOTP-only card before `startSync`. A failed Start still remints, as today. Do not invent a broker TTL. The 12 hours is the founder’s card policy from Basecamp, not a SPAN or fee formula. If a Kotak reference later names a different session lifetime, the card follows the reference (`docs/reference/india/kotak-neo/`).

**Acceptance.** Start inside the window does not ask for TOTP when the vault is valid. Start past 12 hours shows the card and does not print the code. Touch ID still unlocks the saved profile on Edit.

---

## Wave 2 — Live conditions and CI depth

**Goal.** Basecamp Notch 2.1 and 2.2. The desk shows the condition that is true now, and depth is either a ladder or Unusable.

**Files**

- Live: `notch/BarWorkingCompare.swift`, `notch/BarPlanStateView.swift`. Add the after-session read of the same compare (last vs invalidation / target) from the declaration snapshot, labeled as after the session, not as a live last.
- Depth: `agent/src/api/glance.rs`, `agent/src/kotak_rest_quotes.rs`, Notch depth tile on the NFO cockpit (`notch/BarNfoCockpitMosaic.swift` seed already includes `.depth`). Dogfood script notes: `plans/kotak-dogfood-2026-09-18.md` S3, `plans/s8-scoped-dogfood-2026-09-19.md` (options depth already glanced).

**Acceptance.** During the session, Working shows waiting / true / broken for invalidation using obtain last, with the honesty chip when last is unavailable (`BarWorkingCompare.labeledLast`). After the session, the same ticket shows the last comparison that was stored, or an explicit “not captured” line. Depth obtain on the shipping book is `success` with levels, or Unusable on a sequence gap — never `{bids:[],asks:[]}`.

**Out.** Condition-fire history on the journal object waits for Wave 7 (it needs the one day-sheet). Do not call `routeOrder`. Do not gate COM polls on `isIstMarketSession()` (`notch/NotchViewModel.swift` already comments that COM must not use the NSE window).

**Blockers.** Combined NFO+options depth sighting is a weekday founder session (roadmap S3 leftover). Code can land the honesty; the signature is human.

---

## Wave 3 — Kill latch

**Goal.** Quitting Station while an L3 kill is armed leaves the broker block in force. A restarted Enforcer re-arms the watcher from disk. Basecamp 1.9.

**Spec.** Execute `plans/kill-switch-lifetime-spec.md` Part B Phase 1. Do not rewrite the spec. Invariants to land, each with the test name in §7:

1. `latch_survives_sigterm`
2. `boot_reconciles_latch_to_hosts`
3. `boot_rearms_watcher_when_armed`
4. `tamper_after_restart_is_reverted`
5. `station_quit_does_not_kill_latched_agent`
6. `station_launch_attaches_to_latched_orphan`
7. `expired_latch_self_dismisses`
8. `latch_read_failure_fails_closed`
9. `dismiss_clears_latch_only_after_hosts_clean`

**Files the spec already names.** New `agent/src/kill_latch.rs`. `agent/src/api/kill_switch.rs`, `agent/src/lib.rs`, `agent/src/kill_switch_audit.rs` (audit DB off `temp_dir()`). `station/StationApp/AgentSupervisor.swift` `shutdown` and the reap inside `spawnAgent`. New `GET /api/daemon/kill-switch/state`.

**Policy already decided (2026-07-31, spec §6).** Quit is not consent. Reboot is still an accepted bypass until a later LaunchDaemon (`SMAppService`). Do not add that daemon here. Do not auto-dismiss on SIGTERM. Do not edit the Console `crates/agent` mirror. Empty host set still refuses (R8).

**Acceptance.** Arm L3, quit Station, hosts lines remain, relaunch attaches, dismiss is audited and clears hosts only after removal succeeds. `station/StationTests/AgentSupervisorTests.swift` crash-loop backoff still passes.

**Out.** In-app DNS setup UI, user-set L2/L3 duration (roadmap T7 / K4), AlertBus. Countdown display should read latch `expires_at_ms` when the latch exists; until K4, the duration stays the existing 90s policy constant.

---

## Wave 4 — Margins and the Plan sizer

**Goal.** The trader sees funds and a proposed size, and sees an honest hole where the margin calculator is unspecified. Basecamp 1.7 and 1.8.

### 4.1 Funds glance (display)

Obtain `funds` on the shipping book (`kotak_neo` cash, `binance_com` spot) onto the Plan strip and a Station row. Source is `GET /api/station/obtain`. This is the glance, not parked S8 pulse+ledger (`plans/s8-account-chrome.md` stays parked: Settings → Broker pulse + full ledger).

Dark when obtain is dark. Never invent a balance.

### 4.2 Sizer

Blocked until a `docs/reference/` doc exists for the sizing rule. The prototype identity in `plans/harness-remaining.md` (authored qty, DualNoBlend, no stop → do not invent one) is the product shape. The formula `size = planned_risk / |entry − invalidation|` from the completion roadmap may be implemented only after that doc cites a primary source. If the source does not specify a default percent of capital, write `NOT SPECIFIED IN SOURCE` and ship a typed risk box fed by `DeskRulesStore.dailyFloor` as a display input, with size dashed when funds are dark.

Settings floor still does not POST loss-limits and still does not arm DNS.

Post-Confirm risk mosaic (A7) ships in Swift only after the HTML host in `plans/harness-remaining.md` Phase 3 is unblocked (founder sign-off + reference doc). Phase P0 HTML is already checked off. Do not merge `PROTOTYPE-notch-risk-engine.html` into the host.

### 4.3 Set SL copy

`BarProtectiveSlPlanChrome.presentation` keeps `showsSetSlButton: false`. Replace status strings that say “Set SL at ₹…” with copy that states the stop is recorded on the plan and is not an order. Tests: `notch/Tests/NotchTests/BarProtectiveSlPlanChromeTests.swift`.

### 4.4 Margin calculator

Leave `extract_margin_estimate()` as the unavailable hole. Do not parse Kotak `SpanMarginPrsnt` into it (`docs/reference/india/kotak-neo/MARGIN-CALCULATOR.md`, `FUNDS-LIMITS.md`). Glance tests must keep `margin` unavailable on NFO and crypto options until a reference names host, path, auth, SPAN vs exposure, and multi-leg behavior together.

**Acceptance.** Plan shows dashed size when invalidation or funds are missing, a number when both exist and the reference doc allows the formula, and the margin chip still says unavailable. Set SL button remains absent.

---

## Wave 5 — Option UI, name, pill, cancel

**Goal.** Close Basecamp Notch items 3–6 without violating book separation or the journal sheet.

| Ticket | Do | Do not |
| --- | --- | --- |
| Option UI | Founder signs a density borrow (nouns, spacing) from the crypto options board onto NFO, book by book, or this item is dropped | One shared view, shared currency, `if crypto` inside NFO chrome |
| Option name | One string from the founder, applied to the option surface title | An agent-chosen name |
| Pill | If the founder signs removal: Station-hosted `NotchLauncher.start()` does not show `CollapsedView`; expand still works from ⌥Space and from Station “Open Notch”. If unsigned: finish honesty (exposure stays dark — `DeskHonestyPillTests.exposureStaysDarkNotQtyTimes1000`) and keep the pill | Delete `CollapsedView.swift` while harness-remaining Phase 2 is still the desk-honesty wave |
| Delete | A Notch control that calls the existing cancel route with a required `cancel_reason_chip`, copy “Cancel declaration” | SQL or API hard delete; hiding cancelled rows |

**Files.** `notch/BarCryptoOptionsDeclareView.swift`, `notch/BarOptionsDeclareView.swift`, `notch/BarNfoCockpitMosaic.swift`, `notch/CollapsedView.swift`, `notch/NotchLauncher.swift`, `notch/NotchViewModel.swift` cancel call ~3193, `agent/src/api/bar.rs` `cancel_declaration_handler`.

---

## Wave 6 — Sparkle trust and the agent button

**Goal.** Basecamp 1.10 and 1.11. Updates install without Gatekeeper theater. The restart button cannot become the kill bypass Wave 3 just closed.

### 6.1 DNS (operator, not a code wave)

Follow `docs/runbooks/updates-domain-vercel.md`. `SUFeedURL` stays `https://updates.tradeautopsy.in/appcast.xml`. Do not retarget the plist at `updater-omega.vercel.app`. Verify:

```bash
dig +short CNAME updates.tradeautopsy.in @8.8.8.8
dig +short TXT _vercel.tradeautopsy.in @8.8.8.8
curl -fsS https://updates.tradeautopsy.in/appcast.xml | head
```

GoDaddy records are in that runbook (CNAME to the Vercel project host, TXT `vc-domain-verify=…`). One target only. The conflicting GoDaddy-Pages runbook (`docs/runbooks/updates-domain-godaddy.md`) is backup and must not be applied on top of the Vercel CNAME.

### 6.2 Notarize by default

`/.github/workflows/release.yml` currently defaults `notarize` off and tells users to right-click Open. `scripts/sign-and-notarize-station.sh` exists. Turn the release the founder ships into a Developer ID + notarized DMG. Keep an explicit unsigned lane for contributors without the Apple secrets. Sparkle install must still contain `Contents/MacOS/tradeautopsy-agent` on port 9137.

Beta channel stays later (`plans/station-sparkle-update-infrastructure-2026-09-25.md` Phase 4). Do not redo Phase 1 client integration.

### 6.3 One-button stop and start

**Files.** A Station control (toolbar next to `AgentHealthToolbarStatus`, or Health — `station/StationApp/Views/` Health panel is `HealthPanelView` in `StationShellView.swift`). Implementation calls a new supervisor method that:

- if `GET /api/daemon/kill-switch/state` says latched (Wave 3), refuses to SIGKILL and tells the trader the Enforcer is holding a kill
- otherwise `shutdown()` then `spawnAgent()`, then `retryAgent()`’s health sync

**Acceptance.** Unlatched: button stops the listener and a new process binds 9137. Latched: button does not reap; hosts block remains. Menu-bar health dot follows the same supervisor state.

---

## Wave 7 — One journal object and medium honesty

**Goal.** N2: Plan, Working, and Debrief are one current-date object. Medium gaps from inputs A and B that are real and small enough to name.

| Ticket | Files / proof | Acceptance |
| --- | --- | --- |
| N2 day sheet | Notch debrief payload `notch/BarPostTradeDebriefPayload.swift`; journal card snapshot in `JournalWeek.swift`; roadmap §2.1–2.3 | The day sheet shows the same declaration’s plan snapshot, working compare, and debrief moments. Empty Moment C sets Due (Wave 0.3). |
| Condition fire log | Stored on that object when Working marks a rule true | History is a list of rule id + time. It does not place or flatten. |
| Morning brief route | Add `GET /api/daemon/morning-brief` or point the client at an existing route. Client: `notch/NotchViewModel.swift` ~4840, models `notch/BriefMorningBriefModels.swift` | 200 with desk facts (book, clock, yesterday from Today) or an honest empty. No −₹2,100 demo chart. Header B + gate D (`plans/harness-remaining.md`). |
| Intervention accessory | `notch/BarInterventionCardSpec.swift` `PrimaryAccessory.manageInWebBar` | Accessory opens Notch Kill or is `.none`. No `https://localhost:3000` and no web Bar URL. |
| Setups / Process / Book | Leave `BarPatternsChartView` / `BarFidelityChartView` as honest empty until real ticket counts exist (roadmap Wave 4, threshold labeled) | No demo 88–96% series. |
| Manual fill Path A | Founder runs a real read-only fill through debrief, or the harness impulsive lane | Journal shows the row. This is a proof, not a new writer in Station. |

**Friction audit (Basecamp 1.5–1.6).** Write the notes from dogfood of Waves 0–3 into this wave’s PR description. Do not open a redesign.

---

## Leave alone (after these waves, or never in them)

- **Phase Z / T9.** Seven questions in `TRADEAUTOPSY-CONTEXT.md` §5.1 are unanswered. Declarations stay on Console `/api/bar/v1` until a founder memo answers them. S1 does not authorize the cut.
- **S8 account chrome** and Console S8 waterfalls. `plans/s8-account-chrome.md`. Scoped S8 glance is already signed (`plans/s8-scoped-dogfood-2026-09-19.md`). Do not checkout `feat/s8-notch-account-chrome` to finish Wave 4.
- **LaunchDaemon / reboot-proof kill.** Spec Phase 2. Reboot remains a written bypass.
- **M3 flatten, `routeOrder`, Set SL send, withdraw permission.** ADR 0002 and the Binance.US withdraw hard block stay.
- **AlertBus, Health helpline, brokers marketplace, India Tier II, NFO as a P&L owner, guided TCC, pairing multi-Mac, Sparkle beta, Report honesty.** Real backlog (input B mediums, `plans/broker-marketplace-handoff.md`, `plans/india-multi-book-stack-2026-09-25.md`). They do not unblock daily use.
- **Rebuilding LiveBook, Sparkle client, outbox refresh, or the five Journal facets from scratch.** They are on `main`. Extend them.
- **Second tag store, second P&L owner, labs series inside Kill or Today.** Roadmap durable law.
- **OpenAlgo runtime, ports 5000/8765.** AGENTS.md invariant 11.

---

## Dogfood and tests already in tree

| Proof | Path |
| --- | --- |
| Declare → Working → Debrief sink, optional match inject, capture accept | `scripts/notch-live-journal.sh`, `scripts/notch-live-journal-lib.py`, `plans/NOTCH-LIVE-JOURNAL-HARNESS.md` |
| Why match waits on `status=matched` | `plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md` |
| Fill-matched contract | `plans/FILL-MATCHED-INJECT-SKETCH.md` |
| Strong testing phases | `plans/STRONG-TESTING-SYSTEM-2026-09-27.md` |
| Kotak depth / funds | `plans/kotak-dogfood-2026-09-18.md` |
| Journal projection | `station/StationTests/JournalWeekProjectionTests.swift` |
| Capture dead-letter | `agent/tests/capture_delivery_phase3.rs` |
| Paired-device 401 refresh | `agent/tests/station_paired_device.rs` |
| Bar forward | `agent/tests/bar_forward.rs` |
| Kill apply / audit (teeth, not latch) | `agent/tests/kill_switch_apply.rs`, `agent/tests/kill_switch_audit.rs` |
| Pill honesty | `notch/Tests/NotchTests/DeskHonestyPillTests.swift` |
| Set SL cut | `notch/Tests/NotchTests/BarProtectiveSlPlanChromeTests.swift` |
| Working compare | `notch/Tests/NotchTests/BarWave2WorkingTests.swift` |

Cloud agents do not run the Mac Notch. A ticket is done when its unit tests pass and the harness command is named for the founder.

---

## What Composer 2.5 implements first

One PR. Wave 0.1 only.

**Title shape:** `fix: keep PLAN up when declare refresh 401s`

**Scope**

1. `agent/src/api/bar.rs` `declare_handler`: upstream HTTP 401 uses `archive_kept_response` (local declare stays). HTTP 400 validation still applies `LiveBookEvent::Cancel`.
2. `agent/tests/bar_forward.rs`: cover that 401 path.
3. `notch/NotchViewModel.swift` `submitBarDeclaration`: 2xx + `archive_error` keeps `declarationId`, refetches live state, shows the sign-in line, and does not clear optimistic armed.

**Do not touch** `agent/src/outbox.rs`, `JournalViewModel.swift`, kill switch, Sparkle, or the margin hole.

**Done when** the new bar-forward test passes, existing `station_paired_device` tests pass, and a successful refresh still archives to Console.

---

Ready for Composer 2.5 — first implementation ticket: **Wave 0.1 PLAN JWT** — on declare, an upstream 401 after refresh failure must archive-keep the LiveBook row and leave the Notch on Working with a sign-in line (`agent/src/api/bar.rs` `declare_handler`, `notch/NotchViewModel.swift` `submitBarDeclaration`), without redoing paired-device refresh (`d6327cc`).

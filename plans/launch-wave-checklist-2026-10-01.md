# Launch wave checklist — Station beta / dogfood honesty

**Date:** 2026-10-01  
**Repo:** `tradeautopsyco-blip/tradeautopsy-station`  
**Tip:** `main` @ PR #23 (`a66b451`) — FixedRisk sizing + desk hotkeys, on top of PR #21–#22 lineage.

**Authority for language and wave law:** [`plans/station-completion-roadmap-2026-09-17.md`](./station-completion-roadmap-2026-09-17.md)  
**Merged sequencing (code wins over September stale rows):** [`plans/MASTER-STATION-NOTCH-BASECAMP-2026-10-01.md`](./MASTER-STATION-NOTCH-BASECAMP-2026-10-01.md)  
**Sizing / preview design:** [`docs/design/risk-engine.md`](../docs/design/risk-engine.md)  
**Harness arc (multi-trade):** [`docs/design/harness-trade-arc.md`](../docs/design/harness-trade-arc.md) — §1 table is **stale** vs PR #21; treat code + this checklist as truth.  
**Screen layout:** [`docs/design/harness-open-plan-working.md`](../docs/design/harness-open-plan-working.md)  
**Backlog after LiveBook:** [`docs/design/founder-backlog.md`](../docs/design/founder-backlog.md)  
**Hotkey contract:** [`docs/design/hotkey-actions.md`](../docs/design/hotkey-actions.md)  
**Sizing reference (what may author qty):** [`docs/reference/behavioral-science/POSITION-SIZING.md`](../docs/reference/behavioral-science/POSITION-SIZING.md)  
**Kill lifetime (Phase 1 shipped; M3 flatten still out):** [`plans/kill-switch-lifetime-spec.md`](./kill-switch-lifetime-spec.md)

This is a **Launch Wave** for honest beta/dogfood on **this Mac** — not a rewrite of the September roadmap. Do not call beta honest until the numbered slices below are dogfooded or explicitly accepted as known gaps.

---

## Product constraints (check every slice)

These are locks, not suggestions. Cite them in acceptance tests and dogfood notes.

| Lock | Where it lives |
| --- | --- |
| **DualNoBlend** — one book, one currency; never blend INR + USDT into one hero | Roadmap “Books”; `notch/BarAccountChrome.swift`; `CONTEXT.md` |
| **Never invent** quotes, PnL, fee %, default risk %, leverage, chain, OI | `AGENTS.md`; reference library discipline; `POSITION-SIZING.md` header |
| **No `routeOrder`** / live place from sizer; **Confirm fail-closed**; **Kill ≠ Stop** | ADR 0002; `docs/design/risk-engine.md` §1; `BarPlanKillChrome.swift` |
| **Fees OUT** of this launch (optional later: manual fee field, Station only) | `POSITION-SIZING.md`; preview `fees_reason: fee_schedule_unspecified` |
| **Kill UI stays** — copy must say **rolling soon** for full flatten-to-user-conditions; today’s Kill is DNS/desk lock + warning, **not** max-loss flatten | `BarPlanKillChrome.swift` `warningBody`; `hotkey-actions.md` §Kill copy |
| **Console OUT** of sizing/hotkeys scope; tags/playbooks Console↔Station sync **deferred** | This checklist §Deferred; `founder-backlog.md` item 5 |
| **Multi-trade LiveBook must not regress** to single-slot replace | `agent/src/live_book.rs` `push_pending`; PR #21 tests — re-run dogfood §5 steps 9–11 |

**Beta pricing (go-to-market only, not code):** Beta Pro ₹499/mo or ₹4,999/yr founding; Free forever with limits — mention only when packaging a external beta invite, not as an engineering gate.

---

## 1. Already shipped (as of 2026-10-01 / through PR #23)

Evidence paths only — no “done” without these.

### Data spine & prove (pre–Harness UX)

| Item | Evidence |
| --- | --- |
| LiveBook mutate + declare apply-first; 401 after failed refresh **keeps** local row | `agent/src/live_book.rs`, `agent/src/api/bar.rs` `archive_kept_response` on 401 |
| Capture outbox `ensure_fresh` + 429/5xx retry | `agent/src/outbox.rs` (MASTER plan: do not rebuild) |
| Journal **Post due** facet + Station trip cite wiring | `station/StationApp/Presentation/JournalWeek.swift` `JournalFacet.due`; `JournalTripCiteInventory.swift`; `JournalViewModel.rebuild` passes cites |
| Kotak **12h TOTP card** on Start | `BrokersViewModel.swift` `totpCardAfterSeconds` |
| Kill **latch** Phase 1 (quit does not drop latched Enforcer) | `agent/src/kill_latch.rs`; `AgentSupervisor.isKillLatched()` |
| Morning brief route (contract envelope) | `agent/src/api/morning_brief.rs`; `GET /api/daemon/morning-brief` |
| Sparkle client + appcast URL in plist | `SparkleSoftwareUpdateController.swift`; `Info.plist` `SUFeedURL` |

### PR #21 — Multi-trade LiveBook

| Item | Evidence |
| --- | --- |
| `pending_declarations[]` + selected mirror; declare **appends** | `agent/src/live_book.rs` `push_pending`, `set_selected` |
| Working list + **Plan another →** empty form | `notch/BarPlanStateView.swift`; `NotchViewModel.presentBarDeclarationForm()` |
| Declaration-scoped cancel/protective | `LiveBookEvent::Cancel` / `Protective` by id |

### PR #22 lineage — Harness Phases 1–3 (UX + honest holes)

| Item | Evidence |
| --- | --- |
| Open/Plan/Working/Debrief surfaces + partial backlog UI | Commit `2d1968e`; `docs/design/founder-backlog.md` status |
| Risk preview strip + agent `POST /api/daemon/risk/preview` | `agent/src/api/risk.rs`, `notch/BarPlanRiskPreview*.swift` |
| Local tags/playbooks stores; condition-fire ids; debrief picker improvements | `BarDeclarationTagsStore.swift`, `BarPlaybookStore.swift`, `BarPostTradeView.swift` |
| Setups/Process placeholders honest-empty (no fake %) | `notch/BarNotchShell.swift` `BarPatternsChartView` / `BarFidelityChartView` captions |
| SL suggestor UI; stop still **—** when formula unlocked | `BarPlanSlSuggestor*.swift` |

### PR #23 — FixedRisk preview + desk hotkeys

| Item | Evidence |
| --- | --- |
| Nautilus **FixedRisk** in agent (`commission_rate = 0`); locked calc profiles author `authored_qty` | `agent/src/risk/fixed_risk.rs`, `agent/src/risk/preview.rs`; `POSITION-SIZING.md` |
| India: lots × `lot_size`; **post-size margin cap only** (no margin-as-sizer) | `preview.rs` `nfo_future` branch + `margin_gate_exceeded` |
| Plan **Apply size** from preview | `notch/BarPlanRiskPreviewRow.swift` `Apply size` |
| **HotkeyRegistrar** loads `DeskHotkeyPreferences`; empty prefs → ⌥Space / ⌥⇧Space only | `station/StationApp/HotkeyRegistrar.swift`; `docs/design/hotkey-actions.md` |
| Kill warning + **“rolling soon”** flatten honesty | `notch/BarPlanKillChrome.swift`; `BarPlanKillChromeTests.swift` |
| Release hygiene: AppIcon in bundle; wasm target in release workflow | PR #23 `station/Scripts/compile-app-icon.sh`, `.github/workflows/release.yml` |

---

## 2. Launch Wave — must ship or dogfood before “beta honest”

Numbered thin slices. Each should be **dogfoodable in one session** on the founder Mac. Owner tags: **station** · **notch** · **agent** · **ops** · **console** (other repo).

### 2.1 Spine proof — declare survives a bad day (**station** · **notch** · **agent** · **console**)

- [ ] **401 declare:** With expired/revoked Caller JWT, declare still on Working after refresh; sign-in line visible — not `clearOptimisticArmedStorage()` on 2xx + `archive_error`. Harness: [`plans/NOTCH-LIVE-JOURNAL-HARNESS.md`](./NOTCH-LIVE-JOURNAL-HARNESS.md), `./scripts/notch-live-journal.sh`.
- [ ] **Capture ACK or named dead letter:** Toolbar capture → `ACKED` or Settings/outbox shows `validation` / `station session revoked` / max attempts — [`plans/TOOLBAR-CAPTURE-OUTBOX-DEADLETTER-2026-09-27.md`](./TOOLBAR-CAPTURE-OUTBOX-DEADLETTER-2026-09-27.md).
- [ ] **Match fidelity (Console-gated):** Synthetic lane `POST …/test/fill-matched` with `BAR_TEST_INJECT_FILL_MATCHED=1` on Console — [`plans/FILL-MATCHED-INJECT-SKETCH.md`](./FILL-MATCHED-INJECT-SKETCH.md). Production match on real fills remains **console** work ([`plans/CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md`](./CLOSED-TRIP-DEBRIEF-FINDINGS-2026-09-27.md)).

### 2.2 Multi-trade regression guard (**agent** · **notch**)

- [ ] Second Confirm **does not** drop first pending (`agent/tests/live_book.rs` multi-row cases + manual §5).
- [ ] `matched_declaration_id` / `undeclared_position` still **single-slot** — document behavior when two fills overlap; do not invent links by symbol alone ([`docs/design/harness-trade-arc.md`](../docs/design/harness-trade-arc.md) §4–5). Fix only if dogfood hits a real collision.

### 2.3 Sizing & R:R after #23 (**notch** · **agent**)

- [ ] **Dogfood Apply size** on shipping books: Binance spot (C1), Kotak cash (C1), Kotak NFO future with known lot (C2), USDM (C7/C8 per lock log). Dark funds → **—**, not a number ([`docs/design/risk-engine.md`](../docs/design/risk-engine.md) §1.2).
- [ ] **Unlocked books stay dashed:** options premium, Coin-M, MCX, CDS — `profile_not_locked` / `position_sizing_unspecified` in preview.
- [ ] **No default risk %** until founder lock (§3) — typed percent only; Settings floor is display, not auto-fill ([`notch/DeskRulesStore.swift`](../notch/DeskRulesStore.swift)).
- [ ] **R:R gate** still display-only until lock — bands in `BarIntradayDeclareValidator.swift`; roadmap Wave 3.2 refuse-vs-warn **open**.

### 2.4 Hotkeys dogfood (**station** · **notch**)

- [ ] Save bindings in Harness Settings → Desk hotkeys (`BarDeskHotkeyPreferencesView.swift`); verify `HotkeyRegistrar.reloadSavedBindings` replaces ⌥Space/⌥⇧Space when saved.
- [ ] **`kill` id** opens warning only — never skips Confirm on Kill card ([`hotkey-actions.md`](../docs/design/hotkey-actions.md)).
- [ ] **`confirm_declare`** respects same gates as button — fail-closed.
- [ ] **No default map** for Kill/Confirm/Cancel until founder picks Option B in hotkey doc.

### 2.5 Desk honesty — pill, Today, exposure (**notch** · **station**)

- [ ] Collapsed pill: book currency or dash; **no** qty×1000 exposure hero — test `DeskHonestyPillTests.exposureStaysDarkNotQtyTimes1000`.
- [ ] Today closed hero + DualNoBlend remaining — [`plans/station-journal-settings-today.md`](./station-journal-settings-today.md) Wave T acceptance.
- [ ] Cull or gate any remaining demo P&L on **Open** (morning brief must not tell a fake day story) — prototype notes [`station/prototypes/PROTOTYPE-morning-brief.NOTES.md`](../station/prototypes/PROTOTYPE-morning-brief.NOTES.md).

### 2.6 N2 — one journal object (**station** · **notch** · **agent**)

- [ ] Plan + Working + Debrief **same `declaration_id`** on current-date journal lander — [`plans/harness-remaining.md`](./harness-remaining.md) Phase 4 (still open).
- [ ] Overlay tabs do not forget selection when switching Open/Plan/Working/Debrief.
- [ ] Post-due sidebar count matches matched rows with empty post ([`JournalWeek.swift`](../station/StationApp/Presentation/JournalWeek.swift)).

### 2.7 Live conditions + depth (**notch** · **agent** · **ops** weekday)

- [ ] Working compare uses obtain last with honesty chip — `BarWorkingCompare.swift`.
- [ ] **S3 leftover:** NFO + options depth together; COM gap → Unusable; glance 20 rows — roadmap “data spine”; dogfood [`plans/kotak-dogfood-2026-09-18.md`](./kotak-dogfood-2026-09-18.md).

### 2.8 Kill honesty vs lifetime (**agent** · **station** · **notch**)

- [ ] Latch dogfood: arm L3, quit Station, hosts stay, relaunch attaches — [`kill-switch-lifetime-spec.md`](./kill-switch-lifetime-spec.md) §7 tests.
- [ ] **Restart agent** blocked while latched — `StationAppCoordinator.retryAgent()` message.
- [ ] UI copy: **not** max-loss flatten; **rolling soon** for condition-based flatten — do not regress PR #23 string.
- [ ] Reboot bypass still **accepted** until LaunchDaemon — roadmap lock §6.

### 2.9 Kotak / S6 auth leftovers (**station** · **ops**)

- [ ] **12h TOTP** exercised on real Start (not just unit policy).
- [ ] **S6 founder leftover:** live cash holdings / positions / obtain(funds) / orders after TOTP remint — roadmap table; not a cloud-agent proof.

### 2.10 Release path (**ops** · **station**)

- [ ] **Signed + notarized** DMG default for external beta — `release.yml` `notarize: true` + Apple secrets; [`plans/station-sparkle-update-infrastructure-2026-09-25.md`](./station-sparkle-update-infrastructure-2026-09-25.md) (Sparkle integrated; notarize/DNS remain).
- [ ] **`updates.tradeautopsy.in`** serves appcast — [`docs/runbooks/updates-domain-vercel.md`](../docs/runbooks/updates-domain-vercel.md).
- [ ] **CI before ship:** manual `workflow_dispatch` [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) — agent `cargo test --all-features`, Notch `swift test`, Station tests; Rust edition **2021** (`agent/Cargo.toml`).
- [ ] Bump `CFBundleShortVersionString` / `CFBundleVersion` + appcast entry together (currently plist **0.2.1** build **5**; appcast may lead — reconcile on release).

---

## 3. Founder decisions still open

Answer in product (roadmap §614–625 + MASTER conflicts). **Do not encode guesses in code.**

| # | Decision | Shipping default until locked |
| --- | --- | --- |
| 1 | Overlay name: **Harness** / Plate / Kit | Copy uses **Harness** ([`station-completion-roadmap-2026-09-17.md`](./station-completion-roadmap-2026-09-17.md) §Common language) |
| 2 | Open (morning): header **B + gate D** vs variant A | Client calls morning-brief; founder picks facts ([`PROTOTYPE-morning-brief.NOTES.md`](../station/prototypes/PROTOTYPE-morning-brief.NOTES.md)) |
| 3 | Default planned risk: **1% of obtain(funds)** vs typed box only | **Typed only** — `POSITION-SIZING.md` “no product default for risk_pct” |
| 4 | R:R refuse: **hard gate** vs warn + tag | Warn bands today; gate open (roadmap Wave 3.2) |
| 5 | Multi-conditional on live: prompt-only vs auto (auto needs `routeOrder` lock) | Prompt-only |
| 6 | Kill **reboot-bypass** acceptable until LaunchDaemon? | Yes per kill spec §6 / roadmap lock |
| 7 | Collapsed **pill** remove vs keep with honesty | Keep unless founder signs Station-only chrome (MASTER §Conflicts) |
| 8 | Indian vs crypto **option UI** merge | **No** — dual surfaces; density borrow needs sign-off |
| 9 | **Option surface name** string | Blocked — Harness/Plan language until named |
| 10 | Hotkey **default map** Option A vs B | **Option A** — empty prefs ([`hotkey-actions.md`](../docs/design/hotkey-actions.md)) |

---

## 4. Explicitly deferred / out of this launch

| Area | Why out | Pointer |
| --- | --- | --- |
| **Fees engine** / `commission_rate` ≠ 0 | No fee schedule in reference | `POSITION-SIZING.md`; preview fees null |
| **Console** sizing, hotkeys, tags/playbooks sync | Station scope only this wave | `founder-backlog.md` §5 |
| **`routeOrder`**, place/modify/cancel, Set SL send | UBI read-only ADR 0002 | `host_policy.rs` |
| **M3 flatten**, max-loss trip from Book list | Kill is DNS lock today | Roadmap Wave 4 Book; Kill copy |
| **Real flatten Kill** to user conditions | **Rolling soon** label only | `BarPlanKillChrome.swift` |
| **Post-Confirm dual-outcome risk mosaic** | harness-remaining Phase 3 blocked | [`harness-remaining.md`](./harness-remaining.md) |
| **Margin calculator** / SPAN as broker RMS | Always unavailable hole | `margin_estimate.rs`, `MARGIN-CALCULATOR.md` |
| **Console S8** account chrome / waterfalls | Parked | [`plans/s8-account-chrome.md`](./s8-account-chrome.md) |
| **Phase Z** — cut `/api/bar/v1` | After N2 + local truth answered | MASTER §Order decision |
| **OpenAlgo runtime** ports 5000/8765 | Invariant 11 | `AGENTS.md` |
| **Derived Setups/Process/Book** with fake series | Honest empty only until ticket threshold | Roadmap Wave 4 |
| **Report, AlertBus, India Tier II, marketplace** | MASTER “Leave alone” | MASTER deduped PARK row |
| **σ 2–5**, Black-76, PCR, max pain | Dark by policy | `harness-remaining.md` |

---

## 5. Dogfood script (founder Mac, ~30–45 min)

Run on **shipping books** you actually trade (e.g. Binance.US read-only + Kotak cash). Gate B: signed-in agent, Console up for archive proof.

1. **Start agent** — Station shows port **9137** healthy; no secrets in logs.
2. **DualNoBlend** — Connect INR book + USD book; confirm **separate strips**, no blended hero (`BarAccountChrome`).
3. **Open → Plan** — Empty form; set symbol, entry, stop, target; funds **lit** or confirm **—** when dark.
4. **Risk preview** — Wait for preview row; on locked book, tap **Apply size**; qty fills; re-preview shows ladder risk.
5. **Confirm declare** — Working list shows row; refresh Station/expand Harness — row **still there** (N1).
6. **Plan another** — Second symbol Confirm; **both** pendings in list (PR #21 regression).
7. **Hotkeys** — Bind `focus_working`, `plan_another`, `kill` in Settings; exercise each; Kill shows warning with **rolling soon** / not max-loss flatten.
8. **Working** — Select each row; invalidation compare shows waiting/true/dark honestly (`BarWorkingCompare`).
9. **Cancel one** — Selected declaration cancel with reason chip; other pending remains.
10. **Journal** — Week view: **Post due** facet; matched card shows **cited trip net** or dash — never invented FIFO.
11. **Today** — Closed hero only; floor remaining dashes if mixed currency.
12. **Kill** — Arm from Plan warning → overlay countdown → I’m Calm; **Stop** broker sync does not disarm kill policy (Kill ≠ Stop).
13. **Quit + relatch** — With kill latched, quit Station; verify block persists; relaunch attaches (Wave 3 spec).
14. **Capture** — Toolbar region capture → outbox ACK or readable dead letter.
15. **Optional Console inject** — `./scripts/notch-live-journal.sh --inject-matched` if flag on; else note 404 as honest block.

Record failures in a dated note under `plans/` — do not mark checklist items done without a line of evidence.

---

## 6. Release / ops

| Task | Owner | Notes |
| --- | --- | --- |
| Manual **CI** green on `main` before tag | ops | `.github/workflows/ci.yml` `workflow_dispatch` |
| **Release** workflow: notarized DMG + EdDSA appcast | ops | `.github/workflows/release.yml`; commit appcast like `ba5798d` |
| DNS **updates.tradeautopsy.in** | ops | Runbook verification |
| **station-wire** version bump when protocol changes | station/agent | README release discipline |
| **Kotak TOTP + live S6** row | ops/founder | Weekday session; not CI |
| **Withdraw permission** hard block on Binance.US keys | station | `AGENTS.md` test user invariant — every validation flow |
| **Beta invite copy** (optional) | product | ₹499/mo · ₹4,999/yr founding; Free tier limits — no plist hardcoding |

---

## 7. Honest status summary

| Theme | Shipped enough to dogfood | Still blocks “beta honest” |
| --- | --- | --- |
| LiveBook / multi-plan | PR #21 + apply-first | Founder refresh/cancel script §5; single-slot match/fill edge cases |
| Harness UX | PR #22 partial UI | N2 one object; Open without demo story |
| Sizing | PR #23 FixedRisk + Apply | Unlocked asset classes; R:R gate decision; post-Confirm mosaic |
| Hotkeys | PR #23 wiring | Saved-map dogfood; no unsafe defaults |
| Kill | Latch + warning copy | M3 flatten explicitly out; reboot bypass documented |
| Console archive | Declare forward | Production **matched** on real fills |
| Ship | Sparkle in app | Notarized default + DNS + CI ritual |

**Next doc hygiene:** Update [`docs/design/harness-trade-arc.md`](../docs/design/harness-trade-arc.md) §1 “Still one slot” table to match PR #21 — stale doc misleads agents.

---

*Checklist only — no product code in the PR that adds this file.*

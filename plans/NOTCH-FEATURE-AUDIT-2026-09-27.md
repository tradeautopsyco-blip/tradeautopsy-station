# Notch / Station Feature Audit — Live Trade-Flow

**Date:** 2026-09-27 (Asia/Calcutta)  
**Method:** Live E2E against Station Debug + agent on `127.0.0.1:9137` via wire-v1 HMAC (StationWire). GUI keystroke/screencapture **TCC-blocked** — Notch screen chrome not visually exercised; PLAN/bar judged by signed API + process state.  
**Do not invent passes.** Unit tests are **not** used as WORKING evidence for trade flows.

## Executive summary (scoreboard)

| Verdict | Count | Meaning in this audit |
|--------|------:|------------------------|
| **WORKING** | 12 | Live hop succeeded end-to-end for the shipping path (or safe read path) |
| **BROKEN** | 3 | Live hop returned wrong/failed result while infra was reachable |
| **BLOCKED** | 14 | Trade flow cannot complete — hard gate (expired Station Caller / Console bar auth) |
| **UNKNOWN** | 8 | GUI-only or fire-path not exercised (TCC / refuse to arm kill) |
| **N/A** | 11 | Not Live Station v1 shipping (REFERENCE / culled / not offered) |

**Headline:** The day desk **broker sync + Today + wire + market quote reads** work with **Kotak Neo cash** active (Binance.com spot also polled in sync-hint). The **full Notch trade circuit** (Open → Plan/declare → Working/armed → Debrief, plus Match/Settings loss-limits, Stop-me) is **BLOCKED**: Station Caller JWT in Keychain is **invalid/expired** (`GET /api/daemon/auth/station/session` → `signed_in:false`, Console 401). Every `/api/daemon/bar/*` live call returns **401 Authentication required**. No declare was accepted; no armed Working state was observed.

---

## Live environment (discovery)

| Fact | Evidence |
|------|----------|
| Station Debug running | PID 58040 `TradeAutopsy Station` (DerivedData Debug) |
| Agent listening | PID 58060 `tradeautopsy-agent` · `localhost:9137` LISTEN |
| Wire | HMAC + `x-daemon-secret` required; bare curl → 412 PROTO / SIG — **not** a broken bar route |
| Health | `GET /api/daemon/health` **200** `status=ok` build `177d9405878f` |
| Station Caller Keychain | Service `TradeAutopsy` / account `station_caller_tokens` **present** (created 2026-09-21, modified 2026-09-23 UTC) |
| Session proof | `GET /api/daemon/auth/station/session` **401** `SESSION_PROOF` · Console: Invalid or expired token · `signed_in:false` |
| Active broker | `GET /api/daemon/broker/sync-state` **200** · `brokerSlug=kotak_neo` · `kotak_neo_wasm` · `calcProfileId=equities_inr_cash` |
| Caps (Kotak) | fills/funds/holdings/instruments/positions **fresh**; orders **unavailable**; quote **unknown** |
| Kill DNS | `killDnsActive=false` |
| Sync-hint books | `binance-com-spot` + `kotak-nse-bse-cash` both polled; fill_count **0** each |
| Shipping pair (product) | Live Station v1: **Binance.com spot** + **Kotak Neo cash** (`nse_cm`/`bse_cm`, CNC+MIS). NFO / USDM / Coin-M / Options = REFERENCE UI, not Start shipping books |

---

## Scoreboard by area

### A. Station desk routes

| # | Name | Where | Expected (1 line) | Verdict | Evidence |
|---|------|-------|-------------------|---------|----------|
| A1 | Today | `station/StationApp/Views/Today/TodayView.swift` · `GET /api/daemon/today` | Day P&L / pulse for connected book | **WORKING** | Live **200**; `brokerSlug=kotak_neo`, `quoteCurrency=INR`, hero nulls/empty trades (honest empty day), `degradedReason=null` |
| A2 | Journal | `Views/Journal/JournalView.swift` · capture outbox | Journal list + capture accept path | **WORKING** (outbox) / **UNKNOWN** (GUI list) | `GET …/journal/toolbar-capture/outbox/status` **200** `acked:2`; GUI Journal screen TCC-blocked |
| A3 | Brokers | `Views/Brokers/BrokersView.swift` · sync-state | Connect/start shipping brokers; show sync health | **WORKING** | Live sync-state **200** Kotak connected; sync-hint shows both shipping books polled; credentials Keychain services present for provider keys |
| A4 | Health | `BackendBox/Views/HealthPanelView.swift` · `/api/daemon/health` | Agent health panel | **WORKING** | Health **200** `status=ok`; route wired in `StationShellView.swift` |
| A5 | Market Data | `BackendBox/Views/MarketDataKeysView.swift` | Market-data / Backend Box keys | **UNKNOWN** | Code route exists; no live UI exercise (TCC); no separate market-data write probe run |
| A6 | AI / Workflow | `BackendBox/Views/AIWorkflowKeysView.swift` | AI/workflow keys surface | **UNKNOWN** | Code route exists; not exercised live |
| A7 | Settings | `Views/SettingsView.swift` | Thin Station settings | **UNKNOWN** | Code route exists; not exercised live |
| A8 | Device login / session | Auth + `GET /api/daemon/auth/station/session` | Signed-in Station Caller for Console-dependent features | **BROKEN** | Keychain token **exists** but session proof **401** expired/invalid → `signed_in:false` |

### B. Notch PLAN screens (Harassed sidebar: Open / Plan / Working / Debrief / Match / Patterns / Fidelity / Triage / Settings)

| # | Name | Where | Expected (1 line) | Verdict | Evidence |
|---|------|-------|-------------------|---------|----------|
| B1 | Open (Morning brief) | `BarNotchShell` → `BriefLeftView` · `fetchMorningBrief` | Session open brief before declare | **BLOCKED** | No local morning-brief daemon route (404); brief/live paint depends on Console/bar identity; session **unsigned**; GUI not opened (TCC) |
| B2 | Plan (declare) | `BarDeclarationFlowView` / cash cockpit · `POST /api/daemon/bar/declare` | Submit pre-trade declaration → arm | **BLOCKED** | Live declare **401 Authentication required**; cannot complete Plan→armed |
| B3 | Working (live / armed) | `BarPlanStateView` · `GET /api/daemon/bar/live-state` | Live plan paint after declare | **BLOCKED** | Live-state **401 Authentication required**; no pending_declaration observed |
| B4 | Debrief | `BarPostTradeView` · bar post-trade | Post-trade debrief after session | **BLOCKED** | Upstream bar auth required; declare never armed; POST debrief via wrong local shape → 405; flow not completable |
| B5 | Match (escrow) | `BarEscrowMatchView` · live-state `escrowMatchReport` | Escrow match report for declared trade | **BLOCKED** | Needs live-state (401); no report payload obtainable |
| B6 | Patterns | `BarNotchShell.swift` `BarPatternsChartView` ~L970 | Setup mix chart | **N/A** (honest empty shell) | UI is `BarPlaceholderCard` “Not enough tickets…” — not a trade-flow surface; no series in live day |
| B7 | Fidelity | `BarFidelityChartView` ~L979 | Plan fidelity series | **N/A** / **BLOCKED** | Placeholder + optional `fidelityPct` from escrow; escrow unreachable (401) |
| B8 | Triage | `BarTriageRouteView` ~L1001 | Open-position triage | **WORKING** (empty honest) | `GET /api/daemon/positions` **200** `positions:[]` → UI empty card path; no open positions to triage |
| B9 | Notch Settings (loss-limits) | `BarSettingsView` · `…/bar/profile/loss-limits` | Read/ack loss limits | **BLOCKED** | Live GET loss-limits **401 Authentication required** |

### C. Shell / hosting / kill

| # | Name | Where | Expected (1 line) | Verdict | Evidence |
|---|------|-------|-------------------|---------|----------|
| C1 | Collapsed pill | `CollapsedView` / `FloatingNotchHost` | Pill visible while Station healthy | **UNKNOWN** | Process running + hosted launcher code; GUI TCC — pill not screenshot-confirmed this run |
| C2 | ⌥Space toggle Notch | `HotkeyRegistrar` · `FloatingNotchHost.toggle` | Expand/collapse PLAN Notch | **UNKNOWN** | Carbon hotkey registered in code; keystroke TCC-blocked — not live-pressed |
| C3 | ⌥⇧Space open Station | `HotkeyRegistrar` openStation | Bring Station window forward | **UNKNOWN** | Same — not live-pressed |
| C4 | planSurfaceOnly | `NotchLauncher(isHostedByStation:)` · `ExpandedView` | Expand always `BarCircuitPanelView`; no PULSE/BRIEF/CAPTURE tabs | **WORKING** (code path live host) | Debug Station hosts Notch; `planSurfaceOnly` fail-closed in `ExpandedView.swift` L138–141; multi-tab path not Station shipping |
| C5 | Kill overlay / fire | `KillSwitchOverlay*` · `POST /api/daemon/kill-switch` | Escalate to DNS block | **UNKNOWN** | **Not fired** (would lock broker DNS). `killDnsActive=false`; audit empty |
| C6 | Kill dismiss | `POST /api/daemon/dismiss-kill-switch` | Clear non-armed / dismiss path | **WORKING** | Live **200** `{ok:true,dns_active:false,fog_active:false}` |
| C7 | Kill audit | `GET /api/daemon/kill-switch/audit` | Ed25519 audit tail | **WORKING** | Live **200** `{ok:true,entries:[]}` |
| C8 | Stop-me | `POST /api/daemon/bar/stop-me` | Trader stop gate on bar | **BLOCKED** | Live **401 Authentication required** |
| C9 | Status menu Toggle Notch | `StatusItemController` | Menu item toggles Notch | **N/A** | Menu = Open Station / Launch at Login / Quit only — Toggle Notch explicitly deferred in handoff |

### D. Agent bar routes + wire

| # | Name | Where | Expected (1 line) | Verdict | Evidence |
|---|------|-------|-------------------|---------|----------|
| D1 | Wire v1 HMAC | `agent/src/wire.rs` · StationWireClient | Auth machine integrity to agent | **WORKING** | Signed calls → 200 on health/today/positions/manifest/sync; unsigned → 412 |
| D2 | `GET …/bar/live-state` | `agent/src/api/bar.rs` → Console `/api/bar/v1/live-state` | PLAN brain live JSON | **BLOCKED** | Wire OK → agent → Console **401 Authentication required** |
| D3 | `POST …/bar/declare` | bar.rs → Console declarations | Accept declaration | **BLOCKED** | **401 Authentication required** |
| D4 | `GET …/bar/declarations` | bar.rs | List declarations | **BLOCKED** | **401** |
| D5 | `GET/POST …/bar/profile/loss-limits` | bar.rs | Loss limits gate | **BLOCKED** | **401** |
| D6 | `POST …/bar/protective` | bar.rs | Protective SL status/actions | **BLOCKED** | **401** (PLAN also cuts venue `place_sl` send — `BarProtectiveSlPlanChrome`) |
| D7 | `POST …/bar/live-interference` | bar.rs | Interference echo | **BLOCKED** | **401** |
| D8 | `POST …/bar/swing-check-in` | bar.rs | Swing check-in | **BLOCKED** | **401** |
| D9 | Bar proxy architecture | handoff / `bar.rs` L1 | Agent forwards to Console until Phase Z | **WORKING** (as designed) | Live 401 is Console identity, not missing route |

### E. Shipping books vs REFERENCE surfaces

| # | Name | Where | Expected (1 line) | Verdict | Evidence |
|---|------|-------|-------------------|---------|----------|
| E1 | Kotak Neo cash book | `kotak-nse-bse-cash` · sync + quote | Shipping Start book; cash quotes/fills | **WORKING** (sync) / **BROKEN** (quote freshness) | Sync caps fresh; quote `nse_cm\|2885` **200** with last=`1226` but status **`unknown`**, as_of **2026-09-26T19:15:23Z** (stale clock vs “fresh” positions) |
| E2 | Binance.com spot book | `binance-com-spot` | Shipping Start book; spot quotes/fills | **WORKING** (polled) / **BROKEN** (quote freshness) | sync-hint polled; quote `btcusdt` **200** last present but status **`stale`**, as_of **2026-09-26T18:31:16Z**; active slug currently Kotak |
| E3 | Cash declare cockpit | `BarCashCockpitView` | Plan UI for cash/spot shipping | **BLOCKED** | UI code present; declare API 401 — no live ticket submitted |
| E4 | Instrument search | `/instruments/search` | Search for declare symbol | **BROKEN** | Signed requests → **401 SIG_INVALID** (HMAC path/query mismatch on this hop) — search not usable via same signer recipe |
| E5 | Kotak NFO cockpit | `BarNfoOptionsCockpitView` · `kotak-nse-nfo` | NFO options declare | **N/A** | Not Live Station v1 shipping book (`shipping_book_id_for_slug("kotak-nse-nfo")` none) |
| E6 | Binance Options declare | `BarCryptoOptionsDeclareView` · `binance-com-options` | Public options chain/declare | **N/A** | REFERENCE desk book; Start shipping slug maps to **spot** only |
| E7 | Binance USDM / Coin-M | `BarUsdmTicketView` · usdm/coinm books | Futures tickets | **N/A** | Explicitly non-shipping (`shipping_book_id_for_slug` none for usdm) |
| E8 | Scalper / Swing forms | `BarScalperSessionView` / `BarSwingDeclarationView` | Alternate archetypes | **BLOCKED** | Same bar declare 401 gate |
| E9 | Multi-tab PULSE/BRIEF/CAPTURE | `ExpandedView` non-plan tabs | Full-tab Notch | **N/A** | Culled for Station-hosted Notch (`planSurfaceOnly`) |
| E10 | Web `/dashboard/bar` | Console | Web Bar desk | **N/A** | Culled — must not restore |
| E11 | Status-menu Toggle Notch | handoff §2.D | Optional later | **N/A** | Not implemented |

---

## Full trade-flow attempts (what failed)

### Flow 1 — Open → Plan → Working → Debrief (primary)

1. **Precondition auth:** `GET /api/daemon/auth/station/session` → **401** expired token.  
2. **Open:** No daemon morning-brief; Notch brief needs signed-in Console path → **stop**.  
3. **Plan:** `POST /api/daemon/bar/declare` (minimal BTCUSDT / Kotak-capable payload) → **401 Authentication required**.  
4. **Working:** `GET /api/daemon/bar/live-state` → **401**. No armed/pending state.  
5. **Debrief:** Not reachable without prior declare + bar auth.

**Result: BLOCKED at Station Caller / Console bar auth. Zero declarations accepted this run.**

### Flow 2 — Match / Patterns / Fidelity / Triage / Settings

- Match/Settings: blocked on live-state / loss-limits **401**.  
- Patterns/Fidelity: placeholder cards — not a completable trade flow.  
- Triage: positions empty **200** — honest empty WORKING for “nothing to triage”.

### Flow 3 — Broker + desk (shipping)

- Kotak Neo **connected** (wasm), Today **200**, sync-hint both books polled → **WORKING** as broker desk spine.  
- Quotes return last prices but mark **stale/unknown** with yesterday timestamps → **BROKEN freshness** for live tick paint.  
- Did **not** place venue entry orders (agent invariant: never places entry orders).

### Flow 4 — Kill / Stop

- Stop-me **401** (bar auth).  
- Kill **not fired** (safety). Dismiss + audit read **WORKING**.

---

## Top blockers / broken / unknown (priority)

1. **Station Caller JWT expired** — `auth/station/session` 401; `signed_in:false` (Keychain item exists, Console rejects).  
2. **All `/api/daemon/bar/*` PLAN routes 401** — declare / live-state / loss-limits / stop-me / protective / interference.  
3. **Full Open→Plan→Working→Debrief trade flow impossible** until re-login (WorkOS device login).  
4. **Binance spot quote status=stale** (last as_of 2026-09-26 18:31Z).  
5. **Kotak cash quote status=unknown** (last as_of 2026-09-26 19:15Z) while positions cap claims fresh.  
6. **`/instruments/search` SIG_INVALID** under wire signer used here — symbol search hop broken for this client recipe.  
7. **GUI Notch chrome UNKNOWN** — TCC blocks screencapture/keystroke (⌥Space / pill / declare form visuals).  
8. **Kill fire UNKNOWN** — deliberately not armed.  
9. **Market Data / AI Workflow / Station Settings screens UNKNOWN** — not live-clicked.  
10. **Active broker is Kotak; Binance spot secondary in hint** — dual-book paint/freshness not proven on a single declared trade.

---

## How to unblock the trade circuit (operator)

1. In Station: complete **device login** again (WorkOS AuthKit) until `GET /api/daemon/auth/station/session` returns `signed_in:true`.  
2. Re-hit `GET /api/daemon/bar/live-state` → expect **200** JSON (not 401).  
3. Then dogfood: Open → Plan declare on **Kotak cash** or **Binance spot** → Working armed → Debrief; only then re-score B1–B5/B9 as WORKING/BROKEN from live fills.

---

## Artifact paths

- This report: `/Users/bishnu/tradeautopsy-station/plans/NOTCH-FEATURE-AUDIT-2026-09-27.md`  
- Discovery dump: `/tmp/ta-e2e/discovery.json`  
- Copy: `/tmp/notch-feature-audit.md`

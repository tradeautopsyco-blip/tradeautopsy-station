# Architecture Deepening v1 — Issue Scale Draft

> Generated from: `.github/PRD-architecture-deepening-v1.md` + `.github/TRD-architecture-deepening-v1.md`  
> Cross-repo: `plans/architecture-deepening-cross-repo-handoff.md`  
> Use with `/to-issues` (or issue scale): each block is one tracer-bullet issue.  
> **Do not start Issue 12 until Issues 2–5 and 8–10 have real fills / seams.**  
> Every issue inherits PRD group **N** invariants (port 9137, wire-v1, Keychain, withdraw hard block, kill switch always on, no secrets in observables, reference-library discipline).

---

## Issue 1 — Cross-repo contract freeze + station-wire hop tables

**Phase:** 0  
**Repos:** `station-wire/`, `.github/`, brain ACK on `tradeautopsy`  
**PRD stories:** A1–A7  

**What to build:**  
Freeze Notch → `/api/daemon/bar/*` → brain `/api/bar/v1/*` hop table; document agent→brain auth (**Station Caller Bearer**, not loopback HMAC and not `x-daemon-secret` identity); freeze fill-ingress identity/completeness/redaction fields; freeze kill command_type/ack semantics; bump/prepare compatibility.phase notes; record Contract ACK in both repos.

**Acceptance criteria:**
- [x] `station-wire/v1.json` lists daemon bar paths and brain bar paths (dual hop tables)
- [x] Auth difference loopback vs upstream documented in wire file (wire HMAC vs Bearer)
- [x] ACK note exists in Station (`station-wire/CONTRACT-ACK.md`) linking Console A8 IV tip
- [x] Fill ingress required fields listed in wire file
- [x] Kill command_type list listed in wire file (`fog_of_war`, `clear_fog`)

**Blocked by:** None — can start immediately  

---

## Issue 2 — Live Binance.US spot client + BrokerAdapter

**Phase:** 1  
**Repos:** `agent/`  
**PRD stories:** B8–B20  

**What to build:**  
Binance.US spot client + `BrokerAdapter` mirroring Binance.com layering. `build_runtime_adapter("binance_us")` uses live adapter for real keys; CountingPoll only for `TA_TEST_*` / `TA_FAKE_*`. Cite `docs/reference/crypto/binance-us/spot/`. Poll fills, balances, open orders into existing broker_sync loop. No secrets in logs.

**Acceptance criteria:**
- [ ] Real-key path does not use CountingPoll
- [ ] Test-prefix path still uses CountingPoll
- [ ] Fixture/unit mapping myTrades → internal fills
- [ ] Integration or mock HTTP proves non-empty merge into recent trades
- [ ] Reference docs cited in module docs/comments per AGENTS.md

**Blocked by:** Issue 1 (contract ACK preferred; adapter impl may start in parallel if ACK in flight)  

---

## Issue 3 — exchangeInfo boot into Today / round-trip

**Phase:** 1  
**Repos:** `agent/`  
**PRD stories:** C21–C24  

**What to build:**  
Populate `ExchangeInfoSymbolCache` from Binance.US `GET /api/v3/exchangeInfo` at boot and/or first US sync. Heuristic fallback only when fetch fails. Test that runtime is not permanently `empty()`.

**Acceptance criteria:**
- [ ] Boot/sync path assigns non-empty cache from fixture or mock
- [ ] Fetch failure does not crash agent; heuristic remains
- [ ] Today/round-trip can use cache when present

**Blocked by:** Issue 2  

---

## Issue 4 — Validation gate on sync/start

**Phase:** 2  
**Repos:** `agent/` (+ Station error mapping if needed)  
**PRD stories:** D25–D30  

**What to build:**  
Invoke broker validation on `POST /api/daemon/broker/sync/start` before adapter build. `withdrawDetected` refuses start. Swift connect validation remains. Align posture taxonomy meaning across Swift/Rust.

**Acceptance criteria:**
- [ ] withdrawDetected never starts poll loop
- [ ] Clear failed posture returned to Station
- [ ] CI test covers withdrawDetected on sync/start
- [ ] Test/fake prefix skip policy documented if any

**Blocked by:** Issue 2  

---

## Issue 5 — Pause single source of truth

**Phase:** 2  
**Repos:** `agent/`, `station/`  
**PRD stories:** E31–E36  

**What to build:**  
Agent `user_paused` is authoritative. Station stop/start/status reconcile from agent; do not overlay Paused solely from UserDefaults when agent disagrees. Stop still does not restart agent.

**Acceptance criteria:**
- [ ] After Stop, relaunch UI + fetch status → Paused
- [ ] Start clears pause only after agent start OK
- [ ] Lifecycle tests updated
- [ ] Kill switch / SSE unaffected by Stop

**Blocked by:** Issue 4  

---

## Issue 6 — Swift LoopbackWireClient unification

**Phase:** 3a  
**Repos:** `station/`, `notch/`  
**PRD stories:** F37–F43  

**What to build:**  
One shared Swift loopback wire client. Migrate AgentSupervisor, broker runtime, Today client, Notch, Settings. Delete duplicate HMAC/canonical implementations. Live + test fake adapters.

**Acceptance criteria:**
- [ ] Single signer module; no third copy in Notch/Settings
- [ ] Tampered canonical string rejected by agent in test
- [ ] No secrets logged by client
- [ ] All former call sites use shared client

**Blocked by:** None — can start immediately (parallel with 2–5)  

---

## Issue 7 — Brain: US fill / bar / kill tolerance

**Phase:** 3b  
**Repos:** `FExEVIL/tradeautopsy` (brain)  
**PRD stories:** G44–G48  

**What to build:**  
Confirm bar v1 + fill ingest accept Station Binance.US payloads; daemon command/kill accept slug `binance_us`; fix crypto-hostile assumptions. Link ACK to Station Issue 1.

**Acceptance criteria:**
- [ ] Brain ACK comments on Station contract issue
- [ ] Fill ingest accepts sample US redacted payload
- [ ] Kill/command path accepts `binance_us`
- [ ] Wire revision bump only if fields change

**Blocked by:** Issue 1; validate against Issue 2 payloads  

---

## Issue 8 — BarSurfacePhase authority + shell bridge

**Phase:** 4  
**Repos:** `notch/`, `station/`  
**PRD stories:** H49–H54  

**What to build:**  
Single phase authority from live-state/session rules. Wire StationAppCoordinator / NavigationPolicy to it. Remove production stuck-`.declaration` provider behavior. Preserve NavigationPolicy stickiness. Notch and shell agree.

**Acceptance criteria:**
- [ ] Production launch does not hardcode eternal `.declaration`
- [ ] Coordinator tests driven by live phase changes
- [ ] Notch chrome and shell routes agree on phase
- [ ] NavigationPolicy semantics preserved

**Blocked by:** Issue 6 recommended (shared client); functionally after Notch live-state works  

---

## Issue 9 — Broker session module deepen

**Phase:** 5  
**Repos:** `station/`  
**PRD stories:** I55–I60  

**What to build:**  
One broker session module absorbing shallow pass-throughs. Keep Keychain + agent HTTP seams with fakes. Document single global sync slot vs connection identity. UI behavior unchanged except honesty.

**Acceptance criteria:**
- [ ] Pass-through sync wrappers absorbed/removed
- [ ] Lifecycle tests pass via session interface + fakes
- [ ] Keychain-only secrets unchanged
- [ ] Global sync slot documented on interface

**Blocked by:** Issues 5, 6  

---

## Issue 10 — KillSwitch module + crypto DNS + Today seam

**Phase:** 6  
**Repos:** `agent/`, `station/`, `notch/` (+ brain ACK)  
**PRD stories:** J61–J70  

**What to build:**  
KillSwitch module with fire/dismiss/ack local parity. `binance_us` DNS hosts (not Kotak). Resolve `fog_active`. Today circuit banner from agent/SSE, not Notch privates.

**Acceptance criteria:**
- [ ] Ack and dismiss both clear DNS in test env
- [ ] `binance_us` host map test passes
- [ ] fog_active removed or has a real reader + test
- [ ] Today banner independent of NotchViewModel fields
- [ ] Brain ack path still compatible

**Blocked by:** Issues 1, 7; Issue 2 for crypto slug realism  

---

## Issue 11 — BarProxy collapse + wire doc

**Phase:** 7  
**Repos:** `agent/`, `station-wire/`  
**PRD stories:** K71–K75  

**What to build:**  
One BarProxy module + path table; absorb near-copy handlers; preserve 429 retry; align station-wire hop tables with runtime.

**Acceptance criteria:**
- [ ] bar_forward tests pass through proxy module
- [ ] Duplicate handler bodies removed
- [ ] station-wire lists both hop layers
- [ ] 429 retry behavior preserved

**Blocked by:** Issue 1  

---

## Issue 12 — BarSessionRuntime deepen

**Phase:** 8  
**Repos:** `notch/`  
**PRD stories:** L76–L81  

**What to build:**  
Bar session runtime owning wire usage, SSE/FSM, live-state projection, phase, declare, kill overlay. Reducers internal. Tests at runtime interface. Today does not depend on Notch privates for kill/sync.

**Acceptance criteria:**
- [ ] Declare → phase transition tested at runtime interface
- [ ] No undecomposed public hub required for those flows
- [ ] Injectable HTTP adapter retained for declare tests
- [ ] Notch UX behavior preserved (no visual redesign requirement)

**Blocked by:** Issues 2, 5, 8, 10  

---

## Issue 13 — Joint smoke + audit doc update

**Phase:** 9  
**Repos:** both repos, `.github/`  
**PRD stories:** M82–M86  

**What to build:**  
Publish `.github/ARCHITECTURE_DEEPENING_SMOKE_CHECKLIST.md`; run founder Binance.US smoke; update or supersede `AUDIT_CURRENT_VS_PLANNED.md` CountingPoll claim; CI green without live secrets.

**Acceptance criteria:**
- [ ] Smoke checklist checked with evidence notes
- [ ] Audit doc no longer claims US production = CountingPoll only
- [ ] CI green
- [ ] Cross-repo ACK still valid for shipped wire revision

**Blocked by:** Issues 2–12  

---

## Quick reference — locked decisions

| Topic | Decision |
|-------|----------|
| Test broker | Binance.US live adapter; CountingPoll for TA_* only |
| Withdraw | Hard block on connect **and** sync/start (account-level until MANIFEST says otherwise) |
| Pause | Agent sole authority |
| Phase | One authority → Notch + shell |
| Wire | One Swift LoopbackWireClient |
| Kill switch | fire/dismiss/ack parity; US DNS hosts |
| Sequencing | Live fills before Notch deepen |
| Marketplace | Out of scope |
| UDS auth | Not in this program |

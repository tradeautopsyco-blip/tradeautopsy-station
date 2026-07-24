# Architecture Deepening — Cross-Repo Handoff Plan

**Date:** 2026-07-09  
**Station repo:** `FExEVIL/tradeautopsy-station`  
**Brain / Console repo:** `FExEVIL/tradeautopsy` (also referenced as tradeautopsy1 / Untitled/tradeautopsy)  
**Source:** `/improve-codebase-architecture` review (2026-07-09) + `AUDIT_CURRENT_VS_PLANNED.md`  
**Artifacts in Station:**
- PRD: `.github/PRD-architecture-deepening-v1.md`
- TRD: `.github/TRD-architecture-deepening-v1.md`
- This plan: `plans/architecture-deepening-cross-repo-handoff.md`

**How to use this note:** Paste into a session opened on **either** repo. Do not re-litigate the locked decisions below. Station owns local execution; brain owns hosted bar/daemon contracts. Both must land in the order below or the merge will be frictionful.

---

## The ONE goal (locked)

Close every architectural friction found in the Station architecture review so that **Binance.US live sync, wire auth, BarSurfacePhase, broker session pause, kill switch, and bar proxy** work as one coherent loop with the TradeAutopsy brain — no stubs, no split sources of truth, no silent empty polls.

## The ONE priority, right now

**Ship live Binance.US `BrokerAdapter` + wire validation on sync start** (Station agent), then align brain fill-ingress / bar contracts so Today, pulse, and Notch stop running on empty data. Everything else in this plan is sequenced after that north star works end-to-end.

---

## Why two repos must move together

| Concern | Station (`tradeautopsy-station`) | Brain (`tradeautopsy`) |
|---------|----------------------------------|-------------------------|
| Broker credentials | Keychain + agent sync | Never stores broker API secrets |
| Fill ingestion | Poll → `recent_trades` → optional bar fill ingress | Accepts redacted fills / bar ingest |
| Bar declare / live-state / stop-me | Notch → agent `/api/daemon/bar/*` → forward | Hosted `/api/bar/v1/*` |
| Kill switch fire / ack / command poll | Agent DNS + audit + SSE | Hosted command + ack + fog signals |
| Wire auth | Loopback wire-v1 HMAC (Station↔agent); `x-user-id` machine hint only | Agent→brain uses Station Caller **Bearer** (A8); never secret+user-id identity |
| Today / round-trips | Local engine in agent | Behavioral intelligence consumes uploads, not local Today |

**Invariant (both repos):** Web does not replace Backend Box for live broker setup. Station is self-contained at the broker layer. Brain never becomes the credential store.

---

## Locked product / architecture decisions

1. **Binance.US is the AGENTS.md test broker** — live spot adapter required; `CountingPollAdapter` remains only for `TA_TEST_*` / `TA_FAKE_*` keys.
2. **Withdraw permission = hard block** — enforced at connect (Swift) **and** at agent `sync/start` (Rust). Account-level `enable_withdrawals` / `canWithdraw` until MANIFEST Blocker 1 is resolved; do not invent key-level checks.
3. **Agent is sole pause authority** — `user_paused` in agent; Station must not invent Paused from UserDefaults alone after this work.
4. **One BarSurfacePhase authority** — live notch/session phase drives both Notch chrome and Station shell navigation.
5. **One Swift loopback wire client** — supervisor, broker runtime, Today, Notch, Settings share one signer matching `agent/src/wire.rs`.
6. **Kill switch always on** — Stop pauses broker sync only; fire/dismiss/ack must not diverge (SSE vs DNS vs audit).
7. **Reference-library discipline** — any new Binance.US financial/sync code cites `docs/reference/crypto/binance-us/spot/` before implementation; gaps stay `NOT SPECIFIED IN SOURCE`.
8. **No UDS / token-file auth in this program** — AGENTS.md Phase 0 model stands; do not implement UDS while deepening.
9. **Broker marketplace** (App-Store catalog) is **out of scope** for this program — see `plans/broker-marketplace-handoff.md`.
10. **station-wire/v1.json is the shared contract surface** — both repos update it when paths or auth semantics change; Station notch uses `/api/daemon/bar/*`, brain documents `/api/bar/v1/*`, wire file must list **both** hop tables.

---

## Dependency graph (merge order)

```
Phase 0  Cross-repo contract freeze (wire + bar path table + fill ingress schema)
    │
    ▼
Phase 1  Station: Live Binance.US adapter + exchangeInfo boot
    │
    ▼
Phase 2  Station: Validation on sync/start + pause single source of truth
    │
    ├──► Phase 3a  Station: Loopback wire client unification
    ├──► Phase 3b  Brain: Confirm bar/daemon routes + fill ingest tolerate live US fills
    │
    ▼
Phase 4  Station: BarSurfacePhase single authority + shell bridge
    │
    ▼
Phase 5  Station: Broker session module collapse (shallow chain)
    │
    ▼
Phase 6  Station: Kill-switch module + crypto DNS hosts + ack/dismiss parity
    │         Brain: command/ack payloads stay compatible
    │
    ▼
Phase 7  Station: Bar brain-proxy collapse + station-wire path table
    │
    ▼
Phase 8  Station: Bar session runtime deepen (NotchViewModel) + Today↔kill-switch seam
    │
    ▼
Phase 9  Joint smoke: Binance.US founder account across Station + brain
```

**Rule:** Do not start Phase 8 (Notch deepen) until Phases 1–2 produce real fills. UI deepening on empty polls is theater.

---

## Phase briefs (shareable)

### Phase 0 — Contract freeze (both repos)

**Owner:** Station leads; brain reviews and ACKs.

**What:**
- Freeze bar hop table: Notch → `/api/daemon/bar/{declare,live-state,stop-me,...}` → brain `/api/bar/v1/...`
- Freeze fill ingress schema fields brain already accepts (connection id, slug, asset class, environment, completeness flags, redaction rules)
- Freeze kill-switch command_type strings and ack semantics
- Update `station-wire/v1.json` compatibility.phase / station_version when this program ships
- Document agent→brain auth (secret header only; no loopback HMAC upstream)

**Brain must confirm:**
- Hosted bar routes remain stable or provide a versioned alias
- Fill ingest does not reject Binance.US symbols / fee assets that Station will send
- Kill-switch ack does not assume Indian-broker-only host maps

**Done when:** Both repos have a short “Architecture Deepening Contract ACK” note (PR comment or issue) pointing at the same wire revision.

---

### Phase 1 — Live Binance.US adapter (Station)

**Owner:** Station only (brain idle unless fill ingress breaks).

**What:**
- `BinanceUSSpotClient` + `BinanceUSSpotBrokerAdapter` mirroring `.com` layering
- `build_runtime_adapter("binance_us")` uses live adapter for real keys; CountingPoll for test prefixes
- Boot `ExchangeInfoSymbolCache` from Binance.US `GET /api/v3/exchangeInfo` (cite REST.md)
- Poll fills, balances, open orders into existing `broker_sync` loop
- 90-day backfill behavior becomes meaningful

**Brain impact:** Live fills may hit bar fill ingress / outbox — monitor for schema or rate issues; fix brain only if ingest rejects valid redacted payloads.

**Done when:** Manual smoke: connect Binance.US → Syncing with non-empty fills → Today hero non-null for a known round-trip.

---

### Phase 2 — Validation + pause truth (Station)

**Owner:** Station.

**What:**
- Call existing `broker_validation` on `sync/start`; refuse `withdrawDetected`
- Agent is sole `Paused` authority; Station metadata may cache display but must refresh from agent
- Stop/Start lifecycle tests prove no UserDefaults-only pause lie after restart

**Brain impact:** None.

---

### Phase 3a — Loopback wire client (Station)

**Owner:** Station.

**What:**
- One Swift `LoopbackWireClient` used by AgentSupervisor, broker runtime, Today client, Notch, Settings
- Header posture consistent (`x-daemon-source` / caller integrity where required)
- Negative tests: drifted canonical string fails against agent

**Brain impact:** None (loopback only).

---

### Phase 3b — Brain tolerance for live US traffic (Brain)

**Owner:** Brain.

**What:**
- Verify `/api/bar/v1/*` and fill ingest against Station Phase 1 payloads
- Confirm daemon command poll + kill-switch ack still work when broker slug is `binance_us`
- Fix any crypto-hostile assumptions (INR-only, Kotak/Zerodha-only DNS expectations on brain side if any)

**Station impact:** May need small payload field tweaks — coordinate via Phase 0 contract.

---

### Phase 4 — BarSurfacePhase authority (Station)

**Owner:** Station.

**What:**
- Single phase derivation from live-state / declaration / debrief
- Bridge into `StationAppCoordinator` / `NavigationPolicy` (delete dead always-`.declaration` provider behavior)
- Notch and shell agree on phase

**Brain impact:** None (phase is local UX). Live-state shape from brain must remain stable (Phase 0).

---

### Phase 5 — Broker session deepen (Station)

**Owner:** Station.

**What:**
- Collapse shallow pass-through chain into one broker session module
- Preserve Keychain seam and agent HTTP seam (two adapters: live + fake)
- Identity vs global sync slot: document and align (agent single active adapter today)

**Brain impact:** None.

---

### Phase 6 — Kill-switch deepen (Station + brain ACK)

**Owner:** Station implements; brain confirms command/ack contract.

**What:**
- One kill-switch module: fire / dismiss / ack with identical local side effects
- `hosts_for_broker("binance_us")` maps to Binance.US domains (not Kotak default)
- Remove or actually consume `fog_active`
- Today circuit banner reads agent/SSE kill-switch state, not NotchViewModel private fields

**Brain must:**
- Keep ack and `fog_of_war` / command_type strings stable
- Not assume dismiss-only cleanup on Station

---

### Phase 7 — Bar proxy + wire doc (Station)

**Owner:** Station; brain only if path aliases needed.

**What:**
- Collapse near-copy `/api/daemon/bar/*` handlers into one proxy module + path table
- Align `station-wire/v1.json` with daemon vs brain paths
- Preserve 429 retry behavior

---

### Phase 8 — Bar session runtime (Station)

**Owner:** Station.

**What:**
- Deepen NotchViewModel into Bar session runtime (wire, SSE, phase, declare, kill overlay)
- Keep pure reducers internal; tests cross the runtime interface
- Today no longer depends on notch internals for kill-switch

**Depends on:** Phases 1–4, 6 (real data + phase + kill-switch seams exist).

---

### Phase 9 — Joint smoke (both)

**Owner:** Founder / QA with both apps.

**Checklist (minimum):**
- [ ] Binance.US connect → withdraw block proven (fake + real posture)
- [ ] Live fills appear in agent recent trades and Today
- [ ] Pulse strip matches Today hero
- [ ] Notch declare → brain live-state → phase armed/livePlan
- [ ] Stop pauses sync; kill switch still fires; agent stays up
- [ ] Kill-switch L3 blocks Binance.US hosts (not Kotak)
- [ ] Ack and dismiss both clear local DNS + SSE consistently
- [ ] No secrets in logs/SSE/UI models
- [ ] Brain receives redacted behavioral/fill ingress without raw auth material

---

## Repo ownership matrix (for issue scaling)

| Workstream | Primary repo | Secondary | Issue label hint |
|------------|--------------|-----------|------------------|
| Binance.US adapter + exchangeInfo | Station | — | `station-agent` |
| Validation on sync/start | Station | — | `station-agent` |
| Pause single source of truth | Station | — | `station` + `station-agent` |
| Loopback wire client | Station | — | `station` + `notch` |
| BarSurfacePhase bridge | Station | — | `station` + `notch` |
| Broker session module | Station | — | `station` |
| Kill-switch module + DNS US | Station | Brain ACK | `station-agent` + `brain-daemon` |
| Bar proxy + station-wire | Station | Brain if alias | `station-agent` + `wire` |
| Bar session runtime | Station | — | `notch` |
| Fill ingest / bar route tolerance | Brain | Station | `brain-bar` |
| Contract ACK + smoke | Both | — | `cross-repo` |

---

## What brain should NOT do

- Do not build a parallel Binance.US credential UI in Console for this program.
- Do not store broker API keys in brain DB.
- Do not change loopback port away from 9137.
- Do not require UDS auth.
- Do not block Station Phase 1 on marketplace / multi-broker Console registry work.

---

## What Station should NOT do in this program

- Broker marketplace App-Store catalog (separate handoff).
- Desk route content (Pre-trade, Live, Journal placeholders).
- Stats screen (Today Slice 2).
- Implementing key-level withdraw introspection beyond documented account-level signals.
- Replacing wire-v1 with a new auth scheme.

---

## Cascade for agents

```
This handoff
  → Station: PRD-architecture-deepening-v1.md + TRD-architecture-deepening-v1.md
  → /to-issues (or /issue scale) from the PRD — one vertical slice per phase/workstream
  → Brain: mirror Phase 0 + 3b + 6 ACK issues against tradeautopsy tracker
  → Implement Station issues in dependency order
  → Phase 9 joint smoke before calling the program done
```

**Issue scaling note:** The Station PRD user stories are numbered and grouped by workstream so `/to-issues` can map **one issue ≈ one tracer-bullet phase** without inventing scope. Prefer many thin vertical slices over horizontal “rewrite all of notch” tickets.

---

## Success definition (both repos)

> A trader connects Binance.US on Station, sees real fills and Today P&L, declares in the Notch with shell phase following, can pause sync without killing safety systems, and kill switch correctly targets Binance.US — while brain accepts bar and fill traffic without schema or auth friction.

Anything less is not “merged and frictionless.”

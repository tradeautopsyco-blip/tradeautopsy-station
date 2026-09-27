# Strong testing system — Station + Notch (TradeAutopsy)

**Date:** 2026-09-27 Asia/Calcutta  
**Intent:** Prove every Notch/Station surface with **live behavior** and **journal sinks**, not only unit tests.  
**User preference:** full trade-flow / end-to-end over per-feature unit noise.

---

## 1. Problem today

| Layer | What exists | Gap |
|---|---|---|
| Unit (Swift/Rust) | ~512 Notch tests; StationTests; agent `cargo test`; `scripts/dogfood-suite.sh` (public seams) | Does **not** prove Plan→journal, option chain paint, Settings sync, or Debrief |
| Live wire probes | Ad-hoc HMAC scripts (`plans/NOTCH-LIVE-*.md`) | No durable suite; blocked by Console JWT (~15 min) + broker TOTP |
| GUI E2E | Almost none | macOS TCC blocks screencapture/keystrokes from automation |
| Journal truth | Capture outbox status only | No assert that declare moods / debrief / captures land in Journal |

Screenshot reality (Plan cockpit): market tiles can show **unavailable / 3 holes / no licensed series** while the declare form is still the journal path. Those must be scored as **separate lanes**.

---

## 2. Design principles

1. **Journal is the source of truth for Plan / Working / Debrief.** A feature “works” only if the sink accepts the payload (declare id, mood snapshot, capture acked, debrief record) — not if a button looks clickable.
2. **Tools are provenance-honest.** Quote / history / depth / option_chain / greeks / OI must assert *envelope status* (fresh / stale / unavailable / unlicensed) — never invent numbers.
3. **DualNoBlend stays sacred.** Spot vs Equity vs Options vs USDM vs Coin-M never blend books; tests fail on cross-book paint.
4. **Three gates before live circuit:** (A) agent wire healthy, (B) Console Station session signed in, (C) broker sync fresh for the book under test.
5. **Pyramid stays thin at the top:** many fast unit/contract tests; fewer live journal runs; rare full human dogfood.

```
                    ┌─────────────────────────┐
                    │  L4 Human dogfood matrix │  rare
                    └───────────┬─────────────┘
                    ┌───────────▼─────────────┐
                    │  L3 Live journal + tools │  nightly / on-demand
                    └───────────┬─────────────┘
                    ┌───────────▼─────────────┐
                    │  L2 Wire contract / stub │  CI
                    └───────────┬─────────────┘
                    ┌───────────▼─────────────┐
                    │  L1 Unit (Swift/Rust)    │  every PR
                    └─────────────────────────┘
```

---

## 3. Surface inventory (what to cover)

### 3.1 Notch session rail (live trade flow)

| Screen | Elements to cover | Sink / assert |
|---|---|---|
| **Open** | Morning brief, start gate (calm/confidence/rule) | Local unlock rules; optional brief API |
| **Plan** | Asset chips (Spot/Equity/Options/USDM/Coin-M), style (Intraday/Scalper/Swing), view (Cockpit/Hero/Focus), state check (4 scales), Planned/Reactive, symbol/qty/SL/target, Confirm declare | `POST …/bar/declare` → Console declarations; `plan_snapshot` moods; live-state `pending_declaration` |
| **Working** | Armed plan, escrow match rows, protective, stop-me, live interference, swing check-in | live-state + bar routes; fills from broker sync |
| **Debrief** | Adherence (stop/size as declared), notes, post-trade submit | `post-trade-debrief` (correct method/shape) → journal trip |

### 3.2 Notch analysis rail

| Screen | Assert |
|---|---|
| Match (escrow) | Rows only from live-state / fills — no invented actuals |
| Patterns | Chart binds or honest empty |
| Fidelity score | Uses escrow fidelityPct when present |
| Triage | Routes without crashing; auth errors surfaced |

### 3.3 Notch Settings

Tabs: **broker · risk · behavior · desk · notifications**

| Tab | Live checks |
|---|---|
| Broker | Mirrors sync-state (connected/connecting/degraded/offline); capability badges (fills/funds/holdings/instruments/orders/positions/quote) |
| Risk | Loss-limits GET/PUT via bar profile (needs Console) |
| Behavior | Intraday/Scalper/Swing archetype persistence |
| Desk | Desk rules readout display-only (does not fire Kill) |
| Notifications | Toggle persistence (local) |

### 3.4 Plan cockpit **tools** (market tiles)

Each tool = one `/api/station/…` family. Score **status**, not pretty UI.

| Tool | Endpoint family | Pass criteria |
|---|---|---|
| Quote / LAST | `/api/station/quote` | Lit for active book **or** honest `unavailable` + provenance |
| History | `/api/station/history` + obtain | Licensed series **or** `no licensed series` (never Yahoo invent) |
| Depth | `/api/station/depth` | Ladder **or** Unusable on sequence gap |
| Option chain | `/api/station/option_chain` | Envelope apply; Options book only |
| Open interest | `/api/station/open_interest` | Same honesty |
| Greeks | `/api/station/greeks` | Venue mark table only; empty if model unspecified |
| Index | `/api/station/index` | Glance or unavailable |

Hole counter (“3 holes”) must equal count of dark named inputs — regression-test the counter logic in unit; live-test against real envelopes.

### 3.5 Station desk (inside TradeAutopsy app)

| Area | Live checks |
|---|---|
| Today | INR/USD profile; DualNoBlend; learningBaseline when empty |
| Journal | List + toolbar-capture outbox acked; linked trade id |
| Brokers | Connect / Start / TOTP remint / sync-state |
| Health | Agent :9137; Kill audit |
| Market Data / AI Workflow | Key presence; no secret echo in health JSON |
| Settings | Device login (Station Caller JWT) |

### 3.6 Journal capture (Notch → Journal)

| Step | Assert |
|---|---|
| Draft save | Local persistence keys |
| Accept | `POST …/journal/toolbar-capture/accept` |
| Outbox | `…/outbox/status` pending→acked |
| Screenshot attach | Pending patch; optional trade link |

---

## 4. Proposed durable artifacts (build order)

### Phase A — Spec + scoreboard (this doc) ✅
Lock lanes, gates, pass/fail language.

### Phase B — `scripts/notch-live-journal.sh` (L3 harness)
Wire-v1 HMAC runner against Debug agent:

1. Gate A/B/C (health, session, sync-state) — **fail fast with named gate**.
2. **Plan declare probe** with full S1 moods + stance + symbol/qty/SL → assert declaration id + live-state pending.
3. **Cancel / stop-me** cleanup (no orphan plans).
4. **Journal capture** accept → outbox status delta.
5. **Tools matrix** for active book: quote/history/depth/option_chain/greeks/OI/index → CSV/Markdown scoreboard (`fresh|stale|unavailable|unlicensed|auth|error`).
6. **Settings mirrors**: sync badges match sync-state JSON.
7. Write `plans/NOTCH-LIVE-SCOREBOARD-<date>.md` automatically.

### Phase C — Expand `dogfood-suite.sh` (L1/L2 CI)
Keep unit public seams; add **contract** tests only:

- `declare_from_body` mood → plan_snapshot (already partial).
- Option-chain / depth / greeks **envelope parsers** (pure apply, no HTTP).
- Hole-counter unit.
- Settings sync posture mapping.

Do **not** put live JWT tests in PR CI.

### Phase D — Nightly live job (optional)
Machine with Station Debug + Keychain ACL Always Allow + fresh Console session bootstrap (or operator-triggered). Artifact: scoreboard MD + JSON under `/tmp/ta-live/`.

### Phase E — Human matrix (L4)
One checklist per shipping book (Kotak cash, Binance spot): Open→Plan→fill→Working→Debrief with screenshots into Journal. Run when shipping.

---

## 5. Pass / fail language (strong rules)

| Result | Meaning |
|---|---|
| **WORKING** | Sink or tool returned success with expected shape |
| **HONEST-DARK** | Unavailable / unlicensed / NSE closed — UI matches envelope; **not a fail** |
| **AUTH** | Console/session 401 — gate B |
| **BROKER** | Sync stale/unavailable for required class — gate C |
| **LIE** | Number painted without licensed/fresh provenance — **hard fail** |
| **BLOCKED** | Upstream dependency missing (no declare id → no debrief) |

Market closed (e.g. NSE closed) ⇒ tools may be HONEST-DARK; Plan declare can still WORKING if Console auth is up.

---

## 6. Element checklist — Plan screen (from current UI)

Use as the first concrete matrix row set:

**Nav:** Open · Plan · Working · Debrief · Match · Patterns · Fidelity · Triage · Settings  

**Plan chips:** Spot/Equity/Options/USDM/Coin-M × Intraday/Scalper/Swing × Cockpit/Hero/Focus  

**State check:** Calm 1–5 · Confidence 1–5 · Frustration 1–5 · Excitement 1–5 · Planned|Reactive  

**Numbers:** Symbol · Quantity · (SL/target/invalidation as shown per desk) · Confirm  

**Tiles:** LAST · History · Depth · Session · (option chain when Options)  

**Chrome:** NSE/market strip · holes badge · Hide notch  

Each cell → L1 unit where pure · L3 live where I/O.

---

## 7. Prerequisites to run L3

1. Station Debug running; agent on `127.0.0.1:9137`.
2. Settings → Station sign-in (WorkOS) — token ~15 min; re-auth or automate refresh.
3. Brokers → Start book under test (Kotak TOTP / Binance keys) until sync capabilities fresh.
4. For Options tools: F&O master present; licensed option_chain.
5. Operator accepts: live declare creates a real Console declaration (cancel after).

---

## 8. What we will not do

- Treat `swift test` green as “Notch features work.”
- Automate full GUI clicks while TCC blocks accessibility (unless user grants).
- Invent LTP/history/greeks in tests or product.
- Blend COM USD and Kotak INR in Today/Plan asserts.

---

## 9. Immediate next steps

1. Implement Phase B harness (`scripts/notch-live-journal.sh`) + auto scoreboard.
2. First run focus: **Plan declare journal sink** + **tools honesty matrix** on Spot/Intraday/Cockpit (matches current screenshot).
3. Second run: Options chip + option_chain/greeks/OI when master lit.
4. Wire Settings broker tab badges to sync-state asserts.
5. Only then expand Debrief after a real (or paper) fill path.

---

## 10. Related artifacts

- `plans/NOTCH-FEATURE-AUDIT-2026-09-27.md`
- `plans/NOTCH-LIVE-TRADEFLOW-2026-09-27.md`
- `plans/NOTCH-LIVE-RECHECK-2026-09-27.md`
- `plans/NOTCH-LIVE-RETEST-NOW-2026-09-27.md`
- `scripts/dogfood-suite.sh`

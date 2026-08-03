# TradeAutopsy — context for a reviewer

**Date:** 2026-07-28 · **Owner:** Bishnu (solo founder) · **Audience:** a friend who wants to understand the product in depth

This describes TradeAutopsy **as currently planned** — what exists today, and exactly what it looks like once the planned Console and Station work is finished. It is a reading document; the clickable twin is [`station/prototypes/CONSOLE-STATION-SHARE.html`](./station/prototypes/CONSOLE-STATION-SHARE.html).

Authoritative sources this summarises (do not treat this file as the decision record):

| Source | Role |
|---|---|
| `~/.scratch/tradeautopsy/founder-clarity/CLARITY.md` | The spine. "If it's not here, it's not a decision." |
| `Tradeautopsy1/Untitled/tradeautopsy/founder-issues/CONSOLE-STATION-EXECUTION-PLAN.md` | Sequencing authority — Phases 0→5 |
| `.../founder-issues/ELEMENTS.md` | The 22-element cut — every capability has exactly one owner |
| `.../founder-issues/SYSTEMS.md` | Cross-cutting laws (S1 events, S2 metrics ownership, S3 feedback loop) |
| `STATION_NOTCH_SPACE_HANDOFF.md` | Current Station + Notch code status |
| `CONTEXT.md` + `docs/adr/` | Station vocabulary and architecture decisions |

---

## 1. What the product is

> TradeAutopsy helps a trader **declare before the trade, stay inside limits, and journal what happened** — personal-first now, multi-user later.

That sentence is doing real work. It is not a charting tool, not a broker, and not an analytics dashboard. It sits **between the trader's psychology and their broker execution** and behaves like a circuit breaker: you state your intent before you act, the system holds you to the limits you set while you're inside the trade, and afterwards you get an honest record of what actually happened.

The three moments, in order:

1. **Before** — *declare*. You say what you're about to do and under what limits. This is the gate.
2. **During** — *live state*. Fills stream in, day P&L and risk are computed continuously, and if you cross a limit the system intervenes. Escalation ends in real teeth: a DNS-level block of broker hosts so you physically cannot reach the exchange. That's the **Kill Switch**.
3. **After** — *journal*. What you captured in the moment, plus imported trade history, matched and reconciled into something you can review.

"Autopsy" is the point: the product assumes you will break your own rules, and optimises for making that visible and expensive rather than pretending discipline is a personality trait.

---

## 2. The mental model: four systems, never five

Every capability sits under exactly one system. Surfaces are not systems — this distinction is enforced in the docs because blurring it is how the codebase previously grew fog.

| System | Owns | Never owns |
|---|---|---|
| **Console** (web, hosted) | Hosted auth, Neon Postgres, journal **store**, trade history, deep analytics | Broker API secrets · kill teeth · the trader's day-to-day desk |
| **Station** (native macOS app) | The Mac shell, the Notch surface, Backend Box, supervises the Enforcer — **the day-to-day desk** | LLM inference |
| **Enforcer** (Rust daemon, loopback `127.0.0.1:9137`) | Broker polling, Keychain access, all broker HTTP, DNS kill enforcement, Ed25519 audit, Today math | Hosted auth · placing entry orders |
| **Behavioral Engine** (offline) | Bias and detector math — the *truth* layer for behavioural patterns | Live enforcement · secrets |

### Surfaces

- **Notch** — the floating always-available surface on the Mac (the pill near the notch). This is the trader-facing desk during a session: declare, armed state, live-state paint, capture to journal.
- **Station desk window** — a normal macOS window with Today, Brokers, and Settings.
- **Console web** — a thin personal brain in the browser: journal, trades, workspace, insights. Deliberately *not* a second desk.
- **Web "Bar" UI** — **removed**. It duplicated the Notch. The route `/dashboard/bar` is culled and must not be restored.

### Why native, not just web

Three constraints force a Mac app:

1. **Latency is a product bug.** The trader must feel the consequence of a fill or a limit trip *immediately* — local compute, paint, then share upward. A round-trip to a hosted analytics path is too slow to be honest (`SYSTEMS.md` S3).
2. **Secrets stay on device.** Broker credentials live in the macOS Keychain and are never handed to adapter code.
3. **Teeth need the OS.** A DNS-level block of broker hosts is not something a web app can do.

---

## 3. Architecture as it stands today (2026-07-28)

```
Station desk window   → SessionModel / StationDS / SessionPollingHost
Floating Notch pill   → NotchViewModel / BarNotchShell   (PLAN surface only)
Enforcer :9137        → broker poll · Keychain · DNS kill · /api/daemon/bar/* proxy
Console /api/bar/v1   → hosted PLAN brain (declare, live-state, loss-limits)
```

Locked invariants (from `README.md`, never violated):

- The Notch never calls `tradeautopsy.in` directly — all egress goes through the Enforcer on 9137.
- The Enforcer never renders UI.
- The Enforcer never places entry orders.
- All behavioural signals reach the brain through `ingestSignal()` only.
- No LLM inference inside Station.

**Where the code actually is:** the Notch was severed from the Station desk, then re-hosted as a *PLAN-only* floating panel — expanding it always lands on the circuit-breaker panel, with no PULSE/BRIEF/CAPTURE tab chrome. That work is code-complete and manually QA'd (2026-07-28). Hotkeys: `⌥Space` toggles the Notch, `⌥⇧Space` opens Station.

**The honest gap:** live declare/live-state data still flows Notch → Enforcer → **Console** `/api/bar/v1`. Station is not yet local truth. Closing that gap is Phase Z, below.

**Broker reality:** the Binance.com spot adapter is live; Binance.US fill sync is still a stub. Binance is explicitly *not* a long-term product commitment — the plan requires a named next broker before the final phase.

---

## 4. The plan: Phases 0 → 5

Phases 0–3 are shared and sequential. Phase 4 splits into two tracks that run in parallel. Phase 5 is the exit proof, and it comes **last** — it is not an early feature freeze.

| Phase | What it does | Status per the plan file |
|---|---|---|
| **0 · Fence live CRITICAL** | Make three live security holes unreachable: admin AI-analytics routes, org self-mint (`POST /api/org` → self-`fund_admin`), org invite-accept + risk override. Disable, don't redesign. | Exit boxes **not yet ticked** |
| **1 · Auth unlock (A8)** | Research → founder-locked memo: *one* safe auth standard across Console, Station, Enforcer. Nothing touching identity proceeds until this is `LOCKED`. | ✅ memo + `LOCKED` |
| **2 · Cull fog** | Shrink the surface: ~54 fog `/dashboard/*` routes, Org as a product, parked Algo Labs, Settings orphans, dead engine weight, and the web Bar UI. APIs fenced where their UI is gone. | ✅ complete |
| **3 · Desk-truth research gates** | Lock ownership memos before building: who computes Risk & Capital (R4.1), the Signals catalog (G3.1), which trading rules Station needs (I2.1/I4.1), Engine landing order (E5.1/E12.1). Tax necessity (X4.1) → *no Tax pressure, deferred*. | ✅ locked / explicitly deferred |
| **4 · Parallel completion** | Console track ∥ Station track. This is where the product is actually built. | **In flight** |
| **5 · One dogfood day-1** | Prove the whole spine on one real path, against local-truth Station + thin Console. | Not started |

Three hard gates deserve naming, because they're the discipline of the whole plan:

- **No cull before A8 is `LOCKED`.**
- **Fence ≠ invent product.** Closing a hole does not license rebuilding the feature.
- **S1 alone never authorizes cutting `/api/bar/v1`.** Only the Phase Z questions plus a completed migration do.

---

## 5. What it looks like when the plan is done

This is the part your friend probably cares about most: the end state, exactly as planned.

### 5.1 Station = the product

Station stops being a companion and becomes where the trader lives day to day.

- **Desk spine.** Today (day P&L, pulse, WAC, local SQLite), Brokers, Health, Market Data, thin Settings, and the Station ↔ Enforcer bridge.
- **Brokers as a *system*, not an integration.** Every broker is a sandboxed **Wasm component** behind one WIT contract — the Universal Broker Interface. An adapter exports `fetch_fills(cursor) -> FillEvent[]` and imports `broker_http_call`. It **never holds a raw credential**: the Enforcer looks up the Keychain entry, attaches auth, makes the real HTTP call, and hands back the body. Direct HTTP from adapter code is structurally impossible, not merely discouraged.
- **A human gate on top of the machine gate.** Passing the sandbox and the FillEvent contract tests is necessary but never sufficient. Every adapter also needs a signed **B6 capability sheet** — a researched document of rate limits, history depth, refuse lists, real broker-API facts — before it can go live.
- **Kill Switch with teeth.** Escalating protections ending in an always-on DNS-level block, applied uniformly regardless of which adapter is connected.
- **Phase Z: local truth.** This is the destination. Declarations, live state, and journal capture persist **on the Mac**. The Notch operates with **no live Console dependency**, and only then is the hosted `/api/bar/v1` dependence cut.

Phase Z has seven questions that must be answered *before* that cut, and they are honestly still open:

1. Where do declarations and live state persist on the Mac?
2. What replaces WorkOS / Neon Auth for "who is the trader"?
3. Where does the journal live — same store as declare? Does it sync?
4. Multi-Mac and reinstall: backup and restore?
5. Does Console remain for billing / multi-user later, or is it truly gone?
6. Rename path: keep `/api/bar/v1` as a local loopback, or new names?
7. What is the dogfood proof that Phase Z shipped?

### 5.2 Console = a thin personal brain

Console keeps what a browser is genuinely better at, and gives up everything that duplicates the desk.

**Keeps:** one safe sign-in path (the implemented A8 standard) · **Journal** — the store, plus a collapse of roughly eighteen tangled write paths down to one capture→accept flow · **Trades** — import, list, and a calendar that shares its flow with the dashboard heatmaps, with one unified matcher/calculator shared to both Console and Station · **Workspace** — canvas, v1 widgets, a Station→web cell, with embed trust established before any embeds (and renamed, last) · **Insights**, **Pattern Registry**, **TAI** (the coach — never touching a live broker) · a securely rebuilt **Admin** for platform admins only · **Workflow Labs**.

**Gives up:** the web Bar UI · Org / multi-tenant as a product · fat Console settings · parked Algo Labs · any second trader desk on the web. Notably, **Tax is not required** for Console to be considered complete — a Tax Center is only built after Phase 5, and only if that necessity research says it's needed.

### 5.3 The laws that keep it coherent

Three cross-cutting rules apply to both tracks, and they're the most interesting engineering content in the plan:

- **S2 — calculate once, share.** Every metric has exactly one owner. If Station computes loss-chase or day P&L, Console **uses** that number; it does not recompute it "to verify." Two owners of the same number is banned outright.
- **S1 — event-driven.** Once Station owns live session, Console should *react* to events (fill landed, declare accepted, day closed) rather than the Notch polling Console forever. The bus technology choice is deliberately deferred until ownership actually works.
- **S3 — fastest feedback loop.** Local compute → paint → then share upward. Deep analytics must never block the desk loop. Measure the latency, then add technology only against a measured wall.

### 5.4 How we'll know it's real: day-1 dogfood

Phase 5 signs eight capabilities against local-truth Station plus thin Console:

| # | Capability | Signed when |
|---|---|---|
| 1 | Login / auth | One sign-in path; no multi-profile sprawl |
| 2 | Health | Reflects the actually-working system |
| 3 | Trade history import + list | Usable, on shared calculation |
| 4 | Journal read/write + one capture→accept | One path works end to end |
| 5 | Circuit breaker / declare + live state **on the Notch** | Notch not dark; on-device, not Console-dependent |
| 6 | Session brief/debrief (thin) | Kept only if actually used in dogfood |
| 7 | Settings for that path | Thin; Station Backend Box primary |
| 8 | Station ↔ Enforcer bridge | Bridge works |

**The path, in one line:** log in → see trades and journal → declare and hold live state on the Notch → the Notch is not dark.

---

## 6. Deliberately not building this

A reviewer's instinct is to suggest features, so here's the standing no-go list. Each of these is a decision, not an oversight:

| Not doing | Why |
|---|---|
| Org / multi-tenant / seats | Removed as a product. Personal-first is a locked law. |
| Billing, Psychology packs | On HOLD until after this plan, as separate efforts |
| Tax Center / fat tax UI | Only after Phase 5, and only if the necessity research says yes. Currently: no pressure, deferred. |
| Algo Labs fat, SDK/MCP, marketplace | Parked. Workflow Labs is the single exception. |
| A second trader desk on the web | The Notch is the desk. This is the mistake the web Bar made. |
| LLM inference inside Station | Architecture invariant |
| The Enforcer placing entry orders | Architecture invariant — it can stop you, never trade for you |
| Deleting `/api/bar/v1` early | Hard gate: only Phase Z answers plus completed migration authorize it |

---

## 7. Vocabulary (so the repo reads cleanly)

| Term | Means |
|---|---|
| **Enforcer** / agent | The Rust daemon on `127.0.0.1:9137`. Sole execution authority for adapters; mediates Keychain and all broker HTTP. |
| **Notch** | The floating macOS surface — trader-facing desk. PLAN surface only when hosted by Station. |
| **Backend Box** | Station-side settings/infrastructure surface (market-data keys, etc.). The *broker* portion was superseded by the Universal Broker Interface. |
| **UBI** — Universal Broker Interface | The system letting anyone (including AI authors) supply a broker adapter as a sandboxed Wasm component behind one WIT contract. |
| **FillEvent** | The single normalised output shape every adapter produces. A WIT export type, not just a Rust struct. |
| **B6 capability sheet** | The mandatory human-authored, signed research doc required before any adapter goes live. |
| **Phase Z** | Station becomes local truth; the Notch stops depending on Console for live operation. |
| **Declare** | Stating intent and limits *before* the trade. The gate. |
| **Kill Switch** | The teeth — escalating protections ending in DNS-level broker blocking. |
| **Fog** | Routes and pages that exist but serve no trader path. The thing Phase 2 deleted. |
| **Fence** | Disable/delete a live blast radius without redesigning the feature. |
| **RESEARCH → FINALIZE / LOCKED** | A question that must become a founder-locked memo before anyone builds on it. |

Two aliases to be careful with: **"Brain"** means Console, and only in v1. **"Circuit breaker"** is ambiguous — it's the Live Session *gate* plus the Kill Switch *teeth*; name which one you mean.

---

## 8. What to click, and what to poke at

**To see it:** open [`station/prototypes/CONSOLE-STATION-SHARE.html`](./station/prototypes/CONSOLE-STATION-SHARE.html) — a self-contained offline twin of Station, Console, and the Notch, with sample data. Works on a phone. `NOTCH-ui.html` is the approved visual identity for the expanded Notch. Ignore `PROTOTYPE-station.html`; it's an older speculative mock.

**Honest open questions**, if you want to be useful rather than polite:

- The seven Phase Z questions above — especially #2 (what replaces hosted auth for "who is the trader" when Station is local truth) and #4 (multi-Mac backup/restore).
- Whether Console genuinely survives Phase Z as thin sync and billing, or should go away entirely.
- The S1 bus technology, deliberately unchosen until L1 ownership works.
- Whether a solo founder can finish a Phase 4 this wide before the dogfood proof, or whether the tracks need narrowing further.

# ADR 0020: Interactive Brokers transport (P8 W5.1)

**Status:** DRAFT (founder accepts later — stays DRAFT until B6 row 22 hosts,
Flex/Portal wire facts, and threat review close; do not flip **Enabled** on
`interactive_brokers` until ACCEPTED)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned **0020** at P9 kickoff (`plans/multi-asset-p9-territories-2026-09-24.md`,
`issues/brokers/P9-KICKOFF.md` when filed). **Not ADR 0015** — that number is
**Bybit** header-HMAC (`docs/adr/0015-bybit-header-hmac-session.md`). This ADR
owns IB W5.1 transport only.

**Program:** `issues/brokers/FULL-COVERAGE-PROGRAM.md` Phase 8 (W5.1) ·
`issues/brokers/ALL-BROKERS-PROGRAM.md` §4.4 ·
`issues/brokers/P5-P9-LAUNCH-EXECUTION-PLAN.md` §5 (IB ADR before parallel slug
work). Blocks P9-A spot FX, P9-D IB CFDs, and first US equity books on slug
`interactive_brokers` — **one transport for all IB books** (plan friction killer
“IB once”).

B6 sheet (`issues/brokers/sheets/interactive_brokers.md`) is owned by the
parallel B6 lane; URLs, report types, and Portal session shape below are
provisional oracle + program leads until that lane cites IB official docs (URL
+ fetch date per row).

## Context

1. **Station-first architecture.** TradeAutopsy Station is a loopback agent
   (Axum, `127.0.0.1:9137`, `AGENT_PORT` fixed per `AGENTS.md`) + Swift shell.
   All broker egress, Keychain credentials, and auth attachment live in the agent;
   Swift holds Connect UX and status, never secrets (ADR 0005–0009 pattern).
2. **ADR 0001 — uniform Wasm adapters.** Every slug adapter is a Wasm component
   with zero ambient authority. Outbound broker I/O is exclusively through the
   host import `broker-http-call` (`docs/contracts/ubi-data.wit`: request/response
   HTTP shape). The component never opens sockets, never holds IB credentials,
   and never talks to TWS/Gateway directly.
3. **Slug disposition today.** `interactive_brokers` is **Planned** in catalog
   (`descriptor_for_slug("interactive_brokers").is_none()` in integrator tests).
   Oracle pin (`docs/research/nautilus-trader-citation.md`) implements **only**
   path (c): `ibapi` socket to TWS/IB Gateway on `127.0.0.1` (ports 4002/7497
   class) — **zero** Client Portal or Flex mentions in that tree. Station must
   not copy Nautilus transport; this ADR chooses clean-room.
4. **W5.1 gate.** FULL-COVERAGE Phase 8: no IB B6 → first book until transport
   is **ACCEPTED** with refused options recorded. Non-Flex paths require an
   explicit **host-side design PR** outside unchanged `broker-http-call` before
   adapter step 4 (`P5-P9-LAUNCH-EXECUTION-PLAN.md` §5).
5. **First-book scope (transport ADR, not asset sprawl).** This ADR picks **how**
   fills and supporting reads reach the host for the **first** IB tracer book
   (default program lead: US equity cash; same transport later binds spot FX and
   CFD books per P9). SEC §31 + FINRA TAF and per-book calc profiles are **locks
   downstream**, not decided here.
6. **Read-only posture.** Station v1 on any IB path is import/sync/classify only
   — no order placement, no fund transfer, no “wrap” trading APIs. Any transport
   that exposes write surfaces on the same session must stay unreachable from Wasm
   and unimplemented in host v1.

## Decision

**Recommendation (DRAFT — founder ACCEPTED later): Option A — Flex Web Service
(HTTP, read-only Flex reports).**

Rationale in one line: Flex is the only option in the matrix below that keeps the
existing ADR 0001 contract (**HTTP via `broker-http-call` unchanged**) while
staying explicitly read-only at the API boundary; socket and Portal both force a
new host transport and a formal threat review before any Wasm ships.

**If founder rejects Flex at accept time:** reopen this ADR with a recorded
choice of Option B or C plus the mandatory follow-ups in §Consequences — do not
start a second IB transport for a sibling book.

### AuthScheme (provisional name)

**`IbFlexQueryToken`** — Flex token + query id (and related Flex Web Service
credentials) vault-only in Keychain; attached on host HTTPS to IB Flex endpoints
only. Not OAuth loopback unless B6 proves Flex shares a Portal login (unlikely).

### Credential blob (provisional)

Keychain-tagged blob, e.g. `ib_flex_query`: fields TBD by B6 — at minimum
`flexToken`, `queryId`, optional `tokenExpiry` if IB exposes TTL; never TWS
username/password in v1 if Flex-only.

## Options (compared)

| Dimension | **A — Flex Web Service** | **B — Client Portal REST** | **C — TWS / IB Gateway socket (`ibapi`)** |
|-----------|---------------------------|----------------------------|-------------------------------------------|
| Wire | HTTPS to IB Flex report endpoints (read-only report fetch) | HTTPS to local **Client Portal Gateway** (`127.0.0.1` high port) REST | Binary socket protocol to TWS/Gateway on `127.0.0.1` |
| **`broker-http-call` fit** | **Yes — unchanged** (host dials IB; Wasm stays HTTP-shaped) | **Partial** — HTTP-shaped to loopback Portal, but session bootstrap is browser/Portal SSO + cookie or competing auth; often treated as **new host session family**, not a drop-in CEX pattern | **No** — not HTTP; needs **new host import** (socket/frame bridge) or **host-native adapter** with Wasm bypass |
| Read-only v1 | **Strong** — Flex queries are report/export oriented | **Weak by default** — Portal API surface includes trading endpoints; v1 must hard path-allowlist | **Weak by default** — API exposes trading; host must refuse order opcodes |
| Freshness | Batch / scheduled (query run → poll/download) — acceptable for autopsy if B6 documents SLA | Near-real-time REST | Real-time / streaming capable |
| Connect UX | Paste Flex token + query id (or guided Flex Query setup); no TWS install **if** Flex suffices for first book | User runs IB Gateway + Portal; browser login to local gateway | User runs TWS or IB Gateway; socket client in agent |
| Local attack surface | Outbound HTTPS only (plus existing loopback agent) | **Local Portal listener** + browser auth to localhost | **Local TWS/Gateway** + long-lived socket |
| Oracle / prior art | Not in Nautilus pin — greenfield B6 | Not in Nautilus pin — greenfield B6 | Nautilus `ibapi` documents shape only — **refuse code copy** |
| Program FULL-COVERAGE note | “Only path fitting `broker-http-call` unchanged” | “Own host-side design + threat review” | Same + socket import ADR-amendment to WIT |
| **Recommendation** | **Choose for v1 first book (DRAFT)** | Defer unless Flex fails B6 money/fills proof | Defer unless Flex/Portal fail; then **C only with WIT amendment** |

## Considered options (if not Flex at accept)

**Option B — Client Portal REST (deferred / second choice).** Rejected for **v1
default** because: (a) it couples Station to a **local IB Gateway + Portal**
lifecycle (install, upgrade, “Gateway not running” class) before every sync;
(b) the REST surface is not read-only — path allowlisting and Kill posture must
prove no write path is reachable from compromised Wasm; (c) session shape
(cookie / competing bearer) is its own `AuthScheme` + Connect flow, not reuse of
Wave 2 loopback OAuth families. Viable **only** if B6 proves Flex cannot deliver
fills for the first locked book and founder accepts the Portal threat model.

**Option C — TWS socket / host-side `ibapi` (deferred / last resort).** Rejected
for **v1 default** because: (a) **breaks ADR 0001** as implemented — requires
either a new sanctioned import (e.g. `broker-socket-call`) or moving IB fetch
entirely into host Rust with Wasm relegated to parse/map only; (b) long-lived
local socket increases blast radius (any agent bug ↔ trading session attached to
Gateway); (c) Nautilus documents socket only — Station still owes clean-room
protocol implementation or a narrowly scoped crate boundary + security review.
Program explicitly triggers a **host-side design PR** before adapter work if this
wins.

## Consequences

### If Option A (Flex) is ACCEPTED

- **Host HTTP only.** Wasm `interactive_brokers` adapter builds Flex report
  requests as HTTP shapes; agent `broker-http-call` attaches Flex token, dials
  **B6-cited production Flex hosts only**, returns response bytes to the
  component. No TWS binary in the hot path.
- **New host module (names provisional):** `agent/src/ubi/ib_flex_session.rs` —
  token storage, report trigger/poll/download, redaction at log boundary.
- **Sync model.** Fills/trades arrive via Flex Query definitions the user (or
  dogfood script) registers at IB — not live tick sync. Cursor/pagination law
  must be spelled in B6 + adapter tests (report run id, date range, duplicate
  suppression).
- **Downstream integrator (after ACCEPTED + B6):** catalog/components
  `interactive_brokers`, allowlist + `dns_block` for Flex hosts, egress meter,
  `ubi-interactive_brokers-adapter` + fixtures (redacted Flex XML/CSV samples),
  component contract test, Tier I ladder per ADR 0019.
- **Multi-book reuse.** Spot FX (`P9-A`) and IB CFD (`P9-D`) **must not** introduce
  a second transport — same Flex (or successor) session family, per-book fences
  only.

### If Option B or C is ACCEPTED instead

- **Freeze Wasm IB adapter** until host transport lands.
- **Option B:** Portal session module + Connect “Gateway running” gate; allowlist
  **only** `127.0.0.1:<portal-port>` and documented Portal paths; explicit
  write-path refusal tests.
- **Option C:** Amend `docs/contracts/ubi-data.wit` (or document host-native IB
  lane with Wasm parse-only) in a **child ADR**; threat review covers Gateway
  auto-reconnect, order-id exposure, and log redaction for socket frames.
- **No `Enabled`** until ADR 0020 ACCEPTED **and** child design merged.

### Threat notes (all options)

- **Secrets:** Flex token / Portal session / Gateway credentials live **Keychain
  only** — never Swift state, never Wasm linear memory, never logs (`AGENTS.md`
  invariant 5).
- **Localhost trust:** Options B and C expand trust in **local IB processes** and
  localhost listeners — document in founder threat review; Flex reduces local
  listeners but still trusts outbound token bearer.
- **Session expiry:** Map IB auth failures to `session_expired` → guided
  reconnect, **never Kill**, never unbounded retry storm (program §5.3).
- **Write refusal:** Host must not implement order placement on any IB transport
  in v1; Kill drill proves no accidental route to trading endpoints.

## Kill / egress notes

- **Kill always-on; Stop ≠ Kill** (ADR 0019). IB slug must register in
  `hosts_for_broker("interactive_brokers")` (exact helper name per integrator
  convention) so Kill severs **all** chosen transport egress.
- **Option A (Flex):** Allowlist + Kill DNS for **IB Flex Web Service production
  host(s)** only — B6 row 22 cites exact hostnames; refuse paper/trade sibling
  hosts on the same slug until a separate book lock exists.
- **Option B (Portal):** Allowlist loopback Portal gateway host:port; Kill must
  drop Portal REST even when Gateway process is user-owned (agent stops dialing).
- **Option C (socket):** Kill must close host socket client and block reconnect
  until operator clears Kill — document interaction with user-run Gateway.
- **Egress meter:** Count Flex/Portal HTTP per sync; socket option meters
  framed requests separately — no silent unbounded poll loops on report jobs.

## Open questions (B6 + founder)

1. **Flex fills sufficiency:** Does an official Flex Query (trades/executions +
   commissions) cover the **first locked US equity book** money row (SEC §31,
   FINRA TAF) without Portal/socket? If no → Flex cannot be ACCEPTED; reopen
   options B/C.
2. **Flex host allowlist:** Exact production URL set, TLS pins (if any), rate
   limits, and token rotation semantics — row 22.
3. **Report latency SLA:** Maximum staleness acceptable for Tier I dogfood vs
   Tier II Live — product, but B6 must cite IB-documented query run timing.
4. **Portal registrability:** If Flex fails, does Client Portal on macOS meet
   Station Connect constraints (Gateway bundling, headless CI refusal)?
5. **Socket path trigger:** If founder picks C, who owns WIT amendment vs
   host-native fetch — and is Wasm still the parser or full bypass?
6. **Jurisdiction / entity:** IB LLC vs IBSG vs IBUK affects first book lock
   — transport ADR does not pick entity; B6 must not conflate hosts across books.
7. **P9-KICKOFF alignment:** Confirm ADR **0020** is indexed as IB W5.1 in
   kickoff (plan table once showed 0015 before Bybit took that slot).

## Self-check

A B6 owner + integrator can tell what to build **after ACCEPTED**: transport
choice, whether `broker-http-call` stays unchanged, credential blob fields,
Kill hosts class, and refused write surface. **Stated: yes, provisionally.**
Implementer must **not** ship Wasm or **Enabled** until founder flips this ADR
to ACCEPTED and B6 closes open questions 1–2 (minimum).

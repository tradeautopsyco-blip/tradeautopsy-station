# Kill Switch Lifetime — change spec + reusable spec format

**Status:** draft for founder execution · manual fix
**Owner:** Bishnu
**Systems touched:** Enforcer (`agent/`) · Station (`station/`) · Console (follow-up phase)

---

## How to use this document

It has two parts.

- **Part A** is the **format** — an empty skeleton with the rule for each section. Reuse it for every future change of this size. This is the part you asked for.
- **Part B** is that skeleton **filled in** with the kill-switch lifetime fix, using real `file:line` anchors from this repo. Read it to understand the system; execute it to fix the bug.

Part A teaches you the shape. Part B proves the shape works.

---
---

# PART A — THE FORMAT

Sixteen sections. Each one exists because leaving it out causes a specific failure. Write them in this order — the order is the reasoning.

### §0 · Mission
One sentence. An **observable** outcome, not an activity.

- Good: "Quitting Station while a kill is armed must leave the broker still blocked."
- Bad: "Improve kill switch reliability." (not observable — you can't tell when you're done)

### §1 · System map
Name every system, and for each: **what decision does it own?** Ownership, not description. Most architecture bugs are two systems believing they own the same fact, or neither owning it.

### §2 · Tech stack
Exact languages, frameworks, **versions**, build tools, per system. A reader who can't build it can't fix it. Pull versions from the manifests, don't recall them.

### §3 · Ground truth — read before writing code
A short list of `file:line` anchors. "Before you touch anything, read these." Ten entries maximum. If you need more than ten, your change is too big — split it.

### §4 · Current behaviour
A numbered trace, end to end, **with no opinions in it**. Just what happens today. If you can't write this section, you do not understand the bug yet and everything after it will be guesswork.

### §5 · The defect
One block per defect. Each block is exactly three lines:

```
Symptom:   what a user sees
Root cause: the code fact, with file:line
Invariant violated: the rule that should have held
```

Separate the defects. "The kill switch is broken" is not a defect, it's a mood.

### §6 · The policy decision
The **human call**, written as law. This is the section engineers skip and then argue about for three weeks. Answer the question the code cannot: what *should* happen? State it as non-negotiable, and record who decided.

### §7 · Target invariants
Testable statements of the form "after this change, X is always true." These become your test names. If an invariant can't be tested, it isn't one — rewrite it.

### §8 · ADD
New files, new functions, new tables. For each: **why it must be new** rather than reusing something that exists. Default assumption is that something already exists.

### §9 · CHANGE
Existing code to modify, with `file:line` and the before → after behaviour.

### §10 · REMOVE
Code to delete. Be explicit — and equally explicit about what looks removable but **must stay**. Deletion is where AI agents and tired humans do the most damage.

### §11 · DO NOT
Guardrails. The single highest-value section when handing this to an AI agent, and the one you'll thank yourself for at 2am. Include:
- files that must not be edited (stale mirrors, generated code)
- shortcuts that would "fix" the symptom while destroying the guarantee
- locked architectural decisions this change must not reopen
- scope creep that is tempting and specifically out of bounds

Write each as an imperative with a reason. "Do not X, because Y."

### §12 · Order of work
Numbered commits. Each one **independently shippable and independently revertable**. If commit 3 must land with commit 4 to be safe, they're one commit.

### §13 · Verification
Per commit: the exact command or manual steps, and the expected output. "Run the tests" is not verification. Name the test.

### §14 · Rollback
How to undo it, and how to tell you need to. For anything touching a safety mechanism, also: what's the *safe* state to fail into?

### §15 · Open questions
Things you genuinely don't know. Instruction: **ask, do not assume.** An AI agent given no open-questions section will invent answers and present them as facts.

### §16 · Vocabulary
Project-specific terms used in this document. Prevents the reader from silently mapping your word onto their concept.

---
---

# PART B — THE FORMAT, FILLED IN

---

## §0 · Mission

Quitting TradeAutopsy Station while an L3 kill is armed must leave the broker block **in force**, and a restarted Enforcer must **re-arm its tamper watcher** from durable state rather than booting amnesiac.

---

## §1 · System map

| System | Runtime | Owns which decision |
|---|---|---|
| **Console** | Next.js + Postgres, hosted | Deep history, aggregates, *remote* fire decisions queued as commands |
| **Station** | Swift macOS app (window + Notch) | On-device risk/capital math, trader-facing live numbers, *local* fire decisions |
| **Enforcer** | Rust `tradeautopsy-agent`, loopback `127.0.0.1:9137` | **Nothing.** Teeth only: executes hosts-file block, DNS flush, signed audit |
| **Behavioral Engine** | offline | Detection math — not in scope |

Locked by `founder-issues/research/R4.1.md:145` (Console repo): *Station computes · Enforcer teeth only · Console deep/history · DB shared facts.*

The Enforcer deliberately makes no decisions. That is correct and this change must not alter it. What the Enforcer is missing is not judgement — it is **memory and lifetime**.

---

## §2 · Tech stack

**Enforcer** — `~/tradeautopsy-station/agent/Cargo.toml`
- Rust 2021, binary `tradeautopsy-agent`, lib `tradeautopsy_agent`
- axum `0.7` · tokio `1` (`full`) · reqwest `0.12` (rustls) · **rusqlite `0.32`** (bundled) · **notify `6`** (`macos_kqueue`) · ed25519-dalek `2` · hmac/sha2 · keyring `3` (apple-native) · wasmtime `36`
- Ports: `9137` API (`AGENT_PORT`), `9138` Prometheus metrics (`AGENT_METRICS_PORT`)

**Station** — `~/tradeautopsy-station/station/`
- swift-tools `6.0`, `SWIFT_VERSION 5.9`, macOS deployment target `14.0`
- AppKit, SwiftPM + **XcodeGen** (`station/project.yml`), swift-testing
- Bundle id `in.tradeautopsy.station`; agent binary copied in via `postBuildScripts: Copy Agent Binary`

**Console** — `~/Tradeautopsy1/Untitled/tradeautopsy/`
- Node `22.x`, Next.js (webpack build), Postgres (`getPool`), Redis, BullMQ, Jest + Playwright

**Enforcement primitive:** `/etc/hosts` sinkhole via `sudo /usr/bin/tee` and `sudo /usr/bin/sed`, then `dscacheutil -flushcache` + `killall -HUP mDNSResponder`.

---

## §3 · Ground truth — read before writing code

| # | Anchor | Why |
|---|---|---|
| 1 | `agent/src/dns_block.rs:117` `apply_hosts_block` | the actual teeth |
| 2 | `agent/src/dns_block.rs:219-223` `WATCHER_HANDLE`, `ARMED_BROKER` | the RAM-only tamper state |
| 3 | `agent/src/dns_block.rs:226` `start_hosts_watcher` | re-apply on tamper |
| 4 | `agent/src/dns_block.rs:287` / `:297` enable/disable + watcher | the two entry points |
| 5 | `agent/src/api/kill_switch.rs:87` `apply_kill_switch_level` | L1/L2/L3 fan-out, both trigger paths land here |
| 6 | `agent/src/api/kill_switch.rs:144` `dismiss_kill_switch_state` | the reverse |
| 7 | `agent/src/lib.rs:528,544,547` `fog_active`, `last_l3_broker` | the state that dies with the process |
| 8 | `agent/src/lib.rs:616-617` `axum::serve` then `abort_all` | why shutdown never runs |
| 9 | `station/StationApp/AgentSupervisor.swift:55` `shutdown()` | the bypass |
| 10 | `station/StationApp/AgentSupervisor.swift:97` `killBundledAgentOrphans()` | the sweep that guarantees the bypass |

Also read: `agent/src/main.rs` (14 lines — note the total absence of signal handling).

---

## §4 · Current behaviour

**Arming (two paths, one destination):**

1. *Local:* Notch posts to `http://127.0.0.1:9137/api/daemon/kill-switch` (`notch/NotchViewModel.swift:2485`), HMAC-signed (Wire v1, `agent/src/wire.rs`).
2. *Remote:* Console inserts a `fog_of_war` row into `daemon_commands`; the Enforcer polls `GET /api/daemon/command` every 10s (`agent/src/api/daemon_commands.rs:110`, loop body `:131`).
3. Both call `apply_kill_switch_level()` (`agent/src/api/kill_switch.rs:87`).
4. **L1** sets a flag + SSE. **L2** sends a 90s countdown over SSE. **L3** calls `enable_block_with_watcher()`: appends `127.0.0.1 <host> # tradeautopsy-killswitch` lines, flushes DNS, kills browser network helpers, starts a `notify` watcher, appends a signed audit row, emits SSE `requires_ack: true`.
5. If `/etc/hosts` is edited while armed, the watcher re-applies the block (`dns_block.rs:226`).

**Quitting:**

6. `applicationShouldTerminate` returns `.terminateLater` and awaits `coordinator.quit()` (`station/StationApp/StationApp.swift`).
7. `quit()` calls `agentSupervisor.shutdown()` (`StationAppCoordinator.swift:272`).
8. `shutdown()` sends SIGTERM, waits 2s, SIGKILLs, **then `killBundledAgentOrphans()` sweeps every agent process matching the bundle path** (`AgentSupervisor.swift:55-73`).
9. The Enforcer dies. It has no signal handler, so it writes nothing on the way out.
10. Next launch, `spawnAgent()` reaps orphans again *before* spawning (`AgentSupervisor.swift:230`).

**Result after step 9:** the hosts lines remain, but the watcher, `fog_active`, `last_l3_broker` and the command poll loop are all gone.

---

## §5 · The defect

**D1 — Remote kills cannot fire while Station is closed**
- *Symptom:* Console trips the loss limit; nothing happens. Trader keeps trading.
- *Root cause:* the only delivery mechanism is a poll loop inside the agent (`agent/src/api/daemon_commands.rs:110`), started from `agent/src/lib.rs:592`. No app → no agent → no poll. The row sits at `delivered_at IS NULL` indefinitely.
- *Invariant violated:* a fired kill is eventually enforced.

**D2 — A fetched command is marked delivered before it is executed**
- *Symptom:* an agent that dies between fetch and apply loses that kill forever.
- *Root cause:* `app/api/daemon/command/route.ts:87-91` (Console) runs `UPDATE daemon_commands SET delivered_at = NOW()` immediately on read, with no ack from the executor. At-most-once delivery on a safety path.
- *Invariant violated:* a fired kill is enforced **at least** once.

**D3 — Tamper protection is process-lifetime only**
- *Symptom:* quit the app, `sudo sed -i '' /killswitch/d /etc/hosts`, trade freely. Nothing restores the block.
- *Root cause:* `WATCHER_HANDLE` / `ARMED_BROKER` are process-global `Mutex`es (`agent/src/dns_block.rs:219-223`). Nothing outlives the process.
- *Invariant violated:* while a kill is armed, the block is continuously enforced.

**D4 — The Enforcer boots amnesiac**
- *Symptom:* after any restart, the hosts file says "blocked" and the agent believes "not blocked". A dismiss then reports broker `unknown`.
- *Root cause:* `run_agent()` never calls `is_block_active()` at startup and never re-arms the watcher. `fog_active` and `last_l3_broker` initialise to empty (`agent/src/lib.rs:528,547`).
- *Invariant violated:* agent state and machine state agree at boot.

**D5 — The kill latch is not durable, and the audit trail lives in a purgeable directory**
- *Symptom:* no record of "a kill is currently armed" survives a restart; audit evidence can silently vanish.
- *Root cause:* latch state is RAM only; the signed audit DB defaults to `std::env::temp_dir()` (`agent/src/kill_switch_audit.rs:188-196`), which macOS purges.
- *Invariant violated:* safety state and safety evidence are durable.

**D6 — Station is engineered to guarantee the Enforcer dies**
- *Symptom:* "quit the app" is a reliable, supported bypass.
- *Root cause:* `AgentSupervisor.shutdown()` SIGKILLs and then sweeps by executable path (`AgentSupervisor.swift:97-103`), with no check for armed state. `spawnAgent()` reaps again at `:230`.
- *Invariant violated:* the trader cannot disable enforcement by ordinary UI action.

**D7 — Force-quit produces the mirror failure**
- *Symptom:* crash or force-quit → orphan agent holds port 9137 with the block live, but no UI exists to dismiss it. Trader is locked out with no recourse; next launch silently reaps the watcher.
- *Root cause:* same root as D4/D6 — no durable latch, no reconciliation, no attach-in-preference-to-reap.
- *Invariant violated:* every armed kill has a reachable, audited dismiss path.

**Single root cause behind all seven:** *enforcement state lives in the RAM of a process whose lifetime is bound to a GUI app.*

---

## §6 · The policy decision

Decided by founder, 2026-07-31:

> **An armed L3 block survives quitting Station.** It does **not** yet need to survive a reboot.
>
> Quitting is not consent. The block lifts only when (a) its timer expires, or (b) the trader explicitly dismisses/acks through the normal path — which is audited.
>
> Reboot-proofing is deliberately deferred to Phase 2 (LaunchDaemon via `SMAppService`). Until Phase 2 ships, **reboot is a known, accepted bypass** and must be written down as such, not quietly forgotten.

Architecture consequence: Phase 1 is durable-latch-plus-reconciliation. **No root daemon, no privileged helper, no `SMAppService` in this change.**

---

## §7 · Target invariants

After Phase 1, each of these is true and has a test named after it:

1. `latch_survives_sigterm` — SIGTERM to the agent while armed leaves the hosts block in place and the latch on disk.
2. `boot_reconciles_latch_to_hosts` — on start, agent state and `/etc/hosts` agree, in all four combinations of (latch armed?) × (hosts blocked?).
3. `boot_rearms_watcher_when_armed` — a restarted agent re-arms the tamper watcher without rewriting an already-correct hosts file.
4. `tamper_after_restart_is_reverted` — remove the hosts lines after a restart; they come back.
5. `station_quit_does_not_kill_latched_agent` — with the agent latched, `AgentSupervisor.shutdown()` leaves the process alive.
6. `station_launch_attaches_to_latched_orphan` — a latched orphan is attached to, never reaped.
7. `expired_latch_self_dismisses` — once past `expires_at_ms`, the block lifts and an audit `dismiss` row is written.
8. `latch_read_failure_fails_closed` — an unreadable/corrupt latch DB keeps the block, never drops it.
9. `dismiss_clears_latch_only_after_hosts_clean` — latch is cleared after successful hosts removal, not before.

---

## §8 · ADD

**A1 · `agent/src/kill_latch.rs` — durable latch store**
Model it directly on `KillSwitchAuditStore` (`agent/src/kill_switch_audit.rs`) — same rusqlite `0.32` bundled pattern, same open/migrate shape. Single-row table:

```sql
CREATE TABLE IF NOT EXISTS kill_latch (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  active        INTEGER NOT NULL,
  level         INTEGER NOT NULL,
  broker        TEXT    NOT NULL,
  armed_at_ms   INTEGER NOT NULL,
  expires_at_ms INTEGER,
  requires_ack  INTEGER NOT NULL DEFAULT 0
);
```

*Why new:* the audit store is an append-only evidence log. A latch is current mutable state. Overloading the audit table would mean deriving "am I armed?" by replaying history — slower, and it corrupts the evidence log's meaning.

**A2 · `reconcile_kill_state()` in `agent/src/lib.rs`**
Runs in `run_agent()` **before** `axum::serve` (`lib.rs:616`). Four cases:

| latch | `/etc/hosts` | action |
|---|---|---|
| armed | blocked | re-arm watcher only — do not rewrite the file |
| armed | clean | re-apply block + watcher (tamper or reboot); audit `reapply` |
| clear | blocked | stale leftover → remove; audit `cleanup` |
| clear | clean | nothing |

Plus: armed **and** past `expires_at_ms` → dismiss + audit.

**A3 · Graceful shutdown** — `tokio::signal` handling SIGTERM + SIGINT, via `axum::serve(...).with_graceful_shutdown(...)`. Flush the latch, log, exit 0. **Never remove the block.**

**A4 · `GET /api/daemon/kill-switch/state`** — register in the table at `agent/src/api/mod.rs:68+`, Wire v1 verified like its neighbours. Returns `{ active, level, broker, armed_at_ms, expires_at_ms, requires_ack, dns_active }`. Station needs this to ask "are you latched?" before killing.

**A5 · Expiry task** — background loop that auto-dismisses on `expires_at_ms`. Today `L3_COUNTDOWN_SECS = 90` (`agent/src/api/kill_switch.rs:17`) is only a number sent over SSE; nothing enforces it. Add it to the `JoinSet` alongside the heartbeat.

**A6 · Swift: `func isKillLatched() async -> Bool`** on `AgentSupervising` (`station/StationApp/Protocols/StationProtocols.swift:4`), implemented in `AgentSupervisor` against A4 reusing `signedHealthRequest()`'s signing path (`AgentSupervisor.swift:409`), and stubbed in `station/StationTests/Fakes/FakeAgentSupervisor.swift`.

---

## §9 · CHANGE

| # | Where | Before → After |
|---|---|---|
| C1 | `agent/src/api/kill_switch.rs:87` `apply_kill_switch_level` | write latch **before** `enable_block_with_watcher` (crash mid-apply must be recoverable); record `expires_at_ms` |
| C2 | `agent/src/api/kill_switch.rs:144` `dismiss_kill_switch_state` | clear latch **after** hosts removal succeeds, not before |
| C3 | `agent/src/lib.rs:528,547` | `fog_active` / `last_l3_broker` become **derived from the latch**, not independent RAM. One source of truth |
| C4 | `agent/src/lib.rs` `AgentConfig` | add `kill_latch_db_path`; env `AGENT_KILL_LATCH_DB_PATH`; default `~/Library/Application Support/in.tradeautopsy.station/` |
| C5 | `agent/src/kill_switch_audit.rs:188` | default path off `temp_dir()` → Application Support. **Replace** the fallback, don't keep it |
| C6 | `AgentSupervisor.swift:55` `shutdown()` | if `isKillLatched()`, skip `terminateOwnedProcess` **and** skip `killBundledAgentOrphans()`; detach and log. **This is the fix for D6** |
| C7 | `AgentSupervisor.swift:230` (reap inside `spawnAgent`) | attach-before-reap: never reap an orphan that reports latched. `tryAttachToExistingListener()` (`:176`) already attaches — make attach win |

---

## §10 · REMOVE

- **C5's `temp_dir()` fallback** — delete it outright. A silently purged audit DB is evidence loss, and a fallback that "works" is worse than a hard failure here.

**Looks removable — must stay:**
- the hosts-file mechanism (it *is* the teeth)
- the `notify` watcher (still the tamper defence; it gains persistence, not a replacement)
- Wire v1 HMAC on every route (A8 LOCKED — see §11)
- the crash-loop backoff in `AgentRestartTracker` (tested at `station/StationTests/AgentSupervisorTests.swift:9-48`)

**Deferred, not done here:** `~/Tradeautopsy1/Untitled/tradeautopsy/crates/agent/` is a stale duplicate of the Enforcer and should eventually go, but it is a member of that repo's Cargo workspace. File it separately — deleting it inside this change widens the blast radius for no safety gain.

---

## §11 · DO NOT

1. **Do not auto-dismiss the block on SIGTERM, quit, crash, or logout.** Quitting is not consent (§6). This is the single most tempting "fix" and it destroys the entire guarantee.
2. **Do not edit `~/Tradeautopsy1/Untitled/tradeautopsy/crates/agent/**`.** It is a stale mirror. The live Enforcer is `~/tradeautopsy-station/agent/`. The mirror still carries the R8 bug where an unknown broker slug falls back to Kotak hosts — never copy that direction.
3. **Do not let `hosts_for_broker` fall back to a default broker,** and do not treat an empty host set as a successful block. `agent/src/dns_block.rs` already refuses empty (R8). Keep it refusing.
4. **Do not put the latch DB in `temp_dir()`.** That is defect D5; recreating it defeats the change.
5. **Do not weaken or bypass Wire v1 HMAC** to make the new `/state` route easier to call. A8 is LOCKED (`founder-issues/research/A8.md:270`, Console repo): Wire v1 stays machine-only, user identity is the Caller JWT. This change does not reopen it.
6. **Do not add a LaunchDaemon, `SMAppService` registration, privileged helper, or `SMJobBless` in Phase 1.** Explicitly deferred (§6). Adding it drags in codesigning, notarization and a changed trust boundary on 9137.
7. **Do not fail open anywhere.** If the latch DB is unreadable, assume armed and keep the block (invariant 8). The Console side already has fail-open bugs flagged by your own CSO pass — do not add a third.
8. **Do not change `L3_COUNTDOWN_SECS` semantics** (`agent/src/api/kill_switch.rs:17`) until K4 is decided (§15). Wire `expires_at_ms` to the existing 90s for now.
9. **Do not add new markdown files to the Console repo.** `CLAUDE.md` there forbids summary/session docs. This spec lives in the Station repo's existing `plans/`.
10. **Do not "simplify" by removing the poll loop in favour of SSE push** in this change. It's the right eventual direction; it is not this commit.

---

## §12 · Order of work

Each commit ships and reverts alone.

1. **`kill_latch.rs` + unit tests** — pure store, no wiring.
2. **Wire latch into apply/dismiss** (C1, C2) + move audit DB path (C5).
3. **Boot reconciliation** (A2) **+ expiry task** (A5) + derive `fog_active`/`last_l3_broker` from latch (C3).
4. **Graceful shutdown** (A3).
5. **`/state` route** (A4) + Wire v1 verification test.
6. **Swift:** protocol + `isKillLatched` (A6), shutdown guard (C6), attach-before-reap (C7), tests.
7. **Console phase** — D1/D2 (below). Separate PR.

Commits 1–6 are the Station repo. Commit 7 is the Console repo.

---

## §13 · Verification

**Sudo-free Rust tests.** `agent/src/dns_block.rs` already supports `TRADEAUTOPSY_HOSTS_FILE` pointing at a temp file (see the existing `apply_and_disable_block_with_test_hosts_file` test) — use it for everything.

```bash
cd ~/tradeautopsy-station/agent
TRADEAUTOPSY_HOSTS_FILE=/tmp/ta-test.hosts cargo test -p tradeautopsy-agent
```

| Commit | Check |
|---|---|
| 1 | latch round-trips; corrupt DB → error, never "not armed" |
| 2 | after `apply`, latch row exists **before** hosts write; after `dismiss`, latch clears only on clean hosts |
| 3 | all four reconcile cases (invariant 2); expired latch self-dismisses |
| 4 | `kill -TERM <pid>` while armed → hosts still blocked, latch intact, exit 0 |
| 5 | unsigned request to `/state` → 401; signed → correct JSON |
| 6 | `cd ~/tradeautopsy-station/station && swift test` |

**Manual end-to-end (the one that actually proves the bug is dead):**

1. Arm L3 from Notch.
2. `grep tradeautopsy-killswitch /etc/hosts` → lines present.
3. Quit Station normally.
4. `pgrep -f tradeautopsy-agent` → **still running** (this fails today).
5. `grep tradeautopsy-killswitch /etc/hosts` → still present.
6. `sudo sed -i '' /tradeautopsy-killswitch/d /etc/hosts` → re-appears within a second.
7. Relaunch Station → attaches to the existing agent, does not spawn a second one, does not reap.
8. Dismiss through the UI → hosts clean, latch cleared, audit shows `fire` then `dismiss`.

**Known accepted gap:** reboot with Station not auto-launching leaves the trader unblocked until Phase 2. Confirm this is still acceptable before shipping.

---

## §14 · Rollback

Per-commit `git revert`; commits are ordered so reverting N doesn't strand N-1.

**Safe state to fail into:** blocked. If reconciliation is buggy, prefer leaving a block on and forcing a manual `sudo sed -i '' /tradeautopsy-killswitch/d /etc/hosts` over silently dropping enforcement.

**Manual escape hatch** (document it for yourself before you start):
```bash
sudo sed -i '' /tradeautopsy-killswitch/d /etc/hosts
sudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder
rm ~/Library/Application\ Support/in.tradeautopsy.station/kill-latch.db
```

---

## §15 · Open questions

Ask; do not assume.

1. **K4 — is L3 duration user-configurable?** `~/issues/kill-switch/KILL-SWITCH.md` says L2/L3 should be "fully customized by user (when it triggers, for how long)". Today it's a hardcoded 90s. `expires_at_ms` needs a source of truth.
2. **Who may dismiss?** If the trader alone can dismiss an armed L3 at will, the teeth are cosmetic. Is there a cooling-off period, or an ack gate?
3. **Reboot gap acceptable in production?** §6 accepts it for Phase 1. Confirm before shipping to real traders, not after.
4. **Latch DB corruption** — fail closed keeps the block, but with a broken dismiss path. What's the recovery UX?
5. **Does the `notify` watcher survive `sed -i`?** `sed -i ''` replaces the inode rather than modifying in place. Verify the kqueue watcher still fires — if it doesn't, D3 is worse than described and needs a directory watch instead of a file watch. **Test this early; it may change the design.**

---

## §16 · Vocabulary

| Term | Meaning here |
|---|---|
| **Enforcer** | the Rust `tradeautopsy-agent` process. Teeth only — makes no decisions |
| **Teeth** | actual enforcement (hosts block). As opposed to UX (overlay, countdown, blur) |
| **Latch** | durable "a kill is currently armed" state. The thing this change adds |
| **L1 / L2 / L3** | inform · countdown overlay · DNS block. Only L3 has teeth |
| **Fog / fog_of_war** | legacy name for the kill command type on the wire |
| **Wire v1** | HMAC-SHA256 machine-integrity signing on loopback. Not user identity |
| **Reap** | Station killing agent processes matching its bundle path |
| **Reconcile** | compare durable intent against machine reality at boot, converge |
| **Fail closed** | on error, keep enforcing. The correct default for every path here |

---

## Phase 2 preview (not this change)

Promote the Enforcer to a root **LaunchDaemon** via `SMAppService` (macOS 13+). That upgrades §6 from "survives quit" to "survives reboot", and lets you drop the `sudo` shellouts for a privileged helper over XPC. Everything in Phase 1 — durable latch, reconciliation, graceful shutdown — is a prerequisite, which is why it goes first.

## Phase 3 (Console, separate PR)

- **D1:** undelivered `fog_of_war` commands must still be honoured when the agent returns. Needs a validity window rather than silent staleness.
- **D2:** stop marking `delivered_at` at fetch time (`app/api/daemon/command/route.ts:87-91`). Require an execution ack; move to at-least-once.
- Note the fallback path in the same file queries `signal_stream` with a **30-second** window — kills older than 30s are dropped silently if `daemon_commands` errors.
- Fail-open guards already flagged by your own CSO pass (`~/issues/kill-switch/KILL-SWITCH.md`): `lib/pre-order/pre-order-guard.ts:54`, `bar-intervention-route-gate.ts:74`, and the missing `bar_kill_switch` latch at `app/api/intelligence/kill-switch/route.ts:53`.

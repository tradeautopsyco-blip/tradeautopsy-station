# Broker Marketplace — Handoff from grilling session (tradeautopsy1/Untitled/tradeautopsy)

Paste this into a new Claude Code session opened at `/Users/bishnu/tradeautopsy-station` to continue. Do not re-litigate the decisions below — they were reached after a full `/grill-me` pass. Pick up with `/to-prd` (or re-run `/grill-me` first if you want to sanity-check the Station-side specifics against real code).

## The ONE goal (locked)

Ship a broker marketplace inside Station's **Backend Box**: an App-Store-style module system where each broker declares its own connection requirements (OAuth / API key / credentials / TOTP / MPIN / phone), so a user can browse a catalog and connect any supported broker without custom code per user.

## The ONE priority, right now

Design and build the per-broker "requirements schema" + marketplace list/detail UI in Backend Box. Nothing else gets worked until this ships.

## Why this note exists — a scope correction mid-session

The grilling session that produced these decisions ran in the **wrong repo** (`tradeautopsy1/Untitled/tradeautopsy`, the Console/web app). That repo has a broker system too — `lib/brokers/universal/registry.ts` — 27 pre-defined brokers (India/Global/Crypto) with metadata, only 3 with real connector adapters (Kotak Neo, Zerodha, Upstox), plus a `BrokerCredentials` type with `authType: oauth|apikey|credentials` and generic fields (apiKey, pin, totpSecret, etc.).

**That is NOT Backend Box.** Backend Box is defined in that repo's own `CONTEXT.md` (locked decision, 2026-06-08) as Station-native: Rust `BrokerAdapter` trait + macOS Keychain, living in the loopback agent (`tradeautopsy-agent`, port 9137), with an explicit invariant: *"Web does not replace Backend Box for live broker setup; Station is self-contained at the broker layer."*

So: **first task in this new session is to re-explore the real Backend Box / BrokerAdapter code in *this* repo** (likely under `station/StationApp`, `agent/`) before writing anything — don't assume the Console's TypeScript registry shape carries over. It might be useful prior art for the "requirements schema" concept, but the actual implementation target is Rust/Swift + Keychain here.

## Decisions locked during grilling (carry these forward)

1. **Scope = Station only.** Console's sidebar-inconsistency audit (separate product, separate thread) is explicitly out of scope.
2. **Notch vs. main shell are two separate surfaces.** The Notch (⌥Space ambient overlay) is untouched by this work. The main shell (Dock icon / Cmd-Tab window) is a separate concern — see parked item below.
3. **Broker model = curated marketplace, not arbitrary user-typed endpoints.** User picks from a growing, vetted catalog. Reasoning: different brokers need different auth flows (OAuth vs API key vs credentials+TOTP+MPIN) — each needs vetting before real money touches it. This is what "like an App Store" concretely means: a detail page per broker describing what it fetches, how it connects, and what permissions it needs.
4. **Compliance = hybrid, baked-in.** No standalone compliance audit project. Write one short one-time checklist (Keychain-only credential storage/no plaintext, per-broker ToS/automation-permission check, audit logging on connect/disconnect, SEBI algo-tag requirement where applicable, data residency) — every broker-module issue inherits it as part of its definition-of-done.
5. **Cascade = `/to-prd` → plan → `/to-issues`**, markdown artifacts in-repo first, converted to real GitHub issues at execution time. Triage label vocabulary already exists in this org (`ready-for-agent` confirmed present in `tradeautopsy` repo's labels — check if it exists here too or needs creating).

## Explicitly parked, in priority order after the marketplace ships

1. **Dock/Cmd-Tab fix for the desktop shell** — found during grilling (in the *other* repo's `src-tauri/scripts/sign-and-run.sh`) that `LSUIElement=true` was hardcoded in a duplicated, hand-maintained Info.plist independent from the real production Info.plist referenced by `tauri.conf.json` — causing dev builds to have no Dock icon/Cmd-Tab while the window itself is already configured normally (1440×900, resizable). Durable fix = single source of truth, not a one-line flip. **This may or may not apply to this repo's build — re-verify here, don't assume the same bug exists in Station's actual build pipeline.**
2. **HTML visual restyle** of the shell (founder supplied a full HTML mockup: fixed native window ~1100×700, macOS traffic-light chrome, specific dark-theme CSS vars/typography).
3. Console's sidebar audit — separate product, not Station's problem.

## Founder's working-style notes (carry over)

- Wants brutally clear, ONE goal + ONE priority at a time — resist scope creep back into "everything at once."
- Wants information flow paced/low-bandwidth ("50 bits per second") — one question/decision at a time, not dense dumps.
- Never trust "done" claims without grep/diff/screenshot evidence.

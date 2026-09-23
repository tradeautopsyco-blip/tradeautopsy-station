# ADR 0005: Zerodha Kite redirect callback landing point

**Status:** ACCEPTED (founder bundle — go ahead 2026-09-23 IST)

**Date:** 2026-09-23 IST

**Number note:** pre-assigned 0005 by the full-coverage program. The 0004
collision (program §7.6: Z3 vs taxonomy) resolved 0004 → taxonomy
(`0004-ubi-data-taxonomy-book-keyed-catalog.md`); this ADR takes 0005, never
another number.

**Program:** `issues/brokers/ALL-BROKERS-PROGRAM.md` §4.1 Z3 (gate ADR-lite,
F9). B6 sheet (`issues/brokers/sheets/zerodha_kite.md`) is owned by the
parallel B6 lane; Kite login-shape details below are provisional from oracle
mining until that lane cites official docs.

## Context

1. **F9: no inbound web callback today.** The native app is a loopback agent
   (Axum, `127.0.0.1:9137`, `AGENT_PORT` fixed per `AGENTS.md`) + Swift shell.
   Verified 2026-09-23: `agent/src/api/mod.rs` exposes HMAC-wired
   `/api/daemon/…` routes (sync start/stop/retry, Kotak session mint,
   credentials clear/present, kill-switch, bar, auth) plus unauthenticated
   read-only `/api/station/…` extracts (quote, history, chain, oi, depth,
   greeks, index, manifest, obtain, sync-hint). Nothing receives an inbound
   browser redirect. `station/StationApp/Info.plist` carries no
   `CFBundleURLSchemes` — no custom URL scheme exists today.
2. **Kite shape (provisional, B6 lane to verify):** Kite Connect redirect
   login — user authorizes in browser → `request_token` lands at a registered
   redirect URI → host exchanges via SHA-256(`api_key` + `request_token` +
   `api_secret`) → `POST https://api.kite.trade/session/token` → access
   token with daily TTL, used as `Authorization: token <key>:<token>`. Daily
   expiry → `session_expired` → reconnect, never Kill.
3. **Law this ADR sits under.** ADR 0001: no secrets in Wasm, all auth attached
   by the host via `broker-http-call`; the checksum exchange is therefore
   host-side by construction. ADR 0004: format/taxonomy law mirrored here in
   structure only. Program §5.1: "Kite checksum+token" is its own `AuthScheme`
   family; §5.3: credential TTL and trading session are two clocks, two code
   paths, two tests.

## Decision

**Recommendation: Option A — loopback agent route.** The request token lands
directly on the agent, which already owns the loopback surface, the Keychain,
and all broker HTTP. The browser is pointed at an exact registered redirect
URI; no new OS-global scheme, no Swift hop for a secret-bearing URL.

### Landing point (exact)

```text
GET http://127.0.0.1:9137/api/daemon/broker/zerodha/callback?request_token=…&state=…&status=…
```

- Per-slug path, matching the existing per-broker route style
  (`/api/daemon/broker/kotak/session/mint`). Wave 2 siblings add sibling paths
  reusing the same state-nonce machinery — never one shared path that guesses
  the broker, never a flag.
- `state` is a single-use 128-bit nonce minted host-side by a new
  `POST /api/daemon/broker/zerodha/connect/begin` (Z6/Z10 build it), bound to
  `connection_id`, TTL ≈ 10 minutes. The callback accepts only a live,
  unused, matching `state`; anything else is rejected and logged without
  secrets. This is the CSRF + local-process guard: the route carries no Wire
  HMAC (a browser redirect cannot sign), so the nonce is the auth.
- Bind stays `127.0.0.1` (existing invariant); port stays 9137 (existing
  invariant). `status=error` from Kite maps to a surfaced Connect failure,
  never a retry loop, never Kill.
- On success the agent exchanges immediately (host-side, §Consequences) and
  answers the browser with a minimal local success page ("return to Station");
  Swift learns the outcome by polling session status, not via URL.

### Why loopback is the right landing point

1. Token travels one hop, browser → agent, and dies there (single-use,
   exchanged at once). It never crosses a process boundary into Swift.
2. The exchanger (api_secret holder, Keychain writer, Kite HTTP dialer) is
   already the agent. Option B would still need a Swift→agent handoff for the
   exchange, adding a hop that carries the token through a second process.
3. Loopback redirect is the standard native-app OAuth pattern (RFC 8252
   §7.3); Kite app settings register the exact URI, so there is no ambiguity
   about where the token goes.
4. Zero new OS surface: no scheme registration, no scheme-collision or
   hijack class, no `Info.plist` / Xcode project change.

## Considered options (rejected alternative)

**Option B — Swift custom URL scheme (rejected).** E.g.
`tradeautopsy-connect://broker/zerodha/callback?request_token=…&state=…`,
registered via `CFBundleURLSchemes` in `station/StationApp/Info.plist`
(+ `project.yml`/`project.pbxproj` propagation), handled in Swift
(`onOpenURL` / `application(_:open:)`) and forwarded to the agent over the
existing signed loopback API for the checksum exchange.

Rejected because: (a) the exchange must run host-side anyway (ADR 0001 —
Swift never holds api_secret, Wasm never sees any of it), so B adds a
secret-bearing Swift hop with no architectural payoff; (b) URL schemes are
OS-global — first-come registration creates a hijack/claim dispute class
that loopback binding does not have; (c) the token sits in a URL handled by
app-delegate plumbing, widening lifetime and log-redaction surface versus an
agent route that exchanges and drops it in one request; (d) it forks the
Connect flow per transport instead of per family, against program §5.2
(one redirect flow, built once).

## Consequences

- **Checksum exchange runs host-side in `agent/src/ubi`** (program Z6 names
  `agent/src/ubi/zerodha_session.rs`) — never Swift, never Wasm (ADR 0001).
  Inputs (`api_key`, `api_secret`) are read from the Keychain blob inside the
  agent; the SHA-256 checksum is computed in agent memory and POSTed to
  `api.kite.trade/session/token` over host HTTP. `request_token` is
  single-use and never persisted anywhere.
- **Daily token lives ONLY in the Keychain tagged blob.** No copy on disk, in
  prefs, in Swift state, or in Wasm memory. Swift holds handles/status, never
  the token.
- **Two clocks, two paths (program §5.3).** Credential TTL (daily expiry →
  `session_expired` → guided reconnect, never Kill, never auto-retry storm)
  and trading session (market-hours/session-validity) are separate code paths
  with separate tests. Expiry-day behavior is a named dogfood drill (Z11).
- **What downstream lanes build from this:**
  - **Z5 (variant/blob):** new `AuthScheme` variant (proposed name
    `ZerodhaKiteChecksumToken`), new `CredentialBlob` variant (proposed tag
    `zerodha_kite_checksum_token`) with fields `apiKey`, `apiSecret`,
    `accessToken`, `expiresAt` (ISO-8601, date of issue/expiry per B6 row —
    `request_token` is explicitly NOT a field); catalog descriptor
    `Planned` + `manifest_id` + `book_id`. Keychain service: default
    `BROKER_CREDENTIAL_KEYCHAIN_SERVICE` unless Z5 shows an ACL reason for a
    dedicated vault (Kotak precedent).
  - **Z6 (signer/session):** `connect/begin` (mint `state`), callback consume
    + exchange, `Authorization: token <api_key>:<access_token>` attached on
    private paths only (absent on public), expiry → `session_expired`
    mapping with tests.
  - **Z10 (Swift Connect):** family redirect flow built once — Begin →
    open Kite login URL in browser → poll agent session status → Keychain
    confirmation → per-connection INR strip. No scheme handling, no token
    parsing, no shell fork.
- **Threat notes.**
  - *Token-in-URL lifetime:* seconds — lands, exchanges, dropped. Never
    logged (see below), never stored, never forwarded.
  - *Loopback binding:* `127.0.0.1` only, port fixed 9137. Cross-process
    local callers are stopped by the unguessable single-use `state`, not by
    origin checks the agent cannot trust.
  - *Log redaction:* no secrets in logs per `AGENTS.md` invariant 5.
    Callback logs carry status + truncated `state` prefix only;
    `request_token`, `access_token`, checksum, and secrets are redacted at
    the handler boundary, with a test proving it.
  - *`state` hygiene:* 128-bit RNG, single-use (consume-on-read), TTL ≈
    10 min, bound to `connection_id`. Replays and cross-connection reuse
    fail closed.
- **B6 dependency:** redirect-URI registrability (exact loopback URI
  accepted in Kite app settings), TTL source, and the checksum/exchange
  shape must be cited to official Kite docs by the B6 lane before Z5/Z6
  code. If Kite refuses loopback redirect URIs, this ADR reopens — that is
  the one fact that would revive Option B.

## Self-check

A Z5/Z6/Z10 implementer can build from this without asking a question: the
landing path, the `state` contract, the exchange location, the blob fields,
the header shape, the expiry mapping, and the Swift polling model are each
stated exactly. **Stated: yes.**

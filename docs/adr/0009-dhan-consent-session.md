# ADR 0009: Dhan consent-login session landing point

**Status:** ACCEPTED (founder bundle — go ahead 2026-09-24 IST; D1/D2/Q-ADR closed at B6; loopback accepted on faith — dogfood proves registrability, ADR reopens if Dhan refuses)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned 0009 by the full-coverage program — never
another number.

**Program:** `issues/brokers/ALL-BROKERS-PROGRAM.md` Wave 2 (`dhan` book).
B6 sheet (`issues/brokers/sheets/dhan.md`) is owned by the parallel B6 lane;
login-shape details below are verified from official DhanHQ docs at
`issues/brokers/sheets/dhan.md` (SIGNED 2026-09-24; oracle: DhanHQ-py
`63b9030` + OpenAlgo `ad3cd54`).

## Context

1. **F9: no inbound web callback today.** The native app is a loopback agent
   (Axum, `127.0.0.1:9137`, `AGENT_PORT` fixed per `AGENTS.md`) + Swift shell.
   Per ADR 0005: `agent/src/api/mod.rs` exposes HMAC-wired
   `/api/daemon/…` routes plus unauthenticated read-only `/api/station/…`
   extracts. Nothing receives an inbound browser redirect.
   `station/StationApp/Info.plist` carries no `CFBundleURLSchemes` — no
   custom URL scheme exists today.
2. **Dhan consent 3-step (verified at B6 rows 10/14/16):** DhanHQ consent
   login — (1) host `POST https://auth.dhan.co/app/generate-consent?client_id=`
   with `app_id` + `app_secret` headers → `consentAppId`; (2) user authorizes
   in browser at `GET https://auth.dhan.co/login/consentApp-login?consentAppId=`
   → `tokenId` lands at a registered callback; (3) host consumes
   `?tokenId=` with `app_id` + `app_secret` headers → `accessToken` +
   `dhanClientId`/`dhanClientName`/`dhanClientUcc`/`givenPowerOfAttorney`/
   `expiryTime`. Session token used as `access-token` header on
   `https://api.dhan.co/v2/*` reads (`access-token` on all reads; `client-id`
   header additionally on `POST /marketfeed/*` only; `dhanClientId` as a JSON
   field in POST bodies — D2 closed at B6 row 10). Expiry → `session_expired` → reconnect, never
   Kill. Two sibling shapes exist in the oracle and are NOT this ADR:
   PIN+TOTP direct (`POST /app/generateAccessToken?dhanClientId=&pin=&totp=`,
   no browser) and RenewToken/validate (`GET /v2/RenewToken`, `GET
   /v2/profile`) — see Q-ADR below.
3. **Law this ADR sits under.** ADR 0001: no secrets in Wasm, all auth attached
   by the host via `broker-http-call`; the consent generate/consume exchange
   is therefore host-side by construction. ADR 0005: the Kite redirect
   pattern (per-slug loopback callback + state-nonce machinery) mirrored here
   in structure. Program §5.1: Dhan consent session is its own `AuthScheme`
   family; §5.3: credential TTL and trading session are two clocks, two code
   paths, two tests.

## Decision

**Recommendation: Option A — loopback agent route.** The `tokenId` lands
directly on the agent, which already owns the loopback surface, the Keychain,
and all broker HTTP. The browser is pointed at the registered consent-login
URL; no new OS-global scheme, no Swift hop for a secret-bearing URL.

### Landing point (exact)

```text
GET http://127.0.0.1:9137/api/daemon/broker/dhan/callback?tokenId=…&state=…
```

- Per-slug path, matching the existing per-broker route style
  (`/api/daemon/broker/kotak/session/mint`, ADR 0005
  `/api/daemon/broker/zerodha/callback`). Wave 2 siblings add sibling paths
  reusing the same state-nonce machinery — never one shared path that guesses
  the broker, never a flag.
- `state` is a single-use 128-bit nonce minted host-side by a new
  `POST /api/daemon/broker/dhan/connect/begin` (Z6/Z10 build it), bound to
  `connection_id`, TTL ≈ 10 minutes. The callback accepts only a live,
  unused, matching `state`; anything else is rejected and logged without
  secrets. This is the CSRF + local-process guard: the route carries no Wire
  HMAC (a browser redirect cannot sign), so the nonce is the auth.
- Bind stays `127.0.0.1` (existing invariant); port stays 9137 (existing
  invariant). Consent denial/error from Dhan maps to a surfaced Connect
  failure, never a retry loop, never Kill.
- On success the agent consumes immediately (host-side, §Consequences) and
  answers the browser with a minimal local success page ("return to Station");
  Swift learns the outcome by polling session status, not via URL.

### Why loopback is the right landing point

1. Token travels one hop, browser → agent, and dies there (single-use,
   consumed at once). It never crosses a process boundary into Swift.
2. The exchanger (`app_id`/`app_secret` holder, Keychain writer, Dhan auth
   HTTP dialer) is already the agent. Option B would still need a Swift→agent
   handoff for the consume step, adding a hop that carries the `tokenId`
   through a second process.
3. Loopback redirect is the standard native-app OAuth pattern (RFC 8252
   §7.3); the Dhan consent app registers the exact callback URI, so there is
   no ambiguity about where the `tokenId` goes — pending the row-10 probe
   confirming Dhan accepts an exact loopback URI.
4. Zero new OS surface: no scheme registration, no scheme-collision or
   hijack class, no `Info.plist` / Xcode project change.

## Considered options (rejected alternative)

**Option B — Swift custom URL scheme (rejected).** E.g.
`tradeautopsy-connect://broker/dhan/callback?tokenId=…&state=…`,
registered via `CFBundleURLSchemes` in `station/StationApp/Info.plist`
(+ `project.yml`/`project.pbxproj` propagation), handled in Swift
(`onOpenURL` / `application(_:open:)`) and forwarded to the agent over the
existing signed loopback API for the consent consume step.

Rejected because: (a) the consume must run host-side anyway (ADR 0001 —
Swift never holds `app_secret`, Wasm never sees any of it), so B adds a
secret-bearing Swift hop with no architectural payoff; (b) URL schemes are
OS-global — first-come registration creates a hijack/claim dispute class
that loopback binding does not have; (c) the `tokenId` sits in a URL handled
by app-delegate plumbing, widening lifetime and log-redaction surface versus
an agent route that consumes and drops it in one request; (d) it forks the
Connect flow per transport instead of per family, against program §5.2
(one consent flow, built once).

## Consequences

- **Consent generate/consume runs host-side in `agent/src/ubi`** (program Z6
  names the Dhan session module) — never Swift, never Wasm (ADR 0001).
  Inputs (`app_id`, `app_secret`) are read from the Keychain blob inside the
  agent; generate (`POST /app/generate-consent`) + consume
  (`GET /app/consumeApp-consent?tokenId=` — D1 closed at B6 row 10) run to
  `auth.dhan.co` over host HTTP. `consentAppId` and `tokenId` are
  single-use and never persisted anywhere.
- **Session token lives ONLY in the Keychain tagged blob.** No copy on disk,
  in prefs, in Swift state, or in Wasm memory. Swift holds handles/status,
  never the token.
- **Two clocks, two paths (program §5.3).** Credential TTL (`expiryTime` →
  `session_expired` → guided reconnect, never Kill, never auto-retry storm;
  whether RenewToken extends the session is a B6 row, never assumed) and
  trading session (market-hours/session-validity) are separate code paths
  with separate tests. Expiry-day behavior is a named dogfood drill (Z11).
- **What downstream lanes build from this:**
  - **Z5 (variant/blob):** new `AuthScheme` variant (proposed name
    `DhanConsentSession`), new `CredentialBlob` variant (proposed tag
    `dhan_consent_session`) with fields `appId`, `appSecret` (vault-only —
    never leave the agent), `accessToken`, `dhanClientId`, `expiryTime`
    (ISO-8601 per B6 row — `consentAppId` and `tokenId` are explicitly NOT
    fields); catalog descriptor `Planned` + `manifest_id` + `book_id`.
    Keychain service: default `BROKER_CREDENTIAL_KEYCHAIN_SERVICE` unless Z5
    shows an ACL reason for a dedicated vault (Kotak precedent).
  - **Z6 (signer/session):** `connect/begin` (mint `state` + consent
    generate), callback consume (`GET` per D1), `access-token` header attached on private
    paths only (absent on public; `client-id` added on `POST /marketfeed/*`
    only per D2; `dhanClientId` injected into POST bodies per B6 row 2), expiry → `session_expired` mapping with tests.
  - **Z10 (Swift Connect):** family consent flow built once — Begin →
    open Dhan consent-login URL in browser → poll agent session status →
    Keychain confirmation → per-connection INR strip. No scheme handling, no
    token parsing, no shell fork.
- **Threat notes.**
  - *Token-in-URL lifetime:* seconds — `tokenId` lands, consumes, dropped.
    Never logged (see below), never stored, never forwarded.
  - *Loopback binding:* `127.0.0.1` only, port fixed 9137. Cross-process
    local callers are stopped by the unguessable single-use `state`, not by
    origin checks the agent cannot trust.
  - *Log redaction:* no secrets in logs per `AGENTS.md` invariant 5.
    Callback logs carry status + truncated `state` prefix only;
    `tokenId`, `consentAppId`, `accessToken`, and `app_secret` are redacted
    at the handler boundary, with a test proving it.
  - *`state` hygiene:* 128-bit RNG, single-use (consume-on-read), TTL ≈
    10 min, bound to `connection_id`. Replays and cross-connection reuse
    fail closed.
- **B6 dependencies (CLOSED at sign-off 2026-09-24 — implementer proceeds):**
  - **D1 — consume verb: GET.** Official Step 3 shows bare `curl --location`
    (no `--request` flag); SDK-GET correct, OpenAlgo-POST stale (B6 row 10).
  - **D2 — `client-id` required ONLY on `POST /marketfeed/*`** (official
    header table); day-book reads are `access-token`-only; POST bodies carry
    `dhanClientId` as a field (B6 rows 2/10).
  - **Q-ADR — consent-only v1.** PIN+TOTP direct mint IS documented but
    deferred to its own ADR row if ever wanted (B6 row 16).
  - **Loopback registrability: ACCEPTED ON FAITH.** NOT SPECIFIED in official
    docs (B6 row 10). Dogfood (Z11) proves whether Dhan app settings accept
    the exact `http://127.0.0.1:9137/...` URI. If Dhan refuses, this ADR
    reopens — that is the one fact that would revive Option B.

## Self-check

A Z5/Z6/Z10 implementer can build from this without asking a question: the
landing path, the `state` contract, the generate/consume location, the blob
fields, the header shape, the expiry mapping, and the Swift polling model are
each stated exactly. **Stated: yes.** B6 closed D1/D2/Q-ADR 2026-09-24;
loopback accepted on faith — implementer proceeds, dogfood proves.

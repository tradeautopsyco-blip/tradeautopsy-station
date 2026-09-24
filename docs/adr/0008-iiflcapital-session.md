# ADR 0008: IIFLCapital getusersession session landing point

**Status:** DRAFT (founder accepts later; stays DRAFT until B6 closes Q-IP / D1 / D2 / `/trades` envelope / D3 / loopback registrability — see B6 dependencies)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned 0008 by the full-coverage program — never
another number.

**Program:** `issues/brokers/ALL-BROKERS-PROGRAM.md` Wave 2 (`iiflcapital` book).
B6 sheet (`issues/brokers/sheets/iiflcapital.md`) is owned by the parallel B6 lane;
login-shape details below are provisional oracle leads from
`docs/research/iiflcapital-citation.md` (IIFLCapital Postman collection
`0cf607a` + OpenAlgo `broker/iiflcapital` `ad3cd54`, fetched 2026-09-24) until
that lane cites official docs at `developers.iiflcapital.com` (+
`markets.iiflcapital.com` login portal + `api.iiflcapital.com` reference, URL +
fetch date per row).

## Context

1. **F9: no inbound web callback today.** The native app is a loopback agent
   (Axum, `127.0.0.1:9137`, `AGENT_PORT` fixed per `AGENTS.md`) + Swift shell.
   Per ADR 0005: `agent/src/api/mod.rs` exposes HMAC-wired
   `/api/daemon/…` routes plus unauthenticated read-only `/api/station/…`
   extracts. Nothing receives an inbound browser redirect.
   `station/StationApp/Info.plist` carries no `CFBundleURLSchemes` — no
   custom URL scheme exists today.
2. **IIFLCapital SSO + SHA-256 checksum (provisional, B6 lane to verify):**
   browser SSO — (1) user authorizes in browser at
   `GET https://markets.iiflcapital.com/?v=1&appkey=<appKey>&redirecturl=<url>`
   (trading creds + OTP/TOTP) → `authCode` + `clientId` land at the
   registered redirect as query params (or the IIFL page shows `authCode` if
   no redirect is registered — provisional); (2) host exchanges with
   `POST https://api.iiflcapital.com/v1/getusersession` body
   `{checkSum: SHA256(clientId + authCode + appSecret)}` (plain
   concatenation, hex digest — provisional) → `{status: Ok, userSession:
   <JWT>}` → `Authorization: Bearer <userSession>` on every later call.
   Fresh login mandatory every trading day (provisional); expiry /
   `EC044` "Token is not valid" → `session_expired` → reconnect, never
   Kill. Fills ride `GET /v1/trades` (day; 3 req/s oracle cap); general
   envelope `{status: Ok|Not_Ok, message, result[]}` (provisional — B6 locks
   the `/trades` shape: single-day array vs wrapped). There is exactly ONE
   shape in the oracle — no sibling login to scope out (unlike Dhan).
3. **Law this ADR sits under.** ADR 0001: no secrets in Wasm, all auth attached
   by the host via `broker-http-call`; the checksum exchange is therefore
   host-side by construction. ADR 0005: the Kite redirect pattern (per-slug
   loopback callback + state-nonce machinery) mirrored here in structure;
   ADR 0009: the Dhan consent landing-point depth mirrored here in section
   shape. Program §5.1: IIFLCapital session is its own `AuthScheme` family;
   §5.3: credential TTL and trading session are two clocks, two code paths,
   two tests.

## Decision

**Recommendation: Option A — loopback agent route.** The `authCode` lands
directly on the agent, which already owns the loopback surface, the Keychain,
and all broker HTTP. The browser is pointed at the registered SSO URL; no new
OS-global scheme, no Swift hop for a secret-bearing URL.

### Landing point (exact)

```text
GET http://127.0.0.1:9137/api/daemon/broker/iiflcapital/callback?authCode=…&clientId=…&state=…
```

- Per-slug path, matching the existing per-broker route style
  (`/api/daemon/broker/kotak/session/mint`, ADR 0005
  `/api/daemon/broker/zerodha/callback`, ADR 0009
  `/api/daemon/broker/dhan/callback`). Wave 2 siblings add sibling paths
  reusing the same state-nonce machinery — never one shared path that guesses
  the broker, never a flag.
- `state` is a single-use 128-bit nonce minted host-side by a new
  `POST /api/daemon/broker/iiflcapital/connect/begin` (Z6/Z10 build it),
  bound to `connection_id`, TTL ≈ 10 minutes. The callback accepts only a
  live, unused, matching `state`; anything else is rejected and logged
  without secrets. This is the CSRF + local-process guard: the route carries
  no Wire HMAC (a browser redirect cannot sign), so the nonce is the auth.
- Bind stays `127.0.0.1` (existing invariant); port stays 9137 (existing
  invariant). SSO denial/error from IIFL maps to a surfaced Connect failure,
  never a retry loop, never Kill.
- On success the agent exchanges immediately (host-side, §Consequences) and
  answers the browser with a minimal local success page ("return to Station");
  Swift learns the outcome by polling session status, not via URL.

### Why loopback is the right landing point

1. AuthCode travels one hop, browser → agent, and dies there (single-use,
   exchanged at once via `getusersession`). It never crosses a process
   boundary into Swift.
2. The exchanger (`appSecret` holder, SHA-256 computer, Keychain writer,
   IIFL auth HTTP dialer) is already the agent. Option B would still need a
   Swift→agent handoff for the exchange step, adding a hop that carries the
   `authCode` through a second process.
3. Loopback redirect is the standard native-app OAuth pattern (RFC 8252
   §7.3); the IIFL app registers the exact callback URI, so there is no
   ambiguity about where the `authCode` goes — pending the row-10 probe
   confirming IIFL accepts an exact loopback URI.
4. Zero new OS surface: no scheme registration, no scheme-collision or
   hijack class, no `Info.plist` / Xcode project change.

## Considered options (rejected alternative)

**Option B — Swift custom URL scheme (rejected).** E.g.
`tradeautopsy-connect://broker/iiflcapital/callback?authCode=…&clientId=…&state=…`,
registered via `CFBundleURLSchemes` in `station/StationApp/Info.plist`
(+ `project.yml`/`project.pbxproj` propagation), handled in Swift
(`onOpenURL` / `application(_:open:)`) and forwarded to the agent over the
existing signed loopback API for the checksum exchange.

Rejected because: (a) the exchange must run host-side anyway (ADR 0001 —
Swift never holds `appSecret`, Wasm never sees any of it), so B adds a
secret-bearing Swift hop with no architectural payoff; (b) URL schemes are
OS-global — first-come registration creates a hijack/claim dispute class
that loopback binding does not have; (c) the `authCode` sits in a URL handled
by app-delegate plumbing, widening lifetime and log-redaction surface versus
an agent route that exchanges and drops it in one request; (d) it forks the
Connect flow per transport instead of per family, against program §5.2
(one SSO flow, built once).

## Consequences

- **Checksum exchange runs host-side in `agent/src/ubi`** (program Z6 names
  the IIFLCapital session module) — never Swift, never Wasm (ADR 0001).
  Inputs (`appKey`, `appSecret`) are read from the Keychain blob inside the
  agent; `SHA256(clientId + authCode + appSecret)` is computed in agent
  memory and POSTed to `/v1/getusersession` over host HTTP. `authCode` is
  single-use and never persisted anywhere. (Concatenation order + hex form
  provisional — B6 row 16 confirms at official docs.)
- **Session token lives ONLY in the Keychain tagged blob.** No copy on disk,
  in prefs, in Swift state, or in Wasm memory. Swift holds handles/status,
  never the token.
- **Two clocks, two paths (program §5.3).** Credential TTL (daily expiry /
  `EC044` → `session_expired` → guided reconnect, never Kill, never
  auto-retry storm) and trading session (market-hours/session-validity) are
  separate code paths with separate tests. Expiry-day behavior is a named
  dogfood drill (Z11).
- **What downstream lanes build from this:**
  - **Z5 (variant/blob):** new `AuthScheme` variant (proposed name
    `IiflCapitalSession`), new `CredentialBlob` variant (proposed tag
    `iiflcapital_session`) with fields `appKey`, `appSecret` (vault-only —
    never leave the agent), `userSession` (JWT), `clientId`, `expiry`
    (ISO-8601 per B6 row — `authCode` is explicitly NOT a field); catalog
    descriptor `Planned` + `manifest_id` + `book_id`. Keychain service:
    default `BROKER_CREDENTIAL_KEYCHAIN_SERVICE` unless Z5 shows an ACL
    reason for a dedicated vault (Kotak precedent).
  - **Z6 (signer/session):** `connect/begin` (mint `state` + open SSO URL),
    callback consume + `getusersession` exchange, `Authorization: Bearer
    <userSession>` attached on private paths only (absent on public),
    `EC044`/expiry → `session_expired` mapping with tests (reconnect, never
    Kill).
  - **Z10 (Swift Connect):** family SSO flow built once — Begin → open
    IIFL SSO URL in browser → poll agent session status → Keychain
    confirmation → per-connection INR strip. No scheme handling, no token
    parsing, no shell fork.
- **Threat notes.**
  - *AuthCode-in-URL lifetime:* seconds — `authCode` lands, exchanges,
    dropped. Never logged (see below), never stored, never forwarded.
  - *Loopback binding:* `127.0.0.1` only, port fixed 9137. Cross-process
    local callers are stopped by the unguessable single-use `state`, not by
    origin checks the agent cannot trust.
  - *Log redaction:* no secrets in logs per `AGENTS.md` invariant 5.
    Callback logs carry status + truncated `state` prefix only;
    `authCode`, `clientId`-bearing URLs, `checkSum`, `userSession`,
    `appKey`, and `appSecret` are redacted at the handler boundary, with a
    test proving it.
  - *`state` hygiene:* 128-bit RNG, single-use (consume-on-read), TTL ≈
    10 min, bound to `connection_id`. Replays and cross-connection reuse
    fail closed.
- **B6 dependencies (ALL OPEN — ADR stays DRAFT until closed):**
  - **Q-IP — static-IP whitelist scope (row 1).** Oracle reports app-level
    static-IP whitelisting (caller's public IP must match). B6 must confirm
    whether it is enforced per-call on the *read* paths (`/trades`,
    `/orders`, `/positions`, `/holdings`) or only at app registration. If
    per-call on reads, a dynamic-IP desk cannot read at all — Station needs
    a fixed-IP story or the `iiflcapital` slug parks. Q-IP is the one
    dependency that can park the slug outright.
  - **D1 — redirect-param casing (row 16).** Mirror shows lowercase
    `redirecturl` only; OpenAlgo sends both `redirecturl` + `redirectUrl`
    "for compatibility". B6 cites the official casing; Z6 sends exactly
    that, never both.
  - **D2 — `/limits` endpoints reality (row 2).** `/limits/equity` +
    `/limits/fno` appear ONLY in OpenAlgo `funds.py`, not in Postman or the
    mirror's four User endpoints (getusersession/profile/limits/logout).
    B6 cites the official set; Station must not call unlisted paths.
  - **`/trades` envelope shape (row 4).** OpenAlgo `_extract_rows` accepts
    list-or-dict-with-keys plus bare-list and wrapped payloads. B6 + host
    tests lock which shape `/trades` actually returns (single-day array vs
    wrapped) before the adapter maps a field.
  - **D3 — segment list (row 3).** `NCDEXCOMM`/`BSECOMM` appear in mirror
    enums + OpenAlgo maps but have NO Postman contract file (9 files only).
    B6 cites the official segment list; cash v1 needs only NSEEQ/BSEEQ.
  - **Loopback registrability probe (row 10).** NOT in the oracle either
    way. B6/dogfood (Z11) proves whether IIFL app settings accept the exact
    `http://127.0.0.1:9137/...` URI. If IIFL refuses, this ADR reopens — that
    is the one fact that would revive Option B.

## Self-check

A Z5/Z6/Z10 implementer can build from this without asking a question: the
landing path, the `state` contract, the exchange location, the blob fields,
the header shape, the expiry mapping, and the Swift polling model are each
stated exactly. **Stated: yes, provisionally.** Every wire fact above is an
oracle lead marked provisional — implementer does NOT proceed until B6 closes
Q-IP / D1 / D2 / `/trades` envelope / D3 / loopback registrability and the
founder flips this ADR to ACCEPTED.

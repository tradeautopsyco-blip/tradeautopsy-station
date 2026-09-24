# ADR 0014: Groww env-checksum session landing point

**Status:** ACCEPTED (founder bundle — go ahead 2026-09-24 IST; D1/D2/D4/D6/subscription closed at B6; Q-TTL/D5 partial with fail-closed postures — read per-mint `expiry`, refuse unevidenced product strings)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned 0014 by the full-coverage program — never
another number.

**Program:** `issues/brokers/ALL-BROKERS-PROGRAM.md` Wave 2/3 (`groww` book).
B6 sheet (`issues/brokers/sheets/groww.md`) is owned by the parallel B6 lane;
login-shape details below are verified from official Groww Trade API docs at
`issues/brokers/sheets/groww.md` (SIGNED 2026-09-24; oracle: official PyPI
`growwapi-1.5.0` + OpenAlgo `broker/groww` at `ad3cd54`).

## Context

1. **No browser, no redirect, no loopback in this shape.** Groww shape A —
   API key + secret checksum (verified at B6 rows 10/14):
   host `POST https://api.groww.in/v1/token/api/access` with header
   `Authorization: Bearer {api_key}` + JSON body `{"key_type": "approval",
   "checksum": hex(SHA256(secret + timestamp)), "timestamp": "<epoch-seconds-string>"}` → `{"token", "tokenRefId", "sessionName", "expiry" (ISO), "isActive"}`. The api_key travels
   in the header and is NOT part of the hash; only secret + timestamp are
   hashed (D1 closed: timestamp is a STRING, valid 10 min). There is no consent page, no `tokenId` in a
   URL, no callback URI to register — the 0005/0008/0009 redirect machinery
   (per-slug loopback callback, state-nonce CSRF guard, browser success
   page, Swift poll status) has nothing to land on and does NOT apply here.
   Shape B (TOTP, `key_type: totp` + 6-digit code — provisional) needs
   TOTP-seed handling and is deferred to its own ADR row if ever wanted.
   Shape C (OAuth 2.0 redirect — existence noted in official docs intro,
   unmined) is refused for Station on this lane.
2. **No day trade-book — fills are a per-order fan-out (verified at B6
   rows 2/5).**
   Oracle shows no day-level fills endpoint. The fills pipeline is
   `GET /order/list?segment=&page=&page_size=` → filter to
   executed/completed/filled orders → per-order
   `GET /order/trades/{groww_order_id}?segment=&page=&page_size=` →
   `payload.trade_list[]` (`groww_trade_id`, `exchange_trade_id`,
   `exchange_order_id`, `quantity`, `price`, `trade_status`,
   `trade_date_time`, `settlement_number`). OpenAlgo's FNO-on-404 synthetic
   trades (`synthetic_{orderid}`) are refused — Station never fabricates
   fills; a 404 surfaces as a named gap. Page size ≤25 until proven
   (intra-doc 100-vs-25 conflict, B6 row 2 implements the smaller).
3. **Rate table is per-type, and the mint cap is the binding constraint
   (verified at B6 row 4).** Official type table:
   Auth 5/s + 30/min **+ `/v1/token/api/access` capped at 150/24h**;
   Non-Trading (order status/list, trade list, positions, holdings, margin)
   20/s + 500/min; Orders 10/s + 250/min; Live Data 10/s + 300/min. Limits
   apply **per type, not per API**. OpenAlgo observes a hard 429 lockout
   under load (abort overlay after 4 consecutive 429s) and a ~4 RPS safe
   sustained rate on Live Data — the host poller budgets below the caps,
   never at them. The 150/24h mint cap means reconnect must reuse the live
   token and never re-mint in a loop (see §Decision).
4. **Law this ADR sits under.** ADR 0001: no secrets in Wasm, all auth
   attached by the host via `broker-http-call`; the checksum mint is
   therefore host-side by construction. Program §5.1: Groww checksum
   session is its own `AuthScheme` family; §5.3: credential TTL and trading
   session are two clocks, two code paths, two tests. Session tokens live
   ONLY in Keychain tagged blobs; Swift holds handles/status, never tokens.

## Decision

**Recommendation: Option A — host-side env-checksum mint + key-entry
Connect + per-slug fills fan-out.** The agent mints the session token with
Keychain-held credentials, Bearer-attaches it on private paths, and fans
out fills per order inside an explicit rate budget. No browser opens, no
callback route exists, no state nonce is minted.

### Checksum-mint design (exact)

```text
POST https://api.groww.in/v1/token/api/access
Authorization: Bearer {api_key}
Content-Type: application/json
Accept: application/json
x-api-version: 1.0            # D4 closed: mandatory on every call (B6 row 2)

{"key_type": "approval", "checksum": hex(SHA256(secret + timestamp)), "timestamp": "<epoch-seconds-string>"}
→ {"token", "tokenRefId", "sessionName", "expiry" (ISO), "isActive"}
```

(`x-request-id` is NOT sent — no official page requires it.)

- Mint runs host-side in the agent (ADR 0001) — never Swift, never Wasm.
  `api_key` + `api_secret` are read from the Keychain blob inside the
  agent; the checksum is computed at mint time with a fresh epoch-seconds
  timestamp as a STRING (D1 closed at B6 row 10; the official SDK's int
  diverges from the docs — docs win).
- The response `token` + `expiry` (ISO, per-mint) + `tokenRefId` are
  written straight into the Keychain blob with `mintedAt`; they never
  cross into Swift, prefs, disk, Wasm memory, or a URL. No fixed TTL is
  assumed (Q-TTL partial — Station reads `expiry`). Swift learns connect
  success/failure via session status, never via token.
- **Mint-rate guard (binding constraint):** 150 mints/24h
  means a reconnect storm can brick the connection for a day. Reconnect
  therefore (a) reuses the live token until expiry/invalid is proven
  (never speculative re-mint), (b) serializes mint attempts per
  `connection_id` (single-flight — concurrent triggers coalesce to one
  mint), (c) backs off on mint failure (no tight retry loop), and (d) maps
  mint-429 to a surfaced Connect failure with a human-readable cooldown,
  never Kill. The guard is a named host test (see §Consequences).
- Why the 0005/0008/0009 redirect machinery does NOT apply: there is no
  official redirect flow to land (inventing a callback URI would be
  fiction — see §Considered options), no `tokenId`-in-URL to guard (so no
  state nonce, no CSRF class, no loopback route, no success page), and no
  browser step for Swift to open or poll. The Connect UX is key entry, not
  Begin → browser → poll.

### Fills fan-out budget (explicit)

- Fan-out shape (per-slug, cash v1): day order-list
  (`GET /order/list?segment=CASH`, paged at ≤25 — the smaller of the
  intra-doc 100-vs-25 conflict until dogfood proves more) →
  filter to filled/executed statuses →
  `GET /order/trades/{id}?segment=CASH` per surviving order (paged).
  Status/detail endpoints are NOT in the fills loop (waste of type
  budget); positions/holdings/margins reads share the same Non-Trading
  budget and are sequenced, never parallel-stormed.
- Budget math against Non-Trading 20/s + 500/min (verified B6 row 4): a 200-order
  day with 50% filled = ~100 trades calls + ~8 list pages ≈ 110 calls —
  fits in one minute at ~2/s sustained, 5× under the per-second cap and
  4× under the per-minute cap. A 1000-order day ≈ 550 calls — must be
  paced across ≥2 minutes (host paces at ≤10/s burst, ≤300/min sustained —
  self-imposed headroom below the caps), with 429 → backoff-and-resume,
  never abort-and-fabricate. The host poller owns one Non-Trading token
  bucket per `connection_id`; the fan-out never borrows from the Auth,
  Orders, or Live-Data buckets.
- 404 on a per-order trades call (observed for FNO in the oracle —
  provisional) surfaces as a named per-order gap in the fills result, with
  `order_id` + status preserved. No synthetic trade is ever synthesized.

### Key-entry Connect (Swift shape)

- Swift Connect for Groww = key entry (api key + api secret fields),
  Keychain-stored via the existing signed loopback API, then agent mints
  and Swift shows session status. No browser opens, no URL is parsed, no
  shell forks. Per-connection INR strip applies as usual (Z10).

## Considered options (rejected alternatives)

**Option B — treat Groww as redirect-family (rejected).** I.e. build a
loopback callback or custom URL scheme and route Groww through the
0005/0008/0009 Begin → browser → callback → consume machinery.

Rejected because: (a) no official redirect flow exists for shape A — the
checksum mint is a single stateless POST with no browser step, so a
callback URI would be invented fiction with nothing registered at Groww's
end; (b) shape C (OAuth redirect) is unmined and refused for Station v1 —
adopting it would mean mining an unvetted flow to justify machinery the
vetted flow never needs; (c) it would fork the Connect UX per transport
instead of per family, against program §5.2 (one checksum flow, built
once), while adding a secret-bearing URL hop the checksum shape never
creates.

**Option C — shared-sibling fan-out helper (rejected if it shares
slug/crate/fence).** I.e. one generic order-list → per-order-trades helper
used by Groww and any future per-order-fills sibling.

Rejected in the shared form because: per-slug implementation is the
program invariant — one slug, one crate path, one fence. A shared helper
that carries slug-specific status filters, segment heuristics
(`GMKFO`/`GLTFO` prefix rule is an unverified lead — never shared), page
sizes, or gap semantics would leak one broker's guessed behavior into
another's. If a future sibling needs the same loop shape, it copies the
loop per slug with its own cited constants — never imports Groww's.

## Consequences

- **Checksum mint runs host-side in `agent/src/ubi`** (program Z6 names the
  Groww session module) — never Swift, never Wasm (ADR 0001). Inputs
  (`api_key`, `api_secret`) are read from the Keychain blob inside the
  agent; the mint POST + all Bearer reads run to `api.groww.in` over host
  HTTP. The checksum preimage (`secret + timestamp`) is never logged,
  never persisted, never leaves the mint call.
- **Session token lives ONLY in the Keychain tagged blob.** No copy on disk,
  in prefs, in Swift state, in Wasm memory, or in any URL. Swift holds
  handles/status, never the token.
- **Two clocks, two paths (program §5.3).** Credential TTL (Q-TTL
  partial — no fixed hours in docs; Station reads per-mint `expiry`, no
  refresh endpoint, re-mint is re-POST inside the 150/24h budget;
  expiry/invalid → `session_expired` → guided reconnect reusing the live
  token first,
  never Kill, never auto-retry storm) and trading session
  (market-hours/session-validity) are separate code paths with separate
  tests. Expiry-day behavior is a named dogfood drill (Z11).
- **What downstream lanes build from this:**
  - **Z5 (variant/blob):** new `AuthScheme` variant (proposed name
    `GrowwChecksumSession`), new `CredentialBlob` variant (proposed tag
    `groww_checksum_session`) with fields `apiKey`, `apiSecret`
    (vault-only — never leave the agent), `token`, `expiry` (ISO,
    per-mint), `tokenRefId`, `mintedAt`
    (`checksum` and `timestamp` are explicitly NOT fields — recomputed per mint);
    catalog descriptor `Planned` + `manifest_id` + `book_id`. Keychain
    service: default `BROKER_CREDENTIAL_KEYCHAIN_SERVICE` unless Z5
    shows an ACL reason for a dedicated vault (Kotak precedent).
  - **Z6 (signer/session):** host mint (`POST /v1/token/api/access`,
    `key_type: approval`, STRING timestamp per D1),
    `Authorization: Bearer {token}` + `Accept: application/json` +
    `X-API-VERSION: 1.0` attached on every call (D4: all mandatory; NO
    `x-request-id` — uncited),
    expiry/invalid → `session_expired` mapping with tests,
    mint-rate guard (single-flight per `connection_id` + backoff +
    429-cooldown surfacing) with tests, Non-Trading token bucket +
    paced fan-out with tests.
  - **Z10 (Swift Connect):** key-entry Connect built once for the
    checksum family — key + secret entry → Keychain store via signed
    loopback → agent mint → session status → per-connection INR strip.
    No browser, no URL handling, no token parsing, no shell fork.
- **Threat notes.**
  - *Secret-in-entry handling:* the api secret is typed once into
    key-entry, written to the Keychain blob over the signed loopback
    API, and never retained in Swift state, prefs, or logs. Pasteboard
    is cleared on submit where the platform allows.
  - *No token in URLs at all:* this shape has no callback, no redirect,
    no deep link — no URL anywhere in the flow may carry the checksum,
    the timestamp preimage, the api secret, or the session token. Any
    future URL addition reopens this ADR.
  - *Log redaction:* no secrets in logs per `AGENTS.md` invariant 5.
    Mint/fan-out logs carry status + `connection_id` + counts only;
    `api_secret`, `checksum`, `api_key`, and `token` are redacted at
    the handler boundary, with a test proving it.
  - *Reconnect-storm brick risk:* the 150/24h mint cap
    makes an unbounded reconnect loop a day-long self-lockout. The
    single-flight + backoff + reuse-live-token rules above are the
    mitigation, and the mint-guard test must prove a 10-trigger burst
    produces exactly one mint.
- **B6 dependencies (CLOSED at sign-off 2026-09-24 — implementer proceeds):**
  - **Q-TTL — PARTIAL, fail-closed posture.** Dashboard token "Expires
    daily at 6:00 AM"; minted token carries per-mint ISO `expiry` —
    Station reads the field, assumes no fixed hours. No refresh
    endpoint; re-mint is re-POST inside 150/24h (B6 rows 10/16).
  - **D1 — CLOSED: timestamp is a STRING.** Official table `String`,
    quoted `"1719830400"`, all 4 checksum snippets hash strings; the
    official SDK's int diverges from the docs — docs win. Valid 10 min
    (B6 row 10).
  - **D4 — CLOSED: `X-API-VERSION: 1.0` REQUIRED on every call**
    ("All requests must have following headers", B6 row 2).
    `x-request-id` is NOT sent (uncited anywhere official).
  - **D5 — PARTIAL, refuse-until-seen.** Only `CNC` + `MIS` evidenced
    on reads; `INTRADAY`/`MARGIN` strings NOT SPECIFIED — adapter
    refuses them until dogfood evidences (B6 row 8).
  - **D6 — CLOSED.** Official wire `LIMIT/MARKET/SL`; OpenAlgo
    `STOP_LOSS_*` not official. Execution refused v1 anyway (B6 row 3).
  - **Row 0 — subscription gate CLOSED.** Active Trading API
    subscription required, **₹499 + taxes/month** (official FAQ) —
    dogfood prerequisite, no code consequence.

## Self-check

A Z5/Z6/Z10 implementer can build from this without asking a question: the
mint request shape, the checksum recipe, the key-entry model, the fan-out
loop + budget math, the mint-rate guard, the blob fields, the Bearer
attach rule, the expiry mapping, and the Swift no-browser model are each
stated exactly. **Stated: yes.** B6 closed D1/D2/D4/D6/subscription
2026-09-24; Q-TTL/D5 partial with fail-closed postures — implementer
proceeds, dogfood proves.

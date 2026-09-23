# ADR 0006: Upstox OAuth form POST session (authorization_code)

**Status:** ACCEPTED (founder swarm go-ahead — 2026-09-24 IST)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned at P3 kickoff (`issues/brokers/P3-KICKOFF.md`). Not Kite
checksum (ADR 0005); not a flag on `KiteChecksumSession`.

**Program:** Wave 2 OAuth-form family; first tracer slug `upstox`.

**Official sources (verified 2026-09-24 IST):**

| Topic | URL |
| --- | --- |
| OAuth 2.0 authorization code flow (Steps 1–3) | https://upstox.com/developer/api-documentation/authentication |
| Authorize dialog (`GET /login/authorization/dialog`) | https://upstox.com/developer/api-documentation/authorize |
| Token exchange (`POST /login/authorization/token`) | https://upstox.com/developer/api-documentation/get-token |
| API request auth header | https://upstox.com/developer/api-documentation/request-structure |

## Context

1. **P2 built redirect landing once (ADR 0005).** Loopback
   `127.0.0.1:9137` + host Keychain + browser redirect is the default
   transport for India OAuth brokers unless a B6 proves otherwise.
2. **Upstox shape (verified against official docs):** Standard OAuth 2.0
   authorization code flow. User hits
   `GET https://api.upstox.com/v2/login/authorization/dialog` with
   `response_type=code`, `client_id`, `redirect_uri`, optional `state`.
   Upstox redirects to `redirect_uri` with query `code` (and `state` if sent).
   Host exchanges via **application/x-www-form-urlencoded POST** to
   `https://api.upstox.com/v2/login/authorization/token` with body fields
   `code`, `client_id`, `client_secret`, `redirect_uri`, `grant_type=
   authorization_code` (not JSON; not Kite checksum on `api.kite.trade`).
3. **Law:** ADR 0001 — secrets and token exchange stay host-side; Wasm never
   sees `client_secret`. Program §5.1 — sibling variant, not reuse of
   `KiteChecksumSession`.

## Decision

**Recommendation: Option A — loopback agent route** (same family as ADR 0005).

Official Upstox docs require `redirect_uri` to **exactly match** the URL
registered at app creation and recommend a URI **in your control rather than a
public endpoint** ([Authorize NOTE](https://upstox.com/developer/api-documentation/authorize)).
They document no blanket ban on `http://127.0.0.1` loopback; the published
sample uses HTTPS, but the contract is exact registration match, not a fixed
host class. Register the exact loopback URI below in Upstox Developer Apps
(same console probe discipline as ADR 0005 / Kite).

### AuthScheme

Variant **`UpstoxOAuthFormSession`** (name locked at accept with B6 row 14).

### Credential blob

Keychain-tagged blob, e.g. `upstox_oauth_session`: `client_id`, `client_secret`
(vault-only), `access_token`, `access_token_expiry` (see TTL below), `user_id`
(from token response — official Get Token body includes `user_id`).

### Redirect landing (reuse ADR 0005 pattern)

```text
GET http://127.0.0.1:9137/api/daemon/broker/upstox/callback?code=<auth_code>&state=<nonce>
```

- **Authorize URL (host-built):**
  `https://api.upstox.com/v2/login/authorization/dialog?response_type=code&client_id=<api_key>&redirect_uri=<url-encoded exact callback above>&state=<nonce>`
- `state` nonce: mint at connect begin, validate on callback, single-use (same
  discipline as Zerodha; matches [Authorize NOTE](https://upstox.com/developer/api-documentation/authorize)
  on random `state` validation).
- Host exchanges `code` via **application/x-www-form-urlencoded POST** to
  `https://api.upstox.com/v2/login/authorization/token` with
  `Content-Type: application/x-www-form-urlencoded`, fields:
  `code`, `client_id`, `client_secret`, `redirect_uri`, `grant_type=
  authorization_code` ([Authentication Step 3](https://upstox.com/developer/api-documentation/authentication),
  [Get Token](https://upstox.com/developer/api-documentation/get-token)).
- Authorization code is **single-use** ([Authentication Step 3](https://upstox.com/developer/api-documentation/authentication),
  [Get Token](https://upstox.com/developer/api-documentation/get-token)).

### Access token TTL

Per [Get Token](https://upstox.com/developer/api-documentation/get-token):
`access_token` validity runs until **3:30 AM the next calendar day** (Upstox
trading-day boundary), independent of issue time — e.g. token at 8 PM Tuesday
expires 3:30 AM Wednesday; token at 2:30 AM Wednesday expires 3:30 AM same
Wednesday. Persist `access_token_expiry` as that cutoff (IST), not naive
`expires_in` alone if the response omits it.

### API authorization header

Private Upstox REST calls use:

```text
Authorization: Bearer <access_token>
```

([Request structure](https://upstox.com/developer/api-documentation/request-structure)).
OAuth token response may include `token_type: Bearer` in examples
([Authentication Step 3](https://upstox.com/developer/api-documentation/authentication));
implementations must send Bearer on `api.upstox.com` per request-structure, not
Kite’s `Authorization: token api_key:access_token`.

### Session lifecycle

- Expired/invalid token → reconnect prompt (`session_expired` class), **never
  Kill**.
- Trading-hours clock separate from credential TTL (program §5.3).

### Swift Connect

Reuse **redirect family** UI from Zerodha (browser open → callback → sync);
per-broker differences: redirect URI registered with Upstox app, query param
`code` (not Kite `request_token`), no SHA-256 checksum step in Swift.

## Considered options (rejected alternative)

**Option B — Swift custom URL scheme (rejected unless console rejects loopback).**
E.g. `tradeautopsy-connect://broker/upstox/callback?code=…&state=…`, same
rationale as ADR 0005 Option B: exchange and `client_secret` stay host-side, so
Swift adds a token-bearing hop with no payoff. **Revive Option B only if**
Upstox Developer Apps refuses registration of the exact loopback URI above
(probe failure — same reopen rule as ADR 0005).

## Consequences

- New host module `upstox_session.rs` (or shared `oauth_form_session.rs` only
  if a **later** B6 proves byte-identical exchange with another broker — default
  is per-shape code until proven).
- Allowlist + Kill hosts from B6 row 22 (`api.upstox.com`, siblings refused).
- Slugs sharing this exact exchange may reference ADR 0006; any deviation →
  ADR-lite append in P3-KICKOFF table.

## Verified facts (B6 rows 10, 14–16, 22 — cited)

| Row | Fact | Official citation |
| --- | --- | --- |
| 10 | `Authorization: Bearer <access_token>` on API calls | [request-structure](https://upstox.com/developer/api-documentation/request-structure) |
| 14 | Variant `UpstoxOAuthFormSession` / authorization_code + form token POST | [authentication](https://upstox.com/developer/api-documentation/authentication) |
| 15 | Callback query `code` (+ optional `state`) | [authentication Step 2](https://upstox.com/developer/api-documentation/authentication), [authorize](https://upstox.com/developer/api-documentation/authorize) |
| 16 | `POST https://api.upstox.com/v2/login/authorization/token`, form fields + grant | [get-token](https://upstox.com/developer/api-documentation/get-token) |
| 16 | TTL until 3:30 AM next day | [get-token](https://upstox.com/developer/api-documentation/get-token) |
| 22 | Host allowlist root `api.upstox.com` | All cited endpoints |

## Acceptance (founder)

- [x] Official Upstox developer docs cited in B6 rows 10, 14–16, 22 (table above)
- [x] Loopback redirect registrable per official contract (exact match + in-your-control guidance; console probe still required at bind time)
- [x] Status → **ACCEPTED**; Wave 2 `upstox` bindings may start

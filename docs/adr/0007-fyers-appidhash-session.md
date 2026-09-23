# ADR 0007: Fyers OAuth JSON appIdHash session (validate-authcode)

**Status:** DRAFT — founder accept required before first `fyers` bindings

**Date:** 2026-09-24 IST

**Number note:** pre-assigned at P3 kickoff (`issues/brokers/P3-KICKOFF.md`). Not Kite
checksum (ADR 0005); not Upstox form POST (ADR 0006); not a flag on either.

**Program:** Wave 2 JSON-appIdHash family; second tracer slug `fyers`.

**Official sources (verified 2026-09-24 IST via WebFetch where noted):**

| Topic | URL |
| --- | --- |
| App credentials (`client_id`, `secret_id`, `redirect_uri`) | https://myapi.fyers.in/dashboard/ |
| API v3 documentation portal (User Authentication) | https://api-docs.fyers.in/docsv3/#tag/User-Authentication |
| v3 two-step OAuth (generate-authcode → validate-authcode), appIdHash, headers, TTL | [FyersDev `auth.md`](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) (vendor-published reference; mirrors v3 wire documented on the portal above) |

## Context

1. **P2 built redirect landing once (ADR 0005).** Loopback
   `127.0.0.1:9137` + host Keychain + browser redirect remains the default
   transport unless the Fyers app console rejects the exact loopback URI (same
   reopen rule as ADR 0005 / 0006).
2. **Fyers shape (verified against official v3 auth reference):** OAuth-shaped
   authorization code flow on `https://api-t1.fyers.in`. User opens
   `GET /api/v3/generate-authcode` with `client_id`, `redirect_uri`,
   `response_type=code`, `state`. After login, Fyers redirects to
   `redirect_uri?auth_code=<…>&state=<…>`. Host exchanges via **JSON POST**
   to `POST /api/v3/validate-authcode` with `Content-Type: application/json`
   and body `grant_type=authorization_code`, `appIdHash` (SHA-256 **hex digest**
   of `"<app_id>:<secret_id>"`), and `code` set to the `auth_code` from the
   redirect — **not** Upstox’s form POST with plaintext `client_secret`, **not**
   Kite’s checksum on `api.kite.trade`.
3. **Oracle lead only (not evidence):** OpenAlgo `broker/fyers/api/auth_api.py`
   at pin `ad3cd54` posts JSON to `validate-authcode` with `appIdHash` and names
   the callback value `request_token` in Python — official redirect query is
   `auth_code`; exchange field is `code`. Implementers follow official names.
4. **Law:** ADR 0001 — `secret_id` and exchange stay host-side; Wasm never sees
   `secret_id`. Program §5.1 — sibling variant, not reuse of
   `UpstoxOAuthFormSession` or `ZerodhaKiteChecksumToken`.

## Decision (proposed)

**Recommendation: Option A — loopback agent route** (same family as ADR 0005 /
0006).

Official flow requires `redirect_uri` to match the app registered at
[myapi.fyers.in dashboard](https://myapi.fyers.in/dashboard/). There is no
documented requirement for a public HTTPS landing page; exact registration match
applies (console probe still required at bind time, same discipline as ADR 0005).

### AuthScheme

New variant **`FyersOAuthJsonAppIdHashSession`** (name locked at accept with B6
row 14).

### Credential blob (proposed)

Keychain-tagged blob, e.g. `fyers_oauth_json_session`:

- `app_id` (a.k.a. `client_id`, format `XXXXXXXXX-100`)
- `secret_id` (vault-only; never sent plaintext on the wire)
- `access_token` (JWT string from validate-authcode response)
- `refresh_token` (optional; present in official response — refresh path has
  limited lifetime and a documented caveat that it may be discontinued)
- `access_token_expiry` (persist trading-day cutoff per official TTL guidance —
  daily regeneration; errors `-8`, `-15`, `-16`, `-17` / HTTP 401 → reconnect)

`auth_code` / `code` from redirect are **not** blob fields (single-use, exchanged
and dropped).

### Redirect landing (reuse ADR 0005 loopback; differs from Upstox query name)

```text
GET http://127.0.0.1:9137/api/daemon/broker/fyers/callback?auth_code=<auth_code>&state=<nonce>
```

- **Authorize URL (host-built):**
  `https://api-t1.fyers.in/api/v3/generate-authcode?client_id=<url-encoded app_id>&redirect_uri=<url-encoded exact callback above>&response_type=code&state=<nonce>`
- `state` nonce: mint at `POST …/broker/fyers/connect/begin`, validate on
  callback, single-use (same CSRF discipline as Zerodha / Upstox).
- Host computes `appIdHash = SHA256("<app_id>:<secret_id>")` as lowercase hex
  (official example in vendor `auth.md`).
- Host exchanges via **JSON POST** to
  `https://api-t1.fyers.in/api/v3/validate-authcode`:

  ```json
  {
    "grant_type": "authorization_code",
    "appIdHash": "<sha256_hex>",
    "code": "<auth_code>"
  }
  ```

  No `redirect_uri` or plaintext `secret_id` in this body (contrast ADR 0006).

### API authorization header

Private Fyers REST calls use:

```text
Authorization: <app_id>:<access_token>
```

(e.g. `SPXXXXE7-100:eyJ0eXAi…` per vendor `auth.md`) — **not** Upstox Bearer,
**not** Kite `token api_key:access_token` prefix.

Set a non-default `User-Agent` on host HTTP to `api-t1.fyers.in` if using a
minimal client (vendor docs note bare `Python-urllib` can receive opaque 403).

### Session lifecycle

- Access token **expires end of trading day**; treat daily re-login (Steps 1+2)
  as the reliable path ([vendor `auth.md`](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md)).
- Optional refresh via `POST /api/v3/validate-refresh-token` with `appIdHash`,
  `refresh_token`, `pin` — 15-day refresh validity; verify against live docs
  before depending on it.
- Expired/invalid token → reconnect prompt (`session_expired` class), **never
  Kill**.
- Trading-hours clock separate from credential TTL (program §5.3).

### Swift Connect

Reuse **redirect family** UI (browser open → callback → poll agent session
status); per-broker differences: Fyers app console registers exact loopback URI,
callback query **`auth_code`** (not Upstox `code`, not Kite `request_token`),
host-side `appIdHash` only — no secret or hash in Swift.

## Considered options (rejected alternative)

**Option B — Swift custom URL scheme (rejected unless console rejects loopback).**
Same rationale as ADR 0005 / 0006 Option B: validate-authcode and `secret_id`
stay host-side; Swift would add a code-bearing hop with no payoff. Revive only if
myapi app settings refuse the exact loopback URI.

**Reuse ADR 0006 `UpstoxOAuthFormSession` module (rejected).** Byte-level exchange
differs (see parent task answer); default is `fyers_session.rs` until a later B6
proves identity with another broker.

## Consequences

- New host module `fyers_session.rs` — not `oauth_form_session.rs` shared with
  Upstox unless a **later** B6 proves byte-identical HTTP to ADR 0006 (expected
  **no**).
- Allowlist + Kill hosts from B6 row 22 (`api-t1.fyers.in`, siblings refused).
- Slugs sharing this exact JSON + appIdHash exchange may reference ADR 0007;
  any deviation → ADR-lite append in P3-KICKOFF table.

## Verified facts (B6 rows 10, 14–16, 22 — cited)

| Row | Fact | Official citation |
| --- | --- | --- |
| 10 | `Authorization: <app_id>:<access_token>` on REST calls | [FyersDev `auth.md` — Authorization header](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) |
| 14 | Variant `FyersOAuthJsonAppIdHashSession` / JSON validate-authcode | [FyersDev `auth.md` — Step 2](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md), [api-docs v3 User Authentication](https://api-docs.fyers.in/docsv3/#tag/User-Authentication) |
| 15 | Callback query `auth_code` (+ `state`) | [FyersDev `auth.md` — Step 1](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) |
| 16 | `POST https://api-t1.fyers.in/api/v3/validate-authcode`, JSON `grant_type`, `appIdHash`, `code` | [FyersDev `auth.md` — Step 2](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) |
| 16 | `appIdHash = SHA256("<app_id>:<secret_id>")` hex | [FyersDev `auth.md` — appIdHash](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) |
| 16 | Daily access token expiry (regenerate each trading day) | [FyersDev `auth.md` — intro / errors](https://github.com/FyersDev/fyers-skills/blob/master/skills/fyers-trading/references/auth.md) |
| 22 | Host allowlist root `api-t1.fyers.in` | Step 1–2 URLs in citations above |

## Acceptance (founder)

- [ ] Official Fyers v3 docs cited in B6 rows 10, 14–16, 22 (table above; portal + vendor reference)
- [ ] Loopback redirect registrable on myapi app console (exact match probe at bind time)
- [ ] Status → **ACCEPTED**; then Wave 2 `fyers` bindings may start

## Self-check

Implementer can build from this without guessing: loopback path, `state`
contract, `auth_code` vs exchange `code`, JSON + appIdHash location, blob
fields, Authorization header shape, daily TTL class, and Swift polling model are
each stated. **Stated: yes.**

# ADR 0006: Upstox OAuth form POST session (authorization_code)

**Status:** DRAFT — founder accept required before first `upstox` bindings

**Date:** 2026-09-24 IST

**Number note:** pre-assigned at P3 kickoff (`issues/brokers/P3-KICKOFF.md`). Not Kite
checksum (ADR 0005); not a flag on `KiteChecksumSession`.

**Program:** Wave 2 OAuth-form family; first tracer slug `upstox`.

## Context

1. **P2 built redirect landing once (ADR 0005).** Loopback
   `127.0.0.1:9137` + host Keychain + browser redirect is the default
   transport for India OAuth brokers unless a B6 proves otherwise.
2. **Upstox shape (oracle lead only — B6 must cite official docs):** OpenAlgo
   `broker/upstox/api/auth_api.py` at pin `ad3cd54` shows a **form POST** (not
   JSON checksum) to `https://api.upstox.com/v2/login/authorization/token` with
   `code`, `client_id`, `client_secret`, `redirect_uri`, `grant_type=
   authorization_code`. This differs byte-level from Kite’s checksum exchange
   on `api.kite.trade`.
3. **Law:** ADR 0001 — secrets and token exchange stay host-side; Wasm never
   sees `client_secret`. Program §5.1 — sibling variant, not reuse of
   `KiteChecksumSession`.

## Decision (proposed)

### AuthScheme

New variant **`UpstoxOAuthFormSession`** (name locked at accept with B6 row 14).

### Credential blob (proposed)

Keychain-tagged blob, e.g. `upstox_oauth_session`: `client_id`, `client_secret`
(vault-only), `access_token`, `access_token_expiry` (from official TTL — B6
row 16), optional `user_id` if API returns it.

### Redirect landing (reuse ADR 0005 pattern)

```text
GET http://127.0.0.1:9137/api/daemon/broker/upstox/callback?code=<auth_code>&state=<nonce>
```

- `state` nonce: mint at connect begin, validate on callback, single-use (same
  discipline as Zerodha).
- Host exchanges `code` via **application/x-www-form-urlencoded POST** to the
  official token URL (exact path/version from B6 row 16, not this ADR).
- Authorization on API calls: shape from B6 row 10 (typically Bearer or
  vendor header — **TBD until B6 SIGNED**).

### Session lifecycle

- Expired/invalid token → reconnect prompt (`session_expired` class), **never
  Kill**.
- Trading-hours clock separate from credential TTL (program §5.3).

### Swift Connect

Reuse **redirect family** UI from Zerodha (browser open → callback → sync);
per-broker differences: redirect URI registered with Upstox app, query param
`code` (not Kite `request_token`), no SHA-256 checksum step in Swift.

## Consequences

- New host module `upstox_session.rs` (or shared `oauth_form_session.rs` only
  if a **later** B6 proves byte-identical exchange with another broker — default
  is per-shape code until proven).
- Allowlist + Kill hosts from B6 row 22 (`api.upstox.com`, siblings refused).
- Slugs sharing this exact exchange may reference ADR 0006; any deviation →
  ADR-lite append in P3-KICKOFF table.

## Acceptance (founder)

- [ ] Official Upstox developer docs cited in B6 rows 10, 14–16, 22
- [ ] Loopback redirect registrable on Upstox app console (same probe discipline as ADR 0005)
- [ ] Status → **ACCEPTED**; then Wave 2 `upstox` bindings may start

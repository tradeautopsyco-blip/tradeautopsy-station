# ADR 0018: Coinbase Advanced Trade JWT ES256 session

**Status:** ACCEPTED (founder bundle — P4 Wave 4d `coinbase_advanced` 2026-09-24 IST)

**Date:** 2026-09-24 IST

**Program:** `issues/brokers/P4-KICKOFF.md` · B6 `issues/brokers/sheets/coinbase_advanced.md` SIGNED

## Context

1. **Advanced Trade REST auth is JWT ES256, not HMAC.** CDP API keys use an EC private key (PEM) and API key name. Each private REST call carries a fresh JWT in `Authorization: Bearer …` with a **`uri` claim** formatted `"{METHOD} {host}{path}"` (e.g. `GET api.coinbase.com/api/v3/brokerage/accounts`). Issuer `cdp`, expiry ≤120s, header includes `kid` (API key name) and `nonce` (oracle + official docs — Station re-implements clean-room in `coinbase_session.rs`).
2. **No redirect / OAuth loopback.** Connect is key-entry: API key name + PEM pasted once, stored in Keychain (`coinbase_jwt_es256_session`). ADR 0005 machinery does not apply.
3. **Production host only for dogfood.** Live client targets `api.coinbase.com`. `api-sandbox.coinbase.com` is refused on allowlist, dns_block, and host fence — sandbox is not a G7 shortcut.
4. **Book id locked:** `coinbase-advanced-spot` (lock `issues/compliance/locks/coinbase-advanced-spot.md`).

## Decision

**Option A — host-side per-request JWT + key-entry Connect + Wasm fills adapter.**

- `CredentialBlob::CoinbaseJwtEs256Session { api_key, pem_private_key }` — PEM never leaves vault; never copied to Wasm.
- `HostCredentialBlob::CoinbaseJwt` — `prepare_request` calls `coinbase_session::prepare_coinbase_authenticated_request`.
- `AuthScheme::CoinbaseJwtEs256Session` (catalog integrator adds enum variant).
- Fills v1: Wasm `ubi-coinbase-advanced-adapter` → `GET /api/v3/brokerage/orders/historical/fills` only through host import.

## Consequences

- New signing dependency: `p256` + `pem` in agent (host only).
- Every private GET pays JWT mint CPU — acceptable vs 120s reuse complexity.
- Integrator wave still batches `catalog.rs`, `components.rs`, Swift Connect UI.

## References

- B6 rows 10/16/22 — `issues/brokers/sheets/coinbase_advanced.md`
- Station REST snapshot — `docs/reference/crypto/coinbase-advanced/spot/REST.md`
- Oracle shape — `docs/research/nautilus-trader-citation.md` (Coinbase ES256 JWT)

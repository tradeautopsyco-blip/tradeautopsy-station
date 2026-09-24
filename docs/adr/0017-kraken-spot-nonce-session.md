# ADR 0017: Kraken spot nonce + HMAC-SHA512 session (API-key only)

**Status:** ACCEPTED (2026-09-24 IST)

**Date:** 2026-09-24 IST

**Number note:** pre-assigned 0017 by the full-coverage program — never another number.

## Context

1. **No OAuth, no browser, no loopback.** Kraken spot private REST uses API key + **base64 secret** with `API-Key` / `API-Sign` headers and a monotonic **`nonce`** in the POST body (application/x-www-form-urlencoded).
2. **Distinct from Binance and Bybit.** Binance signs sorted query + `signature=`; Bybit signs header HMAC over v5 payload. Kraken signs `path + SHA256(nonce + POST data)` with **HMAC-SHA512** over the **decoded** secret, `API-Sign` = Base64(signature).
3. **Production spot host only.** B6 refuses **`futures.kraken.com`** and **`demo-futures.kraken.com`** on book **`kraken-com-spot`**. Spot has no documented testnet.
4. **ADR 0001.** Wasm never holds secrets; host attaches auth in `prepare_request` / `broker_http_call`.
5. **Private reads use POST.** `TradesHistory` and `Balance` are POST — not mutations; host policy must infer `fills` / `funds` without treating them as order execution.

## Decision

**Option A — reuse HMAC vault + host Kraken spot signer in `kraken_session.rs`.**

- Module: `agent/src/ubi/kraken_session.rs` — constants, nonce clock, `sign_spot_post`, `prepare_kraken_signed_post`, path allowlist helpers.
- `http.rs`: when `HostCredentialBlob::Hmac` and effective host is `api.kraken.com`, delegate to Kraken signing (before Binance query signing).
- **Skip** `agent/src/api/kraken_session.rs` — no OAuth routes.
- Book id: **`kraken-com-spot`**. Slug: **`kraken`**.

### Signing (POST)

```text
post_data = "nonce=" + nonce + ("&" + extra_params if any)
digest = SHA256(nonce_string + post_data)   // nonce string prepended to post_data per Kraken spec
message = path_bytes + digest_bytes
API-Sign = Base64(HMAC-SHA512(base64_decode(api_secret), message))
Headers: API-Key, API-Sign
Body: post_data (application/x-www-form-urlencoded)
```

Official: https://docs.kraken.com/exchange/guides/rest/authentication

### Nonce

Monotonic u64: nanosecond timestamp with in-process atomic increment when collisions occur (oracle lead: `kraken/src/common/credential.rs` + spot client mutex — Station re-implements shape only).

### Fills pipeline (v1)

`POST /0/private/TradesHistory` with `type=trade`, optional `start`/`end` (seconds), paginate `ofs`. Map `result.trades` → FillEvent; apply B6 quote-filter (USDT/USDC/USD/EUR).

## Consequences

- Integrator later: catalog/components, egress meter, Kill DNS for `api.kraken.com`, permission probes.
- Binance / Bybit signing paths unchanged — host must branch on host header, not slug alone.

# ADR 0015: Bybit v5 header-HMAC session (API-key only)

**Status:** ACCEPTED (2026-09-24 IST)

**Date:** 2026-09-24 IST

## Context

1. **No OAuth, no browser, no loopback.** Bybit v5 private REST uses API key + secret HMAC in headers (`X-BAPI-*`). There is no redirect consent flow for Station Connect on this lane.
2. **Same vault shape as Binance COM.** Keys land in existing `CredentialBlob::HmacApiKeySecret` / Keychain HMAC vault. Signing differs from Binance query `signature=` — host must branch on prod host `api.bybit.com`.
3. **Production host only.** B6 refuses `api-testnet.bybit.com` and `api-demo.bybit.com` for live Station.
4. **ADR 0001.** Wasm component never holds secrets; host attaches auth in `broker_http_call` / `prepare_request`.

## Decision

**Option A — reuse HMAC vault + host Bybit v5 header signing in `bybit_session.rs`.**

- Module: `agent/src/ubi/bybit_session.rs` — constants, `prepare_bybit_signed_get`, path allowlist helpers.
- `http.rs`: when `HostCredentialBlob::Hmac` and effective host is `api.bybit.com`, delegate to Bybit signing (minimal touch).
- **Skip** `agent/src/api/bybit_session.rs` — no OAuth routes.
- Book id: **`bybit-com-spot`**. Slug: **`bybit`**.

### Signing (GET)

```text
sign payload = timestamp + api_key + recv_window + queryString
X-BAPI-SIGN = HMAC-SHA256_hex(api_secret, payload)
Headers: X-BAPI-API-KEY, X-BAPI-TIMESTAMP (ms), X-BAPI-RECV-WINDOW (5000)
```

Official rule: https://bybit-exchange.github.io/docs/v5/guide (Create A Request).

### Fills pipeline (v1)

`GET /v5/execution/list?category=spot&symbol=…` with cursor pagination; optional bootstrap via `GET /v5/account/wallet-balance?accountType=UNIFIED`.

## Consequences

- Integrator later: catalog/components, egress meter, Kill DNS, `query-api` permission gate.
- Binance HMAC path unchanged (host must not apply Bybit headers on `api.binance.com`).

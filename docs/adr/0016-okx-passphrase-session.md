# ADR 0016: OKX global passphrase HMAC session

**Status:** ACCEPTED (Phase 4 Wave 4b · `okx_com` · 2026-09-24 IST)

**Date:** 2026-09-24 IST

**Program:** `issues/brokers/P4-KICKOFF.md` · B6 `issues/brokers/sheets/okx_com.md` (SIGNED same date)

## Context

1. **Global host only.** Live REST is `https://www.okx.com` (canonical `www` — official guidance to avoid redirect stripping auth headers). **US** (`us.okx.com`) and **EEA** (`eea.okx.com`) are sibling venues — refused on the first global spot book. **Sandbox/demo** uses header `x-simulated-trading: 1` on the same host — Station **never** sends that header on live Connect or sync (B6 row 23).
2. **Third credential field.** OKX REST auth is HMAC-SHA256 over `timestamp + METHOD + requestPath + body`, Base64-encoded, with headers `OK-ACCESS-KEY`, `OK-ACCESS-SIGN`, `OK-ACCESS-TIMESTAMP`, `OK-ACCESS-PASSPHRASE`. The **passphrase** is set when the API key is created — it is not optional (contrast Binance two-field HMAC).
3. **ADR 0001.** Secrets stay in the host vault; Wasm components call `broker_http_call` with unsigned shape only. Connect is key entry (api key + secret + passphrase) → tagged Keychain blob → host attaches `OK-ACCESS-*` on private reads.

## Decision

**Option A — host-side OK-ACCESS signing + `OkxPassphraseSession` credential blob.**

### Signing (exact)

```text
OK-ACCESS-TIMESTAMP: ISO-8601 UTC with milliseconds, e.g. 2020-12-08T09:08:57.715Z
preimage = timestamp + METHOD + requestPath + body
OK-ACCESS-SIGN = Base64(HMAC-SHA256(apiSecret, preimage))
```

- `requestPath` includes the query string for GET (e.g. `/api/v5/account/balance?ccy=BTC`).
- GET body is empty string in the preimage.
- Implementation: `agent/src/ubi/okx_session.rs` (sign helpers) + `agent/src/ubi/http.rs` (`HostCredentialBlob::OkxSession` attach).

### Credential blob

Tagged Keychain JSON `authScheme: okx_passphrase_session`:

- `apiKey`, `apiSecret`, `passphrase` (all three required).

Connect validation: host `GET /api/v5/account/balance` on `www.okx.com` with signed headers; `code == "0"` required before save.

### Fills v1

Wasm adapter `ubi-okx-adapter`: `GET /api/v5/trade/fills?instType=SPOT` (+ optional `instId`, `after`, `limit` ≤ 100). Retention **last 3 days** on this endpoint; extended history is a separate endpoint (`fills-history`) — not v1 sync unless B6 is amended.

## Consequences

- New `CredentialBlob::OkxPassphraseSession` + `HostCredentialBlob::OkxSession`.
- Allowlist + book fence `okx-com-spot` on `www.okx.com` only; Kill DNS `hosts_for_broker("okx_com")` → `www.okx.com`.
- Catalog / `components.rs` wiring deferred to integrator wave P4-W1 (per-slug lane ships adapter + ADR first).
- Official doc test vector for GET balance pinned in `okx_session` unit tests.

## Sources

- https://www.okx.com/docs-v5/en/#overview-rest-api-authentication-signature (fetched 2026-09-24 IST)
- https://www.okx.com/docs-v5/en/#order-book-trading-trade-get-transaction-details-last-3-days (fetched 2026-09-24 IST)

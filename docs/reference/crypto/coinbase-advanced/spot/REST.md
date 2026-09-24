# Coinbase Advanced Trade — Spot REST (private reads v1)

**Venue:** Coinbase Advanced Trade  
**Host:** `https://api.coinbase.com` — **not** `api-sandbox.coinbase.com`  
**Prefix:** `/api/v3/brokerage`  
**Snapshot date:** 2026-09-24 IST  
**Sources:** [CDP Advanced Trade API](https://docs.cdp.coinbase.com/api-reference/advanced-trade-api/rest-api/) · [API key authentication (JWT)](https://docs.cdp.coinbase.com/coinbase-app/authentication-authorization/api-key-authentication)

---

## Authentication

- **Scheme:** JWT **ES256** signed with CDP EC private key (PEM SEC1 or PKCS#8).
- **Header:** `alg: ES256`, `typ: JWT`, `kid: {api_key_name}`, `nonce: {random hex}`.
- **Payload:** `sub` = API key name, `iss: cdp`, `nbf`, `exp` (≤120s from `nbf`), **`uri`** = `"{METHOD} {host}{path}"` (no `https://`).
- **Request:** `Authorization: Bearer {jwt}`, `Accept: application/json`.

Host implementation: `agent/src/ubi/coinbase_session.rs` (ADR 0018).

---

## Fills (v1 sync)

`GET /api/v3/brokerage/orders/historical/fills`

| Query | Role |
|-------|------|
| `product_ids` | Repeat for each product (singular keys may be ignored server-side) |
| `order_ids` | Optional filter |
| `start_sequence_timestamp` / `end_sequence_timestamp` | ISO range |
| `limit` | Page size |
| `cursor` | Pagination |

Response: `{ "fills": [ … ], "cursor": "…" }`

Fill fields used by adapter: `trade_id`, `product_id`, `side`, `price`, `size`, `commission`, `trade_time`, `order_id`.

---

## Accounts (bootstrap)

`GET /api/v3/brokerage/accounts` — optional balance/bootstrap (paginated with `cursor` / `limit`).

---

## Refuse (v1)

- Order placement / cancel / edit under `/api/v3/brokerage/orders`
- Sandbox host for live dogfood
- WS-only sync paths (documented separately; not v1)

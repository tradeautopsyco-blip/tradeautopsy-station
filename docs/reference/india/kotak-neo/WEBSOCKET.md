# Kotak Neo Trade API — quotes subscribe (WebSocket)

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo live-feed subscribe (`client.subscribe`) for later s1k |
| **Primary source** | [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) `docs/webSocket.md`, `NeoWebSocket.py`, `urls.py` |
| **Snapshot date** | 2026-08-26 |
| **Source version** | Package v2.0.0 / git `main` @ `8cee5bda63bd9334f8501bb23b7f1d2945f93397` |
| **Staleness warning** | Re-verify `WEBSOCKET_URL` and connect payload before any live stream fetch. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> - How `server_id` (`hsServerId`) is applied on the HSM quotes socket after `NeoWebSocket.__init__` stores it: **NOT SPECIFIED IN SOURCE** (assigned, not referenced again in `NeoWebSocket.py` as of this snapshot).
> - Numeric subscribe rate / token limits: `token_limit_reached` exists in code; official counts **NOT SPECIFIED IN SOURCE**.
>
> This file does **not** authorize TickBook, Notch, or live Rust fetch.

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| `docs/webSocket.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/webSocket.md | 2026-08-26 | `subscribe` / `un_subscribe`; field tables |
| README subscribe section | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/README.md | 2026-08-26 | Requires TOTP login + validate in the sample |
| `neo_api.py` `subscribe` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/neo_api.py | 2026-08-26 | Requires `edit_token` and `edit_sid` |
| `NeoWebSocket.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/NeoWebSocket.py | 2026-08-26 | Connect URL + `cn` payload |
| `urls.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/urls.py | 2026-08-26 | `WEBSOCKET_URL` |
| REST quotes | [REST.md](./REST.md) | 2026-08-26 | REST `quote_type` is a different transport |

---

## Concepts

---

### Subscribe is session attach

**Source:** `NeoAPI.subscribe`; `NeoWebSocket.start_websocket` / `on_hsm_open`; `urls.py` `WEBSOCKET_URL`

**Verbatim definition / formula:**

`neo_api.py` `subscribe` (git `8cee5bda`):

```
if self.configuration.edit_token and self.configuration.edit_sid:
    NeoWebSocket(edit_sid, edit_token, serverId, data_center=None)
    get_live_feed(...)
else:
    print("Please complete the Login Flow to Subscribe the Scrips")
```

`NeoWebSocket.start_websocket`:

```
open_connection(neo_api_client.WEBSOCKET_URL, self.access_token, self.sid, ...)
```

`urls.py`:

```
WEBSOCKET_URL = "wss://mlhsm.kotaksecurities.com"
```

`on_hsm_open` connect payload:

```
{"type": "cn", "Authorization": self.access_token, "Sid": self.sid}
```

`webSocket.md` parameters: `instrument_tokens` list of `{instrument_token, exchange_segment}`; `isDepth` / `isIndex` booleans. Exchange segments listed: `nse_cm`, `bse_cm`, `nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo`, Index.

Stock live fields include `ltp`, `op` (Open Price), `h` (High Price), `lo` (Low Price), `c` (Closing Price) — current feed fields, not a history API.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Quotes WS host | `wss://mlhsm.kotaksecurities.com` | URL |
| `Authorization` | `edit_token` (constructor `token`) | connect JSON |
| `Sid` | `edit_sid` | connect JSON |
| `serverId` | Passed into constructor from validate `hsServerId` | stored; further use **NOT SPECIFIED IN SOURCE** on this socket |
| `isDepth` | Subscribe market depth when True | bool |
| `isIndex` | Subscribe index when True | bool |

**Gaps (NOT SPECIFIED IN SOURCE):**

- TLS / path suffix on `mlhsm` beyond the constant string.
- Whether HTTP host mediation allowlists apply to `wss://` (Slice A HTTP list today does not include `mlhsm.kotaksecurities.com`).
- Shared rate envelope with REST quotes or trade book.

> **OUR INTERPRETATION**
>
> - Subscribe is **PrivateRead session attach** (token + sid), not unsigned public, not HMAC.
> - Intended for a **later s1k** stream. v1 poll remains `GET …/quick/user/trades`.
> - Live `ltp` / session OHLC fields are **not** `historical_series` (FAQ: historical market data unavailable — see REST.md).
> - Cash v1: subscribe only `nse_cm` / `bse_cm`.

---

### Order-feed WebSocket is a different URL family

**Source:** `urls.py` `ORDER_FEED_URL*` (`wss://mis.kotaksecurities.com/realtime` and ADC/E21/… variants); `subscribe_to_orderfeed`

Not a market-quote catalog. Listed so Slice A does not confuse order-feed hosts with `mlhsm`.

---

## Documented paths (this snapshot)

| Capability | Method | Path / URL | Auth in SDK | Notes |
| ---------- | ------ | ---------- | ----------- | ----- |
| Quotes HSM | WebSocket connect | `wss://mlhsm.kotaksecurities.com` | JSON `Authorization` + `Sid` after session | `urls.py` + `NeoWebSocket.py` |
| REST quotes | see [REST.md](./REST.md) | GET neosymbol path | consumer key | Different transport |

---

## Verification Checklist

- [ ] Live HSM connect to `wss://mlhsm.kotaksecurities.com` with founder `edit_token` / `edit_sid`
- [ ] Confirm whether `hsServerId` must appear in the URL or payload
- [ ] Confirm whether Station HTTP allowlist must add `mlhsm.kotaksecurities.com` for WS

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Treat subscribe as public market data | `subscribe` requires `edit_token` and `edit_sid` | Session attach |
| Use `server_id` as a documented URL host | Constructor stores it; HSM `open_connection` uses `WEBSOCKET_URL` + token + sid | Left unspecified |
| Claim candle history from `op`/`h`/`lo`/`c` | webSocket.md: live stock fields; FAQ: historical unavailable | Latest-state / session bar only |

No memory fills for the HSM URL. Gaps stay unspecified.

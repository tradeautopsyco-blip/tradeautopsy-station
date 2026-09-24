# Citation pin — `growwapi` official PyPI SDK + OpenAlgo `broker/groww` (Groww oracle)

**Fetched:** 2026-09-24 IST
**Clone:** `/Users/bishnu/oracles/groww-pypi-2026-09-24` (PyPI sdist mirror, NOT a git clone)
**Remote:** https://pypi.org/project/growwapi/ (no official GitHub repo found 2026-09-24 — see note N1)
**Version:** `growwapi-1.5.0.tar.gz`, sha256 `546be8517f1223dda1f5338a549b3499c2b7cabd1e165b64fef21ab386136b66`
**Licence:** MIT (`LICENSE`, "Copyright (c) 2025 Groww (India)") — cite, do **not** vendor into Station, do **not** add to Station `Cargo.toml`.

**Second oracle (already pinned):** OpenAlgo `broker/groww` at `ad3cd54df476b330c4e4b01a31a3ad53deb9012b` (2026-09-19; `git status --porcelain -- broker/groww/` clean 2026-09-24, no dirty files — cite directly, no `git show HEAD:` needed). AGPL — leads only, see `openalgo-citation.md`.

**N1 (no official GitHub SDK repo):** WebSearch 2026-09-24 finds the official org `Groww-OSS` (55 repos, SDK not among listed top repos), the official docs at https://groww.in/trade-api/docs, and the official PyPI package `growwapi` (Author: Groww, `growwapi@groww.in`, MIT) — but **no official GitHub repo for the Python SDK**. Third-party wrappers exist (`NithinSGowda/growwapi` Node, `rctrj/growwapi-go`, `gopalindians/groww-php-sdk`) and are **not** oracles. Pin here = PyPI version + sdist sha256. If Groww later publishes an SDK repo, B6 re-pins.

Station owns the wire. These trees are the **login-shape / host / path / field-map oracle** for the `groww` book. Every signal below is a *lead* ("SDK `client.py` shows a checksum mint — verify at <official URL>"), never evidence. A B6 row with only an oracle citation stays `TBD`.

## Hosts

| Use | SDK const | URL |
|-----|-----------|-----|
| Token mint + all REST v1 | `GrowwAPI.domain` (`groww/client.py:127`); OpenAlgo `GROWW_BASE_URL` (`broker/groww/api/order_api.py:55`, `margin_api.py:10`, `data.py:54`, `auth_api.py:58`) | `https://api.groww.in` + `/v1` |
| Master instrument CSV | `GrowwAPI.INSTRUMENT_CSV_URL` (`groww/client.py:115`); OpenAlgo `master_contract_db.py:402` | `https://growwapi-assets.groww.in/instruments/instrument.csv` |
| Socket token mint (feed — refuse v1) | `GrowwAPI._GROWW_GENERATE_SOCKET_TOKEN_URL` (`groww/client.py:116-117`); OpenAlgo `streaming/nats_websocket.py:77` | `https://api.groww.in/v1/api/apex/v1/socket/token/create/` |
| Websocket feed (refuse v1) | OpenAlgo `streaming/nats_websocket.py:76` (NATS-over-WS shape) | `wss://socket-api.groww.in` |

Official docs roots (B6 sources): https://groww.in/trade-api/docs (SDK), https://groww.in/trade-api/docs/curl (REST), API keys at https://groww.in/trade-api/api-keys (lead from third-party readme — verify). Subscription gate: docs page says Trading API subscription ₹499 + taxes/month is required — B6 row 0 must confirm (proof-of-access prerequisite, like Dhan consent-app issuance).

## Auth shapes (THREE in official docs — B6 + ADR 0014 must pick Station's one)

| # | Shape | Oracle location | Wire |
|---|-------|-----------------|------|
| A | **API key + secret checksum (ADR 0014 program lead — env-only, NO browser)** | SDK `_build_request_data` + `get_access_token` (`groww/client.py:1514-1581`); OpenAlgo `broker/groww/api/auth_api.py:11-113` | `POST https://api.groww.in/v1/token/api/access` header `Authorization: Bearer {api_key}` + JSON `{"key_type": "approval", "checksum": SHA256(secret + timestamp), "timestamp": <epoch-seconds-now>}` → `{"token": <access_token>}`. `authenticate_broker(code)` ignores `code`; creds from `BROKER_API_KEY` / `BROKER_API_SECRET` env. |
| B | **TOTP (no browser, needs TOTP seed)** | SDK same fns (`"key_type": "totp", "totp": <code>`); docs "3rd Approach: TOTP Flow" | Same endpoint, body `{"key_type": "totp", "totp": <6-digit>}`. SDK docstring: "If TOTP api key is provided" — suggests the *key type* is chosen at key issuance (lead — B6 confirms whether approval-keys and totp-keys are distinct issuances). |
| C | **OAuth 2.0 (redirect — refused for Station 0014)** | Docs python-sdk intro: "Use industry-standard OAuth 2.0"; curl docs imply a 1st approach before key/secret (2nd) and TOTP (3rd) | Unmined (redirect flow — Station refuses browser flows on this lane; B6 records existence only). |

**Checksum recipe (exact, both oracles agree):** `hex(SHA256(secret + str(epoch_seconds_now)))`. The api_key travels in the `Authorization` header and is **not** part of the hash. Only two fields are hashed: secret, timestamp.

**Discrepancy D1 (must resolve at official docs):** timestamp JSON type — OpenAlgo sends a **string** (`auth_api.py:43` `timestamp = str(int(time.time()))`); official SDK sends an **int** (`client.py:1535-1537` `timestamp = int(time.time())`). B6 row cites the official type; host signer implements exactly that.

**Question Q-TTL (for B6, biggest auth unknown):** token TTL / expiry / refresh is stated **nowhere** in the SDK. Third-party mirror says "daily Bearer access token" (lead only, not a source). B6 must cite TTL + whether re-mint is just re-POST (checksum flow is stateless, so likely yes) and whether concurrent tokens invalidate each other.

**Question Q-ADR (for ADR 0014):** does Station ship shape A only, or also B? Default: A first (matches program Wave 2/3 lead); B needs TOTP-seed handling (Keychain/UX consequences like Dhan shape B). Shape C is refused — Swift Connect = key-entry, never browser.

## Read paths (base `https://api.groww.in/v1`, leads for B6 row 2)

| Station use | SDK call | Verb + path |
|-------------|----------|-------------|
| Order book (day, all incl. open/pending/executed) | `get_order_list` (`client.py:554-587`); OpenAlgo `direct_get_order_book` (`order_api.py:63-150`) | `GET /order/list?segment=&page=&page_size=` (SDK default page 0/size 25; OpenAlgo polls CASH+FNO, size 25 = "Maximum allowed by Groww API" — verify) |
| **Trade book (fills — per-order only, NO day endpoint)** | `get_trade_list_for_order` (`client.py:709-745`); OpenAlgo `get_order_trades` (`order_api.py:3176-3358`) | `GET /order/trades/{groww_order_id}?segment=&page=&page_size=` → `payload.trade_list[]` (`groww_trade_id`, `exchange_trade_id`, `exchange_order_id`, `quantity`, `price`, `trade_status`, `trade_date_time`, `settlement_number`) |
| Order status / detail | `get_order_status` (`:589`), `get_order_detail` (`:522`), `get_order_status_by_reference` (`:621`) | `GET /order/status/{id}`, `/order/detail/{id}`, `/order/status/reference/{ref}` |
| Positions | `get_positions_for_user` (`:682`), `get_position_for_trading_symbol` (`:653`); OpenAlgo `get_positions` (`order_api.py:835-858`) | `GET /positions/user`, `/positions/trading-symbol` |
| Holdings | `get_holdings_for_user` (`:347-360`); OpenAlgo `get_holdings` (`order_api.py:2111-2139`, the effective def) | `GET /holdings/user` (see D2) |
| Funds / margins | `get_available_margin_details` (`:746-763`); OpenAlgo `funds.py:get_margin_data` | `GET /margins/detail/user` → `payload{clear_cash, collateral_available, net_margin_used, brokerage_and_charges, adhoc_margin, equity_margin_details{cnc/mis_balance_available}, fno_margin_details{...}}` |
| Order-margin check (read-only lead) | `get_order_margin_details` (`:998`); OpenAlgo `margin_api.py` (`POST`, `?segment=`, `X-API-VERSION: 1.0`) | `POST /margins/detail/orders?segment=` (CASH = single position only per OpenAlgo; FNO basket — verify) |
| User profile (auth-probe candidate) | `get_user_profile` (`:1331`) | `GET /user/detail` |
| LTP / quote / OHLC | `get_ltp` (`:402`), `get_quote` (`:366`), `get_ohlc` (`:433`); OpenAlgo `data.py:1200/1720/2043` | `GET /live-data/ltp`, `/live-data/quote`, `/live-data/ohlc` |
| Historical candles | `get_historical_candles` (`:875-903`; `get_historical_candle_data` `:825` deprecated, use Backtesting docs) | `GET /historical/candles`, `/historical/candle/range` (deprecated), `/historical/expiries`, `/historical/contracts` |
| Option chain / greeks (refuse v1) | `get_option_chain` (`:490`), `get_greeks` (`:463`) | `/option-chain/...`, `/live-data/greeks/...` |

**D3 — fills architecture (Station consequence, not just a citation):** there is no day trade-book endpoint. OpenAlgo `get_trade_book` (`order_api.py:436+`) lists orders then calls per-order `/order/trades/{id}` for EXECUTED/COMPLETED/FILLED orders (statuses + `filled_quantity > 0` heuristic), and **synthesises fake trades for FNO on 404** (`:3360-3389`, `trade_id: synthetic_{orderid}` — Station must never do this; B6 records the FNO-404 behavior as a gap). Station fills pipeline = order-list → filter → per-order trades fan-out. Rate budget (Non-Trading 20/s, 500/min) must cover the fan-out — ADR 0014 sizes this.

**Discrepancy D2 (resolved in-oracle, B6 confirms):** OpenAlgo defines `get_holdings` **twice** — `/v1/portfolio/holdings` (`order_api.py:1316`, shadowed dead code) and `/v1/holdings/user` (`:2137`, effective — Python later-def wins). Official SDK = `/holdings/user` (`client.py:360`). Effective oracle matches official; B6 cites `/holdings/user` and notes the dead def as oracle staleness.

## JSON / error aliases (leads for adapter + host error map)

- Success envelope (SDK `_parse_response`, `client.py:1485-1512`): HTTP 2xx → `payload` if present else raw map. OpenAlgo checks `status == "SUCCESS"` + `payload.order_list` / `payload.trade_list` (`order_api.py:114-121, :3320-3328`).
- Failure shape (SDK): `{error: {code, message}}`, with `displayMessage` on 400s (`client.py:1500-1501, :1558-1571`). Status→exception map (`client.py:32-39`): 400 BadRequest, 401 Authentication, 403 Authorisation, 404 NotFound, **429 RateLimit**, 504 Timeout.
- Required headers (SDK `_build_headers`, `client.py:1363-1378`): `Authorization: Bearer {token}`, `Content-Type: application/json`, **`x-api-version: 1.0`**, `x-request-id` (uuid), `x-client-id/platform/version`. OpenAlgo sends only Bearer + Accept (+Content-Type) on reads — **D4: B6 cites whether `x-api-version: 1.0` is required** (docs curl example includes it).
- Official rate table (https://groww.in/trade-api/docs/python-sdk intro, fetch 2026-09-24): Auth 5/s + 30/min **+ `/v1/token/api/access` capped at 150/24h**; Orders (create/modify/cancel) 10/s + 250/min; Live Data 10/s + 300/min; **Non-Trading (order status/list, trade list, positions, holdings, margin) 20/s + 500/min**. Limits apply **per type, not per API**. Live feed ≤1000 subscriptions.
- OpenAlgo's observed behavior (`data.py:1881-1902, :2228-2415`): batch 50 instruments, 0.2s batch delay, **250ms gap (~4 RPS) "observed sustained safe rate on Groww Live Data"**, abort overlay after **4 consecutive 429s** — "hard 429 lockout under load". Host poller respects the type-level budget; B6 row 4 + host tests lock the real set.
- Order-ID prefixes as segment heuristic (OpenAlgo `order_api.py:3227-3228`): `GMKFO`/`GLTFO` ⇒ FNO. Lead only — B6 cites whether the prefix rule is documented or whether segment must come from the order record.

## Vocab as BAR (check the B6 against it — never a source)

Exchanges (SDK `GrowwAPI.EXCHANGE_*`, `client.py:49-54` + OpenAlgo `map_exchange_type`):

| Station coalition | Venue code |
|---|---|
| NSE cash | `NSE` |
| BSE cash | `BSE` |
| (refuse v1) | `MCX`, `MCXSX`, `NCDEX`, `US` (SDK constants exist but docs say CASH+FNO only — "Commodities trading (MCX segment) is not available at this time"); OpenAlgo folds NFO→NSE / BFO→BSE with `segment` carrying the distinction |

Segments (SDK `SEGMENT_*`, `client.py:71-74` + OpenAlgo `SEGMENT_*`):

| Station coalition | Venue code |
|---|---|
| Cash book | `CASH` |
| (refuse v1) | `FNO`, `CURRENCY`, `COMMODITY` |

Products (SDK `PRODUCT_*`, `client.py:63-69` + OpenAlgo `transform_data.py:20-22` / `order_data.py:163-169`):

| OpenAlgo/Station | Venue |
|---|---|
| CNC | `CNC` |
| MIS | `MIS` (inbound) — but `order_data.py` reverse-maps venue `INTRADAY`→MIS and `MARGIN`→NRML, so the book may *return* `INTRADAY`/`MARGIN` — **D5: B6 cites the exact product strings on read payloads** |
| NRML | `NRML` (inbound) / `MARGIN` (book?) — same D5 |
| (refuse on cash v1) | `CO`, `BO`, `MTF`, `ARB` |

Order types `LIMIT/MARKET/SL/SL_M` per SDK (`client.py:57-60`) vs OpenAlgo `STOP_LOSS_LIMIT/STOP_LOSS_MARKET` (`transform_data.py:27-28`) — **D6: oracle mismatch, execution anyway (refuse v1), record for row 3 entitlements only**. Validity: SDK `DAY/EOS/IOC/GTC/GTD` vs OpenAlgo DAY/IOC only (`transform_data.py:230-234` maps GTC→DAY silently — Station must not inherit silent defaults).

## Hard refuse in these trees (never call, never port, never adopt)

| Refused capability | Oracle location |
|---|---|
| Place / modify / cancel orders | SDK `place_order/modify_order/cancel_order` (`client.py:197-346`, `/order/create|modify|cancel`); OpenAlgo `direct_place_order_api/place_order_api/place_smartorder_api/direct_modify_order/modify_order/cancel_order/cancel_all_orders_api` + per-symbol smart-order locks/caches |
| Smart orders (GTT/OCO) | SDK `create/modify/cancel/get_smart_order(_list)` (`client.py:1030-1330`, `/order-advance/*`); OpenAlgo `SEGMENT`/smart paths |
| Order-update / position-update streams, all WS/NATS feed | SDK `feed.py`, `nats_client.py`, `proto/*`, `generate_socket_token`, `FeedConstants` (`constants.py`); OpenAlgo `streaming/*` (`groww_adapter/mapping/nats/nkeys/protobuf`, `nats_websocket.py`) — v1 is REST poll only |
| Option chain / greeks / expiries / contracts analytics | SDK `get_option_chain/get_greeks/get_expiries/get_contracts` — no options-analytics service |
| OAuth redirect login | Docs "1st approach" OAuth 2.0 (unmined) — Station 0014 is env-checksum, no browser |
| Synthetic trades on 404 | OpenAlgo `order_api.py:3360-3389` (`synthetic_{orderid}`) — never fabricate fills; surface the gap |
| Credential storage | OpenAlgo `os.getenv("BROKER_API_KEY"/"BROKER_API_SECRET")` (`auth_api.py:100-101`), `database/*` session/token DBs — Station creds live in Keychain tagged blobs, never env/DB |
| Copied code blocks | any — clean-room rewrite from cited behavior only |

## Realign pointer

B6 lane fills `issues/brokers/sheets/groww.md` 0–25 from **official Groww docs only** (https://groww.in/trade-api/docs + `/curl` + `/python-sdk` variants, URL + fetch date per row). OpenAlgo `broker/groww` + this PyPI SDK are the question list (D1 timestamp type, D2 holdings path, D3 per-order fills fan-out, D4 `x-api-version` header, D5 product strings on reads, D6 order-type vocab, Q-TTL token lifetime, subscription-gate row 0), never sources. Hosts/paths matching here does not imply trade-shape matches — same rule as the Binance/Nautilus cage. ADR 0014 must decide: **env-checksum `approval` variant as the one Station flow** (no redirect, no TOTP-seed handling v1), **Swift Connect = key-entry not browser**, and the **fills fan-out budget** (order-list → per-order trades under the Non-Trading 20/s + 500/min type cap) before Z5/Z6.

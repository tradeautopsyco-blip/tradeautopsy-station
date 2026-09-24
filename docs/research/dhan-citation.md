# Citation pin — DhanHQ-py official SDK + OpenAlgo `broker/dhan` (Dhan oracle)

**Fetched:** 2026-09-24 IST
**Clone:** `/Users/bishnu/oracles/dhan-DhanHQ-py-2026-09-24`
**Remote:** https://github.com/dhan-oss/DhanHQ-py
**SHA:** `63b9030d700f0fd331c124fd948ba0b79a6e7fd8` (commit 2026-08-31, "Update Market Feed example")
**Tag:** `v2.3.0rc1` (pre-release; latest stable `v2.2.0`)
**Licence:** MIT (`LICENSE`, "Copyright (c) 2026 Dhan") — cite, do **not** vendor into Station, do **not** add to Station `Cargo.toml`.

**Second oracle (already pinned):** OpenAlgo `broker/dhan` at `ad3cd54df476b330c4e4b01a31a3ad53deb9012b` (2026-09-19; `git status --porcelain -- broker/dhan/` clean 2026-09-24, no dirty files — cite directly, no `git show HEAD:` needed). AGPL — leads only, see `openalgo-citation.md`.

Station owns the wire. These trees are the **login-shape / host / path / field-map oracle** for the `dhan` book. Every signal below is a *lead* ("SDK `auth.py` shows a consent flow — verify at <official URL>"), never evidence. A B6 row with only an oracle citation stays `TBD`.

## Hosts

| Use | SDK const | URL |
|-----|-----------|-----|
| Auth (consent + PIN/TOTP) | `DhanLogin.AUTH_BASE_URL` (`src/dhanhq/auth.py:10`) | `https://auth.dhan.co` |
| API v2 (all data/trade reads) | `DhanHTTP.API_BASE_URL` (`src/dhanhq/dhan_http.py:36`); OpenAlgo `BASE_URL` (`broker/dhan/api/baseurl.py`) | `https://api.dhan.co` + `/v2` |

No testnet/sandbox host in the SDK. OpenAlgo has a separate `dhan_sandbox` broker dir — that is an **N-A environment row, never a slug** (P3 program B5).

## Auth shapes (THREE in SDK — B6 + ADR 0009 must pick Station's one)

| # | Shape | SDK location | Wire |
|---|-------|--------------|------|
| A | **Consent 3-step (ADR 0009 default)** | `auth.py:18-50` `generate_login_session`, `:52-84` `consume_token_id` | `POST /app/generate-consent?client_id=` headers `app_id` + `app_secret` → `consentAppId` → browser `GET /login/consentApp-login?consentAppId=` → `tokenId` → consume `?tokenId=` headers `app_id` + `app_secret` → `accessToken` + `dhanClientId`/`dhanClientName`/`dhanClientUcc`/`givenPowerOfAttorney`/`expiryTime` |
| B | **PIN + TOTP direct (no browser)** | `auth.py:86-119` `generate_token` | `POST /app/generateAccessToken?dhanClientId=&pin=&totp=` (query params, no headers) → `accessToken` |
| C | **Renew + validate** | `auth.py:121-147` `renew_token`, `:150-178` `user_profile` | `GET /v2/RenewToken` headers `access-token` + `dhanClientId`; `GET /v2/profile` same headers (token-validity check) |

**Discrepancy D1 (must resolve at official docs):** consume verb — SDK uses `requests.get` (`auth.py:73`), OpenAlgo uses `client.post` (`broker/dhan/api/auth_api.py` `consume_consent`). One of them is stale. B6 row 16 cites the official verb.

**Discrepancy D2 (must resolve at official docs):** data header — SDK sends `access-token` **+** `client-id` (`dhan_http.py:41-46`); OpenAlgo sends `access-token` only (`broker/dhan/api/order_api.py` `get_api_response`). B6 row 10/14 cites whether `client-id` is required on `/v2/*` reads.

**Question Q-ADR (for ADR 0009):** does Station ship shape A only, or also B? B is no-browser (like Groww 0014) but PIN-handling has Keychain/UX consequences. Default: A first; B needs its own ADR row if ever wanted.

## Read paths (base `https://api.dhan.co/v2`, leads for B6 row 2)

| Station use | SDK call | Verb + path |
|-------------|----------|-------------|
| Trade book (fills — **the v1 path**) | `Statement.get_trade_book` (`_statement.py:8-22`) | `GET /trades/` (day), `GET /trades/{order_id}` (per-order) |
| Trade history (date range — depth TBD at B6) | `Statement.get_trade_history` (`_statement.py:24-36`) | `GET /trades/{from_date}/{to_date}/{page_number}` |
| Order book | `Order.get_order_list/by_id/by_correlationID` (`_order.py:7-38`) | `GET /orders`, `/orders/{order_id}`, `/orders/external/{correlation_id}` |
| Holdings / positions | `Portfolio.get_holdings/get_positions` (`_portfolio.py:7-23`) | `GET /holdings`, `GET /positions` |
| Funds | `Funds.get_fund_limits` (`_funds.py:7-15`) | `GET /fundlimit` |
| Ledger | `Statement.ledger_report` (`_statement.py:38-49`) | `GET /ledger?from-date=&to-date=` |
| LTP / OHLC / quote | `MarketFeed.ticker_data/ohlc_data/quote_data` (`_market_feed.py`) | `POST /marketfeed/ltp`, `/marketfeed/ohlc`, `/marketfeed/quote` (body `{exchange_segment: [security_ids]}`) |
| Intraday candles (SDK docstring: last 5 trading days) | `HistoricalData.intraday_minute_data` | `POST /charts/intraday` (`securityId`, `exchangeSegment`, `instrument`, `interval` 1/5/15/25/60, `oi`, `fromDate`, `toDate`) |
| Daily candles | `HistoricalData.historical_daily_data` | `POST /charts/historical` (+ `expiryCode` 0-3) |
| Token validity | `DhanLogin.user_profile` | `GET /profile` |

OpenAlgo confirms the same five day-book paths (`broker/dhan/api/order_api.py`: `get_order_book` → `/v2/orders`, `get_trade_book` → `/v2/trades`, `get_positions`, `get_holdings`; `broker/dhan/api/funds.py`: `test_auth_token` → `/v2/fundlimit` doubles as the auth-probe shape).

## JSON / error aliases (leads for adapter + host error map)

- Success envelope (SDK `_parse_response`, `dhan_http.py:73-103`): HTTP 2xx → `{status: success, data: <json>}`; else `{status: failure, remarks: {error_code, error_type, error_message}}` from `errorCode`/`errorType`/`errorMessage`.
- OpenAlgo sees two more failure shapes (`order_api.py:get_api_response`): `{status: failed|error, data: {code: msg}}` and `{errorType, errorCode, errorMessage}`. B6 row 4 + host tests lock the real set.
- Rate-limit signal (OpenAlgo `data.py`): error **805** "too many requests". Official rate table (https://dhanhq.co/docs/v2/ intro, fetch 2026-09-24): Order 10/s, **Data 5/s, Quote 1/s**, Non-trading 20/s; per-min/hour/day caps on orders (250/1000/7000), Data 100k/day, Quote unlimited/day; ≤25 modifications/order. OpenAlgo throttles Data 0.2s + Quote 1.1s (`_apply_rate_limit`) — matches the 5/s + 1/s cells.
- `dhanClientId` is injected into every POST payload by the SDK (`dhan_http.py:57-59`) — host signer must replicate if B6 confirms.

## Vocab as BAR (check the B6 against it — never a source)

Exchange segments (SDK `dhanhq.py:25-35` + OpenAlgo `map_exchange_type`):

| Station coalition | Venue code |
|---|---|
| NSE cash | `NSE_EQ` |
| BSE cash | `BSE_EQ` |
| (refuse v1) | `NSE_FNO`, `BSE_FNO`, `NSE_CURRENCY`, `BSE_CURRENCY`, `MCX_COMM`, `NSE_COMM`, `IDX_I`, `INX_EQ` (Global Stocks US) |

Products (SDK `dhanhq.py:40-46` + OpenAlgo `map_product_type`/`reverse_map_product_type`):

| OpenAlgo/Station | Venue |
|---|---|
| CNC | `CNC` |
| MIS | `INTRADAY` |
| (refuse on cash v1) | `MARGIN` (= NRML), `CO`, `BO`, `MTF` |

OpenAlgo warns: forward map defaults unknown → `INTRADAY` (`transform_data.py:274`) — Station must **not** inherit silent defaults; B6 names the product explicitly. Reverse map is `{"CNC": "CNC", "MARGIN": "NRML", "INTRADAY": "MIS"}` (was once written backwards — issue noted in-tree).

Order types `LIMIT/MARKET/STOP_LOSS/STOP_LOSS_MARKET`, validity `DAY/IOC` (execution — refuse v1, record for row 3 entitlements only).

## Hard refuse in these trees (never call, never port, never adopt)

| Refused capability | Oracle location |
|---|---|
| Place / modify / cancel orders | SDK `_order.py` `modify_order`+; OpenAlgo `broker/dhan/api/order_api.py` `place_order_api`/`modify_order`/`cancel_order`/`cancel_all_orders_api`, smart-order locks/caches |
| Position convert / exit-all | SDK `_portfolio.py` `convert_position`, `exit_all_positions` (`DELETE /positions`) |
| Margin calculators (order-taking) | SDK `_funds.py` `margin_calculator(_multi)`; OpenAlgo `margin_api.py` order paths (read-only `check-margin` shape is a lead only) |
| Forever/GTT, Super, Conditional orders | SDK `_forever_order.py`, `_super_order.py`, `_conditional_order.py`; OpenAlgo `gtt_api.py` + `mapping/gtt_data.py` |
| TraderControl (P&L auto-exit), kill-switch-in-SDK | SDK `_trader_control.py` — Station Kill is DNS-level, never an SDK call |
| Global Stocks (US) | SDK `_global_stocks.py`, `global_stocks_feed.py`, `INX_EQ` — no TradeAutopsy book |
| WebSockets (market feed, full depth, order updates) | SDK `marketfeed.py` (`run_forever`), `fulldepth.py`, `orderupdate.py` — v1 is REST poll only |
| IP whitelist management | SDK `auth.py` `set_ip/modify_ip/get_ip` — account ops, not desk reads |
| Expired-options analytics | SDK `_historical_data.py` `expired_options_data` — no options-analytics service |
| Credential storage | OpenAlgo `os.getenv("BROKER_API_KEY")` (`client_id:::api_key` split), `database/auth_db.py` Fernet cache — Station creds live in Keychain tagged blobs, never env/DB |
| Copied code blocks | any — clean-room rewrite from cited behavior only |

## Realign pointer

B6 lane fills `issues/brokers/sheets/dhan.md` 0–25 from **official DhanHQ docs only** (https://dhanhq.co/docs/v2/ + consent-login pages, URL + fetch date per row). OpenAlgo `broker/dhan` + this SDK are the question list (below in the kickoff reply), never sources. Hosts/paths matching here does not imply trade-book shape matches — same rule as the Binance/Nautilus cage. D1 (consume verb) + D2 (client-id header) + Q-ADR (consent-only vs PIN/TOTP) must be closed before Z5/Z6.

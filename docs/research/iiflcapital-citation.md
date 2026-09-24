# Citation pin — IIFL Markets Open API Postman Collection (official) + OpenAlgo `broker/iiflcapital` (IIFL Capital oracle)

**Fetched:** 2026-09-24 IST
**Clone:** `/Users/bishnu/oracles/iiflcapital-postman-2026-09-24`
**Remote:** https://github.com/IIFLCapital/IIFL-Markets-Open-API-Postman-Collection-Official-
**SHA:** `0cf607a433710381611b4afc3fc602fe46159580` (commit 2025-03-07, "Commit Postman Collection")
**Tag:** none (no tags in repo)
**Licence:** GPLv3 (`LICENSE`, FSF boilerplate) — cite, do **not** vendor into Station, do **not** add to Station `Cargo.toml`.

**Second oracle (already pinned):** OpenAlgo `broker/iiflcapital` at `ad3cd54df476b330c4e4b01a31a3ad53deb9012b` (2026-09-19; `git status --porcelain -- broker/iiflcapital/` clean 2026-09-24, no dirty files — cite directly, no `git show HEAD:` needed). AGPL — leads only, see `openalgo-citation.md`.

**Account move (recorded 2026-09-24):** the old `IIFLSecurities` GitHub account now 404s (profile + API). The live official account is the **user** `IIFLCapital` (https://github.com/IIFLCapital), holding `BridgePy`, `BridgeCore8`, `bridgeGo`, `BridgeJava`, `BridgeNet4.6`, `BridgeNode` (all "official … library to connect and get market data", pushed 2025-10-14) plus the pinned Postman collection (pushed 2025-03-07). Old-account URLs still in search indexes are stale — cite the `IIFLCapital` user only.

**No official REST SDK repo exists.** IIFL ships no Python/Node REST client for the Markets' APIs — only the Postman collection (REST shape oracle, pinned above) and the Bridge streaming libraries (market-data + order/trade-update websockets — refuse v1). There is additionally a third-party mirror of the official docs at `marketcalls/broker-api-docs/iiflcapital-api-docs/` (master branch, 16 files) — used below as a **lead pointer only**, never a source; B6 rows cite `developers.iiflcapital.com` directly. The 2021 PyPI `IIFLapis` package (client_login with passwd/dob) targets the previous API generation — not authoritative for the `getusersession` shape, do not cite for paths.

Station owns the wire. These trees are the **login-shape / host / path / field-map oracle** for the `iiflcapital` book. Every signal below is a *lead* ("Postman shows POST /getusersession with checkSum — verify at <official URL>"), never evidence. A B6 row with only an oracle citation stays `TBD`.

## Hosts

| Use | Oracle const | URL |
|-----|--------------|-----|
| API v1 (auth-token exchange + all data/trade reads) | Postman collection raw URLs (every request); OpenAlgo `BASE_URL` (`broker/iiflcapital/baseurl.py`) | `https://api.iiflcapital.com/v1` |
| Login portal (browser SSO) | OpenAlgo `LOGIN_URL` (`broker/iiflcapital/baseurl.py`); mirror `03-user.md` login flow | `https://markets.iiflcapital.com/` + `?v=1&appkey=<appKey>&redirecturl=<url>` |
| Contract master files | Postman "Instrument Details" folder (9 requests) | `https://api.iiflcapital.com/v1/contractfiles/<SEGMENT>.json` |

Postman collection variables pin the same base four times (`AUTH_BASE_URL`, `SSO_AUTH_BASE_URL`, `OPEN_API_BASE_URL`, `OPEN_API_ORDER_BASE_URL` — all `http://api.iiflcapital.com/v1/`, note `http` in the var vs `https` in every request URL; B6 cites the `https` form). No testnet/sandbox host anywhere. Static-IP whitelisting is reported at the app level (docs.openalgo.in IIFL page: Primary Static IP must match the caller's public IP) — B6 row 1 must confirm whether a dynamic-IP desk can read at all.

## Auth shapes (ONE in oracle — B6 + ADR 0008 must confirm Station's UX around it)

| # | Shape | Oracle location | Wire |
|---|-------|-----------------|------|
| A | **Browser SSO + SHA-256 checksum (ADR 0008 default)** | Postman "User / Get User Session"; OpenAlgo `api/auth_api.py` `_generate_checksum` + `authenticate_broker`; mirror `03-user.md` login flow | Browser `GET https://markets.iiflcapital.com/?v=1&appkey=<appKey>&redirecturl=<url>` → user enters trading creds + OTP/TOTP → redirect to app URL with `authCode` + `clientId` query params (or IIFL page shows `authCode` if no redirect registered) → `POST /getusersession` body `{checkSum: SHA256(clientId + authCode + appSecret)}` (plain concatenation, hex digest) → `{status: Ok, userSession: <JWT>}` → `Authorization: Bearer <userSession>` on every later call |

**Discrepancy D1 (must resolve at official docs):** redirect-param casing — mirror docs show lowercase `redirecturl` only; OpenAlgo sends **both** `redirecturl` and `redirectUrl` (unescaped value) "for compatibility with different IIFL deployments". B6 row 16 cites the official casing.

**Question Q-ADR (for ADR 0008):** daily re-login is mandatory (mirror: "A fresh login (and token) is required every trading day") and the app secret participates in a per-login checksum — Station must hold `appKey` + `appSecret` in Keychain tagged blobs and drive a browser SSO every morning. ADR 0008 decides the Keychain schema + whether Station caches `userSession` intraday only.

**Question Q-IP (for B6 row 1):** is the static-IP whitelist enforced on the *read* paths (`/trades`, `/orders`, `/positions`, `/holdings`) or only at app registration? If enforced per-call, home-broadband desks need a fixed-IP note in the sheet.

## Read paths (base `https://api.iiflcapital.com/v1`, leads for B6 row 2)

| Station use | Oracle call | Verb + path |
|-------------|-------------|-------------|
| Trade book (fills — **the v1 path**) | Postman "Order Management / Trade Book"; OpenAlgo `order_api.py:get_trade_book` | `GET /trades` (day; 3 req/s cap) |
| Order book | Postman "Order Management / Order book"; OpenAlgo `order_api.py:get_order_book` | `GET /orders` (day; 3 req/s cap) |
| Order history (per-order) | Postman "Order Management / Order History"; mirror `05-orders.md` | `GET /orders/{brokerOrderId}` (10 req/s cap) |
| Positions | Postman "Portfolio / Positions"; OpenAlgo `order_api.py:get_positions` | `GET /positions` (3 req/s cap; includes F&O carryforward) |
| Holdings | Postman "Portfolio / Holdings"; OpenAlgo `order_api.py:get_holdings` | `GET /holdings` (3 req/s cap; DEMAT, T+1) |
| Funds / limits | Postman "User / Limits"; OpenAlgo `funds.py:get_margin_data` | `GET /limits` (+ OpenAlgo-only `GET /limits/equity`, `GET /limits/fno` — **not** in Postman; B6 must confirm these two exist officially) |
| Profile (auth-probe + entitlements) | Postman "User / Profile"; mirror `03-user.md` | `GET /profile` (3 req/s cap; returns clientId/clientName/exchanges/products/orderComplexity) |
| Contract master | Postman "Instrument Details" ×9 | `GET /contractfiles/<NSEEQ\|BSEEQ\|NSEFO\|BSEFO\|NSECOMM\|MCXCOMM\|NSECURR\|BSECURR\|INDICES>.json` (2 req/s each) |
| Market quotes (LTP/OHLC/depth-touchline) | Postman "Market Data APIs / Market Quotes"; OpenAlgo `data.py:get_quotes/get_multiquotes` | `POST /marketdata/marketquotes` (batch array of `{exchange, instrumentId}`; 10 req/s) |
| Market depth | Postman "Market Data APIs / Market Depth"; OpenAlgo `data.py:get_depth` | `POST /marketdata/marketdepth` (single `{exchange, instrumentId}`; 10 req/s) |
| Open interest (F&O only) | Postman "Market Data APIs / Open Interest"; OpenAlgo `data.py:_fetch_openinterest` | `POST /marketdata/openinterest` (**single-mode only**, no batch; 10 req/s, 20 registered) |
| Historical candles | Postman "Market Data APIs / Historical Candlestick Chart Data"; OpenAlgo `data.py:get_history` | `POST /marketdata/historicaldata` (`{exchange, instrumentId, interval, fromDate, toDate}`; dates `DD-Mon-YYYY` e.g. `11-nov-2024`; 10 req/s) |

OpenAlgo confirms the same day-book paths (`broker/iiflcapital/api/order_api.py`: `get_order_book` → `/orders`, `get_trade_book` → `/trades`, `get_positions`, `get_holdings`; `funds.py` pooled `/limits` preferred, segment endpoints summed only as fallback).

**Discrepancy D2 (must resolve at official docs):** `/limits/equity` + `/limits/fno` appear **only** in OpenAlgo `funds.py` (`_fetch_limits`), not in the Postman collection or the mirror's `03-user.md` (which lists four User endpoints: getusersession/profile/limits/logout). Either they are undocumented-but-live or OpenAlgo-invented. B6 row 2 / funds row cites the official set; Station must not call unlisted paths.

## JSON / error aliases (leads for adapter + host error map)

- Session envelope: `{status: Ok|Not_Ok, userSession: <JWT>}` (mirror `03-user.md`; OpenAlgo checks `status == "ok"` case-insensitively + `userSession` present).
- General envelope (mirror `02-request-response-structure.md`): `{status, message, result}` where `result` is an **array of per-operation results** in request order, each with its own `status`/`message` (e.g. place-order returns per-leg `brokerOrderId`). OpenAlgo `_extract_rows`/`_ok` accept `result` as list **or** dict with `orders`/`trades`/`positions`/`holdings`/`data`/`positionList` keys, plus bare-list and `{data/orders/trades/positions/holdings}` payloads — B6 row 4 + host tests lock which shape `/trades` actually returns (single-day array vs wrapped).
- Top-level success strings seen: `Ok`, `Success` (OpenAlgo `_SUCCESS_STATUSES = {success, ok}`, case-insensitive; data.py also accepts `true`/`200`).
- Error codes: `EC001`–`EC999` in `status`/`message` (mirror `12-error-codes.md`). Notable for the host map: `EC003` "Something went wrong, please try after some time" (generic retry hint — OpenAlgo treats as rate-limit-adjacent), `EC044` "Token is not valid" (session-expiry lead), `EC055` unauthorized/auth-code, `EC901`-class per-leg validation errors (e.g. bad `exchange` enum), `EC956` DELIVERY/BNPL only on NSEEQ/BSEEQ, `EC969` product allows NORMAL+INTRADAY (endpoint-scoped), `EC992`–`EC999` order-book misses (not-found/modify/cancel/history). RMS rejections (order `rejectionReason`) are **not** API errors — separate mirror file `14-rms-rejections.md`.
- Rate limits (mirror `15-rate-limits.md`, per-second, non-registered vs registered->10-OPS): getusersession 3/3, profile 3/3, limits 10/20, preorder 10/20, SPAN 10/20, place(+modify+cancel combined) 10/20, cancel-all 3/3, **order book 3/3, trade book 3/3, order history 10/10**, positions 3/3, holdings 3/3, historical 10/10, depth 10/10, OI 10/20, quotes 10/10, contract files 2/2 each, logout 2/2. No documented 429 body/header — OpenAlgo paces everything at ~8 req/s (`MIN_INTERVAL 0.125`, process-wide) and retries 429s with `Retry-After`-or-exponential backoff (`rate_limiter.py`, `MAX_RETRIES 3`). Station host adopts the 3/s day-book cells as its poll ceiling until B6 confirms.
- Trade-row aliases (OpenAlgo `mapping/order_data.py:transform_tradebook_data` — leads for fill-field mapping): qty from `filledQuantity|quantity|filledQty`; price from `tradedPrice|averageTradedPrice|price`; id from `brokerOrderId|exchangeOrderId|orderId`; time from `fillTimestamp|exchangeTimestamp|exchangeUpdateTime|brokerUpdateTime`; side `transactionType` BUY/SELL; `quantity` is **lots** on MCXCOMM/NSECOMM/BSECOMM/NCDEXCOMM, absolute elsewhere; `NCDEXCOMM` uses trading-symbol as `instrumentId`.
- Position-row aliases: `netQuantity`, `netAveragePrice`, `realizedPnl` only (no unrealized — OpenAlgo computes MTM from LTP; Station must decide its own PnL rule, never inherit the computed field).
- Holding-row aliases: `nseTradingSymbol`/`bseTradingSymbol` pair (pick populated side), `dpQuantity` = settled (preferred over `totalQuantity` = dp + collateral + t1 + authorized), `averageTradedPrice`, `ltp|previousDayClose`.
- Order-row aliases: `orderStatus` (OPEN/PENDING/TRIGGER_PENDING/PARTIALLY_FILLED/NEW/PUT ORDER REQ RECEIVED/COMPLETE/REJECTED/CANCELLED…), `rejectionReason`, `price`, `slTriggerPrice`, `brokerOrderId`.
- Limits-row aliases (OpenAlgo `funds.py`): `tradingLimit|openingCashLimit`, `collateralMargin`, `utilizedMargin`, `creditForSell`, `adhocMargin`, `utilizedSpanMargin`, `utilizedExposureMargin`.

## Vocab as BAR (check the B6 against it — never a source)

Exchange segments (Postman contractfiles folder + mirror error-enum EC901 + OpenAlgo `map_exchange`/`_map_exchange`/`_normalize_exchange`):

| Station coalition | Venue code |
|---|---|
| NSE cash | `NSEEQ` |
| BSE cash | `BSEEQ` |
| (refuse v1) | `NSEFO`, `BSEFO`, `NSECURR`, `BSECURR`, `MCXCOMM`, `NSECOMM`, `BSECOMM`, `NCDEXCOMM`, `INDICES` |

**Discrepancy D3 (must resolve at official docs):** `NCDEXCOMM` and `BSECOMM` appear in the mirror's EC901/EC987 enums and OpenAlgo's maps, but the Postman collection has **no** `NCDEXCOMM.json` / `BSECOMM.json` contract file (9 files only). B6 row 3 cites the official segment list; cash v1 needs only NSEEQ/BSEEQ but the sheet records the full set honestly.

Products (mirror `03-user.md` profile example + Postman place-order body + OpenAlgo `map_product_type`/`reverse_map_product_type`):

| OpenAlgo/Station | Venue |
|---|---|
| CNC | `DELIVERY` (profile also lists `BNPL` → OpenAlgo reverse-maps to CNC) |
| MIS | `INTRADAY` |
| NRML | `NORMAL` |

OpenAlgo warns by construction: forward map defaults unknown → `INTRADAY` (`transform_data.py:129`) — Station must **not** inherit silent defaults; B6 names the product explicitly. Note EC969 (`product` only allows NORMAL and INTRADAY) is endpoint-scoped per the mirror — another reason B6 cites per-endpoint enums.

Order types `MARKET/LIMIT/SL/SLM` (OpenAlgo maps `SL-M`→`SLM`, then converts SLM→protective SL at placement — execution detail, refuse v1, record for row 3 entitlements only), validity `DAY/IOC` (EC996: IOC not allowed for AMO), complexity `REGULAR/AMO/BO/CO` (Postman place body shows `AMO`; `BO`/`CO` intraday-only per EC955).

## Hard refuse in these trees (never call, never port, never adopt)

| Refused capability | Oracle location |
|---|---|
| Place / modify / cancel orders (incl. bulk place, AMO/BO/CO) | Postman "Order Management / Place|Modify|Cancel"; OpenAlgo `broker/iiflcapital/api/order_api.py` `place_order_api`/`place_smartorder_api`/`modify_order`/`cancel_order`/`cancel_all_orders_api`/`close_all_positions` |
| Pre-order margin + SPAN/exposure calculators (order-taking) | Postman "Margin / Span Exposure|Preorder margin"; OpenAlgo `margin_api.py` `calculate_margin_api` + `mapping/margin_data.py` |
| Bridge streaming libs (market-data + order/trade-update websockets) | `IIFLCapital/BridgePy|BridgeCore8|bridgeGo|BridgeJava|BridgeNet4.6|BridgeNode` repos; OpenAlgo `broker/iiflcapital/streaming/` (mqtt/websocket/adapters) — v1 is REST poll only |
| Order/trade update push events | Mirror `06-order-and-trade-updates.md` + `08-market-data-stream.md` — same refuse, REST poll only |
| Contract-master binary decoding service | Mirror `11-binary-decoding-guide.md` — Station reads JSON contract files, no binary protocol work |
| Brokerage/charges engine in-oracle | Mirror `10-brokerage-and-charges.md` — charges are Station's book, computed from fills, never from a broker calculator |
| Logout-as-kill-switch | Postman "User / Logout" (`POST /profile/logout`) — session hygiene only; Station Kill is DNS-level, never an API call |
| Credential storage | OpenAlgo `os.getenv("BROKER_API_KEY"/"BROKER_API_SECRET")` (`auth_api.py`) — Station creds live in Keychain tagged blobs, never env |
| Copied code blocks | any — clean-room rewrite from cited behavior only; GPLv3 (Postman) + AGPL (OpenAlgo) forbid vendoring in any case |

## Realign pointer

B6 lane fills `issues/brokers/sheets/iiflcapital.md` 0–25 from **official IIFL docs only** (https://developers.iiflcapital.com/apidocs/introduction + `https://markets.iiflcapital.com` login portal + `https://api.iiflcapital.com` reference, URL + fetch date per row). OpenAlgo `broker/iiflcapital` + this Postman collection + the `marketcalls/broker-api-docs` mirror are the question list (D1 redirect casing, D2 `/limits/equity|fno` existence, D3 NCDEXCOMM/BSECOMM segments, Q-IP static-IP enforcement, `/trades` envelope shape), never sources. Hosts/paths matching here does not imply trade-book shape matches — same rule as the Binance/Nautilus cage. D1 + D2 + Q-IP + Q-ADR (daily browser SSO + Keychain appSecret) must be closed before Z5/Z6.

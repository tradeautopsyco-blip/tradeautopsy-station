# Citation pin — NautilusTrader (international venues oracle)

**Fetched:** 2026-09-23 IST
**Temp clone:** `/tmp/nautilus-pin-v2.0.0rc5` (network clone, approved; local dir has no `.git`)
**Remote:** https://github.com/nautechsystems/nautilus_trader
**Tag:** `v2.0.0rc5`
**SHA:** `1b0a49d2792a9432a3aca3fcb617ce7a630d905e` (tag peel = `master` HEAD; tag commit date 2026-09-15)
**Licence:** LGPL-3.0 (`LICENSE`) — cite, do **not** vendor into Station, do **not** add any `nautilus-*` crate to Station `Cargo.toml`.

Station owns the wire. This tree is the **host / path / JSON-alias oracle** for
international-venue books. It is native `tokio` code that owns sockets, signing, and
execution clients — it can never satisfy ADR 0001 (Wasm component, host `broker-http-call`
import only, zero secrets).

## Pin note — there is no `v2.0.0rc6` tag upstream

The local tree `/Users/bishnu/nautilus_trader-develop` reports `version.json`
`v2.0.0rc6`, but `git ls-remote --tags` on 2026-09-23 IST shows the newest v2 tag is
`v2.0.0rc5`. The local tree is a `develop`-branch snapshot (unreleased rc6 version bump),
not a tag. `develop` HEAD on the fetch date was
`3adf5a8dc0c9e676eb1e131911a73ad693ea7b42` (recorded, **not** cited — branch heads move).

All citations below come from the pinned `v2.0.0rc5` temp clone, not the unpinned local dir.

**Drift check (local vs pinned rc5):** 320 files differ tree-wide, but on the citation
surface (`crates/adapters/*/src/common/{consts,urls}.rs`) only two files differ, and
neither changes a host or path:

- `binance/.../consts.rs` — WS subscription quota pacing comment + unit test only.
- `okx/.../consts.rs` — two added error-code test cases (`60014`, `64007`) only.

**No-dependency proof (2026-09-23):** `rg -i nautilus agent/Cargo.toml` → no matches (exit 1).

## Hosts (from `crates/adapters/*/src/common/{consts,urls}.rs` at pin)

Prod hosts are leads for B6 rows 10/22 — each must still be verified at the venue's
official docs before any allowlist or Kill claim. Test/sandbox/demo hosts are **never**
Station live hosts.

| Adapter | Proposed slug | Prod host(s) | Test / sandbox host(s) | Near-miss / sibling (refuse on first book) |
|---------|---------------|--------------|------------------------|--------------------------------------------|
| `binance` | `binance_com` (LIVE) | `api.binance.com`, `fapi.binance.com`, `dapi.binance.com`, `eapi.binance.com` | `testnet.binance.vision`, `testnet.binancefuture.com`, `demo-api/dapi/fapi.binance.com` | `api.binance.us` → separate `binance_us` slug (Parked) |
| `bybit` | `bybit` (Wave 4a) | `api.bybit.com` | `api-testnet.bybit.com`, `api-demo.bybit.com` | — |
| `okx` | `okx_com` (Wave 4b) | `www.okx.com` | — (in-file: none; WS has pap/test hosts) | `us.okx.com`, `eea.okx.com` refused |
| `kraken` | `kraken` (Wave 4c) | `api.kraken.com` | `demo-futures.kraken.com` | `futures.kraken.com` is a sibling book, refused on spot |
| `coinbase` | `coinbase_advanced` (Wave 4d) | `api.coinbase.com` | `api-sandbox.coinbase.com` (not live) | — |
| `deribit` | `deribit` (Wave 5) | `www.deribit.com` | `test.deribit.com` | no real spot book |
| `bitmex` | `bitmex` (Wave 5) | `www.bitmex.com` | `testnet.bitmex.com` | perps refused until a perps lock |
| `interactive_brokers` | `interactive_brokers` (Wave 5, ADR first) | none (socket to TWS/Gateway: `127.0.0.1`, ports 4002/7497 per `common/consts.rs`) | — | does not fit HTTP-only import |
| `hyperliquid` | — (Park) | `api.hyperliquid.xyz` | `api.hyperliquid-testnet.xyz` | wallet custody breaks the model |
| `derive` | — (Park) | `api.lyra.finance` | `api-demo.lyra.finance` | wallet custody |
| `lighter` | — (Park) | `mainnet.zklighter.elliot.ai` | `testnet.zklighter.elliot.ai` | wallet custody |
| `dydx` | — (Park) | `indexer.dydx.trade` (+ Cosmos gRPC endpoints) | `indexer.v4testnet.dydx.exchange` | gRPC does not fit |
| `architect_ax` | — (Park) | `gateway.architect.exchange` | `gateway.sandbox.architect.exchange` | no demand |
| `betfair` | — (Park) | `api.betfair.com`, `identitysso(-cert).betfair.com` | — | no TradeAutopsy asset class |
| `polymarket` | — (Park) | `clob.polymarket.com`, `gamma-api.polymarket.com`, `data-api.polymarket.com` | — | no TradeAutopsy asset class |
| `tardis` | — (N-A slug) | `api.tardis.dev` | — | data vendor, not a broker |
| `databento` | — (N-A slug) | (no `common/`; client hosts under `src/`) | — | data vendor, not a broker |
| `blockchain` | — (N-A slug) | RPC-configured (no fixed venue host) | — | infra, not a broker |
| `sandbox` | — (Refuse) | none (simulated) | — | simulated venue as a broker is fake-live |

WS (`wss://`) hosts exist per adapter in the same files (e.g. Bybit
`stream.bybit.com`, OKX `ws.okx.com:8443`, Kraken `ws.kraken.com` /
`ws-auth.kraken.com`). Station's import is request/response HTTP; WS hosts are recorded
here only so a future transport ADR can find them — not Station hosts today.

## Paths confirmed at pin (leads for future `docs/reference/*/REST.md`)

Public/data paths are citable as *questions* for the B6; execution paths are refused
(see below). Verbs are GET unless noted.

**Bybit** (`bybit/src/http/`, host `api.bybit.com`) — data leads:

- `GET /v5/market/instruments-info`, `/v5/market/tickers`, `/v5/market/kline`, `/v5/market/orderbook`, `/v5/market/recent-trade`, `/v5/market/time`
- `GET /v5/execution/list` (fills lead), `/v5/order/history`, `/v5/position/list`, `/v5/account/wallet-balance`

**OKX** (`okx/src/http/`, host `www.okx.com`) — data leads:

- `GET /api/v5/public/instruments`, `/api/v5/public/time`, `/api/v5/market/tickers`, `/api/v5/market/candles`, `/api/v5/market/books`
- `GET /api/v5/account/balance`, `/api/v5/account/positions`, `/api/v5/account/positions-history`, `/api/v5/account/trade-fee`

**Kraken spot** (`kraken/src/http/spot/`, host `api.kraken.com`) — data leads:

- `GET /0/public/Time`, `/0/public/SystemStatus`, `/0/public/Ticker`, `/0/public/OHLC`, `/0/public/Depth`, `/0/public/Trades`, `/0/public/AssetPairs`
- `POST /0/private/Balance`, `/0/private/TradeBalance`, `/0/private/OpenOrders`, `/0/private/ClosedOrders`, `/0/private/TradesHistory` (fills lead), `/0/private/OpenPositions`, `/0/private/TradeVolume`, `/0/private/GetWebSocketsToken`

**Coinbase Advanced** (`coinbase/src/http/`, host `api.coinbase.com`) — data leads:

- `GET /api/v3/brokerage/accounts`, `/api/v3/brokerage/products`, `/api/v3/brokerage/orders/historical/fills` (fills lead), `/api/v3/brokerage/orders/historical/batch`

**Binance** — paths live under `binance/src/{spot,usdm,coinm,option}/http/` with base
selection in `common/urls.rs` (`get_http_base_url*`, `get_sapi_base_url`,
`get_ws_*_base_url`). Path-level audit is W0.5, not this doc.

**Deribit / BitMEX** — Wave 5; paths cited at ADR time, not here.

## JSON aliases / field maps (leads, clean-room rewrite only)

- Per-adapter wire models + venue→normalized mapping: `crates/adapters/<n>/src/http/{models,parse}.rs` (Kraken: `http/{spot,futures}/`; Binance: per-market `http/` dirs).
- Product / segment / exchange vocab: per-adapter `common/enums.rs` + `instruments.rs` — a **bar** to check the B6 against, never a source.
- Venue-agnostic taxonomy input: `crates/model/src/enums.rs:223` (`AssetClass`) and `:722` (`InstrumentClass`) — cited by the asset-class taxonomy plan.

## Signing shape (for host `http.rs` design — shape only, Station re-implements)

| Venue | Shape at pin | Station consequence |
|-------|--------------|---------------------|
| Bybit | HMAC-SHA256 over `timestamp + api_key + recv_window + payload`; headers `X-BAPI-API-KEY/-TIMESTAMP/-SIGN/-RECV-WINDOW` (`bybit/src/common/credential.rs`, `http/client.rs:465-471`) | smallest delta from COM key/secret |
| OKX | HMAC-SHA256 over `timestamp + method + endpoint + body`; headers `OK-ACCESS-*`; `api_passphrase` third credential (`okx/src/common/credential.rs:104-112`, `http/client.rs:925-948`) | credential blob +1 field |
| Kraken | `API-Sign = HMAC-SHA512(secret, path + SHA256(nonce + POST data))`, u64 nonce with in-µs atomic counter (`kraken/src/common/credential.rs:161-186`) | new signer |
| Coinbase | ES256 JWT (PEM EC key), `alg: ES256`, REST JWT carries URI claim, 120 s expiry (`coinbase/src/common/credential.rs:101-129`, `common/consts.rs:43`) | new `AuthScheme` |
| BitMEX | HMAC-SHA256, `api-expires` style (see `bitmex/src/http/client.rs`) | Wave 5 |
| Deribit | client-credentials / HMAC over JSON-RPC (`deribit/src/common/`) | Wave 5 |
| IB | none over HTTP — `ibapi` socket crate (`interactive_brokers/Cargo.toml:80`) | Wave 5 ADR (socket import vs Client Portal vs Flex) |

Never take: their credential env-var patterns, session handling, or any secret-bearing type.

## Rate limits / weights (leads — verify at official docs)

- Bybit: IP HTTP limit 600/5s shape, `X-Bapi-Limit*` headers, WS order limits (`bybit/src/common/rate_limit.rs:37-47`).
- Binance: WS subscription pacing 300 ms/message (local develop; rc5 has `per_second(5)`) — `common/consts.rs` `BINANCE_WS_SUBSCRIPTION_QUOTA`.
- Kraken: Futures WS 100 req/s; spot WS dynamic (`Exceeded msg rate`) (`kraken/src/common/consts.rs:59-72`).
- OKX / Coinbase / others: error-code tables in `common/consts.rs` + `http/error.rs` (e.g. OKX `60014`/`64007` retryability cases differ local-vs-rc5 — re-check at wave time).

## Hard refuse in this tree (execution — never call, never port)

The data WIT never places, modifies, cancels, transfers, or manages GTT/baskets. The
following exist in the oracle and are explicitly refused for Station use:

| Refused capability | Oracle location (at pin) |
|--------------------|--------------------------|
| Place order | Bybit `POST /v5/order/create` (`http/query.rs` `submit_order`); OKX `/api/v5/trade/order` + `/api/v5/sprd/order` (`http/client.rs`, `execution.rs`); Kraken `/0/private/AddOrder`, `AddOrderBatch` (`http/spot/`, `execution/spot.rs`); Coinbase `POST /api/v3/brokerage/orders` (`http/query.rs`, `execution.rs`); BitMEX order submit (`broadcast/submitter.rs`, `execution.rs`); Deribit order RPC (`execution.rs`, `websocket/client.rs`); Binance per-market `execution.rs` + WS trading clients |
| Modify / amend | Bybit `/v5/order/amend`; Kraken `/0/private/AmendOrder`, `EditOrder`; Coinbase `/api/v3/brokerage/orders/edit`; (`modify_order` in each adapter's `execution.rs` / `http/client.rs`) |
| Cancel (single / batch / all) | Bybit `/v5/order/cancel`, `/v5/order/cancel-batch`, `/v5/order/cancel-all`; Kraken `/0/private/CancelOrder`, `CancelOrderBatch`, `CancelAll`; Coinbase `/api/v3/brokerage/orders/batch_cancel`; OKX `/api/v5/sprd/cancel-order`, `/api/v5/sprd/mass-cancel`; BitMEX `broadcast/canceller.rs` |
| GTT / time-in-force execution semantics | OKX order-type mapping incl. GTD/FOK/IOC (`common/consts.rs:165-171`); Coinbase `ORDER_CONFIG_LIMIT_GTD` etc. (`common/consts.rs:46-53`) |
| Position / leverage / margin management | Bybit `/v5/position/set-leverage`, `/v5/position/switch-mode`, `/v5/position/trading-stop`, `/v5/account/set-margin-mode`; OKX `/api/v5/account/set-position-mode`; Kraken futures equivalents |
| Transfers / convert / repay | Bybit `/v5/account/repay`, `/v5/account/no-convert-repay`, `/v5/user/escrow_sub_members` |
| SAPI / wallet endpoints (Binance) | anything under `get_sapi_base_url` (`common/urls.rs:96`) |
| Whole execution clients | `crates/adapters/<n>/src/execution*.rs`, `websocket/{orders,trading}/` clients, `python/http*.rs` order helpers |

## B6 sources rule (restated)

Official venue docs are the only thing a B6 row cites as fact. An oracle line above is a
*lead* ("Nautilus `bybit/src/http/query.rs` shows `GET /v5/execution/list` — verify at
<official URL>"), never evidence. A row with only an oracle citation stays `TBD`.

## Realign pointer

W0.5 audits live `binance_com` (spot HTTP vs COM Wasm) against
`crates/adapters/binance/src/spot/http/` at this pin. Hosts/paths matching there does not
imply fill-query shape matches — same rule as the `binance-connector-rust` cage.

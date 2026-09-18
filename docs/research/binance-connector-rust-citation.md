# Citation pin — official Binance Rust connector (local clone)

**Fetched:** 2026-09-19 IST  
**Clone:** `/Users/bishnu/binance-connector-rust`  
**Remote:** https://github.com/binance/binance-connector-rust  
**SHA:** `592f16b6bb34ff11d9eb47fbcd956c80be1529ef`  
**Tag / crate:** `binance-sdk` **v70.1.0** (release `592f16b`)  
**Licence:** MIT (`LICENCE`) — cite, do **not** vendor into Station, do **not** add `binance-sdk` to Station `Cargo.toml`.

This tree is OpenAPI-generated HTTP. Station owns the wire. The clone is the **path / host / JSON alias oracle** for named `binance_com` books.

## Hosts (`src/common/constants.rs`)

| Book | SDK const | Prod URL |
|------|-----------|----------|
| `binance-com-spot` | `SPOT_REST_API_PROD_URL` | `https://api.binance.com` |
| `binance-com-usdm` | `DERIVATIVES_TRADING_USDS_FUTURES_REST_API_PROD_URL` | `https://fapi.binance.com` |
| `binance-com-coinm` | `DERIVATIVES_TRADING_COIN_FUTURES_REST_API_PROD_URL` | `https://dapi.binance.com` |
| `binance-com-options` | `DERIVATIVES_TRADING_OPTIONS_REST_API_PROD_URL` | `https://eapi.binance.com` |

Testnet/demo URLs in the same file (`testnet.binancefuture.com`, `demo-fapi.binance.com`) are **not** Station live hosts until B6 names them.

## Paths this clone confirmed (2026-09-19)

| Book | Station call | SDK file |
|------|--------------|----------|
| USDM | `GET /fapi/v3/balance` | `src/derivatives_trading_usds_futures/rest_api/apis/account_api.rs` (`futures_account_balance_v3`) |
| USDM | `GET /fapi/v3/positionRisk` | `…/trade_api.rs` (`position_information_v3`) |
| USDM | `GET /fapi/v1/forceOrders` | `…/trade_api.rs` (`users_force_orders`) |
| USDM | `GET /fapi/v1/ticker/price` | `src/derivatives_trading_usds_futures/rest_api/apis/market_data_api.rs` (`MarketDataApi` / `symbol_price_ticker`). Model `SymbolPriceTickerResponse1` alias `price` description “Price.” Public, **no HMAC**. **Not** `/fapi/v2/ticker/price` this slice. **Not** Options `lastPrice`. |
| Coin-M | `GET /dapi/v1/balance` | `src/derivatives_trading_coin_futures/rest_api/apis/account_api.rs` |
| Coin-M | `GET /dapi/v1/positionRisk` | `…/trade_api.rs` |
| Coin-M | `GET /dapi/v1/forceOrders` | `…/trade_api.rs` |
| Spot | `GET /api/v3/account`, `/api/v3/myTrades`, `/api/v3/exchangeInfo` | `src/spot/rest_api/apis/account_api.rs`, `general_api.rs` |
| Options | `GET /eapi/v1/ticker`, `exchangeInfo`, `openInterest`, `mark`, `depth`, `klines`, `index`, `marginAccount`, `position`, `userTrades` | `src/derivatives_trading_options/rest_api/apis/` |

## JSON aliases this slice copies (USDM)

From `FuturesAccountBalanceV2ResponseInner` / `PositionInformationV3ResponseInner` / `SymbolPriceTickerResponse1`:

- funds: `asset`, `availableBalance` (string). Do not sum `crossUnPnl` into snapshot `unrealized_pnl` (lock: null).
- positions: `symbol`, `positionAmt` (positive long / negative short). `unRealizedProfit` / `entryPrice` / `marginAsset` exist on the model — display copy later; this slice does not blend them into spot WAC.
- last: `SymbolPriceTickerResponse1.price` (string, description “Price.”). TickBook key `{binance-com-usdm}\0{BTCUSDT}`. Never Options `lastPrice`. Never `/fapi/v2/ticker/price` this slice.

## Hard refuse in this crate

`trade_api.rs` also generates `POST /fapi/v1/order` (`new_order`) and siblings. The USDM/Coin-M/options locks refuse TRADE. Do not call those methods from Station. Do not path-depend this crate to “get USDM for free.”

## Realign result (2026-09-19)

Hosts/paths on the thin clients matched this SHA. **Fill-query shape did not.** Audit vs `my_trades` notes in `src/spot/rest_api/mod.rs`:

| Official (this SHA) | Station before this audit | Status after cage |
|---------------------|---------------------------|-------------------|
| `fromId` XOR `startTime`/`endTime` | Wasm honored XOR when pinned; live poller always `from_id: None` | Adapter pages `fromId` after a full 500 row; poller still time-only |
| `startTime`+`endTime` ≤ 24h | `startTime` only | `endTime = start + 24h − 1ms` |
| `commission` + `commissionAsset` | Live Wasm copies both | Hold |
| Quote ≠ always USDT | Native `my_trade_to_broker_fill` hardcoded `USDT` | Quote suffix, same list as Wasm |
| Tick = `PRICE_FILTER.tickSize` | Spot `exchange_info.rs` stores it | Hold for spot; USDM/Coin-M still no live `exchangeInfo` |

Do not treat “hosts match” as “the book is correct.”

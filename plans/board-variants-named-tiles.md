# Plan: Board variants, USDM/Coin-M depth, parked NFO numbers

> Source: founder chat 2026-09-20. Fork B (name then light fapi/dapi **depth** on existing books) + full map (layout now; NFO numbers only after lock lines). Prototype: `station/prototypes/PROTOTYPE-notch-options-dashboard.html` (`OPTIONS_SEEDS` / `CASH_SEEDS` / `LAST_SEEDS`, Edit board, Rail vs Floor). Prior cockpit plan: `plans/nfo-options-cockpit-mosaic.md` (cockpit + rail shipped).

Klines on USDM/Coin-M are **already named and live** (`GET /fapi/v1/klines` / `GET /dapi/v1/klines`, HistoryBook `fapi_klines` / `dapi_klines`, glance `.last` + `.history`). This plan does **not** rebuild them. Remaining futures product is **depth**. NFO candle/payoff/greeks/grid stay dark.

## Architectural decisions

- **Tree:** Station only. Do not clone. Console does not recompute a second book.
- **Surface:** Keep `BarOptionsDeclareSurface` as the only Pre-trade split. OPT* → NFO cockpit; FUT*/unknown → three-zone; dated Binance contract → crypto Options; spot/equity/USDM/Coin-M → cash cockpit. No new routing seam.
- **Confirm:** One LiveBook submit in `BarDeclarationFlowView`. Floor vs rail is dock chrome, not a second declare path. TRADE parked. Station-hosted Notch stays PLAN-only.
- **Board model:** Seed id `cockpit` | `hero` | `focus`. `planDock` is `rail` or `floor`. Tiles are `{kind, x, y, w, h}` from the prototype tables. Mosaic **renders the seed grid**. Seed boards **always rehydrate** from code; custom boards persist extras only.
- **Catalog is book-gated:** NFO/options catalog = session, oi, payoff, depth, chain, plus optional greeks/legs/ladder as **holes**. Spot/equity = session + depth. USDM/Coin-M catalog starts as **session only**; depth is added only after that book’s lock names `GET /fapi/v1/depth` or `GET /dapi/v1/depth`.
- **Futures depth physics:** REST **bounded snapshot** (copy `binance_options_depth.rs`, not spot `@depth`). Public, no HMAC. `market/order_book/bounded_snapshot`. Empty / missing levels → unavailable, never `{bids:[],asks:[]}`. Two owner files (USDM ≠ Coin-M). TickBook/DepthBook key is `{book_id}\0{instrument}` — never letters alone. Never `pricePrecision`. Never TRADE.
- **NFO honesty (unchanged until a later named phase):** Session = `kotak_history_unsupported`. Payoff = `derived/payoff inherits market/option_chain`. Greeks = chips + `pricing_model_unspecified`. `showsStrikeGrid = false`. DualNoBlend INR. No eapi copy. No Black-76. No Yahoo product line. No ÷100 on `dStrikePrice;`.
- **Out of this plan:** crypto Options mosaic, NFO 6-cell glance strip, Live/Post, money-path / PnL owners, new slug / B6, OpenAlgo wrap, pasting the HTML file into Notch.

## Phase 1: Hero, focus, and floor dock

Notch board chrome grows the three prototype seeds. Mosaic layout comes from seed `x/y/w/h`. `focus` docks Plan under the board; `cockpit` and `hero` keep the rail.

## Phase 2: Edit board

Edit-board chrome: catalog chips, dock chip, Done. Validate kinds against the book catalog. Persist custom boards per family. Seed ids always rehydrate from code.

Until depth is named, USDM/Coin-M catalog is `{session}` only.

## Phase 3: Name and light USDM depth

Amend `locks/binance-com-usdm.md`: allow public `GET /fapi/v1/depth`. Then one owner module + Notch glance/tiles.

## Phase 4: Name and light Coin-M depth

Separate lock amendment and a second owner module. Do not reuse the USDM parser.

## Phase 5: NFO numbers — parked behind lock gates (no product code)

Four independent STOPs. An agent that implements any of these without a new lock line has failed the phase.

1. **Session klines** — stay `kotak_history_unsupported`. Cash `historical/details` does **not** licence `nse_fo`. Illegal: Yahoo, eapi klines, compose from quote `ohlc`.
2. **Strike scale + expiry calendar** — `dStrikePrice;` and `lExpiryDate ` stay raw. Illegal: ÷100, OpenAlgo offset, 57500 fixture ladder.
3. **At-expiry identity** — stay `derived/payoff inherits market/option_chain`. Illegal: copy `BarCryptoSettlement` / eapi index, Black-76 “payoff”.
4. **Greek numbers** — stay `pricing_model_unspecified`. Illegal: Black-76 from memory, eapi `mark` on NFO, SPAN as Δ.

`showsStrikeGrid` stays false until (2) is on the lock.

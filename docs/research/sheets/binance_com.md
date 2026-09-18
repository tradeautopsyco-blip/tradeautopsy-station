# B6 · Broker capability sheet — `binance_com`

**Status:** `SIGNED` — research pass 2026-07-24 · **approved for Phase 1+** · market-history amendment 2026-08-26 (public klines / `historical_series`; **not** Kotak)  
**Slug:** `binance_com`  
**Display:** Binance.com (en-IN / global)  
**Dogfood role:** First crypto integration (your venue) — not “only broker forever”  
**Sources:** [binance spot REST](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) · Station [`docs/reference/crypto/binance-global/spot/REST.md`](../../reference/crypto/binance-global/spot/REST.md) (snapshot 2026-07-02; klines re-fetched 2026-08-26) · live code `binance_com_spot_*.rs` · site https://www.binance.com/en-IN

---

| # | Field | Answer |
|---|-------|--------|
| 0 | Slug / venues | `binance_com` · host `https://api.binance.com` · **not** Binance.US (`api.binance.us`). Keys must be created on **binance.com** (en-IN / global), never binance.us. Station live client hardcodes COM (`DEFAULT_BASE_URL`). |
| 1 | Asset class(es) when connected | **crypto_spot** v1 (spot USER_DATA). Margin/futures = refuse v1 |
| 2 | Fetch path | REST poll: `GET /api/v3/myTrades` (fills), `GET /api/v3/account` (balances). Optional later: user-data WS — not required for v1 poll loop. Station already implements REST client |
| 3 | Post-connect options | Spot symbols per `exchangeInfo`; account permissions via apiRestrictions / account flags; environments mainnet (testnet separate refuse unless sheet updated) |
| 4 | Time / volume | myTrades: **symbol required**; `startTime`↔`endTime` max **24 hours**; `limit` default 500 max **1000**; weight 20 (5 with orderId); IP weight 6000/min; 429 rate limit; 418 ban |
| 5 | Order/trade history depth | Per-symbol trade list; not full multi-symbol history in one call. Paginate with `fromId` / time windows |
| 6 | If incomplete history | Incremental sync from last `trade id` per symbol; optional user CSV export; local ledger from first connect for uncovered symbols |
| 7 | Currency | Quote varies by pair (USDT/BTC/…); fees in `commissionAsset` (may ≠ quote). Desk CalcProfile v1: **USD** (flag `quote_not_usd` / `fee_unhandled` when honesty fails) |
| 8 | Calculation factors | qty × price; maker/taker commission; WAC/day PnL in crypto_spot_usd profile; no lots/pips |
| 9 | Compliance | Validate `/sapi/v1/account/apiRestrictions` (or account canWithdraw): **WithdrawDetected** → block save/Start; prefer read or trade-without-withdraw; audit Start/Stop; runtime re-validate on schedule |
| 10 | Secrets shape | HMAC `api_key` + `api_secret` in Keychain; never Console DB as live SoT |
| 11 | Dogfood role | **Enabled** first crypto · your real venue |
| 12 | Refuse list v1 | Binance.US conflation; futures/USDM; withdraw-enabled keys for dogfood; treating COM as US; inventing multi-symbol history in one call |
| 13 | Sources + date | binance-spot-api-docs rest-api.md (fetched 2026-07-24; klines + General API Information re-fetched **2026-08-26**); Station REST.md 2026-07-02 / klines 2026-08-26; code `DEFAULT_BASE_URL = https://api.binance.com` |

---

## Research amendments (2026-07-24)

| Topic | Lock |
|-------|------|
| **COM vs US** | Product + Station code = **COM only**. No US adapter as dogfood. Founder keys for live Start must be COM; Phase 2 connect validates against `api.binance.com`. |
| **Bootstrap symbols** | **Not a fixed founder list.** Adapter derives `{ASSET}USDT` from non-zero account balances (skips stables). Typical dogfood: `BTCUSDT`, `ETHUSDT` when those balances exist. Empty balances → empty fills until funded or explicit symbol config later. |
| **Desk currency** | Always `crypto_spot_usd` for this connection — never INR chrome for COM fills. |

---

## Research amendments (2026-08-26) — market history (`ohlcv` / `historical_series`)

Does **not** change Phase 1 fills (`GET /api/v3/myTrades`). This is **market** `ohlcv` / `historical_series`, not account trade history — **row 5 stays `myTrades`**. Kotak is a **different slug** (`kotak_neo`): no history capability there. Do not invent stitch / Yahoo as a Binance.com capability. Citations fetched 2026-08-26: [rest-api.md](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) (Kline/Candlestick + General API Information + Request Security) · [Market Data Only FAQ](https://github.com/binance/binance-spot-api-docs/blob/master/faqs/market_data_only.md) · [errors.md](https://github.com/binance/binance-spot-api-docs/blob/master/errors.md). Full array indexes: Station [`spot/REST.md`](../../reference/crypto/binance-global/spot/REST.md).

| # | Field | Answer |
|---|-------|--------|
| H1 | Path / host | Public **`GET /api/v3/klines`** on COM host **`https://api.binance.com`**. **Not** `api.binance.us`. Weight **2**. Data Source: Database. |
| H2 | Auth | Security **NONE** (unsigned / public). Heading has no `(USER_DATA)` / `(TRADE)` suffix; Request Security: unspecified ⇒ `NONE` = public market data. **Not HMAC USER_DATA.** Do not attach private credentials (`X-MBX-APIKEY`, `signature`, Keychain secret) — that is **`PrivateCredentialOnPublicCall`**. Market Data Only FAQ: API key is not necessary on this path (also listed on `data-api.binance.vision`). |
| H3 | Intervals | Source enum (case-sensitive): `1s`, `1m`, `3m`, `5m`, `15m`, `30m`, `1h`, `2h`, `4h`, `6h`, `8h`, `12h`, `1d`, `3d`, `1w`, `1M`. Unsupported interval → later S2 code ineligible **`unsupported_interval`**. Exchange reject `-1120 BAD_INTERVAL` does not add intervals. |
| H4 | Range / limit | `limit` default **500**, maximum **1000**. Beyond that documented max → **`unsupported_range`**. No documented max hours between `startTime` and `endTime` on this path (`-1127` exists generally; **not** cited on klines). Empty range → most recent klines up to limit. |
| H5 | Retention | **NOT SPECIFIED IN SOURCE.** How far back klines are kept is not in rest-api.md / Market Data Only FAQ / errors.md (2026-08-26). Do **not** emit **`insufficient_retention`** until a cited source distinguishes it from unspecified. |
| H6 | vs row 5 | Market candles ≠ account `myTrades`. Row 5 remains per-symbol trade list (USER_DATA). Klines identity: family market, capability `ohlcv`, physics `historical_series`. |
| H7 | Kotak | Different slug. `kotak_neo` has **no** `history` / klines capability (FAQ 2026-08-26). Do not copy this path onto Kotak. |
| H8 | Refuse | Place / modify / cancel are **not data**. Yahoo / stitch / caller `source=yahoo` is **not** a Binance.com klines capability. `GET /api/v3/uiKlines` is a presentation sibling, not this S2 path. S2 code has **not** allowlisted `/api/v3/klines` yet. |

---

## Founder sign-off

- [x] Facts match venue posture (COM not US) — Station live host + first-pair lock
- [x] Bootstrap symbol strategy noted: balance → `{ASSET}USDT` (see above)
- [x] Approve for Phase 1+ build

**Signed:** founder research pass (reconciled) **Date:** 2026-07-24

---

## Research amendments (2026-09-19) — official Rust connector as path oracle

Local clone `/Users/bishnu/binance-connector-rust` @ `592f16b6bb34ff11d9eb47fbcd956c80be1529ef` (`binance-sdk` v70.1.0). Pin: [binance-connector-rust-citation.md](../binance-connector-rust-citation.md).

Does **not** change the slug. Default Start stays `binance-com-spot`. Named books `binance-com-usdm` / `binance-com-coinm` / `binance-com-options` are **locks**, not a new B6.

| # | Field | Answer |
|---|-------|--------|
| R1 | Role of the clone | Path / host / JSON-alias **oracle**. Station keeps thin REST clients. **Do not** add `binance-sdk` to Station Cargo. The crate generates TRADE (`POST /fapi/v1/order`). |
| R2 | Row 1 vs named books | Slug v1 connect is still **crypto_spot** USER_DATA. Named USDM/Coin-M books exist on the same slug and are obtain-gated with `book=`. Do not treat row 12 “futures/USDM” as a bar on `binance-com-usdm` obtain. |
| R3 | Testnet | SDK names `https://testnet.binancefuture.com` and `https://demo-fapi.binance.com`. Station live host stays prod. Testnet still **refuse** until this sheet names it. |
| R4 | Wrap | Same as OpenAlgo bar: take official paths, never wrap their generated TRADE surface. |

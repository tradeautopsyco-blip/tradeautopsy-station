# B6 · Broker capability sheet — `binance_com`

**Status:** `SIGNED` — research pass 2026-07-24 · **approved for Phase 1+**  
**Slug:** `binance_com`  
**Display:** Binance.com (en-IN / global)  
**Dogfood role:** First crypto integration (your venue) — not “only broker forever”  
**Sources:** [binance spot REST](https://github.com/binance/binance-spot-api-docs/blob/master/rest-api.md) · Station `docs/reference/crypto/binance-global/spot/REST.md` (snapshot 2026-07-02) · live code `binance_com_spot_*.rs` · site https://www.binance.com/en-IN

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
| 13 | Sources + date | binance-spot-api-docs rest-api.md (fetched 2026-07-24); Station REST.md 2026-07-02; code `DEFAULT_BASE_URL = https://api.binance.com` |

---

## Research amendments (2026-07-24)

| Topic | Lock |
|-------|------|
| **COM vs US** | Product + Station code = **COM only**. No US adapter as dogfood. Founder keys for live Start must be COM; Phase 2 connect validates against `api.binance.com`. |
| **Bootstrap symbols** | **Not a fixed founder list.** Adapter derives `{ASSET}USDT` from non-zero account balances (skips stables). Typical dogfood: `BTCUSDT`, `ETHUSDT` when those balances exist. Empty balances → empty fills until funded or explicit symbol config later. |
| **Desk currency** | Always `crypto_spot_usd` for this connection — never INR chrome for COM fills. |

---

## Founder sign-off

- [x] Facts match venue posture (COM not US) — Station live host + first-pair lock
- [x] Bootstrap symbol strategy noted: balance → `{ASSET}USDT` (see above)
- [x] Approve for Phase 1+ build

**Signed:** founder research pass (reconciled) **Date:** 2026-07-24

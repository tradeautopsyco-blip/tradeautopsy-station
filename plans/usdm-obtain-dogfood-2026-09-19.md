# USDM founder obtain dogfood — 19 Sep 2026 (unsigned)

Loopback agent from **TradeAutopsy Station.app** (Xcode Debug). Named book `binance-com-usdm` USER_DATA slice. Lock: `issues/compliance/locks/binance-com-usdm.md`.

**In:** `obtain` funds / positionbook / forceorder with `book=binance-com-usdm` on live `fapi.binance.com`.  
**Out:** quotes / last / Notch USDM tab / `POST /fapi/v1/order` / testnet / Coin-M / income `REALIZED_PNL`. N1 Confirm/Cancel is a different checklist.

Default Start stays **spot**. Same COM HMAC keys. Enable Futures **reading**. Disable futures TRADE and withdraw.

**Agent SHA:** `tradeautopsy-agent/0.1.0 (11c848f20d23)` · Debug binary in Station.app · dogfood **2026-09-19 ~02:40 IST**

---

## Loopback obtain (while Station is running)

```text
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=funds
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=positionbook
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=forceorder
```

Unsigned obtain is allowed on loopback. Do **not** omit `book=binance-com-usdm` (slug-only obtain stays spot).

| Operation | Result | Notes |
|-----------|--------|--------|
| **funds** | **PASS** `success` | `book_id=binance-com-usdm`; path `/fapi/v3/balance`; 11 holdings (USDT ~1.04, plus USDC/FDUSD/BTC/ETH/BNB/…); `unrealized_pnl` null |
| **positionbook** | **PASS** `success` | `position_count=0`, `rows=[]` (no open USDM size — empty is honest) |
| **forceorder** | **PASS** `success` | inner `status=idle`, `canonical=false`, `events=[]` — **not** `synced` |
| **quotes** (sanity) | **PASS** `unsupported` | `data: null` |
| **slug-only funds** | **PASS** stays spot | `book_id=binance-com-spot`, path `/api/v3/account`, `holdings=[]` — DualNoBlend vs USDM USDT |

Oracle (not a Station dependency):

```bash
binance-cli futures-usds futures-account-balance-v3
binance-cli futures-usds position-information-v3
binance-cli futures-usds users-force-orders --auto-close-type LIQUIDATION
```

Those paths are `GET /fapi/v3/balance`, `/fapi/v3/positionRisk`, `/fapi/v1/forceOrders`.

**Fail if:** spot WAC numbers on this book · INR blend · slug-only obtain paints USDM USDT · TRADE place · testnet host.

**USDM obtain verdict:** **signed** on this Debug rebuild (`11c848f`). Last + named chrome later landed on `36e8b09`. TRADE / Confirm / venue place stay **out**.

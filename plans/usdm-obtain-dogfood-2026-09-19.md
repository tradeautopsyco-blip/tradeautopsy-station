# USDM founder obtain dogfood — 19 Sep 2026 (unsigned)

Loopback agent from **TradeAutopsy Station.app** (Xcode Debug). Named book `binance-com-usdm` USER_DATA slice. Lock: `issues/compliance/locks/binance-com-usdm.md`.

**In:** `obtain` funds / positionbook / forceorder with `book=binance-com-usdm` on live `fapi.binance.com`.  
**Out:** quotes / last / Notch USDM tab / `POST /fapi/v1/order` / testnet / Coin-M / income `REALIZED_PNL`. N1 Confirm/Cancel is a different checklist.

Default Start stays **spot**. Same COM HMAC keys. Enable Futures **reading**. Disable futures TRADE and withdraw.

**Agent SHA (fill after rebuild):** `tradeautopsy-agent/0.1.0 (________)`

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
| **funds** | | `success`; `holdings` from `availableBalance` > 0; empty after drop is `holdings: []` not `data: null`; `unrealized_pnl` null |
| **positionbook** | | `success`; skip `positionAmt` 0; qty `2` must not collapse to `1`; `segment=usdm` |
| **forceorder** | | `idle` if `[]`, `observing` if rows, `unavailable` on fail — **never** `synced` / `complete` / `fresh` |
| **quotes** (sanity) | | `unsupported` on this book |

Oracle (not a Station dependency):

```bash
binance-cli futures-usds futures-account-balance-v3
binance-cli futures-usds position-information-v3
binance-cli futures-usds users-force-orders --auto-close-type LIQUIDATION
```

Those paths are `GET /fapi/v3/balance`, `/fapi/v3/positionRisk`, `/fapi/v1/forceOrders`.

**Fail if:** spot WAC numbers on this book · INR blend · slug-only obtain paints USDM USDT · TRADE place · testnet host.

**USDM obtain verdict:** **unsigned** — fill the table after this Debug rebuild, then mark **signed**.

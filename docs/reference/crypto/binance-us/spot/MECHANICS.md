# Spot Trading — Mechanics Reference

**Exchange:** Binance.US (also applies to Binance Global spot — same mechanics, separate API)  
**Source:** Session PRD (`.github/PRD-today-v1.md`), Binance Trade Analysis FAQ (referenced, not directly quoted in this thread), `binance-us/spot/REST.md`  
**Snapshot date:** 2026-07-02  
**Reviewed:** No  
**TradeAutopsy status:** SHIPPING (Today v1, Slice 1)

---

## 1. What a "position" is

No formal position object exists on spot. A position is **inferred**: the trader's balance of a base asset (e.g. BTC in BTC/USDT) at any point in time. There is no leverage, no borrowed funds (unless explicitly using Margin, out of scope here), no liquidation.

**Flat** = base asset balance from this round-trip's fills returns to zero.

## 2. Position lifecycle (round-trip definition)

A round-trip = inventory path **0 → increases via buys → returns to 0 via sells**, per symbol, per account.

- Opens on first buy fill that establishes non-zero inventory
- Can add to inventory via further buys (averages the cost basis)
- Closes when cumulative sells bring inventory back to exactly zero
- Partial closes are mid-round-trip, not separate round-trips
- If inventory goes to zero then a **new** buy starts a new round-trip

## 3. P&L Calculation — Weighted Average Cost (WAC)

**Confirmed formula (from PRD, sourced from Binance Trade Analysis convention):**

```
Realized PnL = sell_qty × (avg_sell_price − avg_buy_price)
```

- `avg_buy_price` = weighted average of all buy fills contributing to current inventory
- Position average **resets** when inventory returns to flat (zero)
- This is a **performance basis**, not a tax basis — explicitly not FIFO, not tax-lot accounting
- FIFO tax export is a deferred future slice, not part of this formula

**Scope limitation (product decision, v1):** the WAC formula in this document assumes a
USD-pegged quote asset (USDT, USD, USDC, BUSD). For any symbol quoted in a non-USD asset
(e.g. ETHBTC), TradeAutopsy v1 does not convert quote-asset P&L to USD — doing so would
require a fill-time quote-asset price lookup, the same unsolved dependency as the
fee-normalization gap in Section 4. Round-trips on non-USD-quoted pairs are marked
`quote_not_usd: true`, return `realized_pnl_usd: None`, and are excluded from aggregates.
Follow-up: resolving this requires the same `FillTimeFeePriceLookup` already tracked for
Section 4 — a future slice should implement it once and use it for both gaps.

## 4. Fees

- **Net of fees** — realized P&L is calculated after subtracting trading fees
- Fees may be charged in the quote asset, base asset, or BNB (fee discount asset)
- **Fee normalization required:** any fee not already in USD must be converted to USD **at fill time** price, not at reconciliation time
- `NOT SPECIFIED IN SOURCE`: exact fee schema field names from Binance.US `myTrades` response — confirmed fields are `commission` and `commissionAsset` (see `binance-us/spot/REST.md`), conversion logic is TradeAutopsy's own responsibility, not provided by exchange

**Unhandled fee assets (product decision, not exchange-specified):**
When a fee is charged in an asset that is neither the pair's base/quote asset nor a
recognized USD-pegged stablecoin (USDT/USDC/BUSD), TradeAutopsy does not have a fill-time
price source to convert it. Rather than return gross P&L unflagged, the round-trip is marked
`fee_unhandled: true` and excluded from aggregates — same honesty treatment as `unknown_basis`.
Follow-up: `FillTimeFeePriceLookup` (tracked separately) would close this gap for BNB and
other common fee-discount assets.

**Symbol metadata for fee conversion:** base/quote asset identity comes from
`GET /api/v3/exchangeInfo` (`baseAsset`, `quoteAsset` per symbol). Symbol-string parsing is
heuristic fallback only — logged when used; authoritative cache preferred.

## 5. Unknown cost basis

A sell fill with **no corresponding buy history** in TradeAutopsy's synced data (e.g. asset was bought before broker connection, or via a method not synced) has no known cost basis.

**Handling (locked in PRD):**
- Excluded from hero aggregate P&L (would corrupt the number)
- Still shown as a row in the trades table
- Flagged `Unknown basis`
- P&L field shows `—` for that row, not `0` and not an invented number

## 6. Margin / Leverage / Liquidation

**None.** Spot trading on Binance.US has no margin, no leverage, no liquidation risk. Confirmed: Binance.US does not offer margin trading at all — regulatory restriction, not a product gap to model around.

## 7. Funding

**None.** Funding rates are a derivatives concept (see `futures-usdm/MECHANICS.md`). Not applicable to spot.

## 8. Expiry / Exercise

**None.** Spot assets don't expire.

## 9. Session / time concept

No exchange-defined trading session. Binance spot trades 24/7. TradeAutopsy's "session day" is a **product decision** (local Mac calendar day, midnight rollover — locked in Today v1 PRD), not an exchange concept.

## 10. Data required from broker API

Confirmed available from `binance-us/spot/REST.md`:
- `GET /api/v3/myTrades` — `symbol`, `id`, `orderId`, `price`, `qty`, `quoteQty`, `commission`, `commissionAsset`, `time`, `isBuyer`, `isMaker`
- `GET /api/v3/account` — current balances (for reconciliation / sanity check against reconstructed inventory)

**Sufficient for full round-trip reconstruction.** No additional endpoints needed for Spot P&L.

## 11. What TradeAutopsy must store

Per round-trip: `symbol`, `opened_at`, `closed_at`, `avg_entry_price`, `avg_exit_price`, `qty`, `realized_pnl_usd`, `fees_usd` (both `None` when honesty flags apply), `unknown_basis: bool`, `fee_unhandled: bool`, `quote_not_usd: bool`

Per daily snapshot: see PRD `daily_snapshots` fields (`date`, `round_trips_closed`, `net_pnl_usd`, `wins`, `losses`, etc.)

## 12. Known gaps requiring live verification

- Exact `commissionAsset`-to-USD conversion source (use fill-time price of that asset in USDT, needs confirming which endpoint supplies historical price at exact fill timestamp)
- Binance.US `myTrades` retention window — still an open blocker (see `binance-us/spot/CHANGELOG-NOTES.md`)

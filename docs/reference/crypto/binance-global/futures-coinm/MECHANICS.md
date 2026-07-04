# Futures COIN-M — Mechanics Reference

**Exchange:** Binance Global only (`dapi.binance.com`) — NOT available on Binance.US  
**Source:** `futures-coinm/CHANGELOG-NOTES.md`, `futures-usdm/MECHANICS.md` (shared concepts), web search (Tier 4)  
**Snapshot date:** 2026-07-02  
**Reviewed:** No  
**TradeAutopsy status:** NOT BUILDING — reference only

---

## 1. What a "position" is

Same conceptual model as USDⓈ-M Futures (contract exposure, not asset ownership) with one critical difference: **settlement and margin are in the base cryptocurrency itself, not a stablecoin.**

E.g. a BTCUSD_PERP position is margined in BTC, not USDT. This means the trader's margin **value in USD fluctuates with the underlying asset price**, even when the position itself is flat.

## 2. Position lifecycle

Same as USDⓈ-M: inventory-based, `dualSidePosition` hedge mode supported, unified with USDⓈ-M's dual-side setting per the 2026-06-30 architecture merger (see `futures-coinm/CHANGELOG-NOTES.md`).

## 3. P&L Calculation

**NOT SPECIFIED IN SOURCE in this thread.** Structurally different from USDⓈ-M because:

- Inverse contract math: PnL is typically calculated as `contracts × contract_value × (1/entry_price − 1/exit_price)` for inverse contracts (this is general crypto-derivatives knowledge, **not confirmed from an official Binance source in this thread** — must verify against `schema__4_.yaml` before use)
- Settlement currency = the coin itself, meaning "PnL in USD" requires an additional coin-to-USD conversion at settlement/reporting time, compounding the complexity vs USDⓈ-M

**This is the single most error-prone asset class to get wrong** — inverse contract math is unintuitive and easy to implement backwards (long/short PnL sign errors are a known category of bug in inverse-contract engines industry-wide, per general knowledge — not a Binance-specific claim, flagging as caution not fact).

## 4. Fees

Same commission structure pattern as USDⓈ-M, denominated in the settlement coin rather than USDT.

## 5. Margin / Leverage / Liquidation

Same conceptual structure as USDⓈ-M (cross/isolated, leverage brackets) but collateral value itself is volatile since it's held in the underlying crypto. This means:

- Liquidation risk is a function of **both** price movement against the position AND the value of the coin-denominated margin itself
- `NOT SPECIFIED IN SOURCE`: exact liquidation price formula for inverse contracts

## 6. Funding

Same 8-hour-cycle mechanism concept as USDⓈ-M (Tier 4, unconfirmed exact interval from official source). Funding paid/received in the settlement coin, not USDT.

## 7. Expiry / Exercise

Same `contractType` enum as USDⓈ-M: `PERPETUAL`, `CURRENT_QUARTER`, `NEXT_QUARTER`, etc. — COIN-M has historically leaned more on quarterly/dated contracts than USDⓈ-M in practice (general market knowledge, not confirmed from source).

## 8. ⚠️ Architecture merger in progress

Per `futures-coinm/CHANGELOG-NOTES.md`: COIN-M is being integrated into the USDⓈ-M unified architecture, effective 2026-06-30. `dualSidePosition` unified across both. `GET /dapi/v1/pmAccountInfo` deprecated in favor of `GET /fapi/v1/pmAccountInfo`. Any future COIN-M engine work should verify current architecture state — this document may already be stale on that point given the fast-moving merger.

## 9. Why this needs the most caution of any asset class

If TradeAutopsy ever supports COIN-M, the inverse-contract PnL formula must be verified against **actual Binance documentation with worked examples**, not derived from general crypto knowledge. Getting a sign wrong on inverse contract math would show a trader the opposite of their actual P&L — the worst possible failure mode for a discipline/circuit-breaker product.

**Do not implement without a dedicated live doc-pull + worked-example verification pass.**

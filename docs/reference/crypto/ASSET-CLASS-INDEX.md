# Crypto Asset Class Mechanics — Master Index

**Created:** 2026-07-02  
**Governs:** How TradeAutopsy's engine architecture must think about each asset class before building it  
**Standard:** `../CRYPTO-STANDARD.md`

---

## Why this exists

The Today v1 P&L engine (WAC, spot round-trips) is correct **for spot only**. Every other asset class Binance offers has a categorically different P&L shape, risk model, and time concept. This index prevents the mistake of treating "add another asset class" as a parameter change to the spot engine — it isn't. Each one is its own document (`MECHANICS.md`) because each one needs its own engine design, and possibly its own behavioral signal research.

---

## Quick comparison

| Factor | Spot | Margin | Futures USDⓈ-M | Futures COIN-M | Options |
|---|---|---|---|---|---|
| **On Binance.US?** | ✅ Yes | ❌ No | ❌ No | ❌ No | ❌ No |
| **Ownership model** | Own the asset | Own + borrowed | Contract exposure | Contract exposure | Contract right |
| **P&L timing** | Realized on sell | Realized on sell − interest | Continuous mark-to-market | Continuous mark-to-market | Discrete at expiry/exercise |
| **P&L formula basis** | WAC (confirmed) | WAC + interest (formula NOT SPECIFIED) | Mark price based (formula NOT SPECIFIED) | Inverse contract math (formula NOT SPECIFIED — high error risk) | Premium vs settlement (formula NOT SPECIFIED) |
| **Leverage** | None | Up to 20x cross / 10x isolated (Tier 4) | Up to 125x (Tier 4) | Similar to USDⓈ-M (unconfirmed) | N/A for buyers; margin for sellers |
| **Liquidation risk** | None | Yes | Yes | Yes (+ coin-value-of-margin risk) | No (buyers); margin risk (sellers) |
| **Funding/interest** | None | Continuous interest | Periodic funding (~8h, Tier 4) | Periodic funding, paid in coin | None (theta decay instead) |
| **Expiry** | None | None | Optional (dated contracts) | Optional (dated contracts) | **Always** — core mechanic |
| **Session concept** | 24/7, no boundary | 24/7, no boundary | 24/7 + funding windows | 24/7 + funding windows | 24/7 + hard expiry deadline |
| **TradeAutopsy status** | **Shipping (Today v1)** | Reference only | Reference only | Reference only | Reference only |

---

## Reading order if any of these ever get built

1. **Margin** — closest to spot (still "real" asset ownership), smallest conceptual leap. Interest-as-continuous-cost is the main new problem.
2. **Futures USDⓈ-M** — bigger leap (contracts, not assets; continuous mark-to-market; funding; liquidation). USDT-settled keeps the money math simpler than COIN-M.
3. **Futures COIN-M** — same complexity as USDⓈ-M plus inverse contract math and coin-denominated margin. Highest bug risk of the derivatives products — sign errors in inverse math are a known industry failure pattern.
4. **Options** — most structurally different. Discrete settlement, asymmetric risk, theta decay, hard expiry. Likely needs its own behavioral signal taxonomy, not just its own P&L engine.

---

## Hard rule before building any of these

Per `CRYPTO-STANDARD.md` Rule 3 (No Invented Facts): **every formula marked `NOT SPECIFIED IN SOURCE` in the individual `MECHANICS.md` files must be resolved against the actual OpenAPI schema (`schema__X_.yaml` files) or official worked examples before a single line of engine code is written.** General crypto-industry knowledge about "how inverse contracts typically work" is not a citable source — Binance's specific implementation must be confirmed.

This is not bureaucracy. A wrong P&L number in a product whose entire pitch is "trust this number instead of your gut" is worse than no number at all.

---

## Files in this set

- `binance-us/spot/MECHANICS.md` — shipping
- `binance-global/margin/MECHANICS.md` — reference
- `binance-global/futures-usdm/MECHANICS.md` — reference
- `binance-global/futures-coinm/MECHANICS.md` — reference
- `binance-global/options/MECHANICS.md` — reference

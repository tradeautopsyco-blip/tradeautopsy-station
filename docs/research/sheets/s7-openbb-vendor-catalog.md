# S7 — OpenBB as a vendor catalog (not a product)

**Date:** 2026-09-15 IST  
**Status:** research only. **Not a B6 sheet.** Does not enable `obtain(history)` on Kotak.

ADR 0003 + `descriptor.rs` `#365`: OpenBB / ODP adapter labels are forbidden. Yahoo as canonical history already fails Station tests.

## Official OpenBB `equity.price.historical` providers

From OpenBB docs (CLI `--provider` choices): `fmp`, `intrinio`, `polygon`, `tiingo`, `yfinance`. Default is often `yfinance`. OpenBB does not host data.

| Provider | Key? | India NSE cash / NFO rights named here? | Desk |
|----------|------|------------------------------------------|------|
| `yfinance` | no | Yahoo — already `rights_forbid_canonical` | **refuse** |
| `polygon` | yes | US-centric equity candles in OpenBB docs | not a licensed NSE dump; not NFO klines |
| `fmp` | yes | same | not NFO |
| `tiingo` | yes | same | not NFO |
| `intrinio` | yes | same | not NFO |

None of these is a licensed NSE historical product. None is Kotak Trade API history (FAQ 2026-08-26: unavailable).

## Decision

**Skip Tranche 2 product enable.** Keep `kotak-nse-bse-cash` / `kotak-nse-nfo` `obtain(history)` **unsupported**. Fixture `licensed_history` / book `licensed-history` stays the CI gap vendor. Quote stays Kotak. Empty vendor budget → history unavailable.

A later B6 sheet may name one vendor with a Keychain **key** (not a URL), vendor `book_id`, and provenance strip. That sheet will not be called `openbb_*` / `ODP` / `yahoo`.

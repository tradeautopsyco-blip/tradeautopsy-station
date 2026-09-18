# Scoped S8 founder dogfood — 19 Sep 2026 ~01:03 IST

Loopback agent from **TradeAutopsy Station.app** (Xcode Debug). Same process as S6-FO rebuild: `tradeautopsy-agent/0.1.0 (a4f60a0dbabc)`.

**In (this gate):** Harness glance + C1 last on crypto Options chrome (`BarCryptoOptionsDeclareView`). Chip stays **unknown**. History on the bound options book; spot XRPUSDT klines stay off the panel.

**Out:** Settings → Broker pulse/ledger. Console waterfalls / LTP.

---

## Harness (founder shot)

| Surface | Result |
|---------|--------|
| Connected book | Binance.com **Live** |
| Quote chip | **unknown** (amber) — desk last on the bound instrument |
| Instruments | fresh |
| Account | fresh |
| History | **Eligible** — never a last |
| Chrome | Options analytics: `market/quote · option_chain · open_interest`; copy “Spot XRPUSDT klines stay off this panel.” |
| Chain | Live rows; last column paints (e.g. `5`); expiry still raw epoch ms (polish, not this gate) |

Unknown ≠ hole. Last is not `0` from an empty TickBook.

---

## Loopback glance / obtain (`BTC-260925-145000-C`, `binance-com-options`)

| Probe | Result |
|-------|--------|
| `GET /api/station/quote` | **unknown**, `last` present |
| `GET /api/station/depth` | **success**, `bound_levels=1` (thin eapi book, not 5000) |
| `GET /api/station/chain` | **success**, 98 rows |
| `GET /api/station/oi` | **success**, `sumOpenInterest` |
| `GET /api/station/history` | **success**, eapi klines on the options book |
| `GET /api/station/greeks` | **success**, VenuePublished Δ/Γ/Θ/vega strings (S5 API leftover; overlay sighting still optional) |
| obtain `quotes` / `depth` / `history` / `optiongreeks` | **success** on `binance-com-options` |

**Scoped S8 verdict:** **signed**.

Do not checkout `feat/s8-notch-account-chrome` to close this. Monday S3 (COM gap, NFO+options depth together) is a different leftover.

# Derived OHLCV resample — `derived/ohlcv` / `historical_series`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Station-computed resample of one licensed finer OHLCV series (S5 / PRD #358) |
| **Primary source** | [`spot/REST.md`](./REST.md) Klines section · Binance `rest-api.md` Kline/Candlestick (as cited there, fetched 2026-08-26) · Kotak NFO/cash: history **unsupported** |
| **Snapshot date** | 2026-08-31 |
| **Source version** | Station REST.md S2 klines lock 2026-08-26 · matrix `derived/ohlcv` → `HistoricalSeries` |
| **Staleness warning** | Re-verify Binance interval ENUM before treating a new interval as venue-served. Do not invent an aggregation identity from charting folklore. |
| **Author**       | TradeAutopsy Station |

---

> ⚠️ **BLOCKER**
>
> How Station would **build** a coarser bar from a finer licensed series (open/high/low/close/volume/trade-count identity, partial-bar rule, timezone) is **NOT SPECIFIED IN SOURCE** on the Binance kline page or Kotak history (unsupported).
>
> Do **not** implement “first open, max high, min low, last close, sum volume” from memory. Do **not** stitch two venues. Do **not** resample Yahoo. Venue `interval` on `GET /api/v3/klines` wins when the connected broker serves that interval (`market/ohlcv`). This extract stays a hole until a primary names the derived aggregation.

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| Station klines lock | [`spot/REST.md`](./REST.md) “Klines (Candlesticks) — S2 market `ohlcv`” | 2026-08-26 (cited 2026-08-31) | Venue intervals listed. No resample formula. |
| Agent matrix | `agent/src/data/matrix.rs` `known_physics` | 2026-08-31 | `(Family::Derived, "ohlcv")` → `HistoricalSeries`. Also `(Family::Market, "ohlcv")` → `HistoricalSeries`. |
| PRD #358 | Station Data PRD locked decision | 2026-08-25 | “Fetched venue history vs Station-computed derived. Resample from one licensed finer series. Rights inherit.” |
| Kotak S0 | `kotak_neo.s1k.v1` / `kotak_neo.nfo.v1` | 2026-08-31 | `history` **unsupported**. No FO/cash klines. |
| Binance options | `locks/binance-com-options.md` | 2026-08-29 | No klines path this book. |

---

## Concepts

---

### Identity — `derived/ohlcv` / `HistoricalSeries` is not `market/ohlcv`

**Source:** `agent/src/data/matrix.rs` `known_physics`

**Verbatim definition / formula:**

```
(Family::Market, "ohlcv") => HistoricalSeries
(Family::Derived, "ohlcv") => HistoricalSeries
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Market ohlcv | venue klines (COM `GET /api/v3/klines`) | `historical_series` |
| Derived ohlcv | Station-computed series from a licensed finer input | `historical_series` |
| Aggregation identity | **NOT SPECIFIED IN SOURCE** | — |

**Gaps (NOT SPECIFIED IN SOURCE):**

- OHLC roll-up rule, volume sum vs quote-volume, unused kline field 11, partial last bar.

> **OUR INTERPRETATION**
>
> - Same capability id `ohlcv`, different **family**. Do not persist derived as canonical market history.
> - Do not register `derived/iv_smile` or a horizon-σ id.

---

### Venue interval wins — Binance.com spot klines ENUM

**Source:** [`spot/REST.md`](./REST.md) intervals table (verbatim from Binance rest-api.md, fetched 2026-08-26)

**Verbatim definition / formula:**

```
seconds | 1s
minutes | 1m, 3m, 5m, 15m, 30m
hours   | 1h, 2h, 4h, 6h, 8h, 12h
days    | 1d, 3d
weeks   | 1w
months  | 1M
```

Unsupported `interval` → ineligible `unsupported_interval`. Source error `-1120 BAD_INTERVAL`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Path | `GET /api/v3/klines` | public, `api.binance.com` |
| Security | NONE | no HMAC |

**Gaps (NOT SPECIFIED IN SOURCE):**

- How to compose `1h` from stored `1m` bars when the caller could have asked the venue for `1h`.

> **OUR INTERPRETATION**
>
> - If the desk wants `1h`, obtain **market** `ohlcv` with `interval=1h`. That is S2, not this extract.
> - Station resample of finer→coarser is **this** extract and stays unimplemented until a primary names the roll-up.

---

### One licensed finer series — no stitch, no Yahoo

**Source:** PRD #358; REST.md “Not this capability” + S2 quality gaps; `extract_history` Yahoo → `rights_forbid_canonical`

**Verbatim definition / formula:**

PRD:

```
Fetched venue history vs Station-computed derived. Resample from one licensed finer series. Rights inherit.
```

REST.md:

```
Stitch / Yahoo / binance-public-data bulk dump as a Binance.com REST capability.
```

marked **NOT SPECIFIED IN SOURCE (do not invent for the S2 quality layer)**.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Yahoo | research segment; never canonical | ineligible `rights_forbid_canonical` |
| Two-venue stitch | forbidden | product fence |
| Kotak history | unsupported | hole |

> **OUR INTERPRETATION**
>
> - Named input = one licensed COM kline series (or inherit-dark).
> - Kotak books: history unsupported → this extract’s hole (`kotak_history_unsupported`), not a Yahoo fill.
> - Options book: no klines path → hole (`options_history_unspecified`).

---

### Aggregation identity remains blank

**Source:** Full kline section of REST.md. No “resample”, “aggregate”, or “compose from 1m” sentence.

**Verbatim definition / formula:**

```
NOT SPECIFIED IN SOURCE.
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Open of coarse bar | **NOT SPECIFIED** as first finer open | — |
| High / low | **NOT SPECIFIED** as max/min | — |
| Close | **NOT SPECIFIED** as last finer close | — |
| Volume | **NOT SPECIFIED** as sum | — |
| Rights inherit | PRD #358 | if ever computed, inherit finer rights |

> **OUR INTERPRETATION**
>
> - Charting folklore is not a source. Leave `extract_resample` unimplemented (`resample_aggregation_unspecified`) when the finer series is lit on the spot book.

---

## Verification Checklist

- [x] Venue kline intervals copied from REST.md (2026-08-26 source), not from memory.
- [x] `derived/ohlcv` exists on the matrix; no new capability id.
- [ ] A primary that names Station’s coarser-bar aggregation (OHLCV + volume + trades).
- [ ] Kotak still has no history capability.
- [ ] Do not persist derived resample as canonical `market/ohlcv`.
- [ ] Do not stitch Yahoo.

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Implement first-open / max-high / min-low / last-close / sum-volume | Not in REST.md kline section | Left unimplemented |
| Resample because COM already stored 1m | Venue also serves `1h` etc. | Venue interval wins; derived stays hole |
| Fill Kotak from Yahoo | Yahoo is research; Kotak history unsupported | `kotak_history_unsupported` |
| Copy NFO greeks day-count onto bars | Different capability | No |
| Register a new resample id | Matrix already has `derived/ohlcv` | Use that identity; dark envelope |

No memory fills for OHLC aggregation. Gaps stay `NOT SPECIFIED IN SOURCE`.

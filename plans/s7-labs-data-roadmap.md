# Plan: S7 vendor runtime + Labs-ready data types

> Founder 2026-09-16: phase 1 markets = **India · crypto · forex**. Workflow Labs (A1) needs strategy context (FII/DII, AMFI, economy, news, …). Station must be **ready** so Labs is an extract client, not a second fetch spine.
>
> Not Health **H4** (helpline copy). Vendor program = **Data S7** + **T6′** Keychain vendor keys. Market-plane `DATA-ELEMENTS.md` C* is history; **Station Data PRD/TRD + ADR 0002/0003 win**.

## Why this is in S7 (not “later”)

S7 is **quotas, health, non-broker vendors; vendor cannot preempt a healthy broker**.

That is exactly the socket Workflow Labs needs:

- A strategy that reads FII/DII is a **typed obtain**, not Console scraping NSE.
- The edge is **Station-owned extracts** with provenance + rights, so Labs cannot silently become desk last / Kill / Today PnL.
- If we wait until A1 UI, Labs will grow its own Yahoo/OpenBB waterfall (the thing S8 deletes).

**S7 ships Health + live obtain of non-broker types.** You can *quote* (obtain) FII/DII, AMFI, RBI, history, … at S7 — fetch-on-demand, TET, provenance. It is not an empty socket that Labs fills later.

**Aggregation is later.** S7 does not warehouse, stitch, or average vendors. Store / coverage ledger / compose / CONSENSUS come after obtain+health work.

## Architectural decisions

- **Two `product_use` values** on every extract: `desk` (canonical / execution-adjacent) vs `labs` (research). Same obtain envelope. Different rights.
- **`labs` may never**: drive Kill, Today PnL, canonical history, or pretends to be Kotak/Binance last.
- **`desk`**: connected broker first (ADR 0002). Vendor only for a **named gap** (Kotak history).
- **`labs`**: specialized adapters for domains the broker does not have (FII/DII, AMFI, MOSPI, RBI, news). Direct route. Still Keychain keys, allowlisted hosts, per-vendor meter, TET fetchers.
- **No ODP process.** Borrow TET. No `openbb_*` adapter labels. No nseindia.com scrape.
- **Labs is a later client** (Fact Plane Q5). This plan makes the **data plane** ready; it does not un-park Algo Labs fat or build Labs UI in S7.
- **Forex / MCX / CDS / USA-GBR-SGP FX** stay **books** (Skill A + lock). S7 does not mint those markets. It can still declare **labs** types that do not need that book (e.g. RBI reference rate ≠ CDS futures last).

```text
Workflow Labs / TAI          Notch / Today / Kill
        │                            │
        │ product_use=labs           │ product_use=desk
        ▼                            ▼
              Station obtain + rights
        ┌─────────────┴──────────────┐
        │  pick_route + quotas+health │  ← S7 runtime
        ├──────────────┬─────────────┤
        │ Brokers      │ Specialized │
        │ Kotak/Binance│ vendors     │
        │ desk + gaps  │ labs types  │
        └──────────────┴─────────────┘
```

---

## Phase 0 — S7: Health + non-broker obtain (you can quote it)

**Job:** At S7 you **obtain** non-broker data and **see vendor health**. Same path Notch/Labs will use. T6′ = keys in Keychain + Health rows (meter, what/why/error). Not Health H4 copy-schema as the product.

**Quote here means obtain** — typed extract (`history`, `fii_dii`, `amfi_nav`, …). Broker `quotes` (Kotak LTP) stays the connected broker. A vendor does not preempt that last.

**Done when (runtime)**

- [ ] `obtain` uses `pick_route` (Kotak history gap tracer in `s7-source-route.md`)
- [ ] Keys not URLs; vendor `book_id`; per-vendor quota; budget 0 → that vendor dark, broker quote still live
- [ ] Envelope: `product_use` + `provenance_adapter_id`
- [ ] Health: one row per non-broker binding (up / exhausted / unsupported) — T6′

**Done when (you can quote the types — not fixture-only)**

S7 is not complete with only `licensed_history` in CI. First live obtains, `product_use=labs` unless a lock says desk:

- [ ] India history (one B6 vendor) — cash/NFO candles
- [ ] At least one specialized India type (FII/DII **or** AMFI **or** RBI policy) so Labs has a real node
- [ ] Crypto: existing broker obtains unchanged; perps **funding** if that binding is in this tranche

More India types (shareholding, MOSPI, announcements) ride the **same** S7 obtain/health path; they are extra adapters, not a new stage.

**Not S7:** warehouse, gap-fill jobs, averaging two vendors, CONSENSUS merge.

---

## Phase 1 — Desk gaps on shipping books (India + crypto)

**User stories:** Candles and perps market reads the desk already owes.

| Slice | Type | Book | product_use | Vendor |
|-------|------|------|-------------|--------|
| 1.1 | `market/ohlcv` history | cash + NFO | `desk` if licensed; else `labs` strip | **One** B6 history vendor (not Yahoo) |
| 1.2 | Delivery % | extra on cash ohlcv | `labs` (desk chart may display labeled) | same history vendor / bhav |
| 1.3 | India VIX | ohlcv or index last | `labs` until a lock says desk | history vendor or broker if published |
| 1.4 | USDM/Coin-M last, depth, klines, **funding** | perps books | `desk` for last/depth if locked; funding `desk` or `labs` | `binance_com` fapi/dapi — **new capability `funding`** |
| 1.5 | NFO funds finish | NFO | `desk` | Kotak `seg=FO` (already locking) |

---

## Phase 2 — India Labs pack (the strategy edge)

Broker does **not** have these. Specialized adapters. `product_use=labs`. Official or licensed sources only.

| Slice | Type | Family | Source (one package each, Tradier/Fed/SEC pattern) |
|-------|------|--------|------------------------------------------------------|
| 2.1 | FII/DII daily cash + F&O | `economic` or `news` | NSDL / NSE **licensed** dump — not scrape |
| 2.2 | Shareholding (promoter / FII / DII / pledged) | `fundamentals` | BSE/NSE filings product |
| 2.3 | Bulk / block deals | `market` snapshot | exchange file / licensed |
| 2.4 | Exchange announcements | `news` | NSE/BSE announcement API or licensed |
| 2.5 | AMFI NAV | `reference` or `fundamentals` | AMFI official |
| 2.6 | RBI policy corridor (repo/SDF/MSF/CRR/SLR) | `economic` | RBI DBIE |
| 2.7 | G-sec / T-bill / par curve | `economic` | RBI / FBIL / CCIL |
| 2.8 | MOSPI CPI / IIP / GDP | `economic` | MOSPI / data.gov.in |

**Labs edge (examples, not Kill):** “skip longs if FII sold cash 3 days”; “size off India VIX”; “only CNC names with delivery > X”; “fade gap vs G-sec move”; “MF flow via AMFI.”

Each slice: B6 sheet + host fence + TET fetcher + obtain noun + rights `research_fetch` (+ `display` for Labs). **No compose into canonical candles.**

---

## Phase 3 — Crypto Labs pack

| Slice | Type | Notes |
|-------|------|--------|
| 3.1 | Funding history | series, not one tick |
| 3.2 | Perps OI | if not already on desk obtain |
| 3.3 | Force-order tape as **labs observation** | already lossy; Labs may read; still never Kill/PnL |
| 3.4 | Exchange announcements / listing news | `news`, Binance official only if locked |

No CoinMarketCap/Yahoo as canonical. No OpenBB.

---

## Phase 4 — Forex readiness (plane, not FOREX-READY)

No FX book until Skill A + lock. S7 still prepares **types** so the day the book exists, Labs does not invent a path.

| Slice | Type | Needs FX book? |
|-------|------|----------------|
| 4.1 | Quote **bid/ask** + pip on master | Yes — Skill A |
| 4.2 | FX session calendar | Yes |
| 4.3 | FX ohlcv | Yes, or labs vendor after lock |
| 4.4 | Official USDINR **reference** (RBI/FBIL) | **No** — `economic` labs; **must not** paint as CDS last |

**Not in this phase:** USA / GBR / SGP FX **labels** (`is_active`, TAI enums, FOREX-READY). Those are claims without an engine.

---

## Phase 5 — Labs extract client (not S7 code freeze)

After Phase 0–2 exist on Station:

- Workflow Labs (A1) calls Station obtain with `product_use=labs`
- Console does not hold vendor keys
- Fact Plane extract, not a new pipe

T6′ Health: per-vendor meter + helpline (what/why/error) when a Labs node is dark.

---

## Explicitly out of S7 / phase 1 Labs (specific, not a pile)

| Item | Why out |
|------|---------|
| **Congress.gov / Fama-French** | US legislature / academic factors. Not India/crypto/FX strategy on this desk. Add only if a Labs node names them. |
| **Technical-as-vendor** (RSI from FMP) | `derived` on a series we already have. Vendor RSI is unexplained truth. |
| **MCX** | REFERENCE book. Commodity last is a **new book**, not an S7 extra on cash/NFO. |
| **India CDS / USDINR futures** | Registry **N-A**. Not NFO. Not RBI reference. |
| **USA / GBR / SGP FX labels** | Claim without lock. DualNoBlend / MEASURE lie. |
| **Yahoo / OpenBB / ODP runtime** | ADR 0003 + #365. Catalog for research only. |
| **nseindia.com scrape** | Legal/rights. Licensed or official APIs. |
| **Mixing labs FII into Today PnL / Kill** | S4 / money law. |

---

## Vendor list we actually add (not 32)

| Adapter | Role | Phases |
|---------|------|--------|
| `kotak_neo` | Connected broker India | already |
| `binance_com` | Connected broker crypto | already + 1.4 / 3 |
| `licensed_history` | CI gap fixture | 0 |
| **TBD B6 history** | India cash/NFO ohlcv | 1.1 |
| `amfi` | NAVs | 2.5 |
| `rbi` | Policy + G-sec | 2.6–2.7 |
| `mospi` | CPI/IIP/GDP | 2.8 |
| `nse_licensed` / exchange files | FII/DII, bhav, announcements | 2.1–2.4 |
| FX venue TBD | After lock | 4 |

One package per source. Keys in Keychain. TET. Health per binding.

---

## Later — aggregation (not S7)

S7 is **fetch-on-demand**, like OpenBB TET: obtain → vendor HTTP → envelope. Repeat the call, hit the vendor again (plus any short in-memory book you already have: TickBook last, HistoryBook for licensed Binance klines).

Aggregation is a **store/compose** program after S7 obtain+health exist:

| Later piece | Meaning | Law |
|-------------|---------|-----|
| Coverage ledger | What ranges you **have** vs what you asked | DATA-ELEMENTS A4 |
| Persist `historical_series` | One series per instrument+venue+interval | Rights `store`. Yahoo/unofficial NSE stay no-store |
| Gap jobs / schedule | Fill missing bars | A5–A7. Not silent stitch across vendors |
| Derived bars | 5m from 1m | `derived`, inherit rights (#358) |
| CONSENSUS | Compare two sources, flag divergence | **Do not average.** TRD: live CONSENSUS deferred |
| Labs join | FII/DII next to RELIANCE candles in a workflow | Join at **read** with two provenances; not one mashed candle |

Until then: you can **quote** (obtain) every S7 type live. You cannot yet treat Station as Historify.

---

## Tracer order (demoable)

0. pick_route + Health meter + fixture (proves the socket)  
1. **S7 live quote:** named India history obtain + Health green/red for that vendor  
2. **S7 live quote:** FII/DII (or AMFI/RBI) obtain `labs` + Health row  
3. More specialized adapters on the same path  
4. Perps funding  
5. FX types only with a lock  
6. **After S7:** aggregation (store, coverage, gap jobs, CONSENSUS compare)

S7 exit is (1)+(2)+Health, not (6).

---

## Exit (S7)

- Health shows each non-broker vendor (quota / dark / error).  
- You can **obtain** India history and at least one Labs type without Console keys.  
- Kotak LTP still Kotak. Vendor never preempts a healthy broker quote.  
- No warehouse required. Aggregation is the next program.

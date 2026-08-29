# B6 · Broker capability sheet — `kotak_neo`

**Status:** `SIGNED` — research pass 2026-07-24 · **approved for Phase 1+** · market-data amendment 2026-08-26 (quotes + scrip master; **no history**) · NFO-book amendment 2026-08-28 (`kotak-nse-nfo`; cash still refuse FO)  
**Slug:** `kotak_neo`  
**Display:** Kotak Neo  
**Dogfood role:** First equities integration (India)  
**Sources:** [Kotak Neo Trade API guide](https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/) (07 Nov 2025; re-fetched 2026-08-26, page updated 22 May 2026) · [historical data FAQ](https://www.kotakneo.com/support/how-do-i-get-historical-data/) (re-fetched 2026-08-26) · [static IP FAQ](https://www.kotakneo.com/support/are-static-ips-provided-by-kotak/) · migration guide Trade Book → `{{baseUrl}}/quick/user/trades` · Console prior art `lib/brokers/kotak-neo-connector.ts` · `kotak-token-manager.ts` · GitHub [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) `trade_report` / `Quotes.md` / `Scrip_Master.md` · Station [india/kotak-neo REST](../../reference/india/kotak-neo/REST.md) (2026-08-26)

---

| # | Field | Answer |
|---|-------|--------|
| 0 | Slug / venues | `kotak_neo` · login `https://mis.kotaksecurities.com` · trading base from login (`gw-napi.kotaksecurities.com/trading` fallback in Console) |
| 1 | Asset class(es) when connected | **equities** cash v1 — segments **`nse_cm` / `bse_cm` only** (CNC+MIS). Named book **`kotak-nse-nfo`** may FO. Cash still refuses F&O. |
| 2 | Fetch path | REST after session: `GET {BASE_URL}/quick/user/trades` (trade book / fills), `/quick/user/orders`, holdings/positions/limits. Login: `POST /login/1.0/tradeApiLogin` (TOTP) → `POST /login/1.0/tradeApiValidate` (MPIN). No Binance-style HMAC poll. v1 sync = **read-only fills** (no place/modify/cancel) |
| 3 | Post-connect options | Equity cash; products **CNC + MIS** ingested. **v1 poll remains trade book only** (`GET {baseUrl}/quick/user/trades`). Quotes/subscribe exist for a **later s1k** slice (REST `quote_type` + HSM subscribe) — not required for v1 poll. Scrip master: documented file-paths (see 2026-08-26 amendment) |
| 4 | Time / volume | Trade book has **no date-range params**. Official: historical market/session history **unavailable** via Trade API. Rate/latency marketed under 50ms order path. Static IPs **not** provided by Kotak (ISP/VPN self-managed if dashboard requires whitelist — optional, verify at Phase 2 dogfood) |
| 5 | Order/trade history depth | **Current session / day trade book** via `/quick/user/trades` (SDK `trade_report()` with no dates; Console `fetchTodayTrades`). Per-order history exists (`/quick/order/history` by order_id) — **not** multi-year customized history. Treat as **incomplete vs full ledger** |
| 6 | If incomplete history | Incremental from first Station connect; optional broker statements/CSV; **do not pretend** API gives full history |
| 7 | Currency | **INR**; statutory charges in INR; no crypto fee-asset model |
| 8 | Calculation factors | qty (shares) × price INR; product **CNC** (delivery) vs **MIS** (intraday square-off); session calendar NSE/BSE; CalcProfile `equities_inr_cash`; lot/scrip multipliers from scrip master when needed |
| 9 | Compliance | Human TOTP + MPIN to mint session; session expiry (`stCode` 1003 / KotakSessionExpiredError) → reconnect UX (pause sync, don’t Kill); Console models expiry as **end of trading day IST** (`getEndOfTradingDay` ≈ 15:30 IST); no auto-refresh; SEBI/algo policy awareness (product later — v1 = read-only sync); Keychain-only secrets; no withdraw-key analogue |
| 10 | Secrets shape | Dashboard **consumer/access token** + session **trade_token** + **Sid** + **base_url** + expiry in Keychain. **Not** long-lived api_secret HMAC. Console DB path is **legacy — fence (B5)** |
| 11 | Dogfood role | **Enabled** first equities |
| 12 | Refuse list v1 | **Cash book:** F&O segments (`nse_fo`, …); products **NRML / CO / BO** as v1 dogfood; order placement via Station sync; Console DB as live vault; treating Kotak like HMAC key broker; assuming USD desk math; inventing multi-year trade history from `/quick/user/trades`; inventing market **candle / klines / `history`** (FAQ 2026-08-26). Named book **`kotak-nse-nfo`** is a separate lock (not cash). |
| 13 | Sources + date | kotakneo.com API guide 2025-11-07 (re-fetched 2026-08-26, page updated 22 May 2026); historical-data FAQ + static-IP FAQ 2026-07-24 / FAQ re-fetched 2026-08-26; Console connector 2026-07-24; SDK trade_report (no date params); SDK Quotes.md + Scrip_Master.md + `settings.py` 2026-08-26 |

---

## Research amendments (2026-07-24) — open questions closed

| # | Question | Decision |
|---|----------|----------|
| 1 | History depth | **Day/session trade book only.** No API historical trades. Recovery = incremental + statements/CSV. |
| 2 | Products | **CNC + MIS** (cash). Refuse NRML/CO/BO/F&O for v1. |
| 3 | IP whitelist | Kotak does **not** provide static IPs. No whitelist required by product; if founder’s API app later binds IPs, document in connect UX at Phase 2. |
| 4 | Session TTL | Treat as **daily** — expire by market close IST (Console prior art); JWT may be shorter; always honor `stCode` 1003 → reconnect. Live TTL confirm at Phase 2 dogfood. |
| 5 | Dual desk (Notch) | See FULL pack §5.1 — **per-connection strips, no FX blend**. |
| 6 | TOTP/MPIN | Protocol **signed** from official guide + Console prior art. Live account login exercised in **Phase 2** connect dogfood (not a Phase 1 blocker). |

---

## Research amendments (2026-08-26) — market data (quotes + scrip master)

Does **not** change Phase 1 fills (`GET {baseUrl}/quick/user/trades`). Does **not** add a `history` capability. Citations: FAQ and Trade API guide fetched 2026-08-26; SDK git `8cee5bda` 2026-08-26. Full paths: [`docs/reference/india/kotak-neo/REST.md`](../../reference/india/kotak-neo/REST.md) · [`WEBSOCKET.md`](../../reference/india/kotak-neo/WEBSOCKET.md).

| # | Field | Answer |
|---|-------|--------|
| M1 | Scrip master | **GET** `{BASE_URL}/script-details/1.0/masterscrip/file-paths` — official guide (updated 22 May 2026, fetched 2026-08-26) and SDK `PROD_URL["scrip_master"]`. Returns downloadable CSV **paths**. Cash v1: use **`nse_cm` / `bse_cm` only**. Refuse F&O files (`nse_fo`, `bse_fo`, `cde_fo`, `mcx_fo`, …) even when listed in the sample `filesPaths`. |
| M2 | Quotes / subscribe | Exist for **later s1k**. REST: **GET** `{baseUrl}/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}` (`quotes_neo_symbol_api.py`). `quote_type`: `all`, `depth`, `ohlc`, `ltp`, `oi`, `52w`, `circuit_limits`, `scrip_details` (SDK Quotes.md / README). Subscribe: `wss://mlhsm.kotaksecurities.com` with session `Authorization` + `Sid` (`urls.py` / `NeoWebSocket.py`). **v1 poll remains tradebook.** |
| M3 | Quotes vs history | Quotes = LTP and **session** OHLC as a **one-bar / latest_state** slice (`quote_type=ohlc` / live feed `op`/`h`/`lo`/`c`). **Not** `historical_series`. Do not invent klines. |
| M4 | Historical **market** data | **Unavailable.** FAQ (fetched 2026-08-26): “Historical data is unavailable at the moment. This feature is not allowed for this platform.” **No `history` capability.** **S2:** Kotak `obtain(history)` remains **unsupported** — Binance.com `GET /api/v3/klines` is a different slug ([`binance_com.md`](./binance_com.md)). |
| M5 | Auth | **PrivateRead session attach** for quotes/master — not `AuthMode::Public`, not HMAC. REST quotes HTTP in SDK: `Authorization` = consumer key (README: access token without completing login). `scrip_master()` and `subscribe()` wrappers require `edit_token` + `edit_sid`. Trade book still uses Sid/Auth/`sId` (unchanged). |
| M6 | Shared session / rate | **NOT SPECIFIED IN SOURCE.** HTTP 429 is listed on Quotes.md / Scrip_Master.md (“Too many requests”); numeric window and sharing with `/quick/user/trades` are unspecified. |
| M7 | CSV download host | Sample `filesPaths` use `https://lapi.kotaksecurities.com/wso2-scripmaster/…`. Method/auth for GET of those URLs: **NOT SPECIFIED IN SOURCE** (`scrip_master_api.py` does not download). Slice A HTTP allowlist must not assume this host without a cited download call. |
| M8 | Place / modify / cancel | **Not data.** Remain refuse for Station sync (row 12). |

---

## Research amendment (2026-08-27) — NFO master (N3 header CLOSED 2026-08-28; cash still refuse FO)

Does **not** flip row 1 (cash `nse_cm` / `bse_cm` only) or row 12 (cash refuse `nse_fo`, …). Does **not** change Phase 1 fills (`GET {baseUrl}/quick/user/trades`). Fills stay cash-only. **N3 “did not download `nse_fo.csv`” is superseded 2026-08-28** — header lives on lock `/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md`. Cash refuse of FO files on the **cash book** remains (M1). Full write-up: [`docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md`](../../reference/india/kotak-neo/NFO-SCRIP-MASTER.md).

| # | Field | Answer |
|---|-------|--------|
| N1 | Hole | Pre-trade strike grid that paints a fixture 57500 ladder while NFO master is refused is cheating. Dark state is **no contracts** (empty / unavailable). |
| N2 | What `filesPaths` lists | SDK `Scrip_Master.md` sample includes `nse_fo.csv`, `bse_fo.csv`, `cde_fo.csv`, `mcx_fo.csv`. Listing ≠ cash-book allowlist. Row 12 / M1 still refuse those files on the **cash book**. |
| N3 | Official FO CSV schema | **Specified on the lock** `/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md` (live unsigned GET `nse_fo.csv` 2026-08-28). Token `pSymbol` (no `pToken`). Lot `lLotSize` and `iLotSize`. Remaining blockers: strike scale, expiry epoch offset — on the lock, not invented here. Cash columns (`pSymbol` token, `pSymbolName` ticker) stay cash-only (REST.md 2026-08-27). |
| N4 | Matrix identities | Already on the matrix: `reference/expiry` (BoundedSnapshot), `reference/option_symbol` (BoundedSnapshot). Lot size is a **field on the contract row**, not a capability id. `kotak_neo.s1k.v1` does **not** claim expiry / optionsymbol / optionchain. |
| N5 | Extracts today | **Do not exist** for NFO expiry, option-symbol resolve, or FO lot size. `market/option_chain` glance extract is already an unavailable hole. |
| N6 | Future honest envelopes | Complete expiry list **or** empty/unavailable. BANKNIFTY 57500 PE 29 Sep → tradable id **or** empty. Lot size from the master row **or** unavailable — never hardcoded 30. Chain table with ghost strikes while master is refused is forbidden. |
| N7 | Still unspecified | BANKNIFTY lot quantity; weekly expiry weekday; strike scale; expiry epoch/offset — remaining blockers live on the lock, not invented here. Token `pSymbol` and lot `lLotSize`/`iLotSize` are specified (N3 / lock 2026-08-28). OpenAlgo `process_kotak_nfo_csv` is third-party, not official schema. |

---

## Research amendment (2026-08-28) — named book `kotak-nse-nfo`

Lock: `/Users/bishnu/issues/compliance/locks/kotak-nse-nfo.md` (fetch 2026-08-28 IST). Slice 0 lock-only. Does **not** enable NRML on cash. Does **not** allowlist `_fo.csv` on the cash book. Does **not** rewrite row 1 to FO-everywhere.

| # | Field | Answer |
|---|-------|--------|
| F1 | Cash v1 (unchanged) | Segments **`nse_cm` / `bse_cm`**. Products **CNC + MIS**. Cash still refuses **`nse_fo` / NRML / CO / BO**. |
| F2 | Named book | **`kotak-nse-nfo`** may segment **`nse_fo`**. Products **NRML + MIS** scoped (FNO-Index / stock futures / index derivatives). **MIS/BO refused on stock options** (Kotak support 2026-08-28). **CNC on FO = NOT SPECIFIED** — do not enable. |
| F3 | FO CSV header | **Specified on the lock** (live unsigned GET `nse_fo.csv` 2026-08-28). Token **`pSymbol`** (no `pToken`). Lot **`lLotSize`** and **`iLotSize`**. Remaining blockers: strike scale, expiry epoch offset — on the lock, not invented here. |
| F4 | Cash FO files | M1 / row 12 still refuse `_fo.csv` on the **cash book**. Named-book allowlist is the lock, not cash. |

---

## Founder sign-off

- [x] Login flow protocol verified (official guide + Console `tradeApiLogin` / `tradeApiValidate`)
- [x] History depth answered (day/session only — amend rows 4–5)
- [x] Products/segments refuse list confirmed (CNC+MIS · cash only · no F&O)
- [x] Approve for Phase 1+ build (Rust port from Console prior art)
- [x] 2026-08-28 named book `kotak-nse-nfo` amendment (cash still refuse FO; NFO book may `nse_fo` per lock)

**Signed:** founder research pass (reconciled) **Date:** 2026-07-24 · NFO-book amendment **2026-08-28**

# B6 · Specialized labs vendor — `amfi`

**Status:** `SIGNED` — labs NAV obtain 2026-09-17 IST  
**Slug:** `amfi`  
**Display:** AMFI (Association of Mutual Funds in India)  
**Role:** SpecializedNonBroker labs vendor. **Not** a shipping broker. **Not** FII/DII. **Not** RBI.  
**product_use:** `labs` on every obtain (success and dark). Labs must not pretend this is Kotak last.  
**book_id:** `amfi-nav` — never `kotak-nse-bse-cash` / `kotak-nse-nfo` / `licensed-history`

This sheet is the lock. Do not invent NAVs. Do not scrape `nseindia.com`. Do not use Yahoo.

---

## Official source (fetch 2026-09-17 IST)

| What | URL | Fetch |
|------|-----|--------|
| NAV download page (AMFI site) | https://www.amfiindia.com/net-asset-value/nav-download | 2026-09-17 IST |
| Daily all-scheme NAV text file | **https://www.amfiindia.com/spages/NAVAll.txt** | 2026-09-17 IST |

**Host fence (one host):** `www.amfiindia.com` only. Refuse `nseindia.com`, Yahoo, Kotak, Binance, and any other adapter host.

AMFI’s NAV download page names the text-format report. `GET /spages/NAVAll.txt` on `www.amfiindia.com` returned the published file body on 2026-09-17 IST (header + scheme rows dated **16-Sep-2026**). Old-format download is documented on that page as available only until 30 September 2026; this sheet locks the **current** semicolon file served at the URL above.

---

## File shape (as published — not invented)

First line of `NAVAll.txt` (fetched 2026-09-17 IST):

```
Scheme Code;ISIN Div Payout/ ISIN Growth;ISIN Div Reinvestment;Scheme Name;Plan;Option;Net Asset Value;Date
```

Published snippet used as the obtain fixture (one scheme row, not a 100-fund list):

```
Scheme Code;ISIN Div Payout/ ISIN Growth;ISIN Div Reinvestment;Scheme Name;Plan;Option;Net Asset Value;Date

Open Ended Schemes(Children’s Fund - Childrens' Fund)

Axis Mutual Fund

135762;INF846K01WO1;-;Axis Children's Fund;Direct Plan;Growth Option;29.5870;16-Sep-2026
```

NAV `29.5870` and date `16-Sep-2026` are copied from the official file. Do not substitute a made-up scheme list.

---

| # | Field | Answer |
|---|-------|--------|
| 0 | Slug / host | `amfi` · host **`www.amfiindia.com`** · path **`/spages/NAVAll.txt`** |
| 1 | Asset class | Mutual-fund **NAV reference** (labs). Not cash last. Not NFO last. Not a new MarketType / country. |
| 2 | Fetch path | Unsigned **GET** `https://www.amfiindia.com/spages/NAVAll.txt`. Public. No Keychain broker session. |
| 3 | Auth | **Public**. No private credential attach. |
| 4 | Identity | Family `fundamentals` · capability `nav` · physics `historical_series` (matrix-legal; Reference does not allow HistoricalSeries / LatestState). Noun `amfi_nav` is **not** an OpenAlgo execution noun. |
| 5 | product_use | Always **`labs`**. Never desk. Never Kotak last. |
| 6 | Rights | `research_fetch` only. No compose into canonical candles. No Kill. No Today PnL. |
| 7 | Refuse | `nseindia.com` scrape · Yahoo · OpenBB/ODP · FII/DII · RBI · Kotak `pick_route` gap · shipping books `kotak-nse-bse-cash` / `kotak-nse-nfo` / `licensed-history` |
| 8 | Sources + date | amfiindia.com NAV download page + `NAVAll.txt` fetched **2026-09-17 IST** |

---

## Law

- SpecializedNonBroker: obtain `adapter=amfi&operation=amfi_nav` is a **direct** labs route. Do not go through Kotak gap `pick_route`.
- Kotak quotes stay `kotak_neo` when AMFI is present.
- Labs may never drive Kill or Today PnL.
- Do not invent NAVs. Numbers come from the official file (or a fixture snippet of that file).

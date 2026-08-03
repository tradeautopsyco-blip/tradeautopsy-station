# B6 · Broker capability sheet — `kotak_neo`

**Status:** `SIGNED` — research pass 2026-07-24 · **approved for Phase 1+**  
**Slug:** `kotak_neo`  
**Display:** Kotak Neo  
**Dogfood role:** First equities integration (India)  
**Sources:** [Kotak Neo Trade API guide](https://www.kotakneo.com/investing-guide/trading-account/kotak-neo-trade-api-guide/) (07 Nov 2025) · [historical data FAQ](https://www.kotakneo.com/support/how-do-i-get-historical-data/) · [static IP FAQ](https://www.kotakneo.com/support/are-static-ips-provided-by-kotak/) · migration guide Trade Book → `{{baseUrl}}/quick/user/trades` · Console prior art `lib/brokers/kotak-neo-connector.ts` · `kotak-token-manager.ts` · GitHub Kotak-Neo SDK `trade_report`

---

| # | Field | Answer |
|---|-------|--------|
| 0 | Slug / venues | `kotak_neo` · login `https://mis.kotaksecurities.com` · trading base from login (`gw-napi.kotaksecurities.com/trading` fallback in Console) |
| 1 | Asset class(es) when connected | **equities** cash v1 — segments **`nse_cm` / `bse_cm` only**. API also supports F&O — **refuse F&O in v1** |
| 2 | Fetch path | REST after session: `GET {BASE_URL}/quick/user/trades` (trade book / fills), `/quick/user/orders`, holdings/positions/limits. Login: `POST /login/1.0/tradeApiLogin` (TOTP) → `POST /login/1.0/tradeApiValidate` (MPIN). No Binance-style HMAC poll. v1 sync = **read-only fills** (no place/modify/cancel) |
| 3 | Post-connect options | Equity cash; products **CNC + MIS** ingested. Scrip master CSV via masterscrip file-paths when needed later. Quotes/subscribe exist — not required for v1 poll |
| 4 | Time / volume | Trade book has **no date-range params**. Official: historical market/session history **unavailable** via Trade API. Rate/latency marketed under 50ms order path. Static IPs **not** provided by Kotak (ISP/VPN self-managed if dashboard requires whitelist — optional, verify at Phase 2 dogfood) |
| 5 | Order/trade history depth | **Current session / day trade book** via `/quick/user/trades` (SDK `trade_report()` with no dates; Console `fetchTodayTrades`). Per-order history exists (`/quick/order/history` by order_id) — **not** multi-year customized history. Treat as **incomplete vs full ledger** |
| 6 | If incomplete history | Incremental from first Station connect; optional broker statements/CSV; **do not pretend** API gives full history |
| 7 | Currency | **INR**; statutory charges in INR; no crypto fee-asset model |
| 8 | Calculation factors | qty (shares) × price INR; product **CNC** (delivery) vs **MIS** (intraday square-off); session calendar NSE/BSE; CalcProfile `equities_inr_cash`; lot/scrip multipliers from scrip master when needed |
| 9 | Compliance | Human TOTP + MPIN to mint session; session expiry (`stCode` 1003 / KotakSessionExpiredError) → reconnect UX (pause sync, don’t Kill); Console models expiry as **end of trading day IST** (`getEndOfTradingDay` ≈ 15:30 IST); no auto-refresh; SEBI/algo policy awareness (product later — v1 = read-only sync); Keychain-only secrets; no withdraw-key analogue |
| 10 | Secrets shape | Dashboard **consumer/access token** + session **trade_token** + **Sid** + **base_url** + expiry in Keychain. **Not** long-lived api_secret HMAC. Console DB path is **legacy — fence (B5)** |
| 11 | Dogfood role | **Enabled** first equities |
| 12 | Refuse list v1 | F&O segments (`nse_fo`, …); products **NRML / CO / BO** as v1 dogfood; order placement via Station sync; Console DB as live vault; treating Kotak like HMAC key broker; assuming USD desk math; inventing multi-year trade history from `/quick/user/trades` |
| 13 | Sources + date | kotakneo.com API guide 2025-11-07; historical-data FAQ + static-IP FAQ 2026-07-24; Console connector 2026-07-24; SDK trade_report (no date params) |

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

## Founder sign-off

- [x] Login flow protocol verified (official guide + Console `tradeApiLogin` / `tradeApiValidate`)
- [x] History depth answered (day/session only — amend rows 4–5)
- [x] Products/segments refuse list confirmed (CNC+MIS · cash only · no F&O)
- [x] Approve for Phase 1+ build (Rust port from Console prior art)

**Signed:** founder research pass (reconciled) **Date:** 2026-07-24

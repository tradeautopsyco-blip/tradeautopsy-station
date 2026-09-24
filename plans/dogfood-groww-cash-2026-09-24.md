# Dogfood — `groww` cash (`groww-nse-bse-cash`)

**Status:** DRAFT — unsigned, founder executes on a live account (catalog stays **Planned** until this record is signed)
**Lock:** `/Users/bishnu/issues/compliance/locks/groww-nse-bse-cash.md` (must be SHIPPING before drills)
**B6:** `/Users/bishnu/issues/brokers/sheets/groww.md` (`SIGNED` 2026-09-24 IST)
**ADR:** `docs/adr/0014-groww-checksum-session.md` (`ACCEPTED` — no-browser checksum; **150/24h mint cap is the binding constraint**)

> Docs only. No invented venue facts — every drill cites its B6 row. Loops back to ADR 0014 on any STOP.

## Preconditions

- [ ] B6 `SIGNED` (`/Users/bishnu/issues/brokers/sheets/groww.md` — rows 11/25 exits + 4/5/9/16/22 read fully)
- [ ] ADR 0014 `ACCEPTED` (shape A approval-checksum only; STRING timestamp D1; no-browser — no redirect/loopback/OAuth machinery; TOTP shape B deferred, dashboard-token shape C human-only per B6 rows 10/16)
- [ ] Bindings landed: AuthScheme `GrowwChecksumSession`, blob `groww_checksum_session` (`api_key` + `api_secret` vault-only + `access_token` + per-mint `expiry` + `tokenRefId`/`sessionName`; B6 rows 14/15, Z5-locked)
- [ ] Adapter built: first book NSE/BSE cash CNC+MIS, CalcProfile `equities_inr_cash` (B6 rows 1/8/17/21); REST poll only, page_size ≤25 until proven (B6 row 4 intra-doc conflict); mandatory `Authorization: Bearer` + `Accept` + `X-API-VERSION: 1.0` on every call (B6 rows 2/10)
- [ ] Lock SHIPPING for the first book (B6 row 8: trade rows carry NO per-trade charges — only aggregate `brokerage_and_charges` in margin payloads, funds-level, never per-fill PnL input; venue-computed `realised_pnl` display-only)
- [ ] Catalog lists `groww` as **Planned** only (B6 row 18; no `Enabled` flip before sign-off)
- [ ] Agent running on `127.0.0.1:9137` (release build)
- [ ] Live Groww account (production; no testnet/sandbox/paper per B6 row 23)
- [ ] **ACTIVE ₹499/mo Trading API subscription — dogfood cannot proceed without it** (B6 row 0: "Having an active Trading API Subscription" is a prerequisite on both intros; official FAQ "₹499 + taxes per month" — proof-of-access prerequisite like Dhan consent-app issuance)
- [ ] **THE key-entry step (no browser, no redirect, no callback):** API key + secret issued at the Groww Cloud API Keys page ("Requires daily approval on Groww Cloud Api Keys Page", B6 row 10), entered once into Swift key-entry → Keychain blob via signed loopback → agent mints (ADR 0014). There is NO redirect URL to register, no loopback question, no OAuth flow — "OAuth 2.0" occurs exactly once as an SDK-intro bullet with no endpoint (B6 row 10).
  - **STOP:** if any step demands a browser/redirect/callback invention (public relay, custom scheme, OAuth mining) → **STOP all drills, do not invent a flow** → **reopen ADR 0014**.

## Drills (B6 rows 11/25 / Z11)

| # | Drill | Steps | Pass when (cite B6) |
|---|-------|-------|---------------------|
| 1 | Checksum mint drill: key-entry → mint → `/order/list` poll green | Key-entry (`api_key` + `api_secret` → Keychain) → host `POST https://api.groww.in/v1/token/api/access` header `Authorization: Bearer {api_key}` + JSON `{"key_type": "approval", "checksum": hex(SHA256(secret + timestamp)), "timestamp": "<epoch-seconds-STRING>"}` (timestamp valid 10 min) → `{"token", "tokenRefId", "sessionName", "expiry" (ISO), "isActive"}` → `GET /order/list?segment=CASH&page=&page_size=` (≤25) poll with `Authorization: Bearer {token}` + `Accept` + `X-API-VERSION: 1.0` | Fills pipeline starts: day order book (open/pending/executed) appears in Station sync; INR at insert; `expiry` field recorded verbatim per mint (rows 2/10/16). **STRING timestamp** — official table `timestamp \| String`, worked body `"1719830400"`, all 4 snippets hash strings; SDK int diverges, docs win (D1, row 10). **STOP:** mint storm / repeated re-mint loop burns the 150/24h cap toward lockout → **STOP, fix the single-flight + reuse-live-token guard** (ADR 0014; row 16). |
| 2 | Expiry drill | Let per-mint `expiry` (returned ISO field — fixed TTL hours NOT SPECIFIED IN SOURCE, row 16) arrive, or force invalid (revoke on Cloud API Keys page). Observe GA005 ("User not authorised") / 401-class responses. Attempt re-mint INSIDE the 150/24h budget (single-flight, reuse live token first). | Pause + reconnect prompt (re-mint re-POST, stateless — no refresh endpoint exists), **never Kill** (row 9). Dead-token predicate (GA005/401-class) locked in this file at dogfood (Z6). **STOP:** auth-failure yields anything other than pause + reconnect, or tempts a mint storm → record + reopen ADR 0014 if lifecycle assumption breaks. Concurrent-token invalidation NOT SPECIFIED (row 9) — record observation, assume nothing. |
| 3 | Fan-out budget discipline | Run full fills pipeline: order-list (paged ≤25) → filter executed/filled → per-order `GET /order/trades/{groww_order_id}?segment=&page=&page_size=` (≤50) fan-out, plus any positions/holdings/margin reads — ALL inside the shared **Non-Trading 20/s + 500/min** type budget (limits apply per type, not per API — exhausting one API rate-limits the whole type, row 4). Pace ≤10/s burst, ≤300/min sustained (ADR 0014 headroom). | No sustained 429-class errors; 429 → **backoff-and-resume**, never abort-and-fabricate (row 4). Cadence logged; a ~200-order day (~110 calls) fits one minute, a ~1000-order day paces across ≥2 min (ADR 0014 math). GA004 (entity does not exist) on a per-order call surfaces as a named per-order gap — never a synthetic fill (rows 4/5/6). Status/detail endpoints stay OUT of the fills loop (ADR 0014). |
| 4 | Day-only honesty | Use ONLY `GET /order/list` ("history of orders executed for the day") + per-order trades ("retrieve all the trades assigned to that order"). Record that NO day trade-book and NO date-ranged trade-history endpoint exists on any fetched page (row 5). Tell the statements/CSV recovery story for pre-connect history (user-supplied broker statements/CSV + local ledger from first connect, row 6). Incremental day-order cursor from first connect; second poll must not widen cursor into fake history. | Proven scope recorded in this file: day-only, no backfill claimed. Candles (intraday 3mo / daily ~3y / weekly full) never counted as trade history — market data only (row 5). **STOP:** any temptation to claim multi-day/ranged/unbounded history without proof → refuse (row 5). |
| 5 | Product-verbatim watch | Capture `product` verbatim on every order/trade/position row. Expect `CNC` (every example) + `MIS` (margin splits `cnc_margin_used`/`mis_margin_used`, row 3). Watch for `INTRADAY`/`MARGIN`: no official product-enum table exists (D5 partial, rows 3/8). | `product` recorded verbatim, no silent defaults (OpenAlgo GTC→DAY / INTRADAY→MIS mappings refused, row 8). **Either** `INTRADAY`/`MARGIN` evidenced on live rows → **lock amendment** (record exact strings + endpoint + date here, amend Z9 lock) **or** still absent → **keep refusing** unevidenced strings (rows 3/8/11). Order types wire `LIMIT`/`MARKET`/`SL` only; OpenAlgo `STOP_LOSS_*` refused (D6, row 3). |
| 6 | Kill drill | With W0.7/Z7 hosts live, trigger Kill for slug `groww`. | L3 blocks both row-22 CONFIRMED hosts: `api.groww.in` (mint + every `/v1` call) + `growwapi-assets.groww.in` (instrument CSV). Socket host (`wss://socket-api.groww.in`) confirm-or-drop at Z7 BEFORE this drill — if confirmed it dies too; if still oracle-only it stays out (row 22). Slug-scoped: other brokers unaffected. |
| 7 | Strips-no-blend proof | Connect Groww INR alongside Kotak/Zerodha/Dhan/Upstox/Fyers INR + USD strip. | Groww INR is a **separate strip** — N INR books, no aggregation hero; USD strip untouched (row 24, DualNoBlend law). First live proof: all strips live with no blend. |

Notes:
- Per-fill math: `quantity` × `price` per trade row (`groww_trade_id`, `exchange_trade_id`, `trade_date_time` + `created_at` ISO, `settlement_number`); **zero per-trade charges expected** — trade rows carry no brokerage/tax fields (row 8). Reconcile against lock; aggregate `brokerage_and_charges` is funds-level only.
- Timestamps: order/trade/user/mint-`expiry` ISO 8601; mint `timestamp` STRING epoch seconds; LTP `last_trade_time` epoch millis; candles epoch seconds (row 8).
- Entitlements: `GET /user/detail` (`ucc`, `nse_enabled`, `bse_enabled`, `ddpi_enabled`, `active_segments`) checked as validation equivalent + auth probe (rows 2/9); `COMMODITY`-in-segments ignored — commodity via APIs explicitly unsupported, CASH+FNO only (row 1).
- Static IP: NOT SPECIFIED IN SOURCE — no mention anywhere (contrast Dhan's explicit exemption); do not perform IP ops, record any venue demand here (row 9).
- No fund-withdrawal or transfer endpoint on fetched pages — no WithdrawDetected equivalent; v1 read-only (row 9).
- Refuse list (row 12) stays refused throughout: execution verbs (create/modify/cancel), smart/advance orders (OCO/GTT), margin calculator POST, option chain + greeks, WS/streaming as sync path, historical-candles as trade history, OAuth redirect login, synthetic trades on 404, order-ID-prefix segment heuristic, silent product/validity defaults, secrets in Wasm/Console, NRML/CO/BO/MTF on cash book, FNO/commodity/currency/MCX books, paper-as-live.

## Sign-off checklist (Planned → Enabled flip)

- [ ] Drill 1 green: key-entry → checksum mint (STRING timestamp) → `/order/list` poll (no-browser posture proven; 150/24h guard held)
- [ ] Drill 2 green: `expiry`-driven re-mint inside 150/24h → pause + reconnect, never Kill (GA005/401-class predicate locked here)
- [ ] Drill 3 green: Non-Trading type-budget discipline held across fills fan-out (20/s + 500/min shared; 429 backoff-and-resume; cadence logged; GA004 → named gap, never synthetic)
- [ ] Drill 4 green: day-only honesty — no backfill claimed; statements/CSV recovery story recorded; cursor-never-widens holds
- [ ] Drill 5 green: `product` verbatim watch resolved — INTRADAY/MARGIN either evidenced (lock amended) or still absent (refusal kept)
- [ ] Drill 6 green: Kill blocks row-22 hosts (`api.groww.in` + `growwapi-assets.groww.in` + socket host iff Z7-confirmed), slug-scoped
- [ ] Drill 7 green: INR strip live with no blend (alongside Kotak/Zerodha/Dhan/Upstox/Fyers INR + USD)
- [ ] No STOP fired, or every STOP resolved via ADR 0014 revisit (record resolutions here)

**Signed:** —
**Date:** —
**Enabled flip:** after sign-off, one ticket: catalog `Planned` → `Enabled` (agent + Swift). No flip on a DRAFT.

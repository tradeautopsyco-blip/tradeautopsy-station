# Dogfood — `dhan` cash (`dhan-nse-bse-cash`)

**Status:** DRAFT — unsigned, founder executes on a live account (catalog stays **Planned** until this record is signed)
**Lock:** `/Users/bishnu/issues/compliance/locks/dhan-nse-bse-cash.md` (must be SHIPPING before drills)
**B6:** `/Users/bishnu/issues/brokers/sheets/dhan.md` (`SIGNED` 2026-09-24 IST)
**ADR:** `docs/adr/0009-dhan-consent-session.md` (`ACCEPTED` — loopback accepted on faith; this dogfood proves registrability)

> Docs only. No invented venue facts — every drill cites its B6 row. Loops back to ADR 0009 on any STOP.

## Preconditions

- [ ] B6 `SIGNED` (`/Users/bishnu/issues/brokers/sheets/dhan.md` — rows 11/25 exits + 4/5/9/16/22 read fully)
- [ ] ADR 0009 `ACCEPTED` (consent-only shape A; PIN+TOTP shape B deferred per B6 rows 10/16)
- [ ] Bindings landed: AuthScheme `DhanConsentSession`, blob `dhan_consent_session` (B6 rows 14/15, Z5-locked)
- [ ] Adapter built: first book NSE/BSE cash CNC+INTRADAY, CalcProfile `equities_inr_cash` (B6 rows 1/8/17/21)
- [ ] Lock SHIPPING for the first book (charge rates locked; B6 row 8 in-band fields vs lock rates)
- [ ] Catalog lists `dhan` as **Planned** only (B6 row 18; no `Enabled` flip before sign-off)
- [ ] Agent running on `127.0.0.1:9137` (release build)
- [ ] Live Dhan account (production; no testnet/sandbox/paper per B6 row 23)
- [ ] **THE loopback-probe step:** Redirect URL registered **exactly** in the Dhan app settings (web.dhan.co → API key generation):
  `http://127.0.0.1:9137/api/daemon/broker/dhan/callback`
  - B6 rows 10/16: registrability of `http://127.0.0.1:<port>/…` is NOT SPECIFIED IN SOURCE (official page says only "Enter … Redirect URL … and Postback URL"). ADR 0009 accepted loopback on faith; this registration attempt IS the proof.
  - **STOP:** if Dhan refuses the loopback URL (validation error, https-only rule, support says no) → **STOP all drills, do not invent a workaround** (no public relay, no custom scheme, no PIN+TOTP substitution without its own ADR row) → **reopen ADR 0009**.

## Drills (B6 rows 11/25 / Z11)

| # | Drill | Steps | Pass when (cite B6) |
|---|-------|-------|---------------------|
| 1 | Consent login → consume (GET) → `/v2/trades` poll green | begin: `POST /app/generate-consent?client_id=` (host presents `dhanClientId` + `app_id`/`app_secret` from vault) → `consentAppId` → browser `GET /login/consentApp-login?consentAppId=` → user completes login → 302 to registered Redirect URL with `?tokenId=` → daemon consumes via **GET** `/app/consumeApp-consent?tokenId=` → `accessToken` + `expiryTime` (ISO IST) → `GET /v2/trades` poll | Fills appear in Station sync; INR at insert; products `CNC`/`INTRADAY` verbatim on NSE_EQ/BSE_EQ only (rows 2/8/10/16). **STOP:** consume verb other than GET, or `tokenId` shape mismatch → reopen ADR 0009 (D1 verdict, row 10). |
| 2 | Expiry drill | Let `expiryTime` (returned ISO IST field — consent-token duration NOT stated beyond it, row 16) arrive, or force re-login (clear session). Observe `DH-901` / `807` / `808` / `809`. | Pause + reconnect prompt (clear session, re-initiate login), **never Kill** (row 9). Renew (`GET /v2/RenewToken`) is Dhan-Web-tokens-only — do NOT call it on a consent token (row 16). **STOP:** expired token yields anything other than documented re-login path → record + reopen ADR 0009 if lifecycle assumption breaks. |
| 3 | Rate discipline | Poll within official per-second law: Order 10/s, **Data 5/s**, **Quote 1/s**, Non-trading 20/s (+ day caps: order 1000/hr, 7000/day; data 100k/day); no weight system. | No sustained `DH-904` / `805`; `805` ("too-many-requests, may result in the user being blocked") treated as immediate back-off warning; no storm after errors (row 4). Cadence logged. |
| 4 | Ranged-backfill honesty | Prove (not claim) max lookback on `GET /trades/{from-date}/{to-date}/{page}` (pass `0` default): widen `from` until venue stops returning rows; record proven oldest date. Second poll must not widen cursor into fake history. Tell the statements/CSV recovery story for pre-connect history (user-supplied broker statements/CSV + local ledger from first connect). | Proven range recorded in this file; cursor-never-widens holds; day book (`/orders`, `/trades` = "in a day") + ranged history + date-ranged `/ledger` each used inside proven bounds; candles (`/charts/*`) never counted as trade history (rows 2/5/6). **STOP:** any temptation to claim multi-year/unbounded history without proof → refuse (row 5). |
| 5 | Per-trade charges reconciliation | Pull ranged trade-history rows; reconcile in-band `sebiTax`, `stt`, `brokerageCharges`, `serviceTax`, `exchangeTransactionCharges`, `stampDuty` against lock rates (day `/trades` rows carry NO charges — ranged endpoint is the charge source). | Every charge field ties to the lock within tolerance or is variance-logged; no currency unit assumed beyond lock (rows 7/8 — settlement/fee-asset currency NOT SPECIFIED IN SOURCE, lock-time decision). `crossCurrency` rows refused on the cash book (row 7). |
| 6 | Kill drill | With W0.7/Z7 hosts live, trigger Kill for slug `dhan`. | L3 blocks all four row-22 hosts: `api.dhan.co`, `auth.dhan.co`, `api-feed.dhan.co`, `api-order-update.dhan.co` (WS hosts die too though WS is refused as a sync path, row 12). Slug-scoped: other brokers unaffected. |
| 7 | Strips-no-blend proof | Connect Dhan INR alongside Kotak/Zerodha/Upstox/Fyers INR + USD strip. | Dhan INR is a **separate strip** — N INR books, no aggregation hero; USD strip untouched (row 24, DualNoBlend law). |

Notes:
- Static-IP ops: none needed — whitelisting is Order-Placement-only per official docs; fetching order/trade details is exempt (row 9). Do not perform IP set/modify/get.
- Entitlements: `GET /profile` (`activeSegment`, `ddpi`, `mtf`, `dataPlan`) checked as validation equivalent (row 9); Data-APIs-unsubscribed (`DH-902`/`806`) → pause with subscribe guidance, not Kill (row 4).
- No fund-withdrawal endpoint exists on fetched pages — no WithdrawDetected equivalent; v1 read-only (row 9).
- Refuse list (row 12) stays refused throughout: execution verbs, convert/exit-all, margin calculators, forever/GTT, super/conditional, WS sync, postbacks sync, MARGIN/CO/BO/MTF on cash book, FNO/currency/commodity/IDX_I books, paper-as-live.

## Sign-off checklist (Planned → Enabled flip)

- [ ] Drill 1 green: registered Redirect URL → consent → consume GET → `/v2/trades` poll (loopback REGISTRABLE — ADR 0009 accept-condition discharged)
- [ ] Drill 2 green: expiry/re-login → pause + reconnect, never Kill (`DH-901`/`807` observed or forced path proven)
- [ ] Drill 3 green: rate discipline held (10/5/1 rps; `805` warning honored; cadence logged)
- [ ] Drill 4 green: max lookback PROVEN on ranged endpoint + cursor-never-widens + statements/CSV recovery story recorded
- [ ] Drill 5 green: per-trade charges reconciled vs lock (ranged endpoint in-band fields)
- [ ] Drill 6 green: Kill blocks all 4 row-22 hosts, slug-scoped
- [ ] Drill 7 green: INR strip live with no blend (alongside Kotak/Zerodha/Upstox/Fyers INR + USD)
- [ ] No STOP fired, or every STOP resolved via ADR 0009 revisit (record resolutions here)

**Signed:** —
**Date:** —
**Enabled flip:** after sign-off, one ticket: catalog `Planned` → `Enabled` (agent + Swift). No flip on a DRAFT.

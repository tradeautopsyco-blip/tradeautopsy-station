# Station + Notch — LIVE full trade-flow audit
**Date:** 2026-09-27 Asia/Calcutta  
**Method:** Signed Wire v1 against running Debug Station agent `127.0.0.1:9137` (PID 58060). **Not unit tests.**  
**App:** TradeAutopsy Station Debug `177d9405878f` / agent 0.1.0  

## Scoreboard (live)

| Feature / flow step | Verdict | Evidence |
|---|---|---|
| Agent up + health | **WORKING** | `GET /api/daemon/health` 200, status ok, uptime ~3h |
| Kotak Neo broker sync | **WORKING** | `sync-state` synced; fills/funds/holdings/positions/instruments **fresh**; active slug `kotak_neo` |
| Binance.com spot fill poll | **WORKING (empty)** | sync-hint: `binance-com-spot` polled, fill_count=0 |
| Kotak cash fill poll | **WORKING (empty)** | `kotak-nse-bse-cash` polled, fill_count=0 |
| Today desk (INR cash) | **WORKING (empty day)** | Today 200, learningBaseline=true, hero all null, 0 trades |
| Positions | **WORKING (flat)** | open positions [] ; kill_switch_active false |
| Notch live-state poll | **WORKING** | barFeaturesActive true, sync_state GREEN, pending_declaration null, plan_state empty |
| Recent trades toolbar | **WORKING (empty)** | kotak_neo_wasm synced, trades [] |
| Station manifest | **WORKING** | binance_com spot books list quotes/instruments/tradebook/… |
| Instrument search (Kotak) | **WORKING** | RELIANCE → nse_cm token 2885 @ 1226; INFY found |
| Quote while Kotak active (BTCUSDT) | **BROKEN / wrong book** | HTTP 200 but `status:unavailable`, provenance `kotak_neo` — crypto quote not routed to Binance |
| Index glance | **BROKEN / empty** | status unavailable |
| **Plan declare (full flow)** | **BROKEN — auth** | `POST /api/daemon/bar/declare` → **401 Authentication required** (Console Station Bearer) |
| Declarations week list | **BROKEN — auth** | GET declarations → 401 |
| Loss-limits (Settings) | **BROKEN — auth** | GET profile/loss-limits → 401 |
| Stop-me / clear | **BROKEN — auth** | POST stop-me → 401 |
| Cancel declaration | **BLOCKED** | No declaration id (declare never succeeded) |
| Working / armed live plan | **BLOCKED** | Depends on successful declare + fills |
| Debrief / Match / Patterns / Fidelity | **BLOCKED** | Need closed declared trip / Console bar brain |
| Kill audit log | **WORKING** | GET kill-switch/audit 200 (prior dismiss entry present) |
| Console auth begin | **BROKEN — auth** | POST `/api/daemon/auth/begin` → 401 |
| Station Caller token in Keychain | **PRESENT but stale?** | Item `TradeAutopsy` / `station_caller_tokens` exists (mdat 2026-09-23); bar routes still 401; SSE `auth_state` count = **0** |

## Full trade-flow result

**Cannot complete Open → Plan → declare → Working → Debrief end-to-end right now.**

Blocker: Console identity (Station Caller Bearer) is not accepted by `/api/bar/v1/*` proxies. Wire loopback HMAC works; upstream brain auth fails.

Broker plumbing for Kotak cash is otherwise healthy (sync green, scrip search works, Today INR profile loaded). No fills today on either shipping book, so even after auth there is no closed trip to debrief until a real round-trip.

## What you need to unblock

1. Re-sign into TradeAutopsy inside Station (refresh Station Caller JWT).
2. Keep Kotak Neo as active book **or** switch Start desk to Binance.com spot for a crypto round-trip.
3. Then re-run: Open → Plan declare → place/fill at broker → Working → Debrief.

## Artifacts

- Live JSON dumps: `/tmp/ta-live-flow/`
- Earlier code/unit inventory (not the ask): `plans/NOTCH-FEATURE-AUDIT-2026-09-27.md`

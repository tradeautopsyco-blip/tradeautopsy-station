# Station + Notch — LIVE recheck after reconnect
**Date:** 2026-09-27 Asia/Calcutta (recheck)
**Method:** Wire v1 HMAC against Debug agent `127.0.0.1:9137` (live processes). Not unit tests.

## Headline

- **Station app / agent wire:** WORKING
- **Console Station Caller JWT:** EXPIRED (`aud=station`, exp 2026-09-26 20:04 UTC) — refresh token present but short opaque string; session proof still 401
- **Kotak Neo Keychain credentials:** PRESENT (`prod` / connection `…0099`)
- **Active broker sync:** STARTED
- **Full Open→Plan→Working→Debrief:** STILL BLOCKED on Console auth

## Scoreboard

| Feature / flow | Verdict | HTTP | Evidence |
|---|---|---|---|
| Agent health | **WORKING** | 200 |  |
| Console station session | **AUTH** | 401 | Console station/session HTTP 401 Unauthorized: {"success":false,"error":{"code":"INTERNAL_ERROR","message":"Invalid or expired token"}} |
| Notch live-state | **WORKING** | 200 | barFeaturesActive=True notch={'pending_declaration': None, 'plan_state': '', 'sync_state': 'GREEN'} |
| Today desk | **WORKING** | 200 | degraded=sync_unavailable learningBaseline=True |
| Positions / kill flag | **WORKING** | 200 |  |
| Kill audit | **WORKING** | 200 |  |
| Toolbar recent trades | **WORKING** | 200 |  |
| Capture outbox | **WORKING** | 200 |  |
| Kotak Keychain vault present | **WORKING** | 200 | prod.kotak_neo.00000000-0000-4000-8000-000000000099 |
| Kotak sync/start | **WORKING** | 200 |  |
| Broker sync-state after start | **WORKING** | 200 | slug=kotak_neo capabilities={'fills': 'unavailable', 'funds': 'unavailable', 'holdings': 'unavailable', 'instruments': 'stale', 'orders': 'unavailable', 'positions': 'unavailable', 'quote': 'unavailable'} |
| Instrument search RELIANCE | **WORKING** | 200 | master=stale symbols=2 |
| Instrument search BTCUSDT | **WORKING** | 200 |  |
| LTP RELIANCE | **WORKING** | 200 |  |
| LTP BTCUSDT | **WORKING** | 200 |  |
| Plan declare | **AUTH** | 401 | Authentication required |
| Declarations list | **AUTH** | 401 | Authentication required |
| Loss limits | **AUTH** | 401 | Authentication required |
| Stop-me | **AUTH** | 401 | Authentication required |

## Full trade-flow

**Cannot complete** Open → Plan declare → Working → Debrief.

Blockers:
1. Console Station Caller access token expired (~15 min lifetime). Keychain was updated earlier tonight but token already past `exp`.
2. Broker sync was idle on relaunch (`brokerSlug=null`, Today `sync_unavailable`). Kotak vault entry exists; Start from Brokers (or successful sync/start) still required for live fills/quotes.
3. Instrument search returned empty `symbols` while `master_status=fresh` until/unless the right book sync is active.

## What to do next

1. Station **Settings → Station sign-in → Sign in with browser** (WorkOS). Wait until email shows signed in.
2. Brokers: **Start** Kotak Neo (and/or Binance) so sync-state shows an active slug with fresh fills/funds/positions.
3. Ping me — I will re-run declare → stop-me → Working/Debrief immediately.

## Artifacts

- `/tmp/ta-recheck3/` `/tmp/ta-recheck4/` `/tmp/ta-recheck5/`
- Prior: `plans/NOTCH-LIVE-TRADEFLOW-2026-09-27.md`

## Settle note (post sync/start)

- Kotak sync **did start** (`brokerSlug=kotak_neo`, profile `equities_inr_cash`).
- Capabilities mostly **unavailable**; `instruments=stale`; `consecutiveFailures` climbing (session/TOTP likely needs refresh in Brokers UI).
- Instrument search **RELIANCE** returns NSE+BSE tokens (2885 / 500325) with `last_price=0`.
- Today degradedReason moved from `sync_unavailable` → `sync_stale`.

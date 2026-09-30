# Station + Notch — IMMEDIATE full retest
**When:** 2026-09-27 01:43 Asia/Calcutta
**Method:** Live Wire v1 against Debug agent :9137. Not unit tests.

## JWT
```
{
  "aud": "station",
  "exp_iso": "2026-09-26T20:04:02+00:00",
  "expired": true,
  "secs_left": -556,
  "refresh_len": 43,
  "expires_in_field": 900
}
```

## Headline
- Console session: STILL FAILING
- Full declare path: STILL BLOCKED
- Broker: slug=kotak_neo caps={'fills': 'unavailable', 'funds': 'unavailable', 'holdings': 'unavailable', 'instruments': 'stale', 'orders': 'unavailable', 'positions': 'unavailable', 'quote': 'unavailable'} failures=1

| Feature | Verdict | HTTP | Evidence |
|---|---|---|---|
| Agent health | **WORKING** | 200 |  |
| Console session | **AUTH** | 401 | signed_in=False Console station/session HTTP 401 Unauthorized: {"success":false,"error":{"code":"INTERNAL_ERROR","message":"Invalid or e |
| Notch live-state | **WORKING** | 200 | bar=True notch={'pending_declaration': None, 'plan_state': '', 'sync_state': 'GREEN'} |
| Today | **WORKING** | 200 | degraded=sync_stale slug=kotak_neo baseline=True |
| Positions | **WORKING** | 200 |  |
| Kill audit | **WORKING** | 200 |  |
| Toolbar trades | **WORKING** | 200 |  |
| Capture outbox | **WORKING** | 200 |  |
| Kotak vault present | **WORKING** | 200 |  |
| Broker sync-state | **WORKING** | 200 |  |
| Kotak sync/start | **WORKING** | 200 |  |
| Kotak sync/retry | **WORKING** | 200 |  |
| Broker sync-state after | **WORKING** | 200 | slug=kotak_neo caps={'fills': 'unavailable', 'funds': 'unavailable', 'holdings': 'unavailable', 'instruments': 'stale', 'orders': 'unavailable', 'positions': 'unavailable', 'quote': 'unavailable'} failures=1 err=adapter: host refused to return response containing raw credentials |
| Search RELIANCE | **WORKING** | 200 | master=stale n=2 |
| Search BTCUSDT | **WORKING** | 200 |  |
| Search INFY | **WORKING** | 200 |  |
| LTP RELIANCE | **WORKING** | 200 |  |
| LTP BTCUSDT | **WORKING** | 200 |  |
| Plan declare | **AUTH** | 401 | Authentication required |
| Declarations | **AUTH** | 401 | Authentication required |
| Loss limits | **AUTH** | 401 | Authentication required |
| Stop-me | **AUTH** | 401 | Authentication required |
| Cancel declaration | **BROKEN** | 400 | declaration_id required |
| Protective | **AUTH** | 401 | Authentication required |
| Live interference | **AUTH** | 401 | Authentication required |
| Swing check-in | **AUTH** | 401 | Authentication required |
| Post-trade debrief | **METHOD** | 405 |  |

## Full trade-flow
**Cannot complete** Open → Plan → Working → Debrief. Console Station Caller still rejected.

# Station path stress

Loopback agent + stub Console. No broker orders. Re-run with `./scripts/station-path-stress.sh`.

- Ran: `2026-10-01T12:27:54.892712479+00:00`
- CPUs: 4
- Profile: `debug` (`cargo test` debug is unoptimized; a timeout is that build missing the deadline, not a process crash)
- Ladder: `1,4,16,32,64,128,256,512`
- Soak bursts at the passing ceiling: 2
- Per-request deadline: 5000 ms. A break is a status outside the expected set, a transport error, a daemon-secret echo, or that deadline.

## Overall

- Mixed fan: none through concurrency 512
- Earliest per-path concurrency break: kill_latch GET /api/daemon/kill-switch/audit at concurrency 128 (concurrency 128: audit timeout x128)

Peak RPS is achieved throughput of the last passing burst (requests / wall time), not a paced target rate. A row that says `none through concurrency N` stayed inside the deadline for every ladder step and every soak burst.

## Breaks on this ladder

- **n2_morning** `POST /api/daemon/journal/condition-fire` — max burst 512 — payload 2097152B at concurrency 4: condition_fire http 413 x4
- **plan_jwt** `GET /api/daemon/bar/live-state` — max burst 128 — concurrency 256: live_state timeout x125
- **capture_ack** `POST /api/daemon/journal/toolbar-capture/accept` — max burst 512 — payload 2097152B at concurrency 4: toolbar_accept http 413 x4
- **capture_ack** `GET /api/daemon/journal/toolbar-capture/outbox/status` — max burst 128 — concurrency 256: outbox_status timeout x128
- **kill_latch** `GET /api/daemon/kill-switch/audit` — max burst 64 — concurrency 128: audit timeout x128
- **station_auth** `POST /api/daemon/auth/begin` — max burst 512 — soak wave 2 at concurrency 512: phase8_begin timeout x79

The 413s are the default JSON body limit (the pad itself is 2 MiB, so the request is over 2 MiB). They are not handler panics. Timeouts are the 5000 ms per-request deadline; the process stayed up and later paths still answered.

| Subsystem | Wave | Path | Max concurrency still passing | Peak RPS | Soak bursts passed | Payload bytes still ok | First failure mode | Live gate |
| --- | --- | --- | ---: | ---: | --- | --- | --- | --- |
| overall | all | `MIX 22 mixed paths` | 512 | 185 | 2/2 | — | none through concurrency 512 | same stub Console as the per-path rows |
| station_status | status | `GET /api/daemon/health` | 512 | 1855 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/daemon/broker/sync-state` | 512 | 1149 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/daemon/positions` | 512 | 1484 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/daemon/today` | 512 | 1695 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/daemon/events/stream` | 512 | 435 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/daemon/toolbar/recent-trades` | 512 | 1418 | 2/2 | — | none through concurrency 512 | local |
| station_status | status | `GET /api/station/sync-hint` | 512 | 3107 | 2/2 | — | none through concurrency 512 | local |
| n2_morning | N2 | `GET /api/daemon/morning-brief` | 512 | 1204 | 2/2 | — | none through concurrency 512 | local |
| journal_cite | cite | `GET /api/daemon/journal/trip-cites?week_start=2026-09-28&week_end=2026-10-04` | 512 | 1177 | 2/2 | — | none through concurrency 512 | local |
| n2_morning | N2 | `POST /api/daemon/journal/condition-fire` | 512 | 155 | 2/2 | 1048576 | payload 2097152B at concurrency 4: condition_fire http 413 x4 | local |
| conditions | conditions | `POST /api/daemon/bar/capture-working-condition` | 512 | 1140 | 2/2 | — | none through concurrency 512 | local |
| plan_jwt | PLAN | `POST /api/daemon/bar/declare` | 512 | 431 | 2/2 | — | none through concurrency 512 | stub Console; live Caller JWT archive needs Gate B |
| plan_jwt | PLAN | `GET /api/daemon/bar/live-state` | 128 | 41 | — | — | concurrency 256: live_state timeout x125 | stub Console when LiveBook is empty |
| plan_jwt | PLAN | `GET /api/daemon/bar/declarations?scope=week` | 512 | 830 | 2/2 | — | none through concurrency 512 | stub Console |
| cancel | cancel | `POST /api/daemon/bar/cancel-declaration` | 512 | 357 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/stop-me` | 512 | 739 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/stop-me/clear` | 512 | 722 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/protective` | 512 | 413 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/live-interference` | 512 | 729 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/swing-check-in` | 512 | 708 | 2/2 | — | none through concurrency 512 | stub Console |
| n2_morning | N2 | `PATCH /api/daemon/bar/post-trade-debrief` | 512 | 331 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `GET /api/daemon/bar/profile/loss-limits` | 512 | 921 | 2/2 | — | none through concurrency 512 | stub Console |
| plan_jwt | PLAN | `POST /api/daemon/bar/profile/loss-limits` | 512 | 774 | 2/2 | — | none through concurrency 512 | stub Console |
| match | match | `POST /api/daemon/bar/test/fill-matched` | 512 | 408 | 2/2 | — | none through concurrency 512 | stub inject; prod Console must leave BAR_TEST_INJECT_FILL_MATCHED unset |
| capture_ack | capture | `POST /api/daemon/journal/toolbar-capture/accept` | 512 | 554 | 2/2 | 1048576 | payload 2097152B at concurrency 4: toolbar_accept http 413 x4 | stub ACK; live R2/journal needs Gate B |
| capture_ack | capture | `GET /api/daemon/journal/toolbar-capture/outbox/status` | 128 | 32 | — | — | concurrency 256: outbox_status timeout x128 | local |
| capture_ack | capture | `POST /api/daemon/screenshot/presign` | 512 | 735 | 2/2 | — | none through concurrency 512 | stub presign; live upload needs Console + R2 |
| capture_ack | capture | `GET /api/daemon/journal/toolbar-capture/pending/11111111-2222-4333-8444-555555555555` | 512 | 795 | 2/2 | — | none through concurrency 512 | stub Console |
| capture_ack | capture | `PATCH /api/daemon/journal/toolbar-capture/pending/11111111-2222-4333-8444-555555555555` | 512 | 641 | 2/2 | — | none through concurrency 512 | stub Console |
| capture_ack | capture | `POST /api/daemon/journal/manual-fill/accept` | 512 | 160 | 2/2 | — | none through concurrency 512 | local fill + stub capture ACK |
| capture_ack | capture | `GET /api/daemon/journal/toolbar/recent-trades` | 512 | 928 | 2/2 | — | none through concurrency 512 | stub Console |
| kill_latch | kill | `GET /api/daemon/kill-switch/state` | 512 | 1078 | 2/2 | — | none through concurrency 512 | local |
| kill_latch | kill | `POST /api/daemon/kill-switch` | 512 | 269 | 2/2 | — | none through concurrency 512 | L1 desk latch only; L3 DNS is not ramped |
| kill_latch | kill | `POST /api/daemon/dismiss-kill-switch` | 512 | 362 | 2/2 | — | none through concurrency 512 | local |
| kill_latch | kill | `GET /api/daemon/kill-switch/audit` | 64 | 15 | — | — | concurrency 128: audit timeout x128 | local |
| kill_latch | kill | `POST /api/daemon/kill-switch/ack` | 512 | 844 | 2/2 | — | none through concurrency 512 | stub Console |
| station_auth | auth | `GET /api/daemon/auth/station/session` | 512 | 1239 | 2/2 | — | none through concurrency 512 | unsigned-in memory store; live prove needs Keychain Bearer + Console |
| station_auth | auth | `POST /api/daemon/auth/station/begin` | 512 | 1106 | 2/2 | — | none through concurrency 512 | 503 while WORKOS_STATION_CLIENT_ID is unset; live device login is Mac + WorkOS |
| station_auth | auth | `POST /api/daemon/auth/station/complete` | 512 | 1036 | 2/2 | — | none through concurrency 512 | no pending device login in this process |
| station_auth | auth | `POST /api/daemon/auth/station/sign-out` | 512 | 1276 | 2/2 | — | none through concurrency 512 | local |
| station_auth | auth | `POST /api/daemon/auth/begin` | 512 | 699 | 1/2 | — | soak wave 2 at concurrency 512: phase8_begin timeout x79 | stub Console |
| station_auth | auth | `POST /api/daemon/auth/finish` | 512 | 849 | 2/2 | — | none through concurrency 512 | stub Console |
| depth | depth | `GET /api/station/quote?instrument=BTCUSDT&book=binance-com-spot` | 512 | 3097 | 2/2 | — | none through concurrency 512 | honest envelope; live book needs a connected adapter |
| depth | depth | `GET /api/station/history?instrument=BTCUSDT&interval=1m&limit=5` | 512 | 2599 | 2/2 | — | none through concurrency 512 | local |
| depth | depth | `GET /api/station/depth?book=binance-com-spot&instrument=BTCUSDT` | 512 | 2161 | 2/2 | — | none through concurrency 512 | unbound COM book returns immediately; live ladder needs a session |
| depth | depth | `GET /api/station/chain?book=binance-com-options` | 512 | 2475 | 2/2 | — | none through concurrency 512 | local |
| depth | depth | `GET /api/station/oi?book=binance-com-options` | 512 | 2797 | 2/2 | — | none through concurrency 512 | local |
| depth | depth | `GET /api/station/greeks?book=binance-com-options` | 512 | 2872 | 2/2 | — | none through concurrency 512 | local |
| depth | depth | `GET /api/station/index?book=binance-com-options` | 512 | 2167 | 2/2 | — | none through concurrency 512 | local |
| depth | depth | `GET /api/station/manifest` | 512 | 423 | 2/2 | — | none through concurrency 512 | local |
| funds_sizer | funds | `GET /api/station/obtain?adapter=binance_com&book=binance-com-spot&operation=funds` | 512 | 2865 | 2/2 | — | none through concurrency 512 | no USER_DATA session; envelope only. Live funds need Keychain |
| funds_sizer | sizer | `POST /api/daemon/risk/preview` | 512 | 863 | 2/2 | — | none through concurrency 512 | agent preview; Notch BarPlanSizer paint is Mac UI |
| broker | status | `POST /api/daemon/broker/sync/stop` | 512 | 1242 | 2/2 | — | none through concurrency 512 | local |
| broker | status | `POST /api/daemon/broker/sync/retry` | 512 | 1141 | 2/2 | — | none through concurrency 512 | local |
| broker | status | `POST /api/daemon/broker/credentials/present` | 512 | 986 | 2/2 | — | none through concurrency 512 | memory vault in this run; Keychain is Mac |
| broker | status | `POST /api/daemon/broker/credentials/clear` | 512 | 1029 | 2/2 | — | none through concurrency 512 | memory vault; deletes nothing on the host Keychain |
| broker | status | `POST /api/daemon/broker/sync/start` | 512 | 873 | 2/2 | — | none through concurrency 512 | unknown slug; live Start needs Keychain + wasm component |
| totp | TOTP | `POST /api/daemon/broker/kotak/session/mint` | 512 | 859 | 2/2 | — | none through concurrency 512 | rejects before network. 12h card + live mint are Mac + Kotak |
| broker | auth | `POST /api/daemon/broker/groww/connect` | 512 | 888 | 2/2 | — | none through concurrency 512 | rejects before network |
| broker | auth | `POST /api/daemon/broker/dhan/connect/begin` | 512 | 902 | 2/2 | — | none through concurrency 512 | rejects before consent HTTP |
| broker | auth | `POST /api/daemon/broker/zerodha/connect/begin` | 512 | 735 | 2/2 | — | none through concurrency 512 | local pending map only; browser login is Mac |
| broker | auth | `POST /api/daemon/broker/upstox/begin` | 512 | 673 | 2/2 | — | none through concurrency 512 | local pending map only |
| broker | auth | `POST /api/daemon/broker/fyers/begin` | 512 | 713 | 2/2 | — | none through concurrency 512 | local pending map only |
| broker | auth | `GET /api/daemon/broker/zerodha/callback` | 512 | 3185 | 2/2 | — | none through concurrency 512 | missing request_token; no token exchange |
| broker | status | `PUT /api/daemon/vendor-bindings` | 512 | 628 | 2/2 | — | none through concurrency 512 | local |
| broker | status | `GET /instruments/search?q=RE` | 512 | 741 | 2/2 | — | none through concurrency 512 | local |

## Live Mac + Console only

This cloud run does not click the Notch and does not present a real Console URL or Keychain.

| Surface | What this harness proved | What still needs a Mac + live Console |
| --- | --- | --- |
| PLAN JWT archive | Declare stays 200 against a stub, including under concurrency | Gate B (`signed_in: true`) and a real `POST /api/bar/v1/declarations` on `TRADEAUTOPSY_SERVER_BASE_URL` |
| Capture ACK to journal / R2 | Outbox reaches ACK against the stub body `status=accepted` | Gate B plus the real capture accept and screenshot presign |
| Match flip | Agent forwards `test/fill-matched` and will cite when the stub says `test_only` + `matched` | Production leaves `BAR_TEST_INJECT_FILL_MATCHED` unset. A real match is a Console write |
| TOTP 12h card | Mint route returns 400 for a non-`kotak_neo` slug and does not dial Kotak | `KotakSessionMintCardPolicy` (12h) and a real TOTP/MPIN mint. Touch ID is LocalAuthentication |
| Funds | `obtain(funds)` returns an envelope with no USER_DATA session | Identity-only Start with Keychain credentials |
| Sizer paint | `POST /api/daemon/risk/preview` | Notch `BarPlanSizer` is SwiftUI |
| Sparkle / restart | No agent route. Restart is `StationAppCoordinator.retryAgent` | macOS Sparkle against `https://updates.tradeautopsy.in/appcast.xml`, and the Station app supervising the agent. `scripts/verify-updates-feed.sh` checks the feed from a machine that can resolve that host |
| Device login | Session route stays `signed_in: false`. Begin stays 503 without `WORKOS_STATION_CLIENT_ID` | WorkOS AuthKit CLI Auth and `GET /api/auth/station/session` on the live Console. UI shows `user_code` only |
| Binance.US withdraw block | Not called | Validated read-only key, withdraw permission hard-blocked, on a Mac |

Wire identity in this run is the integration-test loopback secret. It is machine integrity only and is not printed.

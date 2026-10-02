# Station live stress soak — 2026-10-01

Started `2026-10-01T12:16:22Z` · ended `2026-10-01T12:18:03Z`
Console base from `~/.tradeautopsy/station.env`: `https://www.tradeautopsy.in`
Agent `127.0.0.1:9137`

Launched `/Users/bishnu/Library/Developer/Xcode/DerivedData/TradeAutopsy_Station-ccdwmgacytrtxkdikkdnlwopgfkd/Build/Products/Debug/TradeAutopsy Station.app` and agent accepted connections.

## Known-good

Gate A=PASS (ok daemon=agent status=ok); Gate B=PASS (profile=ac35ef44…); Gate C=PASS (syncState=not_connected brokerSlug=none · tools may be HONEST-DARK)

## Break points

Passed N is the highest concurrency whose wave fully succeeded. `none (held through concurrency K)` means the path did not fail before the cap.

| Path | Passed N | First failure |
| --- | ---: | --- |
| plan_jwt | 32 | none (held through concurrency 32) |
| station_status | 32 | none (held through concurrency 32) |
| morning_brief_n2 | 32 | none (held through concurrency 32) |
| journal_cite_due | 32 | none (held through concurrency 32) |
| conditions_depth | 32 | none (held through concurrency 32) |
| kill_latch_read | 32 | none (held through concurrency 32) |
| funds_sizer | 32 | none (held through concurrency 32) |
| totp_12h_sync | 32 | none (held through concurrency 32) |
| sparkle_restart | 0 | withheld (supervisor does not respawn after external SIGTERM) |
| sparkle_feed | 16 | none (held through concurrency 16) |
| declare | 2 | cancel HTTP 409 {"ok":false,"error":{"code":"declaration_not_cancellable"}} |
| cancel | 1 | none (first HTTP 200; second HTTP 409 (idempotent 4xx ok)) |
| capture_ack_outbox | 16 | none (held through concurrency 16) |
| capture_dead_letter | 1 | none |
| fill_match | 32 | none (held through concurrency 32) |

The only concurrency break is **declare**. Waves of 1 and 2 (declare then cancel) succeeded. At 4 in flight, one cancel returned HTTP 409 `declaration_not_cancellable`. That row is `superseded`, not left pending. A later declarations read showed `pending_count=0`. A single-threaded second cancel of a fresh id also returns 409 and is treated as idempotent.

Fill match held because production Console answers 404 (`BAR_TEST_INJECT_FILL_MATCHED` unset). That is the safety gate, not a matched fill.

During the ramp, sync was still `not_connected` (funds capability unavailable). About a minute after the run, sync was `synced` on `binance_com` with fills/funds/orders/quote fresh. A follow-up funds obtain at concurrency 32 then returned `status=success` (p95 2.2s) with no failures. Balances were not recorded.

## Notes

- TOTP 12h: binance_com: age_h=1032.7 card_due=True; kotak_neo: age_h=2.8 card_due=False; zerodha_kite: age_h=168.4 card_due=True; mint POST withheld
- Sparkle: HTTP 200 newest short=0.2.2 sparkle:version=6; installed TradeAutopsy Station.app 0.2.1 (5)
- Restart: not performed. External SIGTERM clears `AgentSupervisor.spawnedPID` and the poll only marks a disconnect; it does not respawn.
- Fill match 404 is the production gate (`BAR_TEST_INJECT_FILL_MATCHED` unset), not a crash.
- Declare quantity is 0.001 BTCUSDT and each success is cancelled. No broker order is sent.
- Declare 409 id `aab1e2b2-a83a-4056-96ef-7890366f112e` is `status=superseded` (cleanup cancel also 409). No pending harness plan remained.
- Post-soak sync recheck: `syncState=synced` `brokerSlug=binance_com`. Funds obtain `status=success` at concurrency 32, 0 failures.
- Capture accept wrote about 31 Journal drafts whose text is `Stress soak capture probe — safe to delete.`

## Waves

- **plan_jwt** n=1 ok=1 fail=0 p95=0.599s
- **plan_jwt** n=4 ok=4 fail=0 p95=4.43s
- **plan_jwt** n=16 ok=16 fail=0 p95=4.585s
- **plan_jwt** n=32 ok=32 fail=0 p95=4.687s
- **station_status** n=1 ok=1 fail=0 p95=0.004s
- **station_status** n=4 ok=4 fail=0 p95=0.006s
- **station_status** n=16 ok=16 fail=0 p95=0.017s
- **station_status** n=32 ok=32 fail=0 p95=0.021s
- **morning_brief_n2** n=1 ok=1 fail=0 p95=0.001s
- **morning_brief_n2** n=4 ok=4 fail=0 p95=0.002s
- **morning_brief_n2** n=16 ok=16 fail=0 p95=0.005s
- **morning_brief_n2** n=32 ok=32 fail=0 p95=0.008s
- **journal_cite_due**: matched=0 due_empty_post=0 trip_cites_today=0
- **journal_cite_due** n=1 ok=1 fail=0 p95=1.075s
- **journal_cite_due** n=4 ok=4 fail=0 p95=4.164s
- **journal_cite_due** n=16 ok=16 fail=0 p95=4.712s
- **journal_cite_due** n=32 ok=32 fail=0 p95=4.616s
- **conditions_depth** n=1 ok=1 fail=0 p95=0.003s
- **conditions_depth** n=4 ok=4 fail=0 p95=0.004s
- **conditions_depth** n=16 ok=16 fail=0 p95=0.014s
- **conditions_depth** n=32 ok=32 fail=0 p95=0.021s
- **kill_latch_read**: POST /kill-switch not sent
- **kill_latch_read** n=1 ok=1 fail=0 p95=0.005s
- **kill_latch_read** n=4 ok=4 fail=0 p95=0.004s
- **kill_latch_read** n=16 ok=16 fail=0 p95=0.01s
- **kill_latch_read** n=32 ok=32 fail=0 p95=0.011s
- **funds_sizer**: balances not logged; preview uses a synthetic 10000
- **funds_sizer** n=1 ok=1 fail=0 p95=1.068s
- **funds_sizer** n=4 ok=4 fail=0 p95=1.04s
- **funds_sizer** n=16 ok=16 fail=0 p95=1.331s
- **funds_sizer** n=32 ok=32 fail=0 p95=1.379s
- **totp_12h_sync**: binance_com: age_h=1032.7 card_due=True; kotak_neo: age_h=2.8 card_due=False; zerodha_kite: age_h=168.4 card_due=True; mint POST withheld
- **totp_12h_sync** n=1 ok=1 fail=0 p95=0.002s
- **totp_12h_sync** n=4 ok=4 fail=0 p95=0.001s
- **totp_12h_sync** n=16 ok=16 fail=0 p95=0.003s
- **totp_12h_sync** n=32 ok=32 fail=0 p95=0.005s
- **sparkle_restart**: feed check is sparkle_feed
- **sparkle_feed**: HTTP 200 newest short=0.2.2 sparkle:version=6; installed TradeAutopsy Station.app 0.2.1 (5)
- **sparkle_feed** n=1 ok=1 fail=0 p95=0.045s
- **sparkle_feed** n=4 ok=4 fail=0 p95=0.355s
- **sparkle_feed** n=16 ok=16 fail=0 p95=0.095s
- **declare**: each success cancels
- **declare** n=1 ok=1 fail=0 p95=5.055s
- **declare** n=2 ok=2 fail=0 p95=5.125s
- **declare** n=4 ok=3 fail=1 p95=5.132s
- **capture_ack_outbox**: draft text marked safe to delete
- **capture_ack_outbox** n=1 ok=1 fail=0 p95=4.398s
- **capture_ack_outbox** n=2 ok=2 fail=0 p95=6.222s
- **capture_ack_outbox** n=4 ok=4 fail=0 p95=6.197s
- **capture_ack_outbox** n=8 ok=8 fail=0 p95=6.241s
- **capture_ack_outbox** n=16 ok=16 fail=0 p95=6.397s
- **capture_dead_letter**: dead_letter before=0 after=0
- **fill_match**: 404 is pass (prod flag off). Synthetic declaration id, no real declare.
- **fill_match** n=1 ok=1 fail=0 p95=0.356s
- **fill_match** n=4 ok=4 fail=0 p95=2.117s
- **fill_match** n=16 ok=16 fail=0 p95=2.112s
- **fill_match** n=32 ok=32 fail=0 p95=2.126s

## Re-run

```bash
./scripts/station-live-stress-soak.sh
```

## Log

- 2026-10-01T12:16:22Z server_base=https://www.tradeautopsy.in
- 2026-10-01T12:16:22Z Launched `/Users/bishnu/Library/Developer/Xcode/DerivedData/TradeAutopsy_Station-ccdwmgacytrtxkdikkdnlwopgfkd/Build/Products/Debug/TradeAutopsy Station.app` and agent accepted connections.
- 2026-10-01T12:16:22Z known-good Gate A=PASS (ok daemon=agent status=ok); Gate B=PASS (profile=ac35ef44…); Gate C=PASS (syncState=not_connected brokerSlug=none · tools may be HONEST-DARK)
- 2026-10-01T12:16:22Z agent_pid=9958 app=TradeAutopsy Station.app 0.2.1 (5)
- 2026-10-01T12:16:29Z http=200 signed_in=True profile_prefix=ac35ef44 aud=station
- 2026-10-01T12:16:29Z version=0.1.0 build=tradeautopsy-agent/0.1.0 (ba5798dd47cb) syncState=not_connected brokerSlug=binance_com killDnsActive=False caps=fills=unavailable,funds=unavailable,holdings=unsupported,instruments=fresh,orders=unavailable,positions=unsupported,quote=fresh
- 2026-10-01T12:16:29Z http=200 active=False dns_active=False
- 2026-10-01T12:16:29Z path plan_jwt levels=[1, 4, 16, 32]
- 2026-10-01T12:16:29Z path plan_jwt n=1 ok=1 fail=0 p95=0.599s
- 2026-10-01T12:16:34Z path plan_jwt n=4 ok=4 fail=0 p95=4.43s
- 2026-10-01T12:16:38Z path plan_jwt n=16 ok=16 fail=0 p95=4.585s
- 2026-10-01T12:16:43Z path plan_jwt n=32 ok=32 fail=0 p95=4.687s
- 2026-10-01T12:16:43Z path station_status levels=[1, 4, 16, 32]
- 2026-10-01T12:16:43Z path station_status n=1 ok=1 fail=0 p95=0.004s
- 2026-10-01T12:16:43Z path station_status n=4 ok=4 fail=0 p95=0.006s
- 2026-10-01T12:16:43Z path station_status n=16 ok=16 fail=0 p95=0.017s
- 2026-10-01T12:16:43Z path station_status n=32 ok=32 fail=0 p95=0.021s
- 2026-10-01T12:16:43Z path morning_brief_n2 levels=[1, 4, 16, 32]
- 2026-10-01T12:16:43Z path morning_brief_n2 n=1 ok=1 fail=0 p95=0.001s
- 2026-10-01T12:16:43Z path morning_brief_n2 n=4 ok=4 fail=0 p95=0.002s
- 2026-10-01T12:16:43Z path morning_brief_n2 n=16 ok=16 fail=0 p95=0.005s
- 2026-10-01T12:16:43Z path morning_brief_n2 n=32 ok=32 fail=0 p95=0.008s
- 2026-10-01T12:16:45Z path journal_cite_due levels=[1, 4, 16, 32]
- 2026-10-01T12:16:46Z path journal_cite_due n=1 ok=1 fail=0 p95=1.075s
- 2026-10-01T12:16:50Z path journal_cite_due n=4 ok=4 fail=0 p95=4.164s
- 2026-10-01T12:16:55Z path journal_cite_due n=16 ok=16 fail=0 p95=4.712s
- 2026-10-01T12:17:00Z path journal_cite_due n=32 ok=32 fail=0 p95=4.616s
- 2026-10-01T12:17:00Z path conditions_depth levels=[1, 4, 16, 32]
- 2026-10-01T12:17:00Z path conditions_depth n=1 ok=1 fail=0 p95=0.003s
- 2026-10-01T12:17:00Z path conditions_depth n=4 ok=4 fail=0 p95=0.004s
- 2026-10-01T12:17:00Z path conditions_depth n=16 ok=16 fail=0 p95=0.014s
- 2026-10-01T12:17:00Z path conditions_depth n=32 ok=32 fail=0 p95=0.021s
- 2026-10-01T12:17:00Z path kill_latch_read levels=[1, 4, 16, 32]
- 2026-10-01T12:17:00Z path kill_latch_read n=1 ok=1 fail=0 p95=0.005s
- 2026-10-01T12:17:00Z path kill_latch_read n=4 ok=4 fail=0 p95=0.004s
- 2026-10-01T12:17:00Z path kill_latch_read n=16 ok=16 fail=0 p95=0.01s
- 2026-10-01T12:17:00Z path kill_latch_read n=32 ok=32 fail=0 p95=0.011s
- 2026-10-01T12:17:00Z path funds_sizer levels=[1, 4, 16, 32]
- 2026-10-01T12:17:01Z path funds_sizer n=1 ok=1 fail=0 p95=1.068s
- 2026-10-01T12:17:02Z path funds_sizer n=4 ok=4 fail=0 p95=1.04s
- 2026-10-01T12:17:03Z path funds_sizer n=16 ok=16 fail=0 p95=1.331s
- 2026-10-01T12:17:05Z path funds_sizer n=32 ok=32 fail=0 p95=1.379s
- 2026-10-01T12:17:05Z path totp_12h_sync levels=[1, 4, 16, 32]
- 2026-10-01T12:17:05Z path totp_12h_sync n=1 ok=1 fail=0 p95=0.002s
- 2026-10-01T12:17:05Z path totp_12h_sync n=4 ok=4 fail=0 p95=0.001s
- 2026-10-01T12:17:05Z path totp_12h_sync n=16 ok=16 fail=0 p95=0.003s
- 2026-10-01T12:17:05Z path totp_12h_sync n=32 ok=32 fail=0 p95=0.005s
- 2026-10-01T12:17:05Z path sparkle_restart passed_n=0 failure=withheld (supervisor does not respawn after external SIGTERM)
- 2026-10-01T12:17:05Z path sparkle_feed levels=[1, 4, 16]
- 2026-10-01T12:17:05Z path sparkle_feed n=1 ok=1 fail=0 p95=0.045s
- 2026-10-01T12:17:05Z path sparkle_feed n=4 ok=4 fail=0 p95=0.355s
- 2026-10-01T12:17:05Z path sparkle_feed n=16 ok=16 fail=0 p95=0.095s
- 2026-10-01T12:17:05Z path declare levels=[1, 2, 4, 8, 16]
- 2026-10-01T12:17:10Z path declare n=1 ok=1 fail=0 p95=5.055s
- 2026-10-01T12:17:16Z path declare n=2 ok=2 fail=0 p95=5.125s
- 2026-10-01T12:17:21Z path declare n=4 ok=3 fail=1 p95=5.132s
- 2026-10-01T12:17:21Z path declare first_failure=cancel HTTP 409 {"ok":false,"error":{"code":"declaration_not_cancellable"}}
- 2026-10-01T12:17:25Z path cancel passed_n=1 failure=none (first HTTP 200; second HTTP 409 (idempotent 4xx ok))
- 2026-10-01T12:17:25Z path capture_ack_outbox levels=[1, 2, 4, 8, 16]
- 2026-10-01T12:17:30Z path capture_ack_outbox n=1 ok=1 fail=0 p95=4.398s
- 2026-10-01T12:17:36Z path capture_ack_outbox n=2 ok=2 fail=0 p95=6.222s
- 2026-10-01T12:17:42Z path capture_ack_outbox n=4 ok=4 fail=0 p95=6.197s
- 2026-10-01T12:17:48Z path capture_ack_outbox n=8 ok=8 fail=0 p95=6.241s
- 2026-10-01T12:17:55Z path capture_ack_outbox n=16 ok=16 fail=0 p95=6.397s
- 2026-10-01T12:17:55Z path capture_dead_letter passed_n=1 failure=none
- 2026-10-01T12:17:55Z path fill_match levels=[1, 4, 16, 32]
- 2026-10-01T12:17:55Z path fill_match n=1 ok=1 fail=0 p95=0.356s
- 2026-10-01T12:17:57Z path fill_match n=4 ok=4 fail=0 p95=2.117s
- 2026-10-01T12:17:59Z path fill_match n=16 ok=16 fail=0 p95=2.112s
- 2026-10-01T12:18:02Z path fill_match n=32 ok=32 fail=0 p95=2.126s
- 2026-10-01T12:18:03Z orphan cleanup aab1e2b2-a83a-4056-96ef-7890366f112e HTTP 409

# Station live stress soak

Paper and debug only. The harness signs loopback wire-v1 against the agent on `127.0.0.1:9137` and talks to whatever Console that agent already uses (`~/.tradeautopsy/station.env` → `TRADEAUTOPSY_SERVER_BASE_URL`). It does not place broker orders, does not POST the kill switch, does not mint a Kotak TOTP, and does not install a Sparkle update.

## Re-run

Station (or a Debug agent) must be listening on port 9137 with Gate B signed in for the Console paths. If nothing is listening, the script launches the Debug Station app when that bundle exists, otherwise `/Applications/TradeAutopsy Station.app`.

```bash
./scripts/station-live-stress-soak.sh
```

Useful limits:

```bash
./scripts/station-live-stress-soak.sh --max-read 32 --max-write 16
```

`--no-launch` refuses to open the app and exits if port 9137 is closed.

Results land in `plans/STATION-LIVE-STRESS-SOAK-<date>.md` and `.json`. Secrets, emails, tokens, and balances are stripped before write.

## What each path does

| Path | Probe | Stop rule |
| --- | --- | --- |
| Known-good | Existing `notch-live-journal` dry-run (gates + tools) | Gate A down aborts the ramp |
| PLAN JWT | `GET /api/daemon/auth/station/session` | First non-signed-in / 5xx / timeout |
| Station status | health, sync-state, today, positions | First non-200 |
| Morning brief / N2 | `GET /morning-brief` + local condition-fire | First non-200 |
| Journal cite / Due | trip-cites + declarations Due count (matched, empty post) | First non-200 |
| Conditions / depth | depth GET + condition capture on a synthetic id + N2 fire | First non-200 |
| Kill latch | `GET` state + audit only | First non-200. POST kill is not sent |
| Funds / sizer | obtain funds (status only) + local risk preview + GET loss-limits | First 5xx / timeout. Dark funds is a pass |
| TOTP 12h | sync-state under load + local `lastValidatedAt` vs 12h card | Mint POST is not sent |
| Declare | declare then cancel (BTCUSDT 0.001, harness intent) | First declare or cancel failure |
| Cancel | second cancel of one id after a successful cancel | 5xx is a break. 409/4xx after success is idempotent |
| Capture ACK / outbox | toolbar-capture accept + outbox status | non-200/202, or dead-letter growth |
| Fill match | `POST /bar/test/fill-matched` | 404 (flag off) is a pass. 2xx on this Console is a safety failure |
| Sparkle | GET `https://updates.tradeautopsy.in/appcast.xml` | non-200. Process restart is not performed |

Read ramps: 1, then 4, 16, up to `--max-read`. Write ramps: 1, 2, 4, … up to `--max-write`. A path stops at the first failing wave. **Passed N** is the highest concurrency whose wave was entirely successful.

## Restart

An external SIGTERM of `tradeautopsy-agent` is not used. `AgentSupervisor` clears `spawnedPID` in the termination handler, and the runtime poll then only raises a disconnect warning. It does not respawn. Killing the agent from this soak would leave the desk unhealthy until a manual Retry.

# Notch live journal harness (Phase B)

Founder-facing **live** probe for the Debug agent on loopback (`127.0.0.1:9137`). Complements `scripts/dogfood-suite.sh` (CI unit/integration checks) with gates, a real declare path, and a tools honesty matrix.

**Spec:** [`plans/STRONG-TESTING-SYSTEM-2026-09-27.md`](STRONG-TESTING-SYSTEM-2026-09-27.md) §4 Phase B.

## Run

```bash
chmod +x scripts/notch-live-journal.sh
./scripts/notch-live-journal.sh
```

Spot / Intraday / Cockpit (matches Plan screenshot — tiles may show **3 holes** while declare form is still the journal path):

```bash
./scripts/notch-live-journal.sh --book binance-com-spot --symbol BTCUSDT --stance planned
```

Read-only gates + tools (no declare/cancel):

```bash
./scripts/notch-live-journal.sh --dry-run
```

Skip toolbar capture outbox probe:

```bash
./scripts/notch-live-journal.sh --skip-capture
```

## Gates (fail fast)

| Gate | Route | Pass |
| --- | --- | --- |
| A | `GET /api/daemon/health` | HTTP 200 |
| B | `GET /api/daemon/auth/station/session` | `signed_in: true`, no `SESSION_PROOF` |
| C | `GET /api/daemon/broker/sync-state` | HTTP 200 (report `syncState`; stale ⇒ tools may be **HONEST-DARK**, not a gate fail) |

Gate B failure **skips declare** (no Console Bearer). Exit **2** if Gate A fails; exit **3** if any tool scores **LIE**.

## Secret

`AGENT_DAEMON_SECRET` from the **tradeautopsy-agent** process via `ps eww -p <pid>` (macOS) or `/proc/<pid>/environ` (Linux). Optional dev override: env var. Never printed.

## Journal path

When Gate B passes: `POST /api/daemon/bar/declare` with S1 moods (`mood_stress` = calm, `mood_impulse` = confidence, `mood_frustration`, `mood_excitement`), `stance`, symbol/qty/stops — same shape as `BarDeclarationFlowView` / `live_book::declare_from_body`. Assert UUID `declarationId`, check `GET /api/daemon/bar/live-state` `pending_declaration.plan_snapshot`, cleanup via `POST /api/daemon/bar/cancel-declaration` (`cancel_reason_chip: scratch`).

## Tools honesty matrix

Logical names → routes in `agent/src/api/mod.rs`:

| Tool | Route |
| --- | --- |
| quote | `/api/station/quote` |
| history | `/api/station/history` |
| depth | `/api/station/depth` |
| option_chain | `/api/station/chain` |
| open_interest | `/api/station/oi` |
| greeks | `/api/station/greeks` |
| index | `/api/station/index` |
| instruments_search | `/instruments/search` (wire) |
| instruments_ltp | `/instruments/ltp` (wire) |

Scores: `WORKING` | `HONEST-DARK` | `AUTH` | `BROKER` | `ERROR` | `LIE`. **Honest dark ≠ fail.** Hole badge = count of dark rows among quote/history/depth.

## Outbox

`GET /api/daemon/journal/toolbar-capture/outbox/status` only — harness never posts capture accept unless you extend it manually.

## Outputs

- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.md`
- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.json`

(gitignored — regenerated each run)

## CI

Default `dogfood-suite.sh` does **not** run this harness. Manual: `LIVE=1 ./scripts/notch-live-journal.sh` on a Mac with Debug Station.

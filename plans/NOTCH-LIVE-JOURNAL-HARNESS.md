# Notch live journal harness (Phase B)

Founder-facing **live** probe for the Debug agent on loopback (`127.0.0.1:9137`). Complements `scripts/dogfood-suite.sh` (CI unit/integration checks) with gates, a real declare path, and a tools honesty matrix.

Spec source: `STRONG-TESTING-SYSTEM` Phase B (2026-09-27).

## Run

```bash
chmod +x scripts/notch-live-journal.sh
./scripts/notch-live-journal.sh
```

With Station Debug agent running and device login complete:

```bash
./scripts/notch-live-journal.sh --book binance-com-spot --symbol BTCUSDT
```

Read-only gates + tools (no declare/cancel):

```bash
./scripts/notch-live-journal.sh --dry-run
```

Skip toolbar capture outbox:

```bash
./scripts/notch-live-journal.sh --skip-capture
```

## Gates

| Gate | Route | Pass |
| --- | --- | --- |
| A | `GET /api/daemon/health` | HTTP 200 |
| B | `GET /api/daemon/auth/station/session` | `signed_in: true`, no `SESSION_PROOF` |
| C | `GET /api/daemon/broker/sync-state` | HTTP 200 (report `syncState`; stale sync does not fail the harness) |

Gate B failure **skips declare** (no Console Bearer / expired session). Tools matrix still runs; honest dark tools are not scored as failures.

## Journal path

When Gate B passes, the harness `POST /api/daemon/bar/declare` with S1 mood fields (`mood_stress`, `mood_impulse`, `mood_frustration`, `mood_excitement`), `stance: planned`, symbol/qty/stops aligned with `agent/src/live_book.rs` declare tests. It checks `declarationId`, peeks `GET /api/daemon/bar/live-state`, then `POST /api/daemon/bar/cancel-declaration` with `cancel_reason_chip: scratch`.

## Tools honesty matrix

Probes (from `agent/src/api/mod.rs`):

- Open: `/api/station/quote`, `history`, `depth`, `chain` (option_chain), `oi`, `greeks`, `index`
- Wire: `/instruments/search`, `/instruments/ltp`

Scores: `WORKING` | `HONEST-DARK` | `AUTH` | `BROKER` | `ERROR`. **Honest dark ≠ fail.**

## Outputs

- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.md`
- `plans/NOTCH-LIVE-SCOREBOARD-YYYY-MM-DD.json`

## Secrets

- Never commit `AGENT_DAEMON_SECRET` or Station tokens.
- Optional override for local dev only: `AGENT_DAEMON_SECRET` in the environment (same as the running agent). The script never logs it.

## CI

Default `dogfood-suite.sh` does **not** run this harness. Use `LIVE=1 ./scripts/notch-live-journal.sh` manually on a Mac with Debug Station when dogfooding.

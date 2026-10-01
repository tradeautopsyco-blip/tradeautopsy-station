# Runbook: GitHub Actions CI/CD (tradeautopsy-station)

## Symptom: jobs never start

Workflow runs may fail in a few seconds with no logs:

> The job was not started because recent account payments have failed or your spending limit needs to be increased.

That is **account billing**, not broken YAML. Fix billing on the org/user that owns the repo before expecting green checks.

1. [GitHub → Settings → Billing & plans](https://github.com/settings/billing)
2. Pay failed invoices; raise **GitHub Actions** spending limit above **$0** if capped.
3. Re-run **Actions → CI**.

**macOS minutes cost 10× Linux.** CI deliberately runs the agent test suite on Linux and reserves macOS for kill-switch/DNS integration, Swift, and (on `main` only) an Xcode app build.

## CI (`ci.yml`) — automatic

| Trigger | What runs |
|---------|-----------|
| **Pull request** | Path-filtered: `agent/**` → Linux agent job; `agent/`, `notch/`, `station/` (etc.) → macOS job (no Xcode `.app` on PRs) |
| **Push to `main`** | Full Linux agent + full macOS (Swift, dogfood scoreboard, Debug `Station.app`, zip + DMG artifact, 7-day retention) |
| **workflow_dispatch** | Same as PR by default; enable **station_app** to force Xcode build + artifact |

### Jobs

| Job | Runner | Purpose |
|-----|--------|---------|
| **Path filter** | Ubuntu | Skips unrelated jobs on doc-only PRs |
| **Agent — build and test (Linux)** | `ubuntu-24.04` | `cargo build --lib`, `cargo test --all-features`, Wasm smoke build for fixture + Binance + Kotak adapters |
| **macOS — kill-switch, Swift, optional app** | `macos-15` | macOS-only Rust integration tests (`kill_latch`, hosts/DNS kill switch), `swift test` for Notch + Station, `dogfood-suite.sh` on `main`/manual; Xcode build only on `main` push or manual `station_app` |

Environment: `TRADEAUTOPSY_HOSTS_FILE=/tmp/test-hosts` (same as local agent tests).

Live JWT / Keychain journal harnesses are **not** in CI (`scripts/notch-live-journal.sh`, `LIVE=1`).

## Manual-only workflows

| Workflow | Runner | Purpose |
|----------|--------|---------|
| **Release TradeAutopsy Station** | macOS + Ubuntu | Versioned DMG, GitHub Release, signed Sparkle appcast commit |
| **Deploy Sparkle feed (Vercel)** | Ubuntu | Production appcast — or `./scripts/deploy-updates-feed.sh` |
| **Deploy static content to Pages** | Ubuntu | Optional backup of `updater/` |
| **Agent Dispatch** | Ubuntu | Cursor agents; needs `CURSOR_API_KEY` |

Notarization, Apple signing secrets, Sparkle private key, and Vercel deploy tokens are used only in **Release** / feed deploy — not in PR CI.

## Secrets (`tradeautopsyco-blip/tradeautopsy-station`)

| Secret | Required for |
|--------|----------------|
| `VERCEL_TOKEN`, `VERCEL_ORG_ID`, `VERCEL_PROJECT_ID` | Feed deploy Action |
| `PRIVATE_SPARKLE_KEY` | Release workflow appcast signatures |
| Apple secrets | Release workflow (`notarize: true` by default) |
| `CURSOR_API_KEY` | Agent Dispatch |

## Local substitutes (no Actions minutes)

| Task | Command |
|------|---------|
| Agent tests | `cd agent && TRADEAUTOPSY_HOSTS_FILE=/tmp/test-hosts cargo test --all-features` |
| macOS kill tests | Same on a Mac (integration tests under `agent/tests/kill_*`) |
| Swift | `swift test --package-path notch/` and `swift test --package-path station/` |
| Dogfood scoreboard | `./scripts/dogfood-suite.sh` |
| Feed deploy | `./scripts/deploy-updates-feed.sh` |
| Release | Local DMG + `generate_appcast` + push appcast + deploy script |

## Vercel vs Actions for the feed

- **Preferred:** Action **Deploy Sparkle feed (Vercel)** on appcast commits (uses `VERCEL_TOKEN`).
- **Vercel Git** on project **updater**: set **Root Directory** = `updater`; use the Action or deploy script if Git deploy is blocked on commit-email rules.

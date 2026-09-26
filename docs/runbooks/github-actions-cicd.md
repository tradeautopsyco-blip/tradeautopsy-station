# Runbook: GitHub Actions CI/CD (tradeautopsy-station)

## Symptom

Workflow runs show **failure in ~3–15 seconds** with no logs:

> The job was not started because recent account payments have failed or your spending limit needs to be increased.

This is **account billing**, not a broken workflow file. YAML changes alone will not start jobs until billing is fixed.

## Fix billing (required)

Account that **owns the repo** bills Actions usage. For **`tradeautopsyco-blip/tradeautopsy-station`**, fix billing on the **tradeautopsyco-blip** user:

1. [GitHub → Settings → Billing & plans](https://github.com/settings/billing)
2. Pay any **failed payment** or overdue invoice.
3. [Spending limits](https://github.com/settings/billing/spending_limit) → set **GitHub Actions** limit above **$0** (or remove $0 cap).
4. Re-run a workflow: **Actions → CI → Run workflow**.

Private repos: included minutes are limited; **macOS** runners consume **10×** Linux minutes. This repo uses one consolidated **macOS** job to reduce waste after billing works.

## Pipelines (manual-first — auto push/PR triggers disabled to save minutes)

| Workflow | Trigger | Runner | Purpose |
|----------|---------|--------|---------|
| **CI** | Manual only | macOS-15 ×1 | Rust + Swift tests + Station.app build |
| **Deploy Sparkle feed (Vercel)** | Manual only | Ubuntu | Production appcast — or `./scripts/deploy-updates-feed.sh` locally |
| **Release TradeAutopsy Station** | Manual | macOS + Ubuntu | DMG, GitHub Release, signed appcast |
| **Deploy static content to Pages** | Manual only | Ubuntu | Optional backup |
| **Agent Dispatch** | Manual only | Ubuntu | Cursor agents; needs `CURSOR_API_KEY` |

**tradeautopsy (web):** **Tests**, **Deploy**, and **Elite-Tier Validation** (removed) no longer run on push; use manual **Deploy** / `./scripts/deploy-vercel-production.sh`.

## Secrets (`tradeautopsyco-blip/tradeautopsy-station`)

| Secret | Required for |
|--------|----------------|
| `VERCEL_TOKEN`, `VERCEL_ORG_ID`, `VERCEL_PROJECT_ID` | Feed deploy Action |
| `PRIVATE_SPARKLE_KEY` | Release workflow appcast signatures |
| Apple secrets | Release with `notarize: true` only |
| `CURSOR_API_KEY` | Agent Dispatch |

Copy secrets from the old repo manually (GitHub does not expose secret values).

## If you cannot pay for Actions yet

| Task | Local substitute |
|------|------------------|
| CI | `cargo test` in `agent/`, `swift test` in `station/` |
| Feed deploy | `./scripts/deploy-updates-feed.sh` |
| Release | Local DMG + `generate_appcast` + push appcast + deploy script |

## Vercel vs Actions for the feed

- **Preferred:** Action **Deploy Sparkle feed (Vercel)** on appcast commits (uses `VERCEL_TOKEN`, no git-author block).
- **Vercel Git** on project **updater**: set **Root Directory** = `updater`; may still **block** deploys on commit-email rules — use the Action or deploy script if that happens.

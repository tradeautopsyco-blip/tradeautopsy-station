#!/usr/bin/env bash
# Deploy updater/ to Vercel production (updates.tradeautopsy.in + *.vercel.app).
# Default: isolated copy (avoids Vercel "commit author" blocks when deploying from the git tree).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UPDATER="${ROOT}/updater"

if ! command -v vercel >/dev/null 2>&1; then
  echo "Install Vercel CLI: npm i -g vercel" >&2
  exit 1
fi

isolated_deploy() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  cp "${UPDATER}/appcast.xml" "${UPDATER}/vercel.json" "${UPDATER}/.vercelignore" "$tmp/"
  if [[ -f "${UPDATER}/.vercel/project.json" ]]; then
    mkdir -p "$tmp/.vercel"
    cp "${UPDATER}/.vercel/project.json" "$tmp/.vercel/"
  else
    echo "Missing ${UPDATER}/.vercel/project.json — run: cd updater && vercel link" >&2
    exit 1
  fi
  (cd "$tmp" && vercel deploy --prod --yes)
}

echo "Deploying Sparkle feed to Vercel production…"
if [[ "${VERCEL_DEPLOY_FROM_REPO:-}" == "1" ]]; then
  (cd "$UPDATER" && vercel deploy --prod --yes)
else
  isolated_deploy
fi

#!/usr/bin/env bash
# One-time: push Apple notarization + signing secrets to GitHub Actions.
# Requires: Developer ID Application .p12, Apple ID app-specific password, admin on FExEVIL/tradeautopsy-station.
#
# Usage (fill env vars, then run):
#   export TEAM_ID="XXXXXXXXXX"
#   export APPLE_ID="you@example.com"
#   export APPLE_APP_SPECIFIC_PASSWORD="xxxx-xxxx-xxxx-xxxx"
#   export APPLE_SIGNING_IDENTITY='Developer ID Application: Your Name (TEAMID)'
#   export APPLE_CERTIFICATE_PATH="$HOME/path/to/DeveloperID.p12"
#   export APPLE_CERTIFICATE_PASSWORD='p12-export-password'
#   ./scripts/set-apple-release-secrets.sh
set -euo pipefail

REPO="${GITHUB_REPO:-tradeautopsyco-blip/tradeautopsy-station}"

require() {
  local name="$1" val="${!1:-}"
  if [[ -z "$val" ]]; then
    echo "error: set $name" >&2
    exit 1
  fi
}

require TEAM_ID
require APPLE_ID
require APPLE_APP_SPECIFIC_PASSWORD
require APPLE_SIGNING_IDENTITY
require APPLE_CERTIFICATE_PATH
require APPLE_CERTIFICATE_PASSWORD

if [[ ! -f "$APPLE_CERTIFICATE_PATH" ]]; then
  echo "error: no file at APPLE_CERTIFICATE_PATH=$APPLE_CERTIFICATE_PATH" >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh CLI required (brew install gh)" >&2
  exit 1
fi

if ! command -v base64 >/dev/null 2>&1; then
  echo "error: base64 required" >&2
  exit 1
fi

B64="$(base64 <"$APPLE_CERTIFICATE_PATH" | tr -d '\n')"

gh secret set APPLE_CERTIFICATE_BASE64 -R "$REPO" --body "$B64"
gh secret set APPLE_CERTIFICATE_PASSWORD -R "$REPO" --body "$APPLE_CERTIFICATE_PASSWORD"
gh secret set APPLE_SIGNING_IDENTITY -R "$REPO" --body "$APPLE_SIGNING_IDENTITY"
gh secret set TEAM_ID -R "$REPO" --body "$TEAM_ID"
gh secret set APPLE_ID -R "$REPO" --body "$APPLE_ID"
gh secret set APPLE_APP_SPECIFIC_PASSWORD -R "$REPO" --body "$APPLE_APP_SPECIFIC_PASSWORD"

echo "OK: Apple release secrets set on $REPO (6 secrets). Run: gh secret list -R $REPO"

#!/usr/bin/env bash
# Unsigned cofounder drop: TradeAutopsy Station.app + Applications shortcut in a DMG.
# No Developer ID, no notarization. Gatekeeper will warn on first download.
# macOS + Xcode + xcodegen + cargo. Apple Silicon (arm64) only.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="TradeAutopsy Station"
FROM_APP=""
CONFIG="Release"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--from-app PATH] [--config Debug|Release]

  (default)  cargo --release agent, xcodegen, xcodebuild Release arm64, then DMG
  --from-app PATH   skip the build; pack an existing .app
  --config NAME     xcodebuild configuration (default Release)

Writes dist/TradeAutopsy-Station-<gitsha>.dmg
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-app)
      FROM_APP="${2:?--from-app needs a path}"
      shift 2
      ;;
    --config)
      CONFIG="${2:?--config needs Debug or Release}"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown arg $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: pack-station-dmg.sh runs on macOS only" >&2
  exit 1
fi

export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH:-}"
# Full Xcode is required for xcodebuild. Prefer the app bundle when CLT is selected.
if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app/Contents/Developer ]]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

GIT_SHA="$(git -C "${ROOT}" rev-parse --short=12 HEAD 2>/dev/null || echo unknown)"
DIST="${ROOT}/dist"
mkdir -p "${DIST}"
DMG="${DIST}/TradeAutopsy-Station-${GIT_SHA}.dmg"
DERIVED="${ROOT}/station/build-dmg"

if [[ -n "${FROM_APP}" ]]; then
  APP="$(cd "$(dirname "${FROM_APP}")" && pwd)/$(basename "${FROM_APP}")"
  if [[ ! -d "${APP}" ]]; then
    echo "error: no app at ${APP}" >&2
    exit 1
  fi
else
  if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found (https://rustup.rs)" >&2
    exit 1
  fi
  if ! command -v xcodegen >/dev/null 2>&1; then
    echo "error: xcodegen not found (brew install xcodegen)" >&2
    exit 1
  fi
  if ! command -v xcodebuild >/dev/null 2>&1; then
    echo "error: xcodebuild not found (install Xcode)" >&2
    exit 1
  fi

  echo "Building tradeautopsy-agent (GIT_SHA=${GIT_SHA})..."
  (
    cd "${ROOT}/agent"
    GIT_SHA="${GIT_SHA}" cargo build --release --locked --bin tradeautopsy-agent
  )

  echo "Generating Xcode project..."
  (cd "${ROOT}/station" && xcodegen generate)

  echo "Building ${APP_NAME}.app (${CONFIG}, arm64, ad-hoc)..."
  rm -rf "${DERIVED}"
  xcodebuild \
    -project "${ROOT}/station/TradeAutopsy Station.xcodeproj" \
    -scheme "${APP_NAME}" \
    -configuration "${CONFIG}" \
    -destination "platform=macOS,arch=arm64" \
    -derivedDataPath "${DERIVED}" \
    CODE_SIGN_IDENTITY="-" \
    CODE_SIGNING_REQUIRED=NO \
    CODE_SIGNING_ALLOWED=NO \
    build

  APP="$(find "${DERIVED}" -name "${APP_NAME}.app" -type d | head -1)"
  if [[ -z "${APP}" || ! -d "${APP}" ]]; then
    echo "error: xcodebuild did not produce ${APP_NAME}.app" >&2
    exit 1
  fi
fi

if [[ ! -x "${APP}/Contents/MacOS/tradeautopsy-agent" ]]; then
  echo "error: bundle is missing Contents/MacOS/tradeautopsy-agent" >&2
  exit 1
fi

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/station-dmg.XXXXXX")"
cleanup() { rm -rf "${STAGE}"; }
trap cleanup EXIT

ditto "${APP}" "${STAGE}/${APP_NAME}.app"
ln -s /Applications "${STAGE}/Applications"

rm -f "${DMG}"
echo "Creating ${DMG}..."
hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${STAGE}" \
  -ov \
  -format UDZO \
  "${DMG}"

echo "Wrote ${DMG}"
echo "Send that file (Drive / GitHub). Recipient: open DMG, drag to Applications, right-click Open."

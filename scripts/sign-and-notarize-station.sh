#!/usr/bin/env bash
# Local Developer ID sign + notarize helper for TradeAutopsy Station.
# Optional env vars — skip steps when unset.
#
#   APPLE_CERTIFICATE_BASE64   Base64-encoded .p12 (imported to a temp keychain)
#   APPLE_CERTIFICATE_PASSWORD Password for the .p12
#   APPLE_SIGNING_IDENTITY     e.g. "Developer ID Application: …"
#   TEAM_ID                    Apple Developer team ID
#   APPLE_ID                   Notary Apple ID email
#   APPLE_APP_SPECIFIC_PASSWORD App-specific password for notarytool
#
#   APP_PATH                   Path to TradeAutopsy Station.app (required)
#   ENTITLEMENTS               Defaults to station/StationApp/TradeAutopsy Station.entitlements
#   DMG_OUT                    Output DMG path (default: dist/TradeAutopsy-Station.dmg)
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="TradeAutopsy Station"
APP_PATH="${APP_PATH:-}"
ENTITLEMENTS="${ENTITLEMENTS:-${ROOT}/station/StationApp/TradeAutopsy Station.entitlements}"
DMG_OUT="${DMG_OUT:-${ROOT}/dist/TradeAutopsy-Station.dmg}"
KC="${KEYCHAIN_PATH:-${TMPDIR:-/tmp}/station-signing.keychain-db}"
KC_PASS="${KEYCHAIN_PASSWORD:-station-signing}"

usage() {
  cat <<EOF
Usage: APP_PATH=/path/to/${APP_NAME}.app $(basename "$0")

Signs tradeautopsy-agent and the app bundle, packs a DMG, and notarizes when
Apple credentials are present in the environment (see script header).
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "${APP_PATH}" || ! -d "${APP_PATH}" ]]; then
  echo "error: set APP_PATH to an existing ${APP_NAME}.app" >&2
  usage >&2
  exit 1
fi

if [[ ! -f "${ENTITLEMENTS}" ]]; then
  echo "error: entitlements not found at ${ENTITLEMENTS}" >&2
  exit 1
fi

AGENT_BIN="${APP_PATH}/Contents/MacOS/tradeautopsy-agent"
MAIN_BIN="${APP_PATH}/Contents/MacOS/${APP_NAME}"
if [[ ! -x "${AGENT_BIN}" ]]; then
  echo "error: missing ${AGENT_BIN}" >&2
  exit 1
fi

sign_if_configured() {
  if [[ -z "${APPLE_SIGNING_IDENTITY:-}" ]]; then
    echo "APPLE_SIGNING_IDENTITY unset — skipping codesign"
    return 0
  fi

  if [[ -n "${APPLE_CERTIFICATE_BASE64:-}" ]]; then
    CERT_PATH="${TMPDIR:-/tmp}/station-cert.p12"
    echo -n "${APPLE_CERTIFICATE_BASE64}" | base64 --decode > "${CERT_PATH}"
    security create-keychain -p "${KC_PASS}" "${KC}" 2>/dev/null || true
    security set-keychain-settings -lut 21600 "${KC}"
    security unlock-keychain -p "${KC_PASS}" "${KC}"
    security import "${CERT_PATH}" -P "${APPLE_CERTIFICATE_PASSWORD:-}" -A -t cert -f pkcs12 -k "${KC}"
    security list-keychain -d user -s "${KC}"
  fi

  echo "Signing embedded agent..."
  codesign --force --options runtime --timestamp \
    --sign "${APPLE_SIGNING_IDENTITY}" \
    "${AGENT_BIN}"

  echo "Signing app..."
  codesign --force --options runtime --timestamp \
    --entitlements "${ENTITLEMENTS}" \
    --sign "${APPLE_SIGNING_IDENTITY}" \
    "${APP_PATH}"

  codesign --verify --deep --strict "${APP_PATH}"
}

sign_if_configured

STAGE="$(mktemp -d "${TMPDIR:-/tmp}/station-dmg.XXXXXX")"
cleanup() { rm -rf "${STAGE}"; }
trap cleanup EXIT

ditto "${APP_PATH}" "${STAGE}/${APP_NAME}.app"
ln -s /Applications "${STAGE}/Applications"
mkdir -p "$(dirname "${DMG_OUT}")"
rm -f "${DMG_OUT}"
hdiutil create -volname "${APP_NAME}" -srcfolder "${STAGE}" -ov -format UDZO "${DMG_OUT}"
echo "Wrote ${DMG_OUT}"

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "${TEAM_ID:-}" ]]; then
  echo "Submitting DMG for notarization..."
  xcrun notarytool submit "${DMG_OUT}" \
    --apple-id "${APPLE_ID}" \
    --password "${APPLE_APP_SPECIFIC_PASSWORD}" \
    --team-id "${TEAM_ID}" \
    --wait
  xcrun stapler staple "${DMG_OUT}"
  echo "Notarized and stapled ${DMG_OUT}"
else
  echo "APPLE_ID / APPLE_APP_SPECIFIC_PASSWORD / TEAM_ID not all set — skipping notarization"
fi

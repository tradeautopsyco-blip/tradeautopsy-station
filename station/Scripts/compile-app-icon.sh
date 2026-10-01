#!/usr/bin/env bash
# Compile StationApp/AppIcon.icon into the app bundle as AppIcon.icns.
# Finder ignores a raw .icon folder. actool emits the icns (and Assets.car).
# Xcode 16 actool cannot compile Icon Composer files; fall back to the committed icns.
set -euo pipefail

if [[ -z "${BUILT_PRODUCTS_DIR:-}" || -z "${PRODUCT_NAME:-}" || -z "${SRCROOT:-}" ]]; then
  echo "error: compile-app-icon.sh must run as an Xcode build phase" >&2
  exit 1
fi

APP="${BUILT_PRODUCTS_DIR}/${PRODUCT_NAME}.app"
RES="${APP}/Contents/Resources"
ICON_SRC="${SRCROOT}/StationApp/AppIcon.icon"
FALLBACK="${SRCROOT}/StationApp/AppIcon.icns"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$RES"

compiled=0
if [[ -d "$ICON_SRC" ]]; then
  if xcrun actool "$ICON_SRC" \
    --compile "$TMP" \
    --app-icon AppIcon \
    --platform macosx \
    --target-device mac \
    --minimum-deployment-target "${MACOSX_DEPLOYMENT_TARGET:-14.0}" \
    --output-partial-info-plist "$TMP/partial.plist" \
    >/dev/null 2>"$TMP/actool.err"
  then
    if [[ -s "$TMP/AppIcon.icns" ]]; then
      cp "$TMP/AppIcon.icns" "$RES/AppIcon.icns"
      if [[ -s "$TMP/Assets.car" ]]; then
        cp "$TMP/Assets.car" "$RES/Assets.car"
      fi
      compiled=1
    fi
  fi
fi

if [[ "$compiled" != 1 ]]; then
  if [[ -s "$FALLBACK" ]]; then
    echo "warning: actool did not emit AppIcon.icns; copying committed StationApp/AppIcon.icns" >&2
    if [[ -s "$TMP/actool.err" ]]; then
      cat "$TMP/actool.err" >&2
    fi
    cp "$FALLBACK" "$RES/AppIcon.icns"
  else
    echo "error: AppIcon.icns was not produced. actool output:" >&2
    [[ -s "$TMP/actool.err" ]] && cat "$TMP/actool.err" >&2
    exit 1
  fi
fi

# Do not ship Icon Composer source. Finder will not use it as the app icon.
rm -rf "$RES/AppIcon.icon"

if [[ ! -s "$RES/AppIcon.icns" ]]; then
  echo "error: ${RES}/AppIcon.icns is missing after icon compile" >&2
  exit 1
fi

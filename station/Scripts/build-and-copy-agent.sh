#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
AGENT_DIR="${REPO_ROOT}/agent"

# Xcode Run Script phases inherit a minimal PATH — rustup is usually ~/.cargo/bin.
if ! command -v cargo >/dev/null 2>&1; then
  export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH:-}"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found. Install Rust (https://rustup.rs) and rebuild." >&2
  exit 1
fi

GIT_SHA="$(git -C "${REPO_ROOT}" rev-parse --short=12 HEAD)"

# Parallel `cargo` on macOS can leave corrupt proc-macro .dylibs (dyld: mis-aligned
# LINKEDIT string pool) → wit-parser/serde "cannot find attribute `serde`". Serialize jobs.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

echo "Building tradeautopsy-agent (GIT_SHA=${GIT_SHA}, jobs=${CARGO_BUILD_JOBS})..."
(
  cd "${AGENT_DIR}"
  GIT_SHA="${GIT_SHA}" cargo build --release --locked -j "${CARGO_BUILD_JOBS}" --bin tradeautopsy-agent
)

AGENT_BIN="${AGENT_DIR}/target/release/tradeautopsy-agent"
if [ ! -f "${AGENT_BIN}" ]; then
  echo "error: Agent binary not found at ${AGENT_BIN}" >&2
  exit 1
fi

if [ $# -ge 1 ]; then
  DEST="$1"
elif [ -n "${BUILT_PRODUCTS_DIR:-}" ] && [ -n "${PRODUCT_NAME:-}" ]; then
  DEST="${BUILT_PRODUCTS_DIR}/${PRODUCT_NAME}.app/Contents/MacOS"
else
  echo "error: pass app bundle MacOS path as arg, or set BUILT_PRODUCTS_DIR and PRODUCT_NAME" >&2
  exit 1
fi

mkdir -p "${DEST}"

cp "${AGENT_BIN}" "${DEST}/tradeautopsy-agent"
chmod +x "${DEST}/tradeautopsy-agent"

# UBI Wasm components next to the agent (ADR 0001). Build if missing.
WASM_TARGET="wasm32-wasip2"
WASM_PROFILE="release"
WASM_PACKAGES=(
  "ubi-binance-com-adapter:ubi_binance_com_adapter.wasm"
  "ubi-kotak-neo-adapter:ubi_kotak_neo_adapter.wasm"
  "ubi-zerodha-kite-adapter:ubi_zerodha_kite_adapter.wasm"
  "ubi-upstox-adapter:ubi_upstox_adapter.wasm"
  "ubi-fyers-adapter:ubi_fyers_adapter.wasm"
  "ubi-groww-adapter:ubi_groww_adapter.wasm"
  "ubi-dhan-adapter:ubi_dhan_adapter.wasm"
)

resolve_wasm_path() {
  local pkg="$1"
  local wasm_file="$2"
  local workspace_path="${AGENT_DIR}/target/${WASM_TARGET}/${WASM_PROFILE}/${wasm_file}"
  local crate_path="${AGENT_DIR}/${pkg}/target/${WASM_TARGET}/${WASM_PROFILE}/${wasm_file}"
  if [ -f "${workspace_path}" ]; then
    echo "${workspace_path}"
  elif [ -f "${crate_path}" ]; then
    echo "${crate_path}"
  else
    echo ""
  fi
}

echo "Building UBI adapter Wasm components..."
for entry in "${WASM_PACKAGES[@]}"; do
  pkg="${entry%%:*}"
  wasm_file="${entry##*:}"
  wasm_path="$(resolve_wasm_path "${pkg}" "${wasm_file}")"
  if [ -z "${wasm_path}" ]; then
    echo "  cargo build -p ${pkg} --target ${WASM_TARGET} --release"
    (cd "${AGENT_DIR}" && cargo build --release -j "${CARGO_BUILD_JOBS}" --target "${WASM_TARGET}" -p "${pkg}")
    wasm_path="$(resolve_wasm_path "${pkg}" "${wasm_file}")"
  fi
  if [ -n "${wasm_path}" ] && [ -f "${wasm_path}" ]; then
    cp "${wasm_path}" "${DEST}/${wasm_file}"
    echo "  copied ${wasm_file}"
  else
    echo "warning: missing ${wasm_file} (workspace or ${pkg}/target) — Start will fail for that broker slug" >&2
  fi
done

# Ad-hoc sign so app codesign accepts nested unsigned wasm
codesign --force --sign - "${DEST}/tradeautopsy-agent" || true
for wasm in "${DEST}"/ubi_*_adapter.wasm; do
  [ -f "${wasm}" ] && codesign --force --sign - "${wasm}" || true
done

VERSION="$(grep -E '^version\s*=' "${AGENT_DIR}/Cargo.toml" | head -1 | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/')"
BUILD_ID="tradeautopsy-agent/${VERSION} (${GIT_SHA})"
echo "Embedded build identity: ${BUILD_ID}"

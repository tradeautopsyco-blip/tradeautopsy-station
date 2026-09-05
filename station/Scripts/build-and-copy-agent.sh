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

echo "Building tradeautopsy-agent (GIT_SHA=${GIT_SHA})..."
(
  cd "${AGENT_DIR}"
  GIT_SHA="${GIT_SHA}" cargo build --release --locked --bin tradeautopsy-agent
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

# First-pair Wasm next to agent (B5 / T2 dogfood)
COM_WASM="${AGENT_DIR}/ubi-binance-com-adapter/target/wasm32-wasip2/release/ubi_binance_com_adapter.wasm"
KOTAK_WASM="${AGENT_DIR}/ubi-kotak-neo-adapter/target/wasm32-wasip2/release/ubi_kotak_neo_adapter.wasm"
[ -f "${COM_WASM}" ] && cp "${COM_WASM}" "${DEST}/ubi_binance_com_adapter.wasm"
[ -f "${KOTAK_WASM}" ] && cp "${KOTAK_WASM}" "${DEST}/ubi_kotak_neo_adapter.wasm"

# Ad-hoc sign so app codesign accepts nested unsigned wasm
for f in tradeautopsy-agent ubi_binance_com_adapter.wasm ubi_kotak_neo_adapter.wasm; do
  [ -f "${DEST}/${f}" ] && codesign --force --sign - "${DEST}/${f}" || true
done

VERSION="$(grep -E '^version\s*=' "${AGENT_DIR}/Cargo.toml" | head -1 | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/')"
BUILD_ID="tradeautopsy-agent/${VERSION} (${GIT_SHA})"
echo "Embedded build identity: ${BUILD_ID}"

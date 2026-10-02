#!/usr/bin/env bash
# Station path stress — ramp every loopback path until the first break.
#
# Re-run from the repo root:
#   ./scripts/station-path-stress.sh
#
# Knobs (all optional):
#   STATION_STRESS_LADDER=1,4,16,32,64,128,256,512   concurrency steps
#   STATION_STRESS_SOAK=2                    extra bursts at the passing ceiling
#   STATION_STRESS_TIMEOUT_MS=5000           per-request deadline
#   STATION_STRESS_OUT=plans/STATION-PATH-STRESS-RESULTS.json
#   STATION_STRESS_MD=plans/STATION-PATH-STRESS.md
#
# Boots a test agent on an ephemeral port plus a stub Console. No broker orders,
# no live Kotak/Binance/WorkOS calls, no daemon secret printed.
# Paths that need a Mac, Keychain, Sparkle, or a live Console URL are called out
# in the markdown report; they are not invited here.
#
# `cargo test` without STATION_STRESS=1 compiles this harness and returns
# immediately so CI does not ramp.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export STATION_STRESS=1
export STATION_STRESS_PROFILE="${STATION_STRESS_PROFILE:-debug}"
export STATION_STRESS_OUT="${STATION_STRESS_OUT:-$ROOT/plans/STATION-PATH-STRESS-RESULTS.json}"
export STATION_STRESS_MD="${STATION_STRESS_MD:-$ROOT/plans/STATION-PATH-STRESS.md}"
export TRADEAUTOPSY_HOSTS_FILE="${TRADEAUTOPSY_HOSTS_FILE:-/tmp/ta-station-path-stress-hosts}"

cd "$ROOT/agent"
cargo test --test station_path_stress -- --nocapture --test-threads=1 station_path_stress_ramp

echo "report ${STATION_STRESS_MD}"
echo "json   ${STATION_STRESS_OUT}"

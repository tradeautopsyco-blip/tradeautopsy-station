#!/usr/bin/env bash
# Tiny-check scoreboard for public dogfood seams.
# Add a row when you add a public-seam behavior. Do not test internals.
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
passed=0
failed=0
results=()

run_check() {
  local name="$1"
  shift
  echo "── $name"
  if (cd "$ROOT" && "$@"); then
    results+=("PASS  $name")
    passed=$((passed + 1))
  else
    results+=("FAIL  $name")
    failed=$((failed + 1))
  fi
}

export TRADEAUTOPSY_HOSTS_FILE="${TRADEAUTOPSY_HOSTS_FILE:-/tmp/test-hosts-dogfood}"

run_check "pick_route obtain (licensed_history, never Yahoo)" \
  bash -c "cd \"$ROOT/agent\" && cargo test --test kotak_history_pick_route -- --nocapture"

run_check "health JSON does not leak credential-shaped keys" \
  bash -c "cd \"$ROOT/agent\" && cargo test --test health -- --nocapture"

run_check "DetectCard + RiskDeskMode (default A, DualNoBlend, no invented stop)" \
  swift test --package-path "$ROOT/notch" --filter RiskDeskModeTests

run_check "DetectCard evaluate (plan vs live SL)" \
  swift test --package-path "$ROOT/notch" --filter DetectCardTests

run_check "S8 BarAccountChrome (shipping book of Start ≠ Today fills)" \
  swift test --package-path "$ROOT/notch" --filter BarAccountChromeTests

run_check "DeskHonesty (COM USD + Kotak INR does not blend)" \
  swift test --package-path "$ROOT/station" --filter DeskHonestyTests

run_check "Today blotter (MTM em dash, inspector does not invent)" \
  swift test --package-path "$ROOT/station" --filter TodayRiskDeskTests

run_check "AgentSupervisor (slow 9137 bind stays healthy; short wait stays offline)" \
  swift test --package-path "$ROOT/station" --filter AgentSupervisorTests

echo
echo "=== dogfood score ==="
for line in "${results[@]}"; do
  echo "$line"
done
total=$((passed + failed))
echo "score ${passed}/${total}"
if [ "$failed" -ne 0 ]; then
  exit 1
fi

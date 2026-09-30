#!/usr/bin/env bash
# Formal Gate B soak — dry-run only, no broker orders. Never prints secrets.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG="${ROOT}/plans/NOTCH-GATE-B-SOAK-2026-09-28.log"
JSON="${ROOT}/plans/NOTCH-LIVE-SCOREBOARD-2026-09-28.json"
TICKS="${1:-10}"
INTERVAL_SEC="${2:-120}"
LISTEN_PID="$(lsof -nP -iTCP:9137 -sTCP:LISTEN -t 2>/dev/null | head -1 || true)"
AGENT_CMD="$(ps -p "${LISTEN_PID:-0}" -o command= 2>/dev/null | head -c 240 || true)"

{
  echo "soak_start $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "agent_pid=${LISTEN_PID:-none}"
  echo "agent_cmd=${AGENT_CMD:-unknown}"
  echo "ticks=${TICKS} interval_sec=${INTERVAL_SEC}"
} | tee "$LOG"

pass=0
fail=0
for i in $(seq 1 "$TICKS"); do
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "--- tick ${i}/${TICKS} ${ts} ---" >>"$LOG"
  if ! cd "$ROOT" && timeout 90 ./scripts/notch-live-journal.sh --dry-run >>"$LOG" 2>&1; then
    echo "harness_exit=FAIL" >>"$LOG"
    fail=$((fail + 1))
  else
    echo "harness_exit=OK" >>"$LOG"
    pass=$((pass + 1))
  fi
  ga="$(python3 -c "import json; d=json.load(open('$JSON')); print(d.get('gates',{}).get('A_health','?'))" 2>/dev/null || echo '?')"
  gb="$(python3 -c "import json; d=json.load(open('$JSON')); print(d.get('gates',{}).get('B_station_session','?'))" 2>/dev/null || echo '?')"
  echo "gate_a=${ga} gate_b=${gb}" >>"$LOG"
  if [[ "$gb" != "PASS" ]]; then
    fail=$((fail + 1))
  fi
  [[ "$i" -lt "$TICKS" ]] && sleep "$INTERVAL_SEC"
done

echo "soak_end $(date -u +%Y-%m-%dT%H:%M:%SZ) ticks_ok=${pass} gate_b_fail_ticks=${fail}" | tee -a "$LOG"

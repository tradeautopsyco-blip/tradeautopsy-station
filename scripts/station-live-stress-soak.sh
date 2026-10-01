#!/usr/bin/env bash
# Paper-only live stress soak. Never prints secrets. Never places broker orders.
# See plans/STATION-LIVE-STRESS-SOAK.md
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "${ROOT}/scripts/station-live-stress-soak.py" --root "${ROOT}" "$@"

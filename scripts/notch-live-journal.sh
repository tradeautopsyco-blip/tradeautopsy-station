#!/usr/bin/env bash
# Phase B live journal harness — wire-v1 gates, declare+cancel cleanup, tools honesty matrix.
# Loads AGENT_DAEMON_SECRET from the agent process environ (never printed). See plans/NOTCH-LIVE-JOURNAL-HARNESS.md
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "${ROOT}/scripts/notch-live-journal-lib.py" --root "${ROOT}" "$@"

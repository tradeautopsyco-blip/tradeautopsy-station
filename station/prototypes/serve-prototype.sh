#!/bin/sh
# PROTOTYPE — throwaway. One command to view risk-layer variants.
cd "$(dirname "$0")"
PORT="${PORT:-8766}"
URL="http://127.0.0.1:${PORT}/risk-flow/PROTOTYPE-risk-flow.html?variant=detect"
echo "PROTOTYPE flow (throwaway) — $URL"
echo "Old layout variants (wrong question) — http://127.0.0.1:${PORT}/PROTOTYPE-risk-layers.html?variant=A"
echo "Scenes: detect → today → console → notch → circle. ← →"
python3 -m http.server "$PORT"

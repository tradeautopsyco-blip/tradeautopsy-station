#!/bin/sh
# PROTOTYPE — throwaway. One command to view HTML variants.
cd "$(dirname "$0")"
PORT="${PORT:-8766}"
echo "S7 source route — http://127.0.0.1:${PORT}/PROTOTYPE-s7-source-route.html"
echo "S8 account chrome — http://127.0.0.1:${PORT}/PROTOTYPE-s8-account-chrome.html?variant=C"
echo "Risk flow — http://127.0.0.1:${PORT}/risk-flow/PROTOTYPE-risk-flow.html?variant=detect"
echo "Old risk layout (wrong question) — http://127.0.0.1:${PORT}/PROTOTYPE-risk-layers.html?variant=A"
echo "Today one day + now — http://127.0.0.1:${PORT}/PROTOTYPE-today-one-day.html?variant=A"
echo "Notch Apple HIG — http://127.0.0.1:${PORT}/PROTOTYPE-notch-hig.html?variant=A"
echo "Notch HIG × desk — http://127.0.0.1:${PORT}/PROTOTYPE-notch-desk.html?variant=A"
echo "← → or the bottom bar. Ctrl-C to stop."
python3 -m http.server "$PORT"

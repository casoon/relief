#!/usr/bin/env bash
# Aufgabendateien im Fork abarbeiten (--relief-run), Aktionen über
# AXActionData. Ausgabe wie `relief-cdp run`; Rückgabewert 1 bei nicht
# erfüllten Erwartungen.
#
#   scripts/fork-run-tasks.sh spike/tasks/0[1-5]*.txt
#
# RELIEF_CHROMIUM setzt den Browser (Standard: der eigene Build unter
# ~/chromium/src/out/Relief), RELIEF_LOG das Messprotokoll.
set -euo pipefail

CHROMIUM="${RELIEF_CHROMIUM:-$HOME/chromium/src/out/Relief/Chromium.app/Contents/MacOS/Chromium}"
if [[ $# -eq 0 ]]; then
  echo "Aufruf: $0 <aufgaben.txt>..." >&2
  exit 2
fi
files=$(IFS=,; echo "$*")
profile=$(mktemp -d)
trap 'rm -rf "$profile"' EXIT

args=(--enable-relief --no-first-run --use-mock-keychain
      --user-data-dir="$profile" --relief-run="$files"
      # Verdeckte Fenster rendern nicht, Blink serialisiert dann keinen Baum.
      --disable-backgrounding-occluded-windows)
if [[ -n "${RELIEF_LOG:-}" ]]; then
  args+=(--relief-log="$RELIEF_LOG")
fi
"$CHROMIUM" "${args[@]}" 2>/dev/null

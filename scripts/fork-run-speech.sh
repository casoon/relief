#!/usr/bin/env bash
# Aufgabendateien mit gesprochenen Eingaben im Fork (Paket 26).
#
#   scripts/fork-run-speech.sh spike/tasks/17-sprache.txt
#
# Jede Zeile `sprich: <Text>` wird mit `say` (Stimme Anna, Deutsch) zu einer
# Audiodatei und als `audio:` an die Spracherkennung des Forks gegeben.
# Relief startet über `open`, damit macOS die Freigabe für die
# Spracherkennung Relief zuordnet (einmalig bestätigen). Die Antworten
# spricht Relief mit Lautstärke 0 (RELIEF_SPEECH_VOLUME setzt sie).
# Ausgabe wie scripts/fork-run-tasks.sh; Rückgabewert 1 bei nicht
# erfüllten Erwartungen.
set -euo pipefail

APP="${RELIEF_APP:-$HOME/chromium/src/out/Relief/Relief.app}"
[[ $# -gt 0 ]] || { echo "Aufruf: $0 <aufgaben.txt>..." >&2; exit 2; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
files=()
n=0
for task in "$@"; do
  dir=$(cd "$(dirname "$task")" && pwd)
  out="$work/$(basename "$task")"
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ "$line" =~ ^[[:space:]]*sprich:[[:space:]]*(.*)$ ]]; then
      text="${BASH_REMATCH[1]}"
      n=$((n + 1))
      say -v Anna -o "$work/$n.aiff" "$text"
      printf '# sprich: %s\naudio: %s\n' "$text" "$work/$n.aiff" >> "$out"
    elif [[ "$line" =~ ^([[:space:]]*url:[[:space:]]*)(\.\.?/.*)$ ]]; then
      # Relative Adressen gelten zur ursprünglichen Datei.
      printf 'url: file://%s/%s\n' "$dir" "${BASH_REMATCH[2]}" >> "$out"
    else
      printf '%s\n' "$line" >> "$out"
    fi
  done < "$task"
  files+=("$out")
done

list=$(IFS=,; echo "${files[*]}")
log="$work/stdout.txt"
open -n -W -a "$APP" --stdout "$log" --stderr /dev/null --args \
  --enable-relief --no-first-run --use-mock-keychain \
  --user-data-dir="$work/profile" --relief-run="$list" \
  --disable-backgrounding-occluded-windows \
  --relief-speech-volume="${RELIEF_SPEECH_VOLUME:-0}" \
  ${RELIEF_LOG:+--relief-log="$RELIEF_LOG"}
cat "$log"
grep -q ", 0 nicht erfüllt" "$log"

#!/usr/bin/env bash
# Erzeugt fork/patches/ und fork/relief/ aus einem Chromium-Checkout neu.
#
#   scripts/fork-export.sh <chromium-src>
#
# Voraussetzung: der Checkout wurde mit scripts/fork-apply.sh vorbereitet
# (Branch `relief`, Basis refs/relief/base). Jeder Commit in
# refs/relief/base..relief wird ein Patch; Dateiname aus dem Betreff, ohne
# laufende Nummer (Reihenfolge steht in fork/patches/series). Anschließend
# wird <chromium-src>/relief/ nach fork/relief/ zurückkopiert und
# fork/UPSTREAM auf die Version des Checkouts gesetzt. <chromium-src>/relief/crates/
# ist eine Kopie aus crates/ (scripts/fork-apply.sh) und geht nicht zurück.
# Format: fork/README.md.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "Aufruf: $0 <chromium-src>" >&2; exit 2; }
SRC=$(cd "$1" && pwd)
ROOT=$(cd "$(dirname "$0")/.." && pwd)
FORK="$ROOT/fork"

cr() { git -C "$SRC" "$@"; }

cr show-ref --verify --quiet refs/relief/base \
  || { echo "refs/relief/base fehlt; erst scripts/fork-apply.sh." >&2; exit 1; }
[[ "$(cr rev-parse --abbrev-ref HEAD)" == relief ]] \
  || { echo "Checkout steht nicht auf Branch 'relief'." >&2; exit 1; }
if [[ -n "$(cr status --porcelain --untracked-files=no)" ]]; then
  echo "Nicht committete Änderungen an Chromium-Dateien; erst committen." >&2
  exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# --zero-commit/--no-signature: stabile Dateien ohne Rauschen.
# --full-index: volle Blob-IDs, damit `git am --3way` die Basis findet.
cr format-patch --quiet --zero-commit --no-signature --full-index \
  --keep-subject --no-stat -o "$TMP" refs/relief/base..relief

rm -f "$FORK/patches/"*.patch
: > "$FORK/patches/series.new"
for p in "$TMP"/*.patch; do
  [[ -e "$p" ]] || continue
  name=$(basename "$p")
  name=${name#[0-9][0-9][0-9][0-9]-}
  cp "$p" "$FORK/patches/$name"
  echo "$name" >> "$FORK/patches/series.new"
done
mv "$FORK/patches/series.new" "$FORK/patches/series"

if [[ -d "$SRC/relief" ]]; then
  mkdir -p "$FORK/relief"
  rsync -a --delete --exclude /crates/ "$SRC/relief/" "$FORK/relief/"
fi

# shellcheck disable=SC1090,SC1091
(source "$SRC/chrome/VERSION" && echo "$MAJOR.$MINOR.$BUILD.$PATCH") > "$FORK/UPSTREAM"

echo "$(wc -l < "$FORK/patches/series" | tr -d ' ') Patches exportiert," \
     "Basis $(cat "$FORK/UPSTREAM")."

#!/usr/bin/env bash
# Wendet den Relief-Fork auf einen Chromium-Checkout an.
#
#   scripts/fork-apply.sh <chromium-src> [--force]
#
# 1. Prüft, dass <chromium-src> auf der Version aus fork/UPSTREAM steht
#    (chrome/VERSION); Abweichung bricht ab, --force lässt sie zu (Rebase).
# 2. Legt den lokalen Branch `relief` am aktuellen HEAD an und merkt den
#    Ausgangspunkt als refs/relief/base (für scripts/fork-export.sh).
# 3. Wendet fork/patches/ in der Reihenfolge von fork/patches/series mit
#    `git am --3way` an: jeder Patch wird ein Commit.
# 4. Kopiert fork/relief/ nach <chromium-src>/relief/ und die Quellen der
#    Rust-Crates, die der Fork baut (RUST_CRATES), nach
#    <chromium-src>/relief/crates/ (unversioniert in Chromium; Quelle der
#    Wahrheit ist dieses Repo).
#
# Bricht ein Patch, bleibt `git am` offen stehen: Konflikt im Checkout lösen,
# `git -C <chromium-src> am --continue`, danach dieses Skript erneut mit
# --continue aufrufen oder direkt scripts/fork-export.sh. --continue kopiert
# nur (Schritt 4) und taugt damit auch zum Nachziehen geänderter Quellen.
# Format und Ablauf: fork/README.md.
set -euo pipefail

usage() {
  echo "Aufruf: $0 <chromium-src> [--force | --continue]" >&2
  exit 2
}

[[ $# -ge 1 ]] || usage
SRC=$(cd "$1" && pwd)
MODE=${2:-}
[[ -z "$MODE" || "$MODE" == --force || "$MODE" == --continue ]] || usage

ROOT=$(cd "$(dirname "$0")/.." && pwd)
FORK="$ROOT/fork"
UPSTREAM=$(tr -d '[:space:]' < "$FORK/UPSTREAM")

cr() { git -C "$SRC" "$@"; }

# Version aus chrome/VERSION (MAJOR=…, MINOR=…, BUILD=…, PATCH=…).
version() {
  # shellcheck disable=SC1090,SC1091
  (source "$SRC/chrome/VERSION" && echo "$MAJOR.$MINOR.$BUILD.$PATCH")
}

# Rust-Crates aus crates/, die //relief/BUILD.gn baut (nur src/; das
# GN-Target ersetzt dort Cargo.toml).
RUST_CRATES=(relief-model relief-bridge)

copy_relief() {
  [[ -d "$FORK/relief" ]] || { echo "fork/relief/ fehlt, nichts zu kopieren."; return; }
  mkdir -p "$SRC/relief"
  rsync -a --delete --exclude /crates/ "$FORK/relief/" "$SRC/relief/"
  for crate in "${RUST_CRATES[@]}"; do
    mkdir -p "$SRC/relief/crates/$crate"
    rsync -a --delete "$ROOT/crates/$crate/src/" "$SRC/relief/crates/$crate/src/"
  done
  echo "fork/relief/ und crates/{$(IFS=,; echo "${RUST_CRATES[*]}")}/src -> $SRC/relief/ kopiert."
}

if [[ "$MODE" == --continue ]]; then
  copy_relief
  exit 0
fi

HAVE=$(version)
if [[ "$HAVE" != "$UPSTREAM" ]]; then
  if [[ "$MODE" != --force ]]; then
    echo "Checkout ist $HAVE, fork/UPSTREAM verlangt $UPSTREAM." >&2
    echo "Für einen Rebase mit --force aufrufen." >&2
    exit 1
  fi
  echo "Warnung: wende Patches für $UPSTREAM auf $HAVE an (Rebase)." >&2
fi

if cr show-ref --verify --quiet refs/heads/relief; then
  echo "Branch 'relief' existiert schon in $SRC — Fork ist angewendet." >&2
  echo "Neu anwenden: 'git -C $SRC checkout --detach refs/relief/base &&" \
       "git -C $SRC branch -D relief', dann erneut aufrufen." >&2
  exit 1
fi

if [[ -n "$(cr status --porcelain --untracked-files=no)" ]]; then
  echo "Checkout hat lokale Änderungen an versionierten Dateien; abgebrochen." >&2
  exit 1
fi

cr update-ref refs/relief/base HEAD
cr switch --quiet -c relief

PATCHES=()
while IFS= read -r line; do
  line=${line%%#*}
  line=${line//[[:space:]]/}
  [[ -n "$line" ]] && PATCHES+=("$FORK/patches/$line")
done < "$FORK/patches/series"

if [[ ${#PATCHES[@]} -gt 0 ]]; then
  # Feste Identität, damit Commit-IDs nicht vom Rechner abhängen.
  if ! GIT_COMMITTER_NAME=relief GIT_COMMITTER_EMAIL=relief@invalid \
      git -C "$SRC" am --3way --keep-cr "${PATCHES[@]}"; then
    cat >&2 <<EOF

Patch passt nicht. Konflikt im Checkout lösen, dann:
  git -C $SRC am --continue
  $0 $SRC --continue
Meldet git am "invalid object" bzw. fehlende Blobs für den 3-Wege-Merge,
fehlen im flachen Checkout die Dateien der alten Basis $UPSTREAM.
Einmal holen und neu anwenden (fork/README.md):
  git -C $SRC am --abort
  git -C $SRC checkout --detach refs/relief/base
  git -C $SRC branch -D relief
  git -C $SRC fetch --depth 1 origin tag $UPSTREAM
  $0 $SRC --force
EOF
    exit 1
  fi
fi
echo "${#PATCHES[@]} Patches angewendet (Branch 'relief', Basis refs/relief/base)."

copy_relief

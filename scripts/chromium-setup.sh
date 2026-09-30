#!/usr/bin/env bash
# Chromium für Relief auf macOS (Apple Silicon) auschecken und bauen (Backlog 14).
#
# Wiederholbar: jeder Schritt überspringt, was schon erledigt ist.
#   CHROMIUM_DIR  Ablage des Checkouts         (Standard: ~/chromium)
#   DEPOT_TOOLS   Ablage von depot_tools       (Standard: ~/depot_tools)
#   CHROMIUM_TAG  Stable-Tag                   (Standard: 154.0.8037.58)
#   OUT           Build-Verzeichnis unter src  (Standard: out/Relief)
#
# Platz: Checkout ohne Historie plus Build rund 60–80 GB.
# Netz: Die Hooks laden über eigene Programme nach (depot_tools-Python/vpython,
# cipd, gsutil). Eine Anwendungs-Firewall muss sie zulassen,
# sonst scheitern sie mit „No route to host“.
set -euo pipefail

CHROMIUM_DIR="${CHROMIUM_DIR:-$HOME/chromium}"
DEPOT_TOOLS="${DEPOT_TOOLS:-$HOME/depot_tools}"
CHROMIUM_TAG="${CHROMIUM_TAG:-154.0.8037.58}"
OUT="${OUT:-out/Relief}"

step() { printf '\n== %s (%s)\n' "$1" "$(date '+%H:%M:%S')"; }

step "Voraussetzungen"
# Einige Abhängigkeiten (z. B. third_party/litert) liegen in Git LFS.
command -v git-lfs >/dev/null || { echo "git-lfs fehlt: brew install git-lfs && git lfs install"; exit 1; }
# ANGLE übersetzt Metal-Shader; ab Xcode 26 ist die Toolchain eine eigene Komponente.
xcrun -f metal >/dev/null 2>&1 || { echo "Metal-Toolchain fehlt: xcodebuild -downloadComponent MetalToolchain"; exit 1; }

step "depot_tools"
if [ ! -d "$DEPOT_TOOLS" ]; then
  git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git "$DEPOT_TOOLS"
fi
export PATH="$DEPOT_TOOLS:$PATH"
# Frisch geklont muss depot_tools einmal initialisiert werden (Python-Umgebung).
if [ ! -f "$DEPOT_TOOLS/python3_bin_reldir.txt" ]; then
  "$DEPOT_TOOLS/update_depot_tools"
fi
# Danach nicht bei jedem Aufruf nach Updates suchen, damit Läufe vergleichbar bleiben.
export DEPOT_TOOLS_UPDATE=0

step "macOS-SDK"
ls "$(xcode-select -p)/Platforms/MacOSX.platform/Developer/SDKs"

step "Checkout ohne Historie"
mkdir -p "$CHROMIUM_DIR"
cd "$CHROMIUM_DIR"
if [ ! -f .gclient ]; then
  caffeinate -i fetch --nohooks --no-history chromium
fi

step "Tag $CHROMIUM_TAG"
# `gclient sync --revision src@refs/tags/…` versucht im Checkout ohne Historie
# alle Branches zu holen und hängt. Deshalb nur den Tag flach holen und
# gclient danach auf genau diesen Commit festlegen.
cd "$CHROMIUM_DIR/src"
if [ "$(git rev-parse HEAD)" != "$(git rev-parse -q --verify "refs/tags/$CHROMIUM_TAG^{commit}" || true)" ]; then
  git fetch --depth 1 origin "refs/tags/$CHROMIUM_TAG:refs/tags/$CHROMIUM_TAG"
  git checkout -f --detach "refs/tags/$CHROMIUM_TAG"
fi
COMMIT="$(git rev-parse HEAD)"
cd "$CHROMIUM_DIR"

step "Abhängigkeiten für $COMMIT"
caffeinate -i gclient sync --revision "src@$COMMIT" --no-history -D --force --nohooks
caffeinate -i gclient runhooks

step "gn gen $OUT"
cd "$CHROMIUM_DIR/src"
mkdir -p "$OUT"
cat > "$OUT/args.gn" <<ARGS
# Relief-Entwicklungsbuild: Release, Component-Build für schnelle
# inkrementelle Builds, ohne Symbole, Apples Linker (ARM-Macs).
is_debug = false
is_component_build = true
symbol_level = 0
use_lld = false
ARGS
gn gen "$OUT"

step "Build chrome"
caffeinate -i autoninja -C "$OUT" chrome

step "fertig"
echo "$CHROMIUM_DIR/src/$OUT/Chromium.app/Contents/MacOS/Chromium"

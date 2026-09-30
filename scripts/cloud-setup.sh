#!/usr/bin/env bash
# Setup-Skript für die Cloud-Umgebung auf claude.ai (Paket 10).
#
# Eintragen: Inhalt dieser Datei in das Feld „Setup script“ der Umgebung
# kopieren. Das Skript braucht keine Dateien aus dem Repo; es läuft als root
# auf Ubuntu 24.04, bevor Claude Code startet, und das Ergebnis wird als
# Dateisystem-Abbild zwischengespeichert (erneut nur bei geändertem Skript,
# geänderten Netzfreigaben oder nach etwa sieben Tagen).
#
# Wiederholbar: jeder Schritt überspringt, was schon da ist. Endet immer mit 0,
# weil ein Fehler sonst den Start der Session verhindert; Fehlschläge stehen
# als „!!“ in der Ausgabe.
#
#   RELIEF_CHROME=0    Chrome nicht installieren (Standard: 1)
#   CHROME_VERSION     Chrome for Testing   (Standard: 154.0.8037.57, nächste zu fork/UPSTREAM)
#   CHROME_DIR         Ablage               (Standard: /opt/chrome-for-testing)
#
# Netz: Das Standardnetz „Trusted“ genügt. Chrome kommt als Chrome for Testing
# von storage.googleapis.com (auf der Standardliste), die Bibliotheken aus
# Ubuntu-apt. Nicht nutzbar: dl.google.com (Google-Paket, nicht auf der
# Liste), googlechromelabs.github.io (Versionsliste, nicht auf der Liste;
# deshalb feste Version), Ubuntus `chromium` (Snap, läuft im Container nicht).
#
# Chrome startet im Container als root nur ohne Sandbox. Das Skript legt
# deshalb `/usr/local/bin/google-chrome-stable` als Hülle mit `--no-sandbox`
# an; `relief-cdp` findet sie ohne `CHROME` über den PATH. Vertretbar, weil
# die VM wegwerfbar ist und in der Cloud nur lokale file://-Seiten geladen
# werden. Der Host selbst startet Chrome weiterhin mit Sandbox.
set -uo pipefail

CHROME_VERSION="${CHROME_VERSION:-154.0.8037.57}"
CHROME_DIR="${CHROME_DIR:-/opt/chrome-for-testing}"
WRAPPER=/usr/local/bin/google-chrome-stable

step() { printf '\n== %s\n' "$1"; }
fail() { printf '!! %s\n' "$1"; }

step "Toolchain"
if command -v cargo >/dev/null; then
  cargo --version
  rustc --version
  # rust-version im Workspace: 1.85
  rustc --version | awk '{ split($2, v, "."); exit !(v[1] > 1 || (v[1] == 1 && v[2] >= 85)) }' ||
    fail "rustc älter als 1.85"
  for c in rustfmt clippy; do
    rustup component list --installed 2>/dev/null | grep -q "^$c" ||
      rustup component add "$c" || fail "rustup component add $c"
  done
else
  fail "cargo fehlt"
fi
command -v c++ >/dev/null || fail "C++-Compiler fehlt (relief-bridge nutzt cxx)"

step "Abhängigkeiten (cargo fetch)"
# Das Repo liegt beim Setup nicht an fester Stelle; gesucht wird im
# Arbeitsverzeichnis und unter /home/*/*. Ohne Fund lädt der erste
# `cargo`-Aufruf der Session nach (crates.io ist freigegeben).
repo=""
for d in "$PWD" /home/*/*; do
  if grep -qs '"crates/relief-cdp"' "$d/Cargo.toml"; then
    repo="$d"
    break
  fi
done
if [ -n "$repo" ]; then
  (cd "$repo" && cargo fetch --locked) || fail "cargo fetch in $repo"
else
  echo "Relief-Checkout nicht gefunden, übersprungen"
fi

if [ "${RELIEF_CHROME:-1}" = "1" ]; then
  step "Chrome for Testing $CHROME_VERSION"
  chrome="$CHROME_DIR/$CHROME_VERSION/chrome-linux64/chrome"
  if [ ! -x "$chrome" ]; then
    export DEBIAN_FRONTEND=noninteractive
    # Laufzeitbibliotheken für Ubuntu 24.04 (dieselbe Liste nutzt
    # browser-actions/setup-chrome im CI), dazu Schriften und unzip.
    { apt-get update -q &&
      apt-get install -y -q --no-install-recommends unzip curl ca-certificates \
        fonts-liberation libasound2t64 libatk-bridge2.0-0t64 libatk1.0-0t64 \
        libcairo2 libcups2t64 libdbus-1-3 libexpat1 libgbm1 libglib2.0-0t64 \
        libnss3 libpango-1.0-0 libxcomposite1 libxdamage1 libxfixes3 \
        libxkbcommon0 libxrandr2; } || fail "apt-get install"
    tmp="$(mktemp -d)"
    url="https://storage.googleapis.com/chrome-for-testing-public/$CHROME_VERSION/linux64/chrome-linux64.zip"
    if curl -fsSL --retry 3 -o "$tmp/chrome.zip" "$url"; then
      mkdir -p "$CHROME_DIR/$CHROME_VERSION"
      unzip -q -o "$tmp/chrome.zip" -d "$CHROME_DIR/$CHROME_VERSION" || fail "unzip"
    else
      fail "Download $url (storage.googleapis.com freigegeben?)"
    fi
    rm -rf "$tmp"
  fi
  if [ -x "$chrome" ]; then
    cat >"$WRAPPER" <<EOF
#!/bin/sh
# Angelegt von scripts/cloud-setup.sh (Relief): Chrome ohne Sandbox, nur Cloud-VM.
exec "$chrome" --no-sandbox "\$@"
EOF
    chmod 755 "$WRAPPER"
    "$WRAPPER" --version || fail "Chrome startet nicht (fehlende Bibliothek?)"
  fi
fi

step "Fertig"
exit 0

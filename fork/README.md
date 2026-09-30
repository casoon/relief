# fork/

Relief als Fork von Chromium: alles, was in einem Chromium-Checkout geändert
oder ergänzt wird, liegt hier. Strategie und Messung:
`plan/spezifikation/01-chromium-integration.md`, Abschnitt „Fork-Strategie“;
Entscheidung in `docs/decisions.md`, Takt in `docs/constraints.md`.

```
fork/
├── UPSTREAM          # Chromium-Version, auf die die Patches passen (z. B. 154.0.8037.58)
├── patches/
│   ├── series        # Reihenfolge, ein Dateiname je Zeile
│   └── *.patch       # je Eingriff außerhalb //relief/ ein Patch
└── relief/           # Inhalt von //relief/ (src/relief/), eigener Code
```

## `fork/relief/`

Eigener Code, wird unverändert nach `src/relief/` kopiert. Kein Patch, weil
Chromium dort nichts hat, was sich ändern könnte. Neue Dateien und Umbauten
gehören hierher, nicht in einen Patch.

Die Rust-Crates, die `relief/BUILD.gn` baut, liegen **nicht** hier, sondern
bleiben in `crates/` (Quelle der Wahrheit ist der Cargo-Workspace).
`scripts/fork-apply.sh` kopiert ihre `src/` nach `src/relief/crates/<crate>/src/`
(Liste `RUST_CRATES` im Skript), `scripts/fork-export.sh` lässt
`src/relief/crates/` aus. Nach Änderungen in `crates/` oder `fork/relief/`
kopiert `scripts/fork-apply.sh <chromium-src> --continue` die Quellen neu,
ohne Patches anzufassen.

Starten und messen: `docs/project-state.md` („Chromium bauen“),
Schalter in `relief/relief_switches.h`.

## `fork/patches/`

Nur Eingriffe in Chromium-Dateien außerhalb `//relief/`
(Liste in spezifikation/01, „Nötige Eingriffe außerhalb `//relief/`“).

- **Format:** `git format-patch`-Ausgabe (Mail-Format mit Betreff und
  Nachricht), erzeugt mit `--zero-commit --no-signature --full-index
  --keep-subject --no-stat`. `--full-index` trägt die vollen Blob-IDs der
  Basis, damit `git am --3way` beim Rebase einen 3-Wege-Merge rechnen kann.
- **Ein Patch je Eingriff**, nicht je Datei: Ein Eingriff, der zwei Dateien
  braucht (Member im Header + Erzeugung in der `.cc`), ist ein Patch.
- **Dateiname** aus dem Betreff, ohne laufende Nummer; die Reihenfolge steht
  allein in `series` (wie `patches/series` bei ungoogled-chromium und
  `.patches` bei Electron). Einfügen oder Entfernen benennt nichts um.
- **Betreff** beginnt mit `Relief:`; die Nachricht nennt den Grund, warum der
  Eingriff nicht in `//relief/` geht, und die Nummer aus der Eingriffsliste.
- **Inhalt klein und am stabilen Anker:** neue Zeilen neben Zeilen, die
  Upstream selten ändert (Include-Block, feste Member, Funktionsanfang), nicht
  am Ende wachsender Listen. Ausführlich: spezifikation/01, „Ankerregeln“.
- `UPSTREAM` nennt die Version, gegen die die Patches zuletzt exportiert
  wurden; `scripts/fork-export.sh` schreibt sie.

## Anwenden

```bash
scripts/fork-apply.sh ~/chromium/src
```

Voraussetzungen: `src` steht auf der Version aus `UPSTREAM` (geprüft über
`chrome/VERSION`), versionierte Dateien sind unverändert. Das Skript legt den
lokalen Branch `relief` an, merkt den Ausgangspunkt als `refs/relief/base`,
wendet die Patches mit `git am --3way` als Commits an und kopiert
`fork/relief/` nach `src/relief/`. Danach wie gewohnt `gn gen` und bauen.

Neu anwenden (etwa nach geänderten Patches oder vor einem `gclient sync` auf
ein anderes Tag): Branch verlassen, entfernen, erneut anwenden:

```bash
git -C ~/chromium/src checkout --detach refs/relief/base
git -C ~/chromium/src branch -D relief
scripts/fork-apply.sh ~/chromium/src
```

## Ändern und zurückschreiben

Im Checkout arbeiten: Eingriffe außerhalb `//relief/` als Commits auf dem
Branch `relief` (ein Commit je Eingriff, `git commit --fixup` +
`git rebase -i --autosquash refs/relief/base` zum Nachbessern), eigener Code
direkt in `src/relief/`. Dann:

```bash
scripts/fork-export.sh ~/chromium/src
```

Das erzeugt `fork/patches/` und `series` aus `refs/relief/base..relief` neu,
kopiert `src/relief/` nach `fork/relief/` und setzt `UPSTREAM`.

## Rebase auf ein neues Stable-Release

```bash
# 1. Checkout auf das neue Tag bringen (Paket 14: flacher Checkout)
#    und alte Relief-Commits verwerfen (siehe oben).
# 2. Anwenden trotz Versionsunterschied:
scripts/fork-apply.sh ~/chromium/src --force
# 3. Bei Konflikt: lösen, `git -C ~/chromium/src am --continue`,
#    dann `scripts/fork-apply.sh ~/chromium/src --continue`.
# 4. Bauen, Integrationstests (//relief/testing), dann zurückschreiben:
(cd ~/chromium/src && autoninja -C out/Relief chrome relief_browsertests \
  && out/Relief/relief_browsertests)
scripts/fork-export.sh ~/chromium/src
```

Flacher Checkout: `git am --3way` braucht die Dateien der alten Basis. Fehlen
sie (Meldung „invalid object“ bzw. fehlende Blobs), einmal die alte Version
holen — `git -C ~/chromium/src fetch --depth 1 origin tag <UPSTREAM>` — und
neu anwenden; das Skript gibt die Befehle bei Fehlschlag aus. Ohne 3-Wege
passen Patches nur, wenn der Kontext unverändert ist.

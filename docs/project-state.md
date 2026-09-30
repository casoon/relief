# Projektstand

> Eine normale Website ist eine Fläche.
> **RELIEF** macht ihre Struktur erfahrbar.

**Relief** (Arbeitsname): ein Chromium-Fork, der Webseiten über den
Chromium-Accessibility-Tree semantisch versteht und Darstellung und Bedienung
an die Fähigkeiten der Nutzerin oder des Nutzers anpasst — ergänzend zu
VoiceOver/NVDA, nicht als Ersatz.

## Stand

Forschungsprojekt, Phase 1 (Go am 2026-09-30; VoiceOver läuft neben Relief
unverändert). Zwei Linien: Assistenz im Browser und Prüfen im echten Browser;
Relief ersetzt den geplanten barrierlab-Reader. Es gibt
einen CDP-Spike gegen ein normales Chrome: Rust-Workspace mit sechs Crates,
siehe `docs/architecture.md`. Dazu das Fork-Grundgerüst: Chromium
154.0.8037.58 mit `//relief/` und drei Patches (`fork/`), lokal gebaut in
`~/chromium/src/out/Relief`. Mit `--enable-relief` liest er den AXTree samt
Positionen im Browser-Prozess, auch aus cross-site-iframes, führt ihn als
`SemanticGraph` in der Rust-Runtime nach (über Navigation, Back-Forward-Cache
und Discard hinweg) und führt Befehle in Sprache über `AXActionData` aus
(Tasten als Ersatzweg); die Aufgaben `spike/tasks/01`–`05` laufen dort mit
`--relief-run` vollständig. Browser-Tests je Integrationspunkt
(`relief_browsertests`). Oberfläche ist das Relief-Side-Panel: oben die
Befehlsleiste (Strg+Umschalt+Leertaste; alle Befehle des Spikes, Rückfragen
per Nummer, „ja“, „abbrechen“), darunter der Semantic Inspector
(Strg+Umschalt+I oder `--relief-inspector`): Bereiche, Überschriften und
Bedienelemente live mit Herkunft der Namen, Auswahl und „im Dokument
zeigen“ getrennt.

## Ausführen

Voraussetzungen: Rust ≥ 1.85, ein C++-Compiler (`cxx` in `relief-bridge`),
Google Chrome (Pfad über `CHROME` überschreibbar).

```bash
cargo test --workspace      # browserfrei, auch gegen die Aufnahmen in spike/recordings
cargo run -p relief-cdp -- run spike/tasks/0*.txt spike/tasks/10-real.txt   # Aufgaben mit Erwartungen
cargo run -p relief-cdp -- run spike/tasks/11-korpus.txt --headful          # realer Korpus, sichtbarer Browser
cargo run -p relief-cdp -- test spike/tasks/0*.txt --junit out.xml --report befunde.json   # ohne Fenster, Bericht für CI, Fehler bei verfehlter Erwartung
cargo run -p relief-cdp -- test --fork spike/tasks/0[1-5]*.txt --junit out.xml             # dasselbe im eigenen Build (--headless=new, AX-Weg)
cargo run -p relief-cdp -- repl https://www.gov.uk/     # interaktiv, --headful für sichtbares Fenster
cargo run -p relief-cdp -- measure <url>... --repeat 5  # Zeiten und Graph-Stabilität
cargo run -p relief-cdp -- record spike/tasks/0*.txt   # AX-Aufnahmen nach spike/recordings
cargo run -p relief-cdp -- palette https://www.gov.uk/  # Befehlsleiste im Browser (Studienprototyp)
cargo run -p relief-cdp -- palette-selftest <url> "was ist hier" "gehe zu Suche"   # Leiste per echten Tastenereignissen prüfen
```

Befehle im REPL: „was ist hier", „wo bin ich", „was kann ich tun",
„überschriften", „gehe zu …", „gehe zur Überschrift …", „zum Bereich …",
„nächster/vorheriger Abschnitt", „nächstes/vorheriges Formularfeld",
„lies den Abschnitt [ …]", „welche … gibt es", „öffne …", „fülle … mit …",
„wähle …", „erhöhe/verringere …", „scrolle nach unten/oben/zum Anfang/zum
Ende", „schließe den Dialog" (vollständig: „hilfe"); `url …` lädt eine
andere Seite. Riskante Aktionen fragen zurück; `!` vor demselben Befehl als
nächste Eingabe bestätigt genau diese Rückfrage, einmal. `!` ohne offene
Rückfrage bestätigt nichts.

In Aufgabendateien prüft `assert: <Zusicherung>` den aktuellen Stand eines
Formulars und antwortet mit Befunden (Regel-IDs `form/…`) oder „Keine
Befunde.“: `feldnamen`, `namen-wie-accname`, `fehler-verknüpft [Feld]`,
`fokus-auf-erstem-fehler`, `bestätigungsdialog`, `statusmeldung <Text>`,
`tabfolge <Feld>, …`. `statusmeldung` verlangt zusätzlich, dass die letzte
`do:`-Zeile eine vorhandene Live-Region geändert hat. `07-bestaetigung.txt`
prüft, dass sich die Rückfrage nicht umgehen lässt (beide Hosts).

Im Palettenmodus öffnet Strg+Umschalt+Leertaste die Befehlsleiste; riskante
Aktionen werden mit „ja“ bestätigt, „nein“ verwirft die Rückfrage. Jede
Eingabe landet mit Ergebnisart und
Tastendrücken, ohne Seiteninhalte, in `relief-protokoll.jsonl` (`RELIEF_LOG`).

Resolver fehlender Namen kalibrieren (Stichprobe `spike/kalibrierung/`):

```bash
cargo run -p relief-resolver -- kalibrieren                 # ohne Modell: nur Anfragegrößen
export ANTHROPIC_API_KEY=…                                  # eigener Key, nie ins Repo
export RELIEF_ANTHROPIC_MODEL=claude-sonnet-5               # optional, Standard claude-haiku-4-5-20251001
cargo run -p relief-resolver --features anthropic -- kalibrieren --aufzeichnen antworten.json
cargo run -p relief-resolver -- kalibrieren --wiedergeben antworten.json   # erneut auswerten, ohne Key
```

Der Lauf gibt Trefferquote je Confidence-Band, einen Schwellenvorschlag und
Tokens je Anfrage und Seite aus. Ohne Feature `anthropic` enthält kein Crate
Netzcode für Modelle.

`RELIEF_VERIFY=1` prüft jede übersprungene Aufnahme gegen einen Vollsnapshot;
`CHROME` setzt den Pfad zum Browser, `RUST_LOG=chromiumoxide=debug` zeigt den
CDP-Verkehr.

### CI

`.github/workflows/ci.yml` läuft bei Pull Requests, Push auf `main`,
Release-Tags (`v*`) und per Hand (`workflow_dispatch`) auf `ubuntu-24.04`:

- **rust:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace`.
- **browser:** Chrome for Testing 154.0.8037.57 (nächste zu `fork/UPSTREAM`) über
  `browser-actions/setup-chrome`, dann `relief-cdp run spike/tasks/01`–`06`
  (nur `file://`-Seiten). Der Job schlägt fehl, wenn nicht alle
  `expect:`-Zeilen erfüllt sind (`run` selbst endet immer mit 0).

Actions sind per Commit-SHA gepinnt.

### Cloud-Umgebung (claude.ai)

In der Umgebung auf claude.ai eintragen:

- **Setup script:** Inhalt von `scripts/cloud-setup.sh`. Es prüft die
  Rust-Toolchain (≥ 1.85, rustfmt, clippy, C++-Compiler), holt die Crates
  (`cargo fetch`, falls der Checkout gefunden wird) und installiert Chrome for
  Testing 154.0.8037.57 nach `/opt/chrome-for-testing` samt apt-Bibliotheken.
  `/usr/local/bin/google-chrome-stable` startet ihn mit `--no-sandbox`
  (Chrome als root im Container), `relief-cdp` findet ihn ohne `CHROME`.
  `RELIEF_CHROME=0` lässt Chrome weg.
- **Netzwerk:** Standard „Trusted“ genügt (crates.io, Ubuntu-apt,
  `storage.googleapis.com`). `dl.google.com` und
  `googlechromelabs.github.io` werden nicht gebraucht.
- **Umgebungsvariablen:** keine nötig.

## Chromium bauen (M4)

```bash
./scripts/chromium-setup.sh     # depot_tools, Checkout ohne Historie, Tag, gn gen, Build
```

Aufgaben im eigenen Build über den AX-Weg (Ausgabe wie `relief-cdp run`):

```bash
scripts/fork-apply.sh ~/chromium/src --continue && autoninja -C ~/chromium/src/out/Relief chrome
scripts/fork-run-tasks.sh spike/tasks/0[1-5]*.txt     # RELIEF_LOG=<datei> für das Protokoll
```

Voraussetzungen: Xcode mit macOS-SDK, `git-lfs` (`brew install git-lfs && git lfs
install`), Metal-Toolchain (`xcodebuild -downloadComponent MetalToolchain`),
rund 40 GB Platz; eine Anwendungs-Firewall muss das Python von depot_tools
zulassen. Erstbuild gut zwei Stunden, inkrementell Sekunden. Der eigene Build
lässt sich mit `CHROME=~/chromium/src/out/Relief/Chromium.app/Contents/MacOS/Chromium`
aus `relief-cdp` heraus steuern.

Relief in den Checkout bringen, bauen und starten:

```bash
scripts/fork-apply.sh ~/chromium/src            # Branch relief, Patches, //relief/ + Crate-Quellen
scripts/fork-apply.sh ~/chromium/src --continue # nach Änderungen: nur Quellen neu kopieren
(cd ~/chromium/src && gn gen out/Relief && autoninja -C out/Relief chrome)
~/chromium/src/out/Relief/Chromium.app/Contents/MacOS/Chromium --enable-relief \
  --relief-log=/tmp/relief.log "--relief-activate=In den Warenkorb" \
  "file://$PWD/spike/fixtures/shop-clean.html"
node scripts/fork-measure.mjs --port 9222       # Latenz, Build mit --remote-debugging-port=9222 und --use-mock-keychain starten (--scroll 40: Positionen, --parent <Selektor>: Messknoten z. B. in einen aria-modal-Dialog)
(cd ~/chromium/src && autoninja -C out/Relief relief_browsertests && out/Relief/relief_browsertests)  # Integrationstests
scripts/fork-export.sh ~/chromium/src           # Änderungen im Checkout zurück nach fork/
```

## Geplanter Stack

- Chromium-Fork, eigener Code unter `//relief/`; Ziel macOS, Windows, Linux,
  entwickelt zuerst auf macOS ARM.
- Semantic Runtime in Rust, browserfrei; baut auf den Crates aus
  `casoon/barrierlab` auf (`a11y-perception`, `a11y-dom`, `accname`, `a11y-report`
  in Gebrauch; `a11y-rules` geplant).
- KI optional in Stufen (OS-Modell, lokal, Cloud-API mit eigenem Key),
  Default ohne Modell. Vertrag, Anbieter-Schnittstelle und Privacy-Filter
  stehen (`relief-ai-contract`); ein Adapter für die Anthropic Messages API
  (Feature `anthropic`) und das Kalibrierwerkzeug in `relief-resolver`.
  Schwellen sind noch nicht gemessen, Modellnamen bleiben `Uncertain`; die
  Runtime ruft noch kein Modell auf. Missbrauchsfälle an der Modellgrenze
  sind als feste Tests beschrieben; Modellaufrufe unter Grenzen je Aufgabe
  gehen über `Budget`.

## Wo liegt was

| Pfad | Inhalt |
|---|---|
| `crates/relief-model` | browserfrei: semantisches Datenmodell, Delta-Format, Konverter aus `a11y-perception` |
| `crates/relief-bridge` | browserfrei: Rust-Seite der Grenze zum Fork (`cxx`-Bridge, Mojo-Entwurf), Benchmarks (`cargo bench -p relief-bridge`) |
| `crates/relief-interaction` | browserfrei, auf `relief-model`: Interaction Graph, Befehle, Zielauflösung, Validierung, Bestätigungstoken und Security-Log, Antworttexte, Formular-Zusicherungen (Befunde im Format von `a11y-report`) |
| `crates/relief-ai-contract` | browserfrei: KI-Vertrag (Hypothesen, Intent-Vorschläge, JSON-Schemas), `ModelProvider` mit Stufen, Privacy-Filter, Grenzen je Aufgabe (`Budget`); Sicherheits-Regressionsmatrix in `tests/missbrauch.rs` |
| `crates/relief-resolver` | browserfrei: Resolver fehlender Namen (Ausschnitt, Anthropic-Adapter hinter Feature `anthropic`, Kalibrierung) |
| `crates/relief-cdp` | Spike-Host: steuert Chrome über CDP, führt Aktionen aus |
| `spike/fixtures`, `spike/tasks` | Testseiten und Aufgabendateien (`url:`/`do:`/`assert:`/`expect:`); `06-form-assertions.txt` mit `form-clean.html`/`form-broken.html`/`status-inserted.html`/`form-embedded.html` prüft die Formular-Zusicherungen |
| `spike/recordings` | AXTree-Aufnahmen als Fixtures für browserfreie Tests |
| `spike/kalibrierung` | von Hand beschriftete Stichprobe unbenannter Controls mit Soll-Namen und Begründung |
| `crates/relief-interaction/tests` | Snapshot- und Aufgabentests gegen die Aufnahmen; Erwartungen neu setzen mit `RELIEF_ERWARTUNGEN=neu` |
| `fork/` | Fork-Inhalt für einen Chromium-Checkout: Patch-Serie (`patches/`, `series`, 3 Patches), Basisversion (`UPSTREAM`), `//relief/` (`relief/`: Tab-Helfer, eigener AXTree, Runtime-Sequenz, Aufgaben-Runner, Inspector (`inspector/`), `BUILD.gn`, Browser-Tests in `testing/`); Format in `fork/README.md` |
| `scripts/fork-apply.sh`, `scripts/fork-export.sh` | Fork auf einen Checkout anwenden (inkl. Kopie der Crate-Quellen) bzw. Patches daraus neu erzeugen |
| `scripts/cloud-setup.sh` | Setup-Skript für die Cloud-Umgebung auf claude.ai |
| `.github/workflows/ci.yml` | CI: Rust-Prüfungen und Aufgaben 01–06 gegen Chrome |
| `scripts/fork-measure.mjs` | Messknoten über CDP in den Relief-Build schreiben (Ende-zu-Ende-Latenz) |
| `docs/architecture.md` | Aufbau und Datenfluss |
| `docs/decisions.md` | aktuell gültige Grundsatzentscheidungen |
| `docs/constraints.md` | harte Rahmenbedingungen |
| `site/` | Projektseite (Astro, Theme aus `casoon/gh-pages-template` als Kopie in `site/vendor/`); Workflow `.github/workflows/pages.yml` |
| `pages/` | Inhalte der Projektseite (Englisch, für die Vorstellung des Projekts) |
| `plan/` | Spezifikation (`plan/spezifikation/`) und Arbeitspakete; Übersicht in `plan/status.md` |
| `CLAUDE.md` | Regeln für Agenten-Sessions, auch in der Cloud |
| `.claude/skills/` | Skills `plan-verzeichnis` und `living-docs` für Cloud-Sessions |

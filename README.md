# Relief

English: [README.en.md](README.en.md)

> Eine normale Website ist eine Fläche.
> **RELIEF** macht ihre Struktur erfahrbar.

Relief ist ein Forschungsbrowser auf Basis von Chromium. Er liest den
Accessibility-Tree im Browser-Prozess, baut daraus ein semantisches Modell der
Seite und lässt Menschen die Seite über Absichten bedienen: per Tastatur,
Sprache oder Befehlsleiste, neben VoiceOver oder NVDA, nicht statt ihnen.
Derselbe Kern prüft als Testmodus Bedienabläufe aus Sicht von Tastatur- und
Screenreader-Nutzenden.

Projektseite: <https://casoon.github.io/relief/>

## Stand

Forschungsprojekt in Entwicklung. Phase 0 (Machbarkeit im eigenen
Chromium-Build) ist abgeschlossen; die Oberflächen der Phase 1 laufen im
Fork:

- **Relief-Panel** (Augen-Symbol oder Strg+Umschalt+I): Befehlsleiste
  (Strg+Umschalt+Leertaste), Semantic Inspector mit Befunden aus
  barrierlab, umschaltbar auf die **Semantic View**, in der jede
  Bedienung als geprüfte Aktion auf die Originalseite wirkt.
- **Sprache:** Sprechtaste im Panel bzw. Strg+Umschalt+S, Erkennung auf dem
  Gerät, gesprochene Antworten; „abbrechen“ unterbricht sofort.
- **Sprungmarken** (Strg+Umschalt+M), **Formular-Assistent**,
  **Consent-Dialoge** erkennen und auf Wunsch ablehnen (nie zustimmen).
- **Fähigkeitsprofile:** Ansage, Eingabe und Darstellung nach Fähigkeiten,
  global oder je Website, lokal gespeichert.
- **Testmodus:** Formular-Zusicherungen, Läufe ohne Fenster mit
  JUnit-Bericht, Playwright-Anbindung über die CDP-Domäne `Relief.*`.

Relief ist im eigenen Build standardmäßig an (auch beim Start über Dock
oder Finder), `--disable-relief` schaltet es ab. Einzelheiten:
[`docs/project-state.md`](docs/project-state.md), offene Pakete in
[`plan/status.md`](plan/status.md).

## Aufbau

| Pfad | Inhalt |
|---|---|
| `crates/` | Rust-Kern: Modell, Interaction Graph, KI-Vertrag, Bridge, CDP-Host |
| `fork/` | Chromium-Fork: eigener Code unter `fork/relief/`, Patch-Serie, Basisversion |
| `spike/` | Testseiten, Aufgaben und Aufnahmen |
| `docs/` | Ist-Zustand, Architektur, Entscheidungen, Rahmenbedingungen |
| `plan/` | Spezifikation und Arbeitspakete, Übersicht in [`plan/status.md`](plan/status.md) |
| `site/`, `pages/` | Projektseite |

## Prüfen

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Browser-Aufgaben gegen ein installiertes Chrome und den Chromium-Build
beschreibt [`docs/project-state.md`](docs/project-state.md).

## Grundsätze

- Browseraktionen entstehen nur über validierte Pläne; ein Modell schlägt
  höchstens vor.
- Jede Aussage trägt ihre Herkunft; Unsicheres wird nie als Tatsache
  ausgegeben.
- Screenreader arbeiten neben Relief unverändert.

Gemeinsame Barrierefreiheits-Bausteine kommen aus
[barrierlab](https://github.com/casoon/barrierlab).

## Lizenz

MIT, siehe [`LICENSE`](LICENSE). Chromium-Code behält seine eigenen Lizenzen.

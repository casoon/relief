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
Chromium-Build) ist abgeschlossen, Phase 1 baut die ersten Oberflächen.
Einzelheiten: [`docs/project-state.md`](docs/project-state.md).

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

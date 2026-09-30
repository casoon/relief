# 02 · Rust-Core und Bridge

## Architekturregel [Entscheidung]

Der Rust-Core kennt keine Chromium-Typen. Nicht `fn analyze(node:
ChromiumAXNode)`, sondern ein eigenes, stabiles Modell (→ 03). Nur der
C++-Adapter übersetzt.

Grund: Der Core soll außerdem in WASM, CLI, CI, Serveranalyse und ggf. anderen
Hosts laufen. Ein Chromium-förmiger Core würde genau die Abhängigkeit
wiederholen, die an NVDA stört.

## Wiederverwenden statt neu anlegen

Geplante Crates: `relief-core`, `relief-model`, `relief-rules`,
`relief-interaction`, `relief-ai-contract`, `relief-wasm`,
`relief-chromium-bridge`; Grundsatz: keine Duplikation zwischen Prüf- und
Laufzeit-Code. Mit barrierlab heißt das konkret:

| Geplantes Crate | Existiert schon als | Vorschlag |
|---|---|---|
| `relief-model` | teilweise `a11y-perception` (`AXTree`, `AXNode`, Snapshot) | **daneben gestellt** (`crates/relief-model`, Weg 2, → 03); Konverter von `a11y-perception` |
| `relief-rules` | `a11y-rules` + `a11y-report` | wiederverwenden |
| `relief-wasm` | `a11y-wasm` | wiederverwenden |
| — | `accname` | wiederverwenden (DOM-Fallback) |
| `relief-interaction` | — | neu: Interaction Graph (→ 04) |
| `relief-ai-contract` | — | neu: Intent-/Hypothesen-Typen, JSON-Schema (→ 05, 06) |
| `relief-core` | — | neu: Runtime (Zustand, Update-Anwendung, Validierung, Risiko); Anfang als `Runtime` in `crates/relief-bridge` (Rust-Seite der Grenze, unten) |
| `relief-chromium-bridge` | — | neu, **nicht** browserfrei → gehört in den Fork bzw. als Host-Crate hierher, nie nach barrierlab |

Nach barrierlab-Regel („Zwei Konsumenten, dann Bibliothek") landen neue
browserfreie Crates zuerst in diesem Repo und wandern erst nach barrierlab,
wenn ein zweiter Host sie braucht (z. B. auditmysite den Interaction Graph für
Task-Tests).

### Spannung: CDP-förmiges `AXNode` vs. Chromium-internes `AXNodeData` [Chromium-Seite belegt]

`a11y-perception::AXNode` bildet das CDP-Format ab: String-`node_id`,
Properties als Liste, `backend_dom_node_id` als Identität. Chromium intern
liefert `AXNodeData` (`ui/accessibility/ax_node_data.h:43`, Tag 154.0.8037.58)
mit Integer-IDs, die nur je `AXTreeID` eindeutig sind, Enums statt Strings,
inkrementelle `AXTreeUpdate`s statt Vollbäumen; Positionen kommen über einen
eigenen Kanal (`AccessibilityLocationChangesReceived`). Die `AXTreeID` eines
Frames wechselt mit jedem neuen Dokument (→ 01, Abschnitt 5). Zwei Wege:

1. Adapter übersetzt `AXNodeData` → CDP-förmiges `AXNode`. Wenig neuer Code,
   `linearize`/`diff` sofort nutzbar; verliert Typisierung, Updates werden zu
   Vollsnapshots.
2. Neues typisiertes Modell (`SemanticNode`, → 03) mit inkrementellem
   Update-Pfad; `a11y-perception` bekommt einen Konverter.

**Entschieden: Weg 2** [Entscheidung, Begründung in 03 und
`docs/decisions.md`]. `crates/relief-model` enthält `SemanticGraph`
(ein Baum je `TreeId`), `SemanticNode`, `Fact<T>` und das Delta-Format
`TreeDelta`; `relief_model::perception` konvertiert `a11y_perception::AXTree`.
Der CDP-Host konvertiert jede Aufnahme damit; `relief-interaction` arbeitet
nur auf dem Modell (→ 03, „Interaction auf dem Modell“). Der Fork-Adapter (17)
füllt das Modell direkt aus `AXTreeUpdate`, über die Grenze in
`crates/relief-bridge` (unten).

## Rust im GN-Build [belegt]

Stand Chromium 154.0.8037.58:

- Template `rust_static_library` (`build/rust/rust_static_library.gni`), nicht
  das eingebaute `rust_library`; Parameter u. a. `crate_root` (Standard
  `src/lib.rs`), `edition` (Standard 2024), `allow_unsafe` (Standard aus),
  `cxx_bindings` (Dateien mit `#[cxx::bridge]`, erzeugt die C++-Seite
  automatisch), `cpp_api_from_rust` (Crubit), `crate_name`
  (`rust_static_library.gni:96–143`).
- Rust ist im Chromium-Build standardmäßig an (`build/config/rust.gni:34`),
  `cxx` ebenso (`:121`). Toolchain rustc 1.99.0 aus dem Tree
  (`tools/rust/update_rust.py:41`, `third_party/rust-toolchain/VERSION`).
- Panik bricht ab: `-Cpanic=abort`, in Official-Builds `immediate-abort`
  (`build/config/compiler/BUILD.gn:1032–1043`).
- Regeln für First-Party-Rust (`docs/rust/README.md:42–107`): keine
  Freigabe nötig, nur Owner-Review; `unsafe` erlaubt mit `allow_unsafe`;
  Standard-FFI ist `cxx`, daneben Crubit (nur Rust → C++) und Mojo; andere
  FFI-Werkzeuge (z. B. `cbindgen`) werden nicht unterstützt
  (`docs/rust/ffi.md:34`); Tests über gtest, „we do not support
  `#[cfg(test)]`-style testing“ (`README.md:77–78`), obwohl
  `rust_static_library` `build_native_rust_unit_tests` kennt; instabile
  Sprachfeatures nur mit Freigabe.
- Crate-Namen: `rust_static_library` erzeugt einen verfremdeten, global
  eindeutigen Crate-Namen; Chromium-Code importiert per `chromium::import!`
  (`README.md:88–107`). Drittcrates von crates.io liegen in
  `//third_party/rust` (Template `cargo_crate`, Pflege über
  `chromium_crates_io/Cargo.toml`); Crates außerhalb davon nutzen die
  `build/rust/*.gni`-Templates ohne weiteres Werkzeug (`README.md:125–133`).
- Vorhanden in `//third_party/rust`: `serde`, `serde_json`, `cxx`
  (`third_party/rust/chromium_crates_io/Cargo.toml:25, 46–47`). **Nicht**
  vorhanden: `a11y-perception` (hängt nur von `serde` ab).
- Präzedenz für Rust in einem Utility-Prozess hinter Mojo mit `cxx`:
  `services/data_decoder/xml/BUILD.gn:7–19` (`rust_static_library` mit
  `cxx_bindings`).

### Im Fork gebaut (Paket 17) [belegt]

- `//relief/BUILD.gn`: `rust_static_library("relief_model_rs")` und
  `rust_static_library("relief_bridge_rs")` mit
  `cxx_bindings = [ "crates/relief-bridge/src/cxx_bridge.rs" ]`; der
  C++-Teil bindet `relief/crates/relief-bridge/src/cxx_bridge.rs.h` ein.
  Baut mit rustc aus dem Tree.
- **Quellen:** Der Cargo-Workspace bleibt Quelle der Wahrheit.
  `scripts/fork-apply.sh` kopiert `crates/relief-model/src` und
  `crates/relief-bridge/src` nach `//relief/crates/` (Liste `RUST_CRATES`),
  `scripts/fork-export.sh` lässt `//relief/crates/` aus. Keine Symlinks
  (GN-Pfade müssen im Quellbaum liegen), kein Eintrag in `//third_party/rust`.
- **Crate-Namen [Entscheidung]:** feste Namen per `crate_name =
  "relief_model"` bzw. `"relief_bridge"` (von Chromium „discouraged“, für
  First-Party-Code, der auch mit Cargo baut, der einfachste Weg); dadurch
  bleibt `use relief_model::…` in beiden Builds gleich. `cargo_crate` wäre
  für Drittcrates gedacht.
- **`a11y-perception` [Entscheidung]:** im Fork **weggelassen**, nicht
  eingebunden. `relief-model` hat dafür das Cargo-Feature `perception`
  (Standard an, der GN-Build setzt es nicht); nur der Konverter aus
  CDP-Aufnahmen braucht es, der Browser nicht. Weitere Abhängigkeiten im
  Fork: `//third_party/rust/serde/v1:lib` (mit `derive`); `cxx` kommt über
  `cxx_bindings`.
- `edition = "2021"` wie im Workspace (GN-Standard wäre 2024).
- **`allow_unsafe = true`** ist für `relief_bridge_rs` nötig: der von `cxx`
  erzeugte Code braucht `unsafe`, ohne den Schalter setzt GN `-Funsafe_code`
  (`build/rust/BUILD.gn:94–98`), das `#[allow(unsafe_code)]` am Bridge-Modul nicht aufheben kann. Eigener
  Code bleibt per `#![deny(unsafe_code)]` bzw. `#![forbid(unsafe_code)]` frei
  davon.
- **`cxx`-Version:** 1.0.199 im Tree, 1.0.202 im Cargo-Build; die Bridge baut
  mit beiden.
- **Ausnahmen:** Chromium baut ohne; die Bridge hat kein `Result` an der
  Grenze (Annahme aus 16 bestätigt, die Frage stellt sich nicht). Stattdessen
  gilt: `rust::String(std::string)` bricht bei ungültigem UTF-8 ab →
  C++ nutzt `rust::String::lossy`.
- `relief-interaction` ist im Fork noch nicht dabei; es hängt nur noch an
  `relief-model` (ohne `perception`) und `serde`, eingebunden wird es mit 24.
- Tests der Crates laufen weiter mit `cargo test` in diesem Repo, nicht im
  Chromium-Build.

## Bridge-Optionen [Chromium-Seite belegt]

| Option | Wartbarkeit | Sicherheit / Crash-Isolation | Performance | Aufwand | Anmerkung |
|---|---|---|---|---|---|
| **CXX / In-Process** | gut; `cxx` ist Chromiums Standard-FFI [belegt] | Panik = Absturz des Browser-Prozesses (`panic=abort`) [belegt] | beste (keine Serialisierung) | mittel | Regeln für First-Party-Rust erfüllbar (s. o.) |
| **C ABI** | mäßig (Handarbeit an der Grenze) | wie CXX | sehr gut | mittel–hoch | von Chromium nicht als Werkzeug unterstützt [belegt]; nur wenn CXX nicht passt |
| **Mojo-Service in eigenem Prozess** | gut, Chromium-idiomatisch | **beste**: Utility-Prozess, sandboxbar, Absturz isoliert | Serialisierung pro Update | hoch | Präzedenz `data_decoder` (Rust + `cxx` im Utility-Prozess) [belegt] |
| **Separater Rust-Prozess (eigenes IPC)** | schlecht (zweites IPC-System) | gut | Serialisierung | mittel | nur für Spike/Debug sinnvoll |

Zusätzlicher Grund gegen Arbeit im Callback [belegt]: Der Beobachter läuft
auf dem UI-Thread, und der Renderer wartet auf die Quittung jedes AX-Pakets
(→ 01, Abschnitt 2); schwere Arbeit im Callback bremst die Seite. Das gilt
für beide Varianten: Auch in-process läuft die Runtime auf einer eigenen
Sequenz, nicht im Callback.

## Rust-Seite der Grenze [umgesetzt]

`crates/relief-bridge` (browserfrei, `relief-model` + `cxx`):

- `Runtime`: hält den `SemanticGraph` eines Tabs. **Delta rein**
  (`apply(&TreeDelta)` → neue `GraphVersion`, bei Fehler unverändert),
  **Antworten raus** (`describe(NodeRef)` → Rolle und Name als `Fact`, also
  mit `Certainty`; `focus()`), **ActionPlans raus** (`plan(ActionRequest)`
  prüft Version, Existenz, `ignored`, `disabled`, ob Chromium die Aktion am
  Knoten meldet, und Wert nur bei `SetValue`). Befehle in Sprache
  (`relief-interaction`, arbeitet auf dem Modell) kommen mit 24 dazu; die
  Grenze ändert sich dafür nicht.
- **Variante A** (`src/cxx_bridge.rs`): `#[cxx::bridge]` mit flachen
  geteilten Strukturen (`Delta`, `TreeUpdate`, `Node`, `ApplyResult`,
  `ActionRequest`, `ActionPlan`, `Found`, `Answer`) und `extern "Rust"`-Funktionen
  `new_runtime`, `apply_delta`, `describe_node`, `plan_action`, `find_node`
  (erster Knoten mit genau diesem Namen, der eine Aktion meldet;
  `Runtime::find`, für `--relief-activate`), `node_count`. Regeln: kein
  `Option` in `cxx`-Strukturen (→ `has_*`-Felder, Enum-Variante `Unset`);
  kein `Result` über die Grenze, weil `cxx` daraus C++-Ausnahmen macht
  (Chromium baut ohne Ausnahmen, bestätigt in 17); ungültige
  Enum-Werte aus C++ werden Fehler, keine Panik (`panic=abort`). Rundtest
  Modell → Grenze → Modell und Runtime-über-Grenze = Modell auf allen 27
  verschiedenen Aufnahmepaaren grün (`tests/recordings.rs`); verloren gehen
  nur die CDP-eigenen `ignored_reasons` (Chromium kennt sie nicht).
- **Variante B** (`mojom/relief_runtime.mojom`, Entwurf, nicht gebaut):
  dieselben Strukturen als Mojo-Typen (dort mit nullable Typen), Interface
  `ReliefRuntime { ApplyDelta, Describe, PlanAction }`. Im Utility-Prozess
  kopiert der C++-Empfänger die Mojo-Strukturen in die `cxx`-Strukturen; die
  Rust-Seite ist also in A und B **dieselbe**. B kostet zusätzlich
  Mojo-Serialisierung, IPC und eine Kopie. Der Browser prüft jeden
  `ActionPlan` aus dem Utility-Prozess vor dem Senden erneut.

## Bridge-Messung (2026-09-24)

`cargo bench -p relief-bridge` (criterion, Release) auf den Aufnahmen in
`spike/recordings`, Median aus 5 Läufen (je Lauf der criterion-Median aus 50
Stichproben) [gemessen]. **Vorbehalt:** Apple M4 Pro, 14 Kerne, während
parallel ein Chromium-Build lief (Load Average 320–530); absolute Zeiten
sind dadurch zu hoch und streuen (Spanne zwischen Läufen bis Faktor 2), die
**Verhältnisse** zwischen den Varianten sind aussagekräftiger. Auf einem
ruhigen Rechner wiederholen, bevor eine Zahl als Grenze dient.

Eingaben: `typisch` = Delta mit dem mittleren JSON-Umfang aller Paare
(inros-lackner.de, 571 Knoten, 8 neu/geändert, 2 entfernt, 4,3 KB JSON);
`größte` = ikea.com (3 882 Knoten, 211 neu/geändert, 78 entfernt, 109 KB);
`voll` = der ganze Baum „nachher“ desselben Paars als Delta auf einen
leeren Graphen (573 Knoten / 298 KB bzw. 3 961 Knoten / 2,07 MB JSON).

| Schritt | typisch | größte | voll (573) | voll (3 961) |
|---|---|---|---|---|
| A: Modell → `cxx`-Strukturen (Ersatz fürs Füllen in C++) | 1,6 µs | 33 µs | 85 µs | 0,67 ms |
| A/B: `cxx`-Strukturen → Modell | 2,1 µs | 49 µs | 129 µs | 1,01 ms |
| Runtime: Delta anwenden | 3,0 µs | 94 µs | 129 µs | 1,30 ms |
| B-Obergrenze: JSON serialisieren | 5,9 µs | 140 µs | 383 µs | 2,82 ms |
| B-Obergrenze: JSON deserialisieren | 13,0 µs | 391 µs | 918 µs | 8,35 ms |
| B-Untergrenze: tiefe Kopie der Delta | 1,4 µs | 34 µs | 90 µs | 0,79 ms |
| `relief_interaction::Graph::build` (Neuaufbau, Baum „nachher“) | 70 µs | 680 µs | — | — |

Mojo selbst ist ohne Chromium nicht messbar. Seine Kosten liegen zwischen
zwei Kopien (Serialisieren + Deserialisieren ≈ 2 × Kopie) und JSON hin und
zurück; JSON ist in Chromium verfügbar (`serde_json`), aber für Mojo die
teure Obergrenze.

Folgerungen [gemessen, Verhältnisse]:

- **Rust-Seite je Update, Variante A** (übernehmen + anwenden): typisch
  ~5 µs, größter Schritt ~0,14 ms, Vollstand ikea ~2,3 ms. Budget laut 09:
  < 100 ms p95 für AXTree-Änderung → Graph aktualisiert. Die Rust-Seite
  nutzt davon < 0,01 % (typisch) bis ~2,3 % (Vollstand).
- **Aufschlag Variante B** (ohne IPC): Untergrenze ≈ 2 × Kopie (typisch
  ~3 µs, größter ~70 µs, Vollstand ~1,6 ms), Obergrenze JSON hin und zurück
  (19 µs / 0,53 ms / 11,2 ms). JSON kostet das **~9–11-Fache** der
  `cxx`-Übernahme und das ~6–10-Fache des Anwendens; auch das bleibt unter
  ~11 % des Budgets.
- **Inkrementell statt Neuaufbau** lohnt: Übernehmen + Anwenden der
  größten Delta (0,14 ms) ist ~16× billiger als dasselbe für den Vollstand
  der Seite (2,3 ms); typisch ~50× (5 µs gegen 0,26 ms). Der Interaction Graph wird
  heute immer neu gebaut (70 µs / 0,68 ms) und ist damit teurer als das
  Anwenden der Delta; ein inkrementeller Interaction Graph ist erst nötig,
  wenn Regeln/Heuristiken dazukommen.
- Die Serialisierungskosten entscheiden die Wahl **nicht**. Entscheidend
  sind die nur im Fork messbaren Punkte (unten) und Crash-Isolation.

## Entscheidung [Entscheidung in 17]

1. **Jetzt Variante A** (in-process via `cxx`), so gebaut: Der C++-Adapter
   füllt im AX-Callback nur die `cxx`-Strukturen und übergibt sie per
   `base::SequenceBound<RuntimeHost>` an eine eigene
   ThreadPool-Sequenz, auf der die Runtime läuft. Gemessen (→ 09): UI-Thread
   je Paket p95 < 0,4 ms (Vollbaum ~8 000 Knoten 18–27 ms, davon 3–4 ms
   Füllen der `cxx`-Strukturen, der Rest das eigene `Unserialize`),
   Warteschlange p95 < 0,3 ms, Anwenden in Rust p95 < 0,3 ms (Vollbaum
   5 ms). Die Grenze kostet also nichts, was eine Entscheidung trägt.
2. **Ziel Variante B** [Annahme, unverändert] (Utility-Prozess hinter Mojo), sobald Code mit
   Absturz- oder Missbrauchsrisiko in die Runtime kommt (KI-Adapter,
   Heuristiken auf Seiteninhalt; spätestens Phase 4) — dann gilt die
   ursprüngliche Tendenz: Crash-Isolation und Sandbox. Die gemessenen
   Mehrkosten sprechen nicht dagegen.
3. Der Wechsel A → B ist eine Host-Änderung: Rust-Seite, `cxx`-Strukturen und
   Modell bleiben; dazu kommen Mojo-Traits (Mojo ↔ `cxx`) und die
   Service-Registrierung.

## Nur im Fork messbar

- Gemessen in 17 (Zahlen in 09, „Messung im Fork“): Zeit im UI-Thread je
  AX-Paket (eigenes `Unserialize` + Füllen der `cxx`-Strukturen), Ende zu
  Ende DOM-Änderung → Graph, Relief-Strecke Paketeingang → Graph,
  Speicher grob; Build-Fragen oben („Im Fork gebaut“). In 33 dazu:
  Positionen (UI-Thread je Positionspaket, Vergleich mit dem Stand aus 17),
  Speicher von Browser und Renderer mit und ohne Relief und
  Screenreader-Modus (09, „Nachtrag Paket 33“).
- **Offen → 34:** Variante B: Mojo-Serialisierung der `TreeDelta` in C++,
  IPC-Latenz p50/p95 bis zur Antwort, Start des Utility-Prozesses (kalt;
  eine Instanz je Tab oder geteilt), Speicher. Nicht gebaut, weil A gewählt
  ist; fällig, bevor Code mit Absturz- oder Missbrauchsrisiko in die Runtime
  kommt.
- Kleiner Rust-seitiger Gewinn, nicht umgesetzt: `SemanticGraph::apply`
  nimmt die Delta per Referenz und kopiert jeden Knoten; eine Variante per
  Wert spart auf dem `cxx`-Weg ungefähr eine Kopie (größte Delta ~34 µs).

## Datenfluss hin

```
Renderer: AXUpdatesAndEvents (je Frame)
  → Browser-Prozess, UI-Thread: WebContentsObserver::AccessibilityEventReceived
    (Positionen getrennt: AccessibilityLocationChangesReceived)
  → Adapter: eigener ui::AXTree je AXTreeID (+ AXTreeObserver)
  → Knoten-/Strukturänderung, Positionen → Relief-Update (typisiert, nur Delta)
  → Bridge → Rust Runtime: Modell aktualisieren → Graph inkrementell neu
```

So umgesetzt (Weg B in 01, „Umsetzung im Fork“); die Runtime läuft auf eigener
Sequenz, nicht im Callback.

## Datenfluss zurück

```
Rust: validierte Action { tree, node, kind, value, graph_version }
  → Bridge → Adapter: prüft, ob Knoten noch existiert
  → AXActionData über AXActionHandlerRegistry::GetActionHandler(tree_id)
    bzw. RenderFrameHost::AccessibilityPerformAction → Renderer (ohne Antwort)
  → Ergebnis: nächstes AXTreeUpdate wird im Rust-Diff sichtbar
```

Der Erfolg einer Aktion wird **am nächsten Diff gemessen**, nicht am
Rückgabewert (ein „Click" kommt immer an; ob die Seite reagiert, zeigt erst der
Baum). Dafür deutet `relief_interaction::respond::describe_diff` zwei
Modellstände (über `TreeDelta`, → 03).

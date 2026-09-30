# 10 · Teststrategie

Gemessen wird **Task Completion**, nicht nur WCAG-Verstöße.

Die technische Testsuite und die Nutzerevaluation beantworten verschiedene
Fragen: Automatisierte Tests zeigen, ob das System wie spezifiziert arbeitet;
Tests mit Betroffenen zeigen, ob es eine reale Aufgabe besser löst. Beides ist
für ein Go erforderlich.

## Korpus

| Art | Zweck |
|---|---|
| semantisch saubere Testseiten | Known-Pfad, Regressionsbasis |
| absichtlich fehlerhafte Testseiten | Inferenz, Certainty, Rückfragen |
| reale Webseiten (eingefroren, z. B. als WARC/HAR) | Realismus, reproduzierbar |
| SPAs, Shadow DOM, iframes/OOPIF, SVG, Canvas | Sonderfälle der Baum-Identität und Semantik |
| Formulare, Dialoge, Menüs, Tabellen, Live Regions | Kern-Widgets |

**Aufnahmen [belegt]:** `spike/recordings/` enthält AXTree-Aufnahmen von 20
Seiten (eigene Testseiten, gov.uk, Wikipedia, APG-Beispiele und ein Korpus
aus 9 realen Seiten), je Aufgabenschritt vorher/nachher, aufgenommen mit
`relief-cdp record`. Browserfreie Tests laufen dagegen, auch in
Cloud-Sessions ohne Chrome (siehe unten).

**Wiederverwendung [belegt]:** auditmysite hat 133 Seiten mit 692
Journey-Instanzen vermessen; die URL-Liste dieses Laufs ist nicht erhalten,
der Relief-Korpus ist deshalb neu zusammengestellt. barrierlab plant `a11y-conformance` als geteiltes
Fixture-Korpus — Relief-Fixtures dort einbringen statt parallel pflegen.

## Testfall = Seite + Aufgaben

```yaml
page: fixtures/shop/product-broken-icons.html
tasks:
  - say: "Finde den Preis."
    expect: { answer_contains: "129,00" }
  - say: "Wähle Größe M."
    expect: { node: size-select, value: "M", risk: MEDIUM }
  - say: "Öffne das Warenkorb-Menü."
    expect: { overlay_opened: true, name_certainty: Inferred }
  - say: "Bestellen."
    expect: { confirmation_required: true }
```

Weitere Beispielaufgaben: „Öffne das Hauptmenü.", „Fülle das Kontaktformular
aus.", „Finde den höchsten Wert im Diagramm."

Ein Ablaufbericht dedupliziert denselben Befund über mehrere Seitenzustände,
behält aber die Liste der betroffenen Schritte. Dadurch zählt ein dauerhaft
unbenanntes Feld nach Öffnen und Schließen eines Dialogs nicht mehrfach; ein
neu entstandener oder in seiner Evidenz veränderter Befund bleibt getrennt.

## Ebenen

1. **Rust-Unit/Snapshot-Tests** (browserfrei): Modell → Graph → Intent →
   Validierung, gegen aufgezeichnete Bäume. Laufen in CI, schnell.
2. **CDP-Integration** (Phase 0a): dieselben Aufgaben gegen echtes Chrome.
3. **Fork-Integrationstests** (`//relief/testing/`): je Integrationspunkt ein
   Test — Observer registriert, Update kommt an, `AXActionData` wirkt. Laufen
   nach jedem Upstream-Rebase; ein roter Test = Integrationspunkt gebrochen.
   Umgesetzt [33] als `relief_browsertests` (`InProcessBrowserTest`, neun
   Fälle inkl. Positionen, OOPIF, Navigation mit Back-Forward-Cache, Discard,
   begrenzter Neuaufbau; Liste in 01, „Umsetzung im Fork“):
   `autoninja -C out/Relief relief_browsertests && out/Relief/relief_browsertests`.
   Testseiten liegen in `fork/relief/testing/data/`, nicht in
   `spike/fixtures/`, weil der Testserver aus dem Chromium-Quellbaum liest.
4. **Formative Sessions** mit Betroffenen ab Phase 0c: zunächst Problem und
   Baseline, dann Aufgaben mit dem kleinsten bedienbaren CDP-Prototyp. Ab Phase
   3 folgen breitere manuelle Sessions mit der integrierten Assistenz.
5. **Echte Screenreader-Abläufe** (Linie B): derselbe Aufgabenfall kann über
   Tab-Navigation, sequenzielles Lesen oder Schnellnavigation laufen; Ausgabe
   und Fokus werden gegen Erwartungen geprüft (→ 43).
6. **Reliefs eigene User-Agent-Oberfläche:** Inspector, Befehlsleiste,
   Semantic View, Bestätigungen und Einstellungen werden unabhängig von der
   geöffneten Seite gegen die Baseline aus 13 geprüft. Automatisierte
   Semantik-/Fokustests werden mit manuellen Kernaufgaben je Plattform und
   Assistenztechnik kombiniert (→ 31, 47).

## Browserfreie Tests gegen die Aufnahmen [belegt]

`crates/relief-interaction/tests/`, laufen mit `cargo test --workspace` ohne
Chrome (Cloud-tauglich). Jede Aufnahme geht über `relief_model::perception`
ins Modell (Tree-ID = URL), wie im CDP-Host:

- **`recordings.rs`, Snapshot je Aufnahme:** Für jede Aufnahmedatei jeder
  Seite eine Zusammenfassung des Graphs (Titel, Bereiche, offener modaler
  Dialog, Anzahl Überschriften/Bedienelemente/erreichbare Bedienelemente/
  Texte, H1, Certainty-Verteilung der Namen, Bedienelemente ohne gesicherten
  Namen mit Evidence, Formularfelder mit Wert/Optionen/Zuständen,
  `hasPopup`-Elemente; Listen ab 10 Einträgen gekürzt), verglichen mit
  `tests/erwartungen/<aufgabe>/<seite>.json`. Eine Heuristikänderung zeigt
  ihre Wirkung als Abweichung je Seite; gewollte Änderungen setzt
  `RELIEF_ERWARTUNGEN=neu cargo test -p relief-interaction --test recordings`
  neu, der Git-Diff ist die Durchsicht. Die Dateien halten den geprüften
  Ist-Stand fest, nicht das Soll.
- **`recordings.rs`, Graph-Stabilität:** gleiche Signatur bei zweimal
  geladener Seite (shop-broken, APG-Dialog geschlossen und offen), nach
  Öffnen und Schließen (Größentabelle, APG-Dialog, APG-Menübutton) und bei
  Schritten, die nur Fokus, Wert oder Zustand ändern; andere Signatur, wenn
  Bedienelemente dazukommen, wegfallen oder umbenannt werden. Beobachtet:
  Die Anfangsaufnahme des zweiten APG-Dialog-Laufs entstand vor dem
  Nachladen (57 statt 59 Bedienelemente) — Stabilität gilt erst für den
  geladenen Stand.
- **`aufgaben.rs`:** die Aufgaben aus `spike/tasks/01`–`04`, `10`, `11`
  nachgespielt (`parse` → Zielauflösung mit den Filtern des Hosts → `plan`
  bzw. Antworttext; Wirkung über `respond::describe_diff` auf den
  aufgezeichneten Paaren als Modell). Die Erwartungen sind
  dort festgelegt, nicht aus den Antworten in `index.json` übernommen.
  Festgehaltene Spike-Befunde: Mehrdeutigkeit „Warenkorb“ (Link vs. Button),
  „Suche“ trifft nicht „Besucher“ (casoon.de), Modalität (natives
  `<dialog>`, APG-`aria-modal`: 8 von 67 erreichbar, zeit.de-Consent: 9 von
  599; bahn.de/IKEA-Dialoge nicht modal), `hasPopup` (Menübutton, gov.uk-
  Combobox: Schließen per Escape), Dialog öffnen/schließen im Diff,
  iframe-Consent (sperrt nur den Frame, „Zum Artikel“ erreichbar),
  Schließen-Ziel mit Fokus (IKEA: Suchvorschläge statt Consent, Escape-Ziel
  gemeldet), URL-Erschließung („cart“, „mein konto“ als Inferred mit
  Evidence), `checked` nur über `target_change`, benannte Abschnitte
  zusammengefasst (Wikipedia: 28).
- `tests/common/` spiegelt die Zielfilter des Hosts
  (`crates/relief-cdp/src/main.rs`, `Host::handle`/`pick_by_option`); ändert
  sich dort die Auswahl, muss der Spiegel mit.
- **Seitentyp und primäre Aktion:** Die Zusammenfassung je Aufnahme trägt
  `seitentyp`, `primaere_aktion` und `gruppen` mit Certainty und Evidence;
  `recordings.rs::trefferquote_seitentyp_und_primaere_aktion` vergleicht die
  Anfangsaufnahme jeder Seite mit von Hand festgelegten Soll-Werten (`SOLL`)
  und listet Abweichungen ausdrücklich (`BEKANNT_FALSCH`, derzeit leer).
  Trefferquote mit `-- --nocapture`; Ergebnis in 04.
- Nicht aufgenommen: `05-intents.txt` (Navigation, Vorlesen, Felder,
  Erhöhen) — abgedeckt über die Unit-Tests an `graph::sample_tree`.

Snapshot-Format **[Entscheidung]:** eigene JSON-Erwartungen statt `insta`.
`serde_json` ist schon Dev-Abhängigkeit, der Vergleich sind ~30 Zeilen; ein
Review-Werkzeug (`cargo insta review`) bringt bei 20 Dateien wenig, die
Git-Diff-Durchsicht reicht.

## CI und Cloud-Umgebung

**CI [Entscheidung]:** `.github/workflows/ci.yml`, Auslöser Release-Tags
(`v*`) und `workflow_dispatch`; vor jedem Push prüft `scripts/verify.sh`
lokal dasselbe. Runner `ubuntu-24.04` fest statt `ubuntu-latest`
(`ubuntu-latest` wechselt ab 19.10.2026 auf Ubuntu 26; 24.04 entspricht der
Cloud-Umgebung). Actions per Commit-SHA gepinnt, Tag im Kommentar.

- Job **rust**: fmt, clippy `-D warnings`, `cargo test --workspace`, Cache
  über `Swatinem/rust-cache`.
- Job **browser**: Chrome for Testing in der Fork-Basisversion
  (154.0.8037.58) über `browser-actions/setup-chrome` mit
  `install-dependencies`, dann `relief-cdp run spike/tasks/01`–`05` (68
  Erwartungen, nur `file://`). Weil `run` auch bei verfehlten Erwartungen mit
  0 endet und Erwartungen nicht ladbarer Seiten überspringt, vergleicht der
  Job die Schlusszeile mit der Zahl der `expect:`-Zeilen. Der Job ist
  zugleich der Linux-Nachweis des CDP-Hosts.
- Feste Chrome-Version statt „stable“: reproduzierbar und gleich dem Fork;
  Anheben zusammen mit `fork/UPSTREAM`.

**Cloud-Umgebung [belegt: Doku claude.ai, Stand 2026-09]:** Ubuntu 24.04
x86-64, Setup-Skript läuft als root vor dem Start von Claude Code, Ergebnis
als Dateisystem-Abbild zwischengespeichert, Skript muss mit 0 enden und in
etwa fünf Minuten fertig sein. Die Standard-Freigabeliste „Trusted“ enthält
crates.io, Ubuntu-apt und `storage.googleapis.com`, nicht `dl.google.com`
und nicht `googlechromelabs.github.io`. Daraus `scripts/cloud-setup.sh`:
Chrome for Testing als Zip von `storage.googleapis.com` in fester Version
(die Versionsliste liegt auf `github.io`), Bibliotheken per apt (Liste wie
setup-chrome für Ubuntu 24), Hülle `/usr/local/bin/google-chrome-stable` mit
`--no-sandbox` (Chrome als root verweigert die Sandbox), die chromiumoxide
über den PATH findet. Ubuntus `chromium` ist ein Snap und scheidet aus.

**Offen:** Der Browser-Job und das Setup-Skript sind unter Linux noch nicht
gelaufen (→ Backlog 10).

## Messgrößen

- Task Completion Rate je Seitenklasse.
- Verbesserung gegenüber der persönlichen Baseline der Testperson
  (bestehender Browser + vorhandene Assistenztechnik).
- Kritische Fehlaktionen, Abbrüche und benötigte Hilfestellungen.
- Vertrauen, wahrgenommene Kontrolle und Verständlichkeit von Unsicherheit und
  Bestätigungsdialogen.
- Graph-Stabilität: gleiche Seite → gleicher Graph; Änderungsrate bei
  Mini-Updates.
- Inferenz-Kalibrierung: Anteil korrekter Inferred-Werte je Confidence-Band
  (Grundlage für Schwellen, → 06).
- Falsche HIGH-Freigaben: muss 0 sein.
- Update-Latenz p50/p95.
- Bei Modell-Stufen: Qualität je Stufe (`none`/`os`/`local`/`api`) und
  Kosten pro Anfrage bzw. pro Seitenbesuch bei `api`.
- Plattformen: dieselben Aufgaben auf macOS, Linux, Windows.
- Eigene Oberfläche: Kernaufgaben je Plattform/AT, Fokus-Rückgabe,
  Statusmeldungen ohne störende Wiederholung und offene Ausnahmen mit
  Ablaufdatum.

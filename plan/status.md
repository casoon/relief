# Status

Stand 2026-09-30: Forschungsprojekt, Fokus auf der technischen Lösung.
**Go für Phase 1** (2026-09-30), VoiceOver-Test und Entscheidung in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#entscheidung-go-2026-09-30-entscheidung).
Produktumfang mit zwei Linien (Assistenz, Prüfen; Relief ersetzt den
barrierlab-Reader) in [spezifikation/12](spezifikation/12-produktumfang.md).
CDP-Spike (Phase 0a) erledigt, Ergebnis in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#ergebnis-2026-09-24-belegt).
Chromium-AX-Integrationspunkte gegen 154.0.8037.58 belegt, Ergebnis in
[spezifikation/01](spezifikation/01-chromium-integration.md).
Fork-Strategie festgelegt (Patch-Serie, Takt, `fork/`, `scripts/fork-*.sh`),
Ergebnis in [spezifikation/01](spezifikation/01-chromium-integration.md#fork-strategie).
Datenmodell und Delta-Format umgesetzt (`crates/relief-model`), Ergebnis in
[spezifikation/03](spezifikation/03-semantisches-datenmodell.md).
`relief-interaction` arbeitet auf dem Modell statt auf `a11y_perception`,
beschrieben in
[spezifikation/03](spezifikation/03-semantisches-datenmodell.md#interaction-auf-dem-modell-umgesetzt).
KI-Vertrag, Modell-Anbieter und Privacy-Filter umgesetzt
(`crates/relief-ai-contract`), Ergebnis in
[spezifikation/06](spezifikation/06-ki-und-vision.md#vertrag-umgesetzt) und
[07](spezifikation/07-privacy-und-sicherheit.md#privacy-filter-umgesetzt).
Rust-Seite der Bridge umgesetzt (`crates/relief-bridge`), Kosten und
Empfehlung CXX vs. Mojo in
[spezifikation/02](spezifikation/02-rust-core-und-bridge.md#bridge-messung-2026-09-24).
Snapshot-Tests gegen die Aufnahmen erledigt, beschrieben in
[spezifikation/10](spezifikation/10-teststrategie.md#browserfreie-tests-gegen-die-aufnahmen-belegt).
Chromium 154.0.8037.58 baut auf dem M4 (Erstbuild 2 h 11 min, inkrementell < 20 s),
Ergebnis in [spezifikation/09](spezifikation/09-phasen-und-kriterien.md#chromium-build-auf-dem-m4-2026-09-24-belegt).
Fork-Grundgerüst steht: AXTree → Rust → Graph → `activate` → Diff im eigenen
Build, Go/No-Go-Kriterien gemessen in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#messung-im-fork-2026-09-24-belegt),
Umsetzung in [spezifikation/01](spezifikation/01-chromium-integration.md#umsetzung-im-fork-pakete-17-33-belegt).
Fork-Adapter vervollständigt (Positionen, Discard, OOPIF, Navigation mit
Back-Forward-Cache, begrenzter Neuaufbau) und mit `relief_browsertests`
abgesichert, Messung ohne Regression in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#nachtrag-paket-33-2026-09-24-belegt).
Positionen in iframes in Koordinaten des Hauptdokuments und ohne
Browser-Zoom, beschrieben in
[spezifikation/01](spezifikation/01-chromium-integration.md#umsetzung-im-fork-pakete-17-33-belegt).
Modalität je Frame und Schließen-Ziel umgesetzt, beschrieben in
[spezifikation/04](spezifikation/04-interaction-graph.md#modalität-je-frame-belegt)
und [05](spezifikation/05-intents-und-aktionen.md).
Seitentyp, funktionale Gruppen und primäre Aktion umgesetzt (Trefferquote
13/13 auf den Aufnahmen), beschrieben in
[spezifikation/04](spezifikation/04-interaction-graph.md#seitentyp-gruppen-primäre-aktion-umgesetzt);
Seitentyp in der Risikoeinstufung in [05](spezifikation/05-intents-und-aktionen.md).
Resolver für fehlende Namen gebaut (Ausschnitt, Anthropic-Adapter hinter
Feature, Stichprobe, Kalibrierwerkzeug), beschrieben in
[spezifikation/06](spezifikation/06-ki-und-vision.md#resolver-fehlender-namen-umgesetzt-messung-offen);
der Messlauf wartet auf einen API-Key.
Fork-Adapter auf echten Seiten nachgemessen (Wikipedia, spiegel.de; Consent-iframes
auf spiegel.de und bild.de im Graphen, Positionen gleich dem DOM), Netz des
Builds geklärt (Schlüsselbund, `--use-mock-keychain`), beschrieben in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#nachtrag-paket-35-2026-09-25-belegt)
und [01](spezifikation/01-chromium-integration.md#umsetzung-im-fork-pakete-17-33-belegt).
CDP-Host wartet auf Netzwerk-Ruhe zusätzlich zur DOM-Ruhe (bahn.de stabil),
beschrieben in
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#nachtrag-netzwerk-ruhe-2026-09-25-belegt).
Formular-Zusicherungen im Aufgabenformat (`assert:`) umgesetzt, browserfrei
ausgewertet mit `a11y-dom`, `accname` und `a11y-report`, beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-paket-42).

**Sofort startbar:**
- Cloud: 49 (Zusicherungen nachschärfen), 46 (barrierlab), 48 (Sicherheits-Regressionsmatrix, browserfreier Teil;
  der Fork-Teil braucht 24)
- Cloud + M4: 24 (Aktionen über AXActionData), 20 (Inspector), 36 (Branding),
  44 (Lauf ohne Fenster, JUnit)
- M4: 43 (VoiceOver im Test)
- mit API-Key: 28 (Messlauf, ein Befehl je Modell)

**Reihenfolge:**
- Linie A: 24 und 20 parallel → 25 → dann 26, 29, 38, 39, 40 parallel → 41;
  47 läuft mit 20 und 25 als Abnahme mit.
- Linie B: 43 und 44 parallel → 45; 49 jederzeit.
- Produkt: 36 jederzeit; 37 erst vor einer Weitergabe an Dritte.
- Vor jeder Modellintegration (28 im Fork, 34): 48.

| Nr | Thema | Umgebung | Status | Abhängig von | Datei |
|---|---|---|---|---|---|
| 10 | Cloud-Setup: Nachweis in einer Cloud-Session (CI unter Linux grün) | Cloud | blockiert: Cloud-Sessions HTTP 403 | Zugang | [10](10-cloud-umgebung-und-ci.md) |
| 20 | Semantic Inspector im Fork | Cloud + M4 | offen | 19 ✓ | [20](20-inspector-panel.md) |
| 21 | Befunde aus a11y-rules im Inspector | Cloud + M4 | offen | 20 | [21](21-befunde-im-inspector.md) |
| 24 | Aktionen über AXActionData | Cloud + M4 | offen | 17 ✓ | [24](24-axactiondata-rueckweg.md) |
| 25 | Befehlsleiste nativ im Fork | Cloud + M4 | offen | 20, 24 | [25](25-befehlsleiste-im-fork.md) |
| 26 | Sprachschicht | Cloud + M4 | offen | 25 | [26](26-sprache.md) |
| 28 | Resolver: Messlauf und Schwellen | lokal / Cloud (API-Key) | wartet auf API-Key | 27 ✓, 12 ✓ | [28](28-resolver-fehlende-namen.md) |
| 29 | Semantic View | Cloud + M4 | offen | 25 | [29](29-semantic-view.md) |
| 30 | CDP-Host: barrierlab-Release übernehmen (Nachladen erledigt) | Cloud | wartet auf barrierlab-Release | barrierlab | [30](30-cdp-host-pflege.md) |
| 31 | Plattformen Linux und Windows (Build-Hosts) | offen | Entscheidung nötig | 14 ✓, 17 ✓ | [31](31-plattformen.md) |
| 34 | Bridge-Variante B (Utility-Prozess) bauen und messen | Cloud + M4 | später | 19 ✓, vor KI-Code in der Runtime | [34](34-bridge-utility-prozess.md) |
| 36 | Name und Branding „Relief“ | Cloud + M4 | offen | 19 ✓ | [36](36-branding.md) |
| 37 | Updates und Auslieferung (Signatur, Notarisierung) | M4 + Entscheidung | Entscheidung nötig | 36 | [37](37-updates-und-auslieferung.md) |
| 38 | Tastatur-Sprungmarken aus dem Seitenmodell | Cloud + M4 | offen | 24, 25 | [38](38-tastatur-sprungmarken.md) |
| 39 | Formular-Assistent | Cloud + M4 | offen | 24, 25 (26) | [39](39-formular-assistent.md) |
| 40 | Overlay- und Consent-Dialoge | Cloud + M4 | offen | 24 | [40](40-overlay-und-consent.md) |
| 41 | Fähigkeitsprofile | Cloud + M4 | offen | 25, 26, 29 | [41](41-faehigkeitsprofile.md) |
| 43 | Echte Screenreader-Ausgabe im Test (zuerst VoiceOver) | M4 | offen | 42 ✓ | [43](43-voiceover-im-test.md) |
| 44 | Lauf ohne Fenster, JUnit-Bericht | Cloud + M4 | offen | 42 ✓ | [44](44-kopflos-und-junit.md) |
| 45 | Playwright-Anbindung | Cloud + M4 | offen | 24, 44 | [45](45-playwright-anbindung.md) |
| 46 | barrierlab: Relief als Konsument, Reader ersetzen | Cloud (barrierlab) | offen | — | [46](46-barrierlab-einbinden.md) |
| 47 | Accessibility-Baseline für Reliefs eigene Oberfläche | Cloud + M4, später Windows | offen | 20, 25 | [47](47-relief-ui-accessibility.md) |
| 48 | Sicherheits-Regressionsmatrix für Modellgrenzen | Cloud | offen | 24, vor Modellintegration | [48](48-sicherheits-regressionen.md) |
| 49 | Formular-Zusicherungen nachschärfen (Statusmeldung als Änderung, Prüfbefehle) | Cloud | offen | 42 ✓ | [49](49-zusicherungen-nachschaerfen.md) |
| 90 | Nutzennachweis | — | zurückgestellt | Kontakte | [90](90-nutzennachweis.md) |

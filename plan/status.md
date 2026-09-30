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
Aktionen im Fork über `AXActionData` (Tasten als Ersatzweg, eigene Position
statt Fokus auf Überschriften), Aufgaben 01–05 im Fork 68/68 über
`--relief-run`, Befehlsablauf für beide Hosts in `relief_interaction::session`,
beschrieben in
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#im-fork-über-axactiondata-paket-24-belegt).
Semantic Inspector im Fork (Side Panel, WebUI, live, Auswahl und
Aktivierung getrennt), beschrieben in
[spezifikation/01](spezifikation/01-chromium-integration.md#semantic-inspector-paket-20-belegt);
der VoiceOver-Durchgang durch das Panel steht mit 47 aus.
Bericht für CI (`relief-cdp test`: JUnit, a11y-report mit zusammengefassten
Befunden, `--fork` ohne Fenster über den AX-Weg), beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#bericht-für-ci-umgesetzt-2026-09-30-paket-44).
Tastatur-Sprungmarken im Fork (Overlay, Auswahl über die Sitzung, auch
Klick-`<div>`s; spiegel.de- und bild.de-Consent), beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#tastatur-sprungmarken-umgesetzt-2026-09-30-paket-38).
Befehlsleiste im Fork als Teil des Relief-Panels, Rückfragen (Nummer, „ja“,
„abbrechen“) für alle Hosts, beschrieben in
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#rückfragen-und-befehlsleiste-im-fork-paket-25-belegt).
Formular-Zusicherungen im Aufgabenformat (`assert:`) umgesetzt, browserfrei
ausgewertet mit `a11y-dom`, `accname` und `a11y-report` (Feature
`assertions`, nicht im Fork); `statusmeldung` prüft die Änderung der
Live-Region über `TreeDelta`, `namen-wie-accname` rechnet mit Rendering
(`display`, `visibility`) und über iframes im selben Prozess und Shadow DOM,
`06-form-assertions.txt` läuft in Prüfbefehlen und CI, beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49-55-64).
Feld in einem iframe fremder Herkunft verglichen (lokaler Server,
`url: server:…`; ohne Site Isolation im Host), gleiche IDs in Dokument und
Shadow-Root ergeben `untested` statt falscher Befunde, ebd.
Relief ersetzt in barrierlab den Reader-Host als Konsument
(casoon/barrierlab#29, zum Merge offen); Kandidaten zum Ablegen in
[spezifikation/12](spezifikation/12-produktumfang.md#kandidaten-zum-ablegen-stand-2026-09-30).

Overlay- und Consent-Dialoge erkannt, angesagt (Art und Buttons als
Vermutung mit Evidence) und auf Befehl abgelehnt, nie zugestimmt; kein
Ablehnen ohne Bezahlung wird angesagt, nicht umgangen; auf spiegel.de,
bild.de, welt.de, faz.net, t-online.de, heise.de erkannt, Ablehnen auf
google.de, zdf.de, ikea.com ausgeführt; Einwilligungsseite ohne Dialog
(golem.de) als Cookie-Hinweis, Korpus ohne Fehltreffer, Abo ohne Signalwort
(sueddeutsche.de „Jetzt testen“), Einstellungen nur auf „cookie-einstellungen
öffnen“, beschrieben in
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-pakete-40-80-belegt).

Sicherheits-Regressionsmatrix umgesetzt (Missbrauchsfälle als
Tests, Bestätigungstoken, Grenzen je Aufgabe, Security-Log), beschrieben in
[spezifikation/07](spezifikation/07-privacy-und-sicherheit.md#sicherheits-regressionsmatrix-entscheidung-umgesetzt-im-fork-belegt);
der Bestätigungs-Bypass ist auch im Fork getestet. Hosts schreiben das
Security-Log, ein Anbieter lässt sich nur über `Budget` aufrufen (`Permit`),
die Rückfrage nennt und bindet im CDP-Host das Formularziel und verdeckt
sensible Werte (Paket 58, im Fork belegt).

**Sofort startbar:**
- Cloud: 70 (CDP-Host mit Site Isolation), 76 (Werte außerhalb der Rückfrage)
- Cloud + M4: 26 (Sprache), 29 (Semantic View), 39 (Formular-Assistent), 21 (Befunde im Inspector), 36 (Branding), 45 (Playwright), 91 (Consent: zweite Ebene)
- M4: 43 (VoiceOver im Test)
- mit API-Key: 28 (Messlauf, ein Befehl je Modell)

**Reihenfolge:**
- Linie A: 26, 29, 39 parallel → 41 (40 und 80 erledigt, Nachtrag 91);
  47 läuft mit 20 und 25 als Abnahme mit.
- Linie B: 45 (44 erledigt), 43 zurückgestellt; 70 jederzeit.
- Produkt: 36 jederzeit; 37 erst vor einer Weitergabe an Dritte.
- Vor jeder Modellintegration (28 im Fork, 34): 48 ✓, 58 ✓; 75 (Weg b: Renderer-Anfrage).

| Nr | Thema | Umgebung | Status | Abhängig von | Datei |
|---|---|---|---|---|---|
| 10 | Cloud-Setup: Nachweis in einer Cloud-Session (CI unter Linux grün) | Cloud | blockiert: Cloud-Sessions HTTP 403 | Zugang | [10](10-cloud-umgebung-und-ci.md) |
| 21 | Befunde aus a11y-rules im Inspector | Cloud + M4 | offen | 20 ✓ | [21](21-befunde-im-inspector.md) |
| 26 | Sprachschicht | Cloud + M4 | offen | 25 ✓ | [26](26-sprache.md) |
| 28 | Resolver: Messlauf und Schwellen | lokal / Cloud (API-Key) | wartet auf API-Key | 27 ✓, 12 ✓ | [28](28-resolver-fehlende-namen.md) |
| 29 | Semantic View | Cloud + M4 | offen | 25 ✓ | [29](29-semantic-view.md) |
| 30 | CDP-Host: barrierlab-Release übernehmen (Nachladen erledigt) | Cloud | wartet auf barrierlab-Release | barrierlab | [30](30-cdp-host-pflege.md) |
| 31 | Plattformen Linux und Windows (Build-Hosts) | offen | zurückgestellt (Nutzer, 2026-09-30) | 14 ✓, 17 ✓ | [31](31-plattformen.md) |
| 34 | Bridge-Variante B (Utility-Prozess) bauen und messen | Cloud + M4 | später | 19 ✓, vor KI-Code in der Runtime | [34](34-bridge-utility-prozess.md) |
| 36 | Name und Branding „Relief“ | Cloud + M4 | offen | 19 ✓ | [36](36-branding.md) |
| 37 | Updates und Auslieferung (Signatur, Notarisierung) | M4 + Entscheidung | zurückgestellt (Nutzer, 2026-09-30) | 36 | [37](37-updates-und-auslieferung.md) |
| 39 | Formular-Assistent | Cloud + M4 | offen | 24 ✓, 25 ✓ (26) | [39](39-formular-assistent.md) |
| 41 | Fähigkeitsprofile | Cloud + M4 | offen | 25 ✓, 26, 29 | [41](41-faehigkeitsprofile.md) |
| 43 | Echte Screenreader-Ausgabe im Test (zuerst VoiceOver) | M4 | offen | 42 ✓ | [43](43-voiceover-im-test.md) |
| 45 | Playwright-Anbindung | Cloud + M4 | offen | 24 ✓, 44 ✓ | [45](45-playwright-anbindung.md) |
| 47 | Accessibility-Baseline für Reliefs eigene Oberfläche | Cloud + M4, später Windows | offen | 20 ✓, 25 ✓ | [47](47-relief-ui-accessibility.md) |
| 70 | CDP-Host mit Site Isolation: Frames in anderem Prozess (Aufnahme, DOM-Fakten, Aktionen) | Cloud | offen | 64 ✓ | [70](70-cdp-host-site-isolation.md) |
| 75 | Formularziel und HTML-`autocomplete` im Fork | Cloud + M4 | offen, Weg (b) entschieden | 58 ✓ | [75](75-formularziel-im-fork.md) |
| 76 | Sensible Werte außerhalb der Rückfrage (Antwort, Protokolle) | Cloud + M4 | offen | 58 ✓ | [76](76-werte-ausserhalb-der-rueckfrage.md) |
| 90 | Nutzennachweis | — | zurückgestellt | Kontakte | [90](90-nutzennachweis.md) |
| 91 | Consent: zweite Ebene auf echten Seiten, Nachweis im Fork | Cloud + M4 | offen | 80 ✓ | [91](91-consent-zweite-ebene.md) |

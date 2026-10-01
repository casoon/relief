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
Formular-Assistent (was fehlt noch, Fehler vorlesen, zum ersten Fehler,
Zusammenfassung und Bindung vor dem Absenden) für beide Hosts, beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#formular-assistent-umgesetzt-2026-09-30-paket-39).
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
(`display`, `visibility`) und über iframes und Shadow DOM,
`06-form-assertions.txt` läuft in Prüfbefehlen und CI, beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49-55-64).
Feld in einem iframe fremder Herkunft verglichen (lokaler Server,
`url: server:…`), gleiche IDs in Dokument und Shadow-Root ergeben
`untested` statt falscher Befunde, ebd. Der CDP-Host läuft mit Site
Isolation: Frames in einem anderen Prozess kommen über eine eigene Sitzung
in Aufnahme, DOM-Fakten, Aktionen, Fokus und Tab-Folge (Paket 70), ebd.;
ihre Mutationen und Anfragen gehen ins Änderungssignal, iframes in ihrem
Prozess werden eingehängt, `measure` zählt je Weg (Paket 85), ebd.; sie
werden beim Entstehen angehängt, die Ruhe nach dem Laden endet wie ohne
fremden Frame (Paket 105), ebd.; den Fokus in einem solchen Frame findet
der Host auch, wenn die Seite ihn abgegeben hat (Paket 106), ebd.
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
öffnen“; zweite Ebene mit Buttons je Zweck zusammengefasst angesagt und
nie selbst gewählt, „Auswahl speichern“ als Speichern, gleichnamiger Link
neben dem Button zählt nicht (spiegel.de, heise.de); aufklappbare
Zweck-Titel weder Zustimmen noch Einstellungen, Rückfrage je Zweck nennt
den Zweck (bild.de, faz.net), beschrieben in
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-pakete-40-80-91-110-belegt); im Fork belegt.

Sicherheits-Regressionsmatrix umgesetzt (Missbrauchsfälle als
Tests, Bestätigungstoken, Grenzen je Aufgabe, Security-Log), beschrieben in
[spezifikation/07](spezifikation/07-privacy-und-sicherheit.md#sicherheits-regressionsmatrix-entscheidung-umgesetzt-im-fork-belegt);
der Bestätigungs-Bypass ist auch im Fork getestet. Hosts schreiben das
Security-Log, ein Anbieter lässt sich nur über `Budget` aufrufen (`Permit`),
die Rückfrage nennt und bindet im CDP-Host das Formularziel und verdeckt
sensible Werte (Paket 58, im Fork belegt). Auch die Antwort nach einer
Aktion verdeckt sie, Protokolle den Wert jedes Ausfüll- und Auswahlbefehls
(Paket 76, im Fork belegt), beschrieben in
[spezifikation/07](spezifikation/07-privacy-und-sicherheit.md#sensible-werte-außerhalb-der-rückfrage-umgesetzt-paket-76-im-fork-belegt).
Listen, „wo bin ich“ und Inspector zeigen bei sensiblen Feldern nur
„= (verdeckt)“, „details zu …“ nennt den Wert; unverstandene Eingaben
stehen ohne wertartige Teile im Protokoll (Paket 100, Fork-Teil offen:
120), beschrieben in
[spezifikation/07](spezifikation/07-privacy-und-sicherheit.md#auskünfte-und-unverstandene-eingaben-umgesetzt-paket-100-im-fork-belegt).

Befunde aus `a11y-rules` im Inspector (Stufe `Semantics` auf dem AXTree,
übrige Regeln als nicht geprüft), beschrieben in
[spezifikation/01](spezifikation/01-chromium-integration.md#befunde-im-inspector-paket-21-belegt).

Der Build heißt „Relief“ (`Relief.app`, eigenes Profilverzeichnis,
Platzhaltersymbol, übersetzte Texte der Über-Seite und Menüs),
beschrieben in
[spezifikation/01](spezifikation/01-chromium-integration.md#name-und-branding-paket-36-belegt).

Im Fork nennt und bindet die Rückfrage das Formularziel und verdeckt
Felder mit `autocomplete` für Zahlungs-/Identitätsdaten (Renderer-Anfrage
bei einer Rückfrage und vor dem Ausfüllen, Pakete 75, 112), beschrieben in
[spezifikation/07](spezifikation/07-privacy-und-sicherheit.md#bestätigungstoken-umgesetzt).

Playwright steuert Relief und fragt über die CDP-Domäne `Relief.*`
Seitenmodell und Formular-Zusicherungen ab (`examples/playwright/`),
beschrieben in
[spezifikation/12](spezifikation/12-produktumfang.md#playwright-anbindung-umgesetzt-2026-09-30-paket-45).

Semantic View im Relief-Panel: die Seite als bedienbare Ansicht aus dem
Graph, jede Bedienung als validierte Aktion auf die Originalseite,
beschrieben in
[spezifikation/08](spezifikation/08-assistenz-und-capabilities.md#umsetzung-im-fork-paket-29-belegt).

**Sofort startbar:**
- Cloud + M4: 26 (Sprache)
- M4: 43 (VoiceOver im Test)
- mit API-Key: 28 (Messlauf, ein Befehl je Modell)

**Reihenfolge:**
- Linie A: 26 → 41 (29 erledigt, Nachtrag 113) (40, 80, 91, 110 erledigt);
  47 läuft mit 20 und 25 als Abnahme mit.
- Linie B: 45 und 44 erledigt, 43 zurückgestellt; 130 wartet auf ein erneutes Auftreten (85, 105, 106 erledigt).
- Produkt: 37 erst vor einer Weitergabe an Dritte.
- Vor jeder Modellintegration (28 im Fork, 34): 48 ✓, 58 ✓, 75 ✓.

| Nr | Thema | Umgebung | Status | Abhängig von | Datei |
|---|---|---|---|---|---|
| 10 | Cloud-Setup: Nachweis in einer Cloud-Session (CI unter Linux grün) | Cloud | blockiert: Cloud-Sessions HTTP 403 | Zugang | [10](10-cloud-umgebung-und-ci.md) |
| 26 | Sprachschicht | Cloud + M4 | offen | 25 ✓ | [26](26-sprache.md) |
| 28 | Resolver: Messlauf und Schwellen | lokal / Cloud (API-Key) | wartet auf API-Key | 27 ✓, 12 ✓ | [28](28-resolver-fehlende-namen.md) |
| 113 | Vereinfachte Ansicht und Semantic View über die ganze Tab-Breite | Cloud + M4 | zurückgestellt (Nutzer, 2026-10-01) | 29 ✓ | [113](113-vereinfachte-ansicht.md) |
| 30 | CDP-Host: barrierlab-Release übernehmen (Nachladen erledigt) | Cloud | wartet auf barrierlab-Release | barrierlab | [30](30-cdp-host-pflege.md) |
| 31 | Plattformen Linux und Windows (Build-Hosts) | offen | zurückgestellt (Nutzer, 2026-09-30) | 14 ✓, 17 ✓ | [31](31-plattformen.md) |
| 34 | Bridge-Variante B (Utility-Prozess) bauen und messen | Cloud + M4 | später | 19 ✓, vor KI-Code in der Runtime | [34](34-bridge-utility-prozess.md) |
| 37 | Updates und Auslieferung (Signatur, Notarisierung) | M4 + Entscheidung | zurückgestellt (Nutzer, 2026-09-30) | 36 | [37](37-updates-und-auslieferung.md) |
| 41 | Fähigkeitsprofile | Cloud + M4 | offen | 25 ✓, 26, 29 ✓ | [41](41-faehigkeitsprofile.md) |
| 43 | Echte Screenreader-Ausgabe im Test (zuerst VoiceOver) | M4 | offen | 42 ✓ | [43](43-voiceover-im-test.md) |
| 47 | Accessibility-Baseline für Reliefs eigene Oberfläche | Cloud + M4, später Windows | offen | 20 ✓, 25 ✓ | [47](47-relief-ui-accessibility.md) |
| 92 | Formular-Assistent auf echten Formularen | M4 (Netz) | wartet auf Zustimmung (Absenden auf fremden Seiten) | 39 ✓ | [92](92-formular-assistent-echte-formulare.md) |
| 90 | Nutzennachweis | — | zurückgestellt | Kontakte | [90](90-nutzennachweis.md) |
| 130 | CDP-Host: `tabfolge` in einen fremden Frame, Ursache des seltenen Ausfalls | Cloud | wartet auf erneutes Auftreten | 106 ✓ | [130](130-tabfolge-ausfall-ursache.md) |

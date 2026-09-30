# 12 · Produktumfang

Was Relief über Phase 1–5 hinaus braucht, um als Browser konkurrenzfähig zu
sein, und wie derselbe Kern für Barrierefreiheits-Tests taugt. Stand
2026-09-30, nach dem Go für Phase 1 (→ 09).

Relief bleibt dabei **ein Produkt mit einem semantischen Kern**: Linie A ist
die Assistenzoberfläche für Nutzende, Linie B ein Prüfmodus (`relief test`)
für denselben Kern. Linie B ist keine zweite Browser-Roadmap und kein eigener
Reader-Host.

## Abgrenzung [Entscheidung]

| Werkzeugart | Was sie tut | Was Relief anders macht |
|---|---|---|
| Screenreader (VoiceOver, NVDA, JAWS) | liest vor, was die Seite anbietet, Element für Element | versteht die Seite (Typ, Gruppen, primäre Aktion) und bedient über Absichten; ergänzt den Screenreader, ersetzt ihn nicht |
| Sprachsteuerung des Betriebssystems | bedient über Bildschirmbeschriftungen, Nummern oder Raster | löst Absichten im Seitenmodell auf, auch in iframes, ohne simulierte Zeigerbedienung |
| Tastatur-Erweiterungen | Sprungmarken aus dem DOM | Sprungmarken aus dem AX-Baum, mit Herkunftsangabe |
| KI-gesteuerte Browser und Agenten | handeln über ein Modell, meist in der Cloud | handelt nur über validierte `ActionPlan`s; ein Modell schlägt höchstens vor; ohne Modell voll benutzbar; jede Aussage trägt ihre `Certainty` |
| Screenreader-Testautomatisierung | steuert echte Screenreader im Test | → Linie B, verbunden mit dem semantischen Kern |
| Statische Prüfer | prüfen den Seitenzustand | prüft den Bedienablauf im echten Browser |

Der Kern von Relief ist die Verbindung: Seitenverständnis aus dem AX-Baum im
Browser-Prozess, validierte Aktionen, Herkunftsangaben und ein unverändert
arbeitender Screenreader.

## Grundausstattung eines Browsers [Annahme]

Chromium bringt Tabs, Lesezeichen, Verlauf, Downloads, PDF, Erweiterungen und
DevTools mit. Ein Fork muss selbst lösen:

- **Name und Branding:** „Relief“ als App-Name, Symbol, Bundle-ID, eigenes
  Profilverzeichnis (→ 36).
- **Updates und Auslieferung:** Signatur, Apple-Notarisierung,
  Selbstaktualisierung, Takt der Sicherheitsupdates (→ 37). Größter
  Dauerposten neben dem Rebase (→ 01, „Fork-Strategie“).
- **Sync:** Googles Synchronisierung steht einem Fork nicht zur Verfügung;
  anfangs genügt Import aus Chrome und Safari (nicht eingeplant).
- **Passwörter:** Chromium-Passwortmanager vorhanden; Anbindung an den
  macOS-Schlüsselbund offen (nicht eingeplant).

## Linie A: Assistenz im Browser

Pakete 20–29 decken Inspector, Aktionen, Befehlsleiste, Sprache, KI und
Semantic View ab (→ 09). Dazu kommen:

| Funktion | Nutzen | Paket |
|---|---|---|
| Tastatur-Sprungmarken aus dem Seitenmodell | jedes im SemanticGraph exponierte, ausführbare Element mit wenigen Tasten erreichbar, auch in iframes | 38 |
| Formular-Assistent | Pflichtfelder, Fehler und Fokus nach dem Absenden ansagen; Felder per Sprache füllen | 39 |
| Overlay- und Consent-Dialoge | erkennen, ansagen, auf Wunsch ablehnen; nie selbst zustimmen | 40 |
| Fähigkeitsprofile | kombinierbare Fähigkeiten steuern Ausgabe, Eingabe und Darstellung; keine Diagnosemodi (→ 08) | 41 |
| Änderungen ansagen | „Warenkorb jetzt 1“ aus dem Delta | in 25/26 |
| Fehlende Namen ergänzen | Resolver, auf dem Gerät über das Modell des Betriebssystems möglich (Stufe `os`, → 06) | 28 |

## Linie B: Prüfen im echten Browser

Die Aufgaben-Dateien des CDP-Hosts sind bereits Tests in Nutzersprache:
Elemente über Rolle und Namen, Erwartungen an Fokus, Zustand und Ansage
(`spike/tasks/03-form.txt`). Relief prüft damit seinen eigenen Graphen aus
demselben Chromium-AX-Datenstrom, aus dem Chromium auch die Plattform-APIs
bedient, und den Bedienablauf, nicht nur den Seitenzustand. Dass ein
Screenreader tatsächlich dieselbe Information erhält und ausspricht, ist
eine eigene plattformspezifische Prüfung (→ 43), keine Folgerung allein aus
dem Relief-Graphen.

| Baustein | Paket |
|---|---|
| Formular-Zusicherungen: Beschriftung je Feld, Fehlermeldung mit dem Feld verknüpft, Fokus auf dem ersten Fehler, Bestätigung als Live-Region, Tab-Erreichbarkeit und -Reihenfolge | 42 ✓ (unten), 49 |
| Echte Screenreader-Ausgabe über gemeinsamen Treiber; zuerst VoiceOver, später NVDA | 43 |
| Lauf ohne Fenster, Bericht als JUnit für CI | 44 |
| Playwright-Anbindung: Relief über CDP steuern, Seitenmodell über eine eigene Domäne abfragen | 45 |

Grenzen [Entscheidung, → 09 „Nicht in Version 1“]: kein
WCAG-Konformitätsversprechen; Befunde sind Befunde, keine Zertifizierung.
Für reine Funktionstests bleibt Playwright das bessere Werkzeug; Relief
lohnt sich für den Ablauf aus Sicht von Screenreader- und
Tastaturnutzenden.

### Formular-Zusicherungen [umgesetzt 2026-09-30, Paket 42]

Aufgabendateien kennen die Zeile `assert: <Zusicherung>`. Sie prüft den
**aktuellen** Stand der Seite; der Ablauf davor (Absenden, Dialog öffnen)
steht als `do:`-Zeilen davor. Die Antwort sind Befunde im Format von
`a11y-report` (`Finding`, Regel-ID, `Outcome`, Schweregrad, WCAG), die
`expect:` wie jede Antwort prüft; ohne Befund lautet sie „Keine Befunde.“.
Beispiel mit beiden Seiten: `spike/tasks/06-form-assertions.txt`,
Testseiten `spike/fixtures/form-clean.html` (keine Befunde) und
`form-broken.html` (je Zusicherung mindestens ein Befund).

| Zusicherung | Regel | prüft | Daten |
|---|---|---|---|
| `feldnamen` | `form/field-name` (fail) | jedes wahrnehmbare Feld (textbox, searchbox, combobox, listbox, spinbutton, slider, checkbox, radio, switch) hat einen nichtleeren Namen | Modell |
| `namen-wie-accname` | `form/name-accname` (review) | Chromiums Name = `accname::name` auf den DOM-Fakten; jede Abweichung ist ein Befund mit beiden Werten und vermuteter Ursache | Modell + DOM-Fakten |
| `fehler-verknüpft [Feld]` | `form/error-linked` (fail) | das Feld (ohne Angabe: jedes ungültige) ist `invalid` und hat über `aria-errormessage`/`aria-describedby` eine wahrnehmbare, nichtleere Meldung | Modell |
| `fokus-auf-erstem-fehler` | `form/focus-first-error` (fail) | der Fokus liegt auf dem ersten ungültigen Feld in Dokumentreihenfolge | Modell + Fokus |
| `bestätigungsdialog` | `form/confirm-dialog-name` (fail), `-text` (review), `-focus` (fail), `-cancel` (review) | offener Dialog (der mit dem Fokus, sonst der letzte): Name, Text außer Name und Buttons, Fokus darin, Button „Abbrechen“/„Schließen“/„Nein“ … | Modell + Fokus |
| `statusmeldung <Text>` | `form/status-message` (fail) | der Text steht vollständig in **einer** Live-Region (status, alert, log, timer, marquee oder `aria-live` polite/assertive) | Modell |
| `tabfolge <Feld>, …` | `form/tab-order` (fail) | beobachtete Tab-Folge ab Dokumentanfang erreicht die Felder in dieser Reihenfolge; fehlend und vertauscht getrennt gemeldet | beobachtete Folge |

- **Aufteilung [Entscheidung]:** Auswertung browserfrei in
  `relief-interaction` (`assertions.rs`); der CDP-Host (`relief-cdp`,
  `assertions.rs`) erhebt nur. DOM-Fakten sind eine `a11y-dom`-Arena mit
  Tag, Text und den Attributen aus `dom_attribute_needed` (`id`, `for`,
  `role`, `tabindex`, `hidden`, `type`, `title`, `alt`, `placeholder`,
  `value`, `aria-*`) plus Zuordnung DOM-ID → Knoten; der Host füllt sie aus
  `DOM.getDocument` des Hauptdokuments.
- **Outcome statt Certainty [Entscheidung]:** Befunde tragen ihre
  Belastbarkeit über `a11y-report`: `fail` ist aus AX-Daten belegt,
  `review` ist Heuristik (Abbruchweg an Wörtern erkannt, „verständlicher“
  Text nur auf Vorhandensein geprüft, Namensabweichung ohne sichere
  Ursache), `untested` heißt: Daten fehlten.
- **Tab ohne `ActionPlan` [Entscheidung]:** Der Host drückt echte
  Tab-Tasten wie bei Escape und Scrollen; Tab hat kein Zielelement und
  bewegt nur den Fokus. Start ist ein per Skript fokussiertes, danach
  entferntes Element am Anfang von `body`; Ende, wenn der Fokus auf `body`
  fällt, ein Element wiederkommt oder nach 60 Schritten.
- **Planparameter [Annahme]:** „verständliche Planparameter“ eines
  Bestätigungsdialogs heißt hier: der Dialog nennt außer Name und Buttons,
  was bestätigt wird. Ob das verständlich ist, bleibt ein `review`.
- **Grenzen [belegt im Code]:** Die DOM-Fakten tragen kein Rendering; per CSS
  verborgener Inhalt zählt für `accname` mit (in `form-broken.html` gewollt
  als Abweichung). iframes und Shadow DOM fehlen in den DOM-Fakten, Felder
  dort werden `untested`. `statusmeldung` prüft den Endzustand, nicht die
  Änderung der Live-Region (→ 49).

### Relief ersetzt den barrierlab-Reader [Entscheidung 2026-09-30]

In barrierlab war ein eigenes Prüfwerkzeug geplant („Reader-Host“):
Schnappschuss → handeln → Schnappschuss → Differenz, kopf-los, CI-tauglich,
Motor `a11y-perception`. Relief übernimmt diese Aufgabe mit eigenem Browser,
nativem AX-Baum und echter Screenreader-Ausgabe daneben.

Folgen:

- Linie B lebt in Relief, als Prüfmodus desselben Kerns, nicht als zweites
  Produkt mit eigener Roadmap.
- Diff-Regeln und Ereignisspur sind Teil von `a11y-perception` und werden
  genutzt, nicht nachgebaut; der Abgleich mit echten Screenreadern ist 43;
  Regelprüfungen bleiben bei `a11y-rules`.
- In `barrierlab/docs/consumers.md` ersetzt Relief den Reader-Host (→ 46).

## barrierlab einbinden

`casoon/barrierlab` ist der Ort für Barrierefreiheits-Bausteine, die mehrere
Werkzeuge nutzen (`barrierlab/docs/consumers.md`). Regel dort: **zwei
Konsumenten, dann Bibliothek**; Browserfreiheit bis L3; Pakete nehmen Daten
entgegen, holen sie nicht.

Heute nutzt Relief `a11y-perception` im Host und in den Aufnahmen sowie
`a11y-dom`, `accname` und `a11y-report` in `relief-interaction`
(Formular-Zusicherungen; Workspace, `Cargo.toml`). Relief ist in
`consumers.md` noch nicht eingetragen (→ 46).

| Richtung | Was | Wann |
|---|---|---|
| nutzen | `a11y-rules`, `a11y-report` für Befunde im Inspector | 21 |
| nutzen | `a11y-dom` und `accname` für DOM-basierte Namensprüfungen; der AX-Graph allein genügt dafür nicht | 42 ✓ |
| nutzen | Diff-Regeln aus `a11y-perception` statt eigener Diff-Logik | 49, 25 |
| ablegen | Screenreader-Treiber-Interface samt Adaptern und Phrasen-Protokoll, sobald ein zweiter Konsument ihn braucht | 43 |
| ablegen | Formular-Zusicherungen (42 ✓), sobald ein zweites Werkzeug sie braucht | — |
| ablegen | Interaction Graph für aufgabenbasierte Journeys (→ 00, Kandidat) | nach 24 |
| nicht ablegen | Fork-Adapter, Bridge, alles mit Chromium-Typen | — |

Jedes Ablegen läuft als eigener PR in barrierlab mit Eintrag im Changelog des
Pakets.

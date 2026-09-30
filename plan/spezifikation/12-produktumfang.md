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
| Overlay- und Consent-Dialoge | erkennen, ansagen, auf Wunsch ablehnen; nie selbst zustimmen | 40 ✓ (→ 05), 80 |
| Fähigkeitsprofile | kombinierbare Fähigkeiten steuern Ausgabe, Eingabe und Darstellung; keine Diagnosemodi (→ 08) | 41 |
| Änderungen ansagen | „Warenkorb jetzt 1“ aus dem Delta | in 25/26 |
| Fehlende Namen ergänzen | Resolver, auf dem Gerät über das Modell des Betriebssystems möglich (Stufe `os`, → 06) | 28 |

### Tastatur-Sprungmarken [umgesetzt 2026-09-30, Paket 38]

Kern browserfrei (`relief_interaction::marks`): Marken bekommen alle
Bedienelemente des Interaction Graph und Knoten ohne Bedienrolle, für die
Chromium einen Klick meldet (`DoDefault`, etwa ein `<div>` mit
Klick-Handler), jeweils nur mit Position und erreichbar (offener modaler
Dialog: nur dessen Inhalt, je Frame). Ein Klick-Knoten, der Bedienelemente
enthält, ist ein Wrapper und bekommt keine Marke, ebenso Knoten in einem
schon markierten (Text in einem Link) und Container (Dialog, Landmark,
Dokument, iframe) [belegt: auf spiegel.de nahm ein seitenweiter Wrapper
vorher alle Marken; bild.de markierte den Dialog selbst]. Beschriftungen
aus der Grundreihe `asdfghjkl`, alle gleich lang (keine ist Präfix einer
anderen), in Leserichtung. Nicht gesicherte Namen tragen ein „?“ bzw.
„(Name nicht gesichert)“.

Auswahl über die Sitzung: „marke <buchstaben>“ plant wie ein benanntes
Ziel (Felder fokussieren, sonst auslösen), riskant oder unbenannt → Rückfrage,
„ja“ bestätigt genau dieses Ziel; „sprungmarken“ listet sie. Im Fork zeichnet
ein transparentes, nicht aktivierbares Views-Fenster ohne eigene Eingaben
die Marken über den Inhalt (für Assistenztechnik ausgeblendet);
Strg+Umschalt+M zeigt sie, die Buchstaben wählen, Escape oder jede andere
Taste blendet aus; Rückfragen gehen ins Relief-Panel. `--relief-marks` zeigt
sie nach dem Laden.

Belegt (M4): `relief_browsertests --gtest_filter=*Sprungmarken*` (Kürzel,
Buchstaben lösen einen `<div>` mit Klick-Handler nach Rückfrage aus,
Escape); `scripts/fork-run-tasks.sh spike/tasks/14-marken-fork.txt`
(Testshop 12 Marken = alle Bedienelemente, Formular, kaputter Testshop mit
vier Klick-`<div>`s); `spike/tasks/13-marken-real.txt` (Netz): spiegel.de
„Privacy Center“ 12 Marken, bild.de „Cookie- und Einwilligungsbanner“ 35
Marken samt „Alle akzeptieren“, „Jetzt BILD PUR abonnieren“,
„Einstellungen“; Bildschirmfoto Testshop im PR.

Offen: VoiceOver parallel prüfen (→ 47); Klick-`<div>`s ohne Namen könnten
ihren Text als erschlossenen Namen tragen; Marken folgen Scrollen nur
gebündelt über Deltas (200 ms); das Kürzel greift nur am Widget des
Hauptframes (Fokus in einem cross-site-iframe).

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
| Formular-Zusicherungen: Beschriftung je Feld, Fehlermeldung mit dem Feld verknüpft, Fokus auf dem ersten Fehler, Bestätigung als Live-Region, Tab-Erreichbarkeit und -Reihenfolge | 42 ✓, 49 ✓, 55 ✓, 64 ✓ (unten) |
| Echte Screenreader-Ausgabe über gemeinsamen Treiber; zuerst VoiceOver, später NVDA | 43 |
| Lauf ohne Fenster, Bericht als JUnit für CI | 44 |
| Playwright-Anbindung: Relief über CDP steuern, Seitenmodell über eine eigene Domäne abfragen | 45 |

Grenzen [Entscheidung, → 09 „Nicht in Version 1“]: kein
WCAG-Konformitätsversprechen; Befunde sind Befunde, keine Zertifizierung.
Für reine Funktionstests bleibt Playwright das bessere Werkzeug; Relief
lohnt sich für den Ablauf aus Sicht von Screenreader- und
Tastaturnutzenden.

### Formular-Zusicherungen [umgesetzt 2026-09-30, Pakete 42, 49, 55, 64]

Aufgabendateien kennen die Zeile `assert: <Zusicherung>`. Sie prüft den
**aktuellen** Stand der Seite; der Ablauf davor (Absenden, Dialog öffnen)
steht als `do:`-Zeilen davor. Die Antwort sind Befunde im Format von
`a11y-report` (`Finding`, Regel-ID, `Outcome`, Schweregrad, WCAG), die
`expect:` wie jede Antwort prüft; ohne Befund lautet sie „Keine Befunde.“.
Beispiel mit beiden Seiten: `spike/tasks/06-form-assertions.txt`,
Testseiten `spike/fixtures/form-clean.html` (keine Befunde) und
`form-broken.html` (je Zusicherung mindestens ein Befund), dazu
`status-inserted.html` (Statusregion entsteht mitsamt Text) und
`form-embedded.html` (Namensvergleich mit per CSS verborgenem Label-Inhalt,
Feld im iframe und im Shadow DOM, danach CSS-Inhalt dort),
`form-fremd.html` (Feld in einem iframe fremder Herkunft über den lokalen
Server, `url: server:…`) und `form-shadow-ids.html` (gleiche IDs im
Dokument und im Shadow-Root). Die Datei läuft in den Prüfbefehlen
(`CLAUDE.md`) und in der CI (Glob `0[1-6]`, Entscheidung des Nutzers,
Paket 49).

| Zusicherung | Regel | prüft | Daten |
|---|---|---|---|
| `feldnamen` | `form/field-name` (fail) | jedes wahrnehmbare Feld (textbox, searchbox, combobox, listbox, spinbutton, slider, checkbox, radio, switch) hat einen nichtleeren Namen | Modell |
| `namen-wie-accname` | `form/name-accname` (review) | Chromiums Name = `accname::name_rendered` auf den DOM-Fakten; jede Abweichung ist ein Befund mit beiden Werten und vermuteter Ursache | Modell + DOM-Fakten |
| `fehler-verknüpft [Feld]` | `form/error-linked` (fail) | das Feld (ohne Angabe: jedes ungültige) ist `invalid` und hat über `aria-errormessage`/`aria-describedby` eine wahrnehmbare, nichtleere Meldung | Modell |
| `fokus-auf-erstem-fehler` | `form/focus-first-error` (fail) | der Fokus liegt auf dem ersten ungültigen Feld in Dokumentreihenfolge | Modell + Fokus |
| `bestätigungsdialog` | `form/confirm-dialog-name` (fail), `-text` (review), `-focus` (fail), `-cancel` (review) | offener Dialog (der mit dem Fokus, sonst der letzte): Name, Text außer Name und Buttons, Fokus darin, Button „Abbrechen“/„Schließen“/„Nein“ … | Modell + Fokus |
| `statusmeldung <Text>` | `form/status-message` (fail) | der Text steht vollständig in **einer** Live-Region (status, alert, log, timer, marquee oder `aria-live` polite/assertive), und die letzte `do:`-Zeile hat diese Region **geändert**: nicht mitsamt Text in einen schon wahrnehmbaren Elternknoten neu eingefügt, mindestens ein Knoten darin angelegt oder geändert | Modell + Modell vor dem letzten `do:` |
| `tabfolge <Feld>, …` | `form/tab-order` (fail) | beobachtete Tab-Folge ab Dokumentanfang erreicht die Felder in dieser Reihenfolge; fehlend und vertauscht getrennt gemeldet | beobachtete Folge |

- **Aufteilung [Entscheidung]:** Auswertung browserfrei in
  `relief-interaction` (`assertions.rs`); der CDP-Host (`relief-cdp`,
  `assertions.rs`) erhebt nur. DOM-Fakten sind je Dokument eine
  `a11y-dom`-Arena mit Tag, Text und den Attributen aus
  `dom_attribute_needed` (`id`, `for`, `role`, `tabindex`, `hidden`, `type`,
  `title`, `alt`, `placeholder`, `value`, `aria-*`), dazu `display` und
  `visibility` je Element und die Zuordnung DOM-ID → (Dokument, Knoten); der
  Host füllt sie aus `DOM.getDocument` (mit `pierce`) und
  `DOMSnapshot.captureSnapshot`.
- **Rendering über `a11y_dom::Rendering` [Entscheidung, Paket 55]:** Das
  Dokument der DOM-Fakten (`DomDocument`) erfüllt `Rendering` mit
  `computed_style` (nur `display`, `visibility`), ohne Geometrie
  (`bounds` = `None`); `accname::name_rendered` lässt damit per Stil
  versteckte Knoten weg und setzt Inline-Elemente ohne Leerzeichen an.
  Nichts davon ist in Relief nachgebaut, nur die Erhebung.
- **Stile aus dem DOMSnapshot [Entscheidung, Paket 55]:** ein
  `captureSnapshot` je Prüfung statt `CSS.getComputedStyleForNode` je
  Element. Ein Element ohne Layout-Objekt gilt als `display: none`, als
  `contents`, wenn darunter etwas gerendert wird; die Leerraum-Textknoten,
  die `DOM.getDocument` auslässt, kommen aus dem Snapshot dazu (sonst hinge
  `<span>a</span> <span>b</span>` zu „ab“ zusammen). Verfahren aus
  auditmysite (`accessibility/dom_document.rs`) übernommen [belegt dort im
  Code, hier an `form-embedded.html` gemessen].
- **iframes und Shadow DOM [Entscheidung, Paket 55]:** Ein iframe im selben
  Renderer-Prozess (`contentDocument`) ist ein eigenes Dokument mit eigenem
  ID-Index, weil `<label for>` und `aria-labelledby` nur im eigenen Dokument
  gelten. Shadow DOM des Autors hängt flach unter dem Host, Light-DOM-Kinder
  nur dort, wo ein `<slot>` sie aufnimmt (`distributedNodes`); Shadow DOM des
  Browsers (Innenleben von `<input>`) bleibt draußen. Die Slot-Zuordnung
  (Backend-ID → Knoten) reicht auch in iframe-Dokumente [belegt im Code,
  Paket 64; kein Slot im iframe an einer Seite gemessen].
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
- **Echte Abweichung im Test [Entscheidung, Paket 55]:** `form-broken.html`
  weicht über CSS-Inhalt (`::after`) ab: Chromium zählt ihn mit, `accname`
  auf dem DOM sieht ihn nicht. Der Befund nennt dann „meist CSS-Inhalt“ als
  Ursache; Text in CSS erreicht nicht jede Assistenztechnik gleich, deshalb
  `review` [Annahme, nicht gegen Screenreader gemessen].
- **iframes fremder Herkunft [belegt, Paket 64]:** Der CDP-Host startet
  Chrome mit `--disable-site-isolation-trials` (für `getFullAXTree` mit
  `frameId`, `capture.rs`). Damit liegt ein iframe von `127.0.0.1` in einer
  Seite von `localhost` im selben Prozess, `DOM.getDocument` (`pierce`)
  liefert sein Dokument mit, und sein Feld wird verglichen: In
  `form-fremd.html` meldet `namen-wie-accname` die CSS-Abweichung im iframe
  („Telefon (Rückfrage)“ gegen „Telefon“), per CSS verborgener Label-Inhalt
  dort ist keine Abweichung. Gegenprobe mit Site Isolation (Schalter
  entfernt): Das Feld fehlt schon im Modell (1 statt 2 Bedienelemente), weil
  `getFullAXTree` den Frame in einem anderen Prozess nicht erreicht, und
  deshalb auch in den Befunden, nicht als `untested`.
- **Keine eigene CDP-Sitzung je Frame [Entscheidung, Paket 64]:** Solange der
  Host ohne Site Isolation läuft, gibt es keinen Frame in einem anderen
  Prozess; DOM-Fakten über `Target.attachToTarget` hätten kein Feld im
  Modell, gegen das sie verglichen würden. Ein Host mit Site Isolation
  braucht Aufnahme **und** DOM-Fakten über die Sitzung des Frames → 70.
- **Lokaler Server für fremde Herkunft [Entscheidung, Paket 64]:**
  `url: server:<Pfad>` in einer Aufgabendatei startet (einmal je
  Verzeichnis und Lauf) einen HTTP-Server des Hosts auf `127.0.0.1` mit
  freiem Port und öffnet die Seite als `http://localhost:<Port>/<Datei>`
  (`relief-cdp/src/server.rs`, nur `GET`, nur Dateien darunter). Die Seite
  setzt ihr iframe per Skript auf die jeweils andere Adresse und denselben
  Port; `localhost` und `127.0.0.1` sind verschiedene Sites. Kein Netz,
  läuft in Prüfbefehlen und CI mit.
- **ID-Bereich im Shadow DOM [belegt, Entscheidung, Paket 64]:**
  `accname::IdIndex` (0.13) kennt nur `build(root)` über alle Nachfahren im
  flachen Baum, also einen Bereich je Dokument. Gemessen an
  `form-shadow-ids.html` vor der Änderung: drei falsche `review`-Befunde
  („Name“ und „Postleitzahl“ je als „Name Postleitzahl“, „Ort“ als
  „Kundennummer“). Jetzt vermerkt der Host je Element seinen ID-Bereich
  (Backend-ID des Shadow-Roots, `0` = Dokument; `DomFactsBuilder::scope`);
  steht eine ID des Feldes (`id` für `<label for>`, `aria-labelledby`) als
  `id` oder `label[for]` in mehr als einem Bereich, ist der Vergleich
  `untested` mit Nennung der ID statt eines falschen Befunds. Richtig
  rechnen statt auslassen braucht einen ID-Index je Baumbereich in
  barrierlab (`accname`, `a11y-dom`), nicht in Relief nachgebaut;
  Issue-Text im PR zu Paket 64.
- **Grenzen [belegt im Code]:** Ein Verweis aus einem Shadow-Root auf eine
  ID, die nur im Dokument steht (oder umgekehrt), löst `accname` auf,
  Chromium nicht; das ergibt einen `review`-Befund mit unpassender
  Ursache [Annahme, nicht gemessen]. Ebenso zählt ein umschließendes
  `<label>` jenseits der Shadow-Grenze für `accname` mit. Inhalt
  geschlossener `<details>` (`content-visibility`) meldet der Snapshot mit
  normalem `display`, `accname` zählt ihn dann mit [laut
  auditmysite-Kommentar, hier nicht gemessen]. Frames in einem anderen
  Prozess → 70.
- **Änderung über `TreeDelta` [Entscheidung, Paket 49]:** Der Vergleich
  „vorher/nachher“ nutzt `relief_model::TreeDelta::between`, nicht die
  Diff-Regeln aus `a11y-perception`: `relief-interaction` hängt außerhalb der
  Tests nicht an `a11y-perception` (im Fork-Build gibt es den Konverter
  nicht), und die Delta arbeitet schon auf dem Modell. Der Vorher-Stand ist
  das Modell vor der letzten `do:`-Zeile, auch wenn diese nichts ausgeführt
  hat; ohne Vorher-Stand (keine `do:`-Zeile seit `url:`) wird nur der
  Endzustand geprüft.
- **Wieder wahrnehmbar ist kein Einfügen [belegt, Entscheidung]:** Solange
  ein modaler Dialog offen ist, enthält die CDP-Aufnahme den Rest der Seite
  nicht (`form-clean.html`: `<p role=status>` fehlt vor „Rückruf
  bestätigen“, ist danach `created`). Eine neue Region gilt deshalb nur als
  eingefügt, wenn ihr Elternknoten schon vorher im Modell war; sonst kein
  Befund, weil das Modell „eingefügt“ nicht von „wieder wahrnehmbar“ trennt.
- **Neu eingefügt = Befund [Annahme]:** Dass Screenreader eine mitsamt Text
  eingefügte Live-Region oft nicht ansagen, ist Praxiswissen, hier nicht
  gemessen; der Abgleich mit echter Ausgabe ist 43.

### Bericht für CI [umgesetzt 2026-09-30, Paket 44]

`relief-cdp test <aufgaben>… [--junit datei] [--report datei] [--fork]`
läuft ohne Fenster und endet mit Fehler, sobald eine Erwartung verfehlt
oder eine Seite nicht ladbar ist (anders als `run`, das immer mit 0 endet
und nicht ladbare Seiten überspringt).

- **JUnit:** eine `testsuite` je Aufgabendatei, ein `testcase` je
  `expect:`-Zeile (Name: vorige Eingabe → Erwartung), `failure` mit der
  letzten Antwort; nicht ladbare Seiten als `error`.
- **a11y-report:** die Befunde aller `assert:`-Zeilen, zusammengefasst über
  einen stabilen Schlüssel (Regel, Ergebnis, Seite, Knoten/Selektor,
  Rolle, Name; die Meldung zählt nicht). Die Zustände, in denen ein Befund
  auftrat, stehen als Evidence `{source: "state", field: "nach", value:
  <letzte Eingabe bzw. „Laden“>}`. Belegt mit `spike/tasks/08-zustaende.txt`:
  derselbe `form/field-name`-Befund vor und nach einer Eingabe → ein Befund
  mit beiden Zuständen.
- **`--fork`:** startet den eigenen Build mit `--headless=new
  --enable-relief --relief-run=…` (Aktionen über `AXActionData`) und liest
  dessen Ausgabe. Belegt: `spike/tasks/01`–`05`, `07` → 79/79, dieselbe Zahl
  wie der CDP-Host auf denselben Dateien. Zusicherungen gibt es im Fork nicht
  (Feature `assertions` aus); `--report` gilt nur für den CDP-Host.
- **Beispiel für Projekte:** `examples/ci/relief-test.yml` (nicht aktiv im
  Relief-Repository).

Offen: Laufzeit je Datei im Fork-Bericht (die Ausgabe trägt sie nicht);
Befunde tragen heute keine Knoten-ID (Schlüssel ist dann Regel, Seite,
Rolle, Name) — zwei gleichnamige Felder mit demselben Befund fallen
zusammen.

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
- In `barrierlab/docs/consumers.md` ersetzt Relief den Reader-Host,
  ebenso in `architecture.md`, `project-state.md` und der Paketseite von
  `a11y-perception` (casoon/barrierlab#29, 2026-09-30, zum Merge offen).
  Die lokale Reader-Planung in barrierlab (`plan/reader/`, gitignored) ist
  als „ersetzt durch Relief“ geschlossen; ihr Referenzrahmen bleibt dort
  Nachschlagewerk.

## barrierlab einbinden

`casoon/barrierlab` ist der Ort für Barrierefreiheits-Bausteine, die mehrere
Werkzeuge nutzen (`barrierlab/docs/consumers.md`). Regel dort: **zwei
Konsumenten, dann Bibliothek**; Browserfreiheit bis L3; Pakete nehmen Daten
entgegen, holen sie nicht.

Heute nutzt Relief `a11y-perception` im Host und in den Aufnahmen sowie
`a11y-dom`, `accname` und `a11y-report` in `relief-interaction`
(Formular-Zusicherungen; Workspace, `Cargo.toml`). In `consumers.md` steht
Relief mit diesen vier Crates, künftig zusätzlich `a11y-rules`
(casoon/barrierlab#29).

| Richtung | Was | Wann |
|---|---|---|
| nutzen | `a11y-rules`, `a11y-report` für Befunde im Inspector | 21 |
| nutzen | `a11y-dom` und `accname` für DOM-basierte Namensprüfungen; der AX-Graph allein genügt dafür nicht | 42 ✓ |
| nutzen | Diff-Regeln aus `a11y-perception` statt eigener Diff-Logik (Zusicherungen nutzen `TreeDelta`, → oben) | 25 |
| ablegen | Screenreader-Treiber-Interface samt Adaptern und Phrasen-Protokoll, sobald ein zweiter Konsument ihn braucht | 43 |
| ablegen | Formular-Zusicherungen (42 ✓), sobald ein zweites Werkzeug sie braucht | — |
| ablegen | Interaction Graph für aufgabenbasierte Journeys (→ 00, Kandidat) | nach 24 |
| nicht ablegen | Fork-Adapter, Bridge, alles mit Chromium-Typen | — |

Jedes Ablegen läuft als eigener PR in barrierlab mit Eintrag im Changelog des
Pakets.

### Kandidaten zum Ablegen [Stand 2026-09-30]

Maßstab ist die barrierlab-Regel: abgelegt wird erst, wenn ein zweiter
Konsument es **nachweislich gleich** braucht.

| Kandidat | in Relief | zweiter Konsument | Folge |
|---|---|---|---|
| CDP-`getFullAXTree`-JSON → `a11y_perception::AXNode` | `relief-cdp/src/capture.rs` (`convert_node`) | auditmysite, `accessibility/extractor.rs`; Relief hat die Umwandlung von dort übernommen [belegt, Modulkopf] | echte Doppelung; offen, ob ein barrierlab-Paket CDP-förmiges JSON entgegennehmen darf („keine CDP-Typen in einer öffentlichen API“) — in barrierlab zu entscheiden |
| DOM-Fakten aus `DOM.getDocument` → `a11y-dom` | `relief-cdp/src/assertions.rs` | auditmysite, `accessibility/dom_document.rs` (`build_document`, mit AX-Fakten und Stilen) | Doppelung im Zweck; seit 55 auch im Verfahren (Stile und Leerraum aus dem DOMSnapshot, Shadow DOM mit Slots, von dort übernommen); auditmysite ohne iframes, Relief ohne AX-Fakten [belegt, beide Module]; dieselbe CDP-Frage wie oben |
| Formular-Zusicherungen | `relief-interaction/src/assertions.rs` | keiner bekannt; auditmysites `form_error`-Journey (Live-Region nach Absenden) und `a11y-rules` `forms/label-missing` (statisch) überschneiden sich nur in Teilen [Annahme, nicht im Code verglichen] | bleibt in Relief; zudem über `SemanticGraph` statt `a11y-perception` formuliert |
| Screenreader-Treiber-Interface, Phrasen-Protokoll | noch nicht gebaut (→ 43) | keiner bekannt (die Kalibrierung gegen echte Screenreader war Teil der geschlossenen Reader-Planung) | bleibt in Relief |
| Interaction Graph für aufgabenbasierte Journeys | `relief-interaction/src/graph.rs` | keiner bekannt; auditmysite-Journeys arbeiten auf `a11y-perception` | bleibt in Relief |
| Seitentyp, Gruppen, primäre Aktion | `relief-interaction/src/page.rs` | keiner bekannt | bleibt in Relief |
| Privacy-Filter, Delta-Format, Konverter `AXTree` → `SemanticGraph` | `relief-ai-contract`, `relief-model` | keiner; Relief-eigenes Modell | bleibt in Relief |

Die Liste steht hier und nicht in barrierlab [Entscheidung]: barrierlabs
`docs/` beschreibt den Ist-Zustand, und `consumers.md` nennt, wer was
benutzt; ein Kandidat ohne zweiten Konsumenten ist dort keine Aussage.
barrierlabs `plan/` ist gitignored, offene Punkte laufen dort als Issues.
Wird ein Kandidat reif, bekommt er ein Issue bzw. einen PR in barrierlab.

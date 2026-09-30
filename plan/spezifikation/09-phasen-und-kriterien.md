# 09 · Phasen, Kriterien, Nicht-Ziele

Leitregel: **nicht mit KI anfangen**, und keinen Browser bauen, bevor Phase 0
gegen aktuellen Chromium-Quellcode belegt ist.

Technische Machbarkeit und Nutzwert werden parallel validiert. Ein bestandener
CDP-/Fork-Durchstich allein ist noch kein Go für den Ausbau.

## Phase 0a · CDP-Spike [Entscheidung]

Am 2026-09-24 beschlossen. Idee: die Kernhypothese
*AXTree → Rust → Interaction Graph → Aktion zurück* zuerst **ohne Fork** gegen
ein normales Chrome über CDP beweisen. CDP liefert den nativen AXTree
(`Accessibility.getFullAXTree`; Änderungsereignisse liefert Chrome nicht, siehe
Ergebnis), auditmysite macht genau
das schon, `a11y-perception` verarbeitet genau dieses Format.

Läuft ohne Aufwand auf allen drei Plattformen. Liefert in Tagen statt Wochen: Graph-Logik, Intent-Parser, Validierung,
Testkorpus — alles browserfreier Rust-Code, der im Fork unverändert
weiterlebt. Was der Spike **nicht** beweist: Integrationspunkt im
Browser-Prozess, Performance ohne CDP, `AXActionData`-Rückweg (CDP-Aktionen
laufen über DOM/Input, nicht über AX-Aktionen).

### Ergebnis (2026-09-24) [belegt]

Umgesetzt als Workspace `crates/relief-interaction` (browserfrei: Graph,
Parser, Auflösung, Validierung, Antworten) und `crates/relief-cdp` (Host über
CDP). Aufgaben in `spike/tasks/`, Testseiten in `spike/fixtures/`.

**Kernhypothese über CDP bestätigt.** Alle Erwartungen erfüllt: 45 in den
Standardaufgaben (vier eigene Testseiten: sauber, kaputt, Formular, Consent im
iframe; dazu gov.uk, Wikipedia, drei APG-Beispiele) und 9 in einem gemischten
Korpus realer Seiten im sichtbaren Browser (casoon.de, insights.casoon.de,
in-punkto.com, inros-lackner.de, bahn.de, tagesschau.de, zeit.de, ikea.com,
dwd.de). Abgedeckt: Struktur erfassen, Auswahl ändern, Formular ausfüllen und
absenden, Dialog und Menü öffnen und schließen, Disclosure aufklappen, Suche
füllen, Consent-Dialog im iframe erkennen und bedienen.

Messung (macOS, M4, Chrome, `relief-cdp measure`; Aufnahme = `getFullAXTree`
inkl. iframes):

| Seite | AX-Knoten | Bedienelemente | Aufnahme | Graph-Aufbau | stabil |
|---|---|---|---|---|---|
| Testshop | 80 | 12 | 3–7 ms | 0,06 ms | ja |
| gov.uk | 810 | 88 | 28–31 ms | 0,9 ms | ja |
| tagesschau.de | 3 170 | 132 | 136–181 ms | 1,8 ms | ja |
| ikea.com | 3 878 | 333 | 134–137 ms | 2,3 ms | ja |
| bild.de (2 von 4 iframes) | 5 837 | 668 | 208–212 ms | 4,2 ms | ja |
| Wikipedia-Artikel | 7 039 | 660 | 235–238 ms | 4,6 ms | ja |
| spiegel.de | 7 820 | 696 | 258–264 ms | 4,8 ms | ja |
| zeit.de (sichtbar) | 8 775 | 579 | 280–285 ms | 4,5 ms | ja |
| bahn.de | 391–567 | 46–70 | 19–41 ms | 0,8 ms | nein, lädt nach (mit Netzwerk-Ruhe: 679 Knoten, 65, ja) |

Was daraus folgt:

1. **Die Rust-Seite ist kein Engpass** (< 5 ms auch bei ~9 000 Knoten).
   Teuer ist die Erhebung: ein Vollsnapshot kostet auf großen Seiten
   200–285 ms.
2. **Über CDP gibt es keine AX-Deltas.** Chrome sendet
   `Accessibility.nodesUpdated` nicht — geprüft mit rohem CDP ohne
   chromiumoxide, auch nach `getFullAXTree` und `getPartialAXTree`; nur
   `loadComplete` kommt. Ein CDP-Host bleibt damit bei Vollsnapshots, und das
   Kriterium „< 100 ms p95 auf großen Seiten“ ist **nur im Fork** erreichbar
   (`AXTreeUpdate`/`AXTreeObserver`). Das ist ein technisches Argument für den
   Fork gegenüber CDP-Host und Component Extension — gemessen wird es erst in
   Backlog 17.
3. **„Seite ruht“ über DOM-Mutationen** ersetzt das feste Warten: keine
   Mutation für 150 ms (höchstens 3 s). Aktionen sind damit nach ~160 ms
   statt 700 ms auswertbar, und Abfragen ohne Mutation brauchen gar keine neue
   Aufnahme — geprüft gegen Vollsnapshots, in allen Fällen gleich. Grenzen:
   Wirkungen ohne DOM-Mutation (Checkbox `checked`, per Skript gesetzter
   `value`) — nach eigenen Aktionen wird deshalb immer neu aufgenommen; eine
   Seite, die ihr Dokument ersetzt, muss neu angefordert werden
   (`DOM.documentUpdated`). Seiten, die nach der DOM-Ruhe weiter nachladen
   (bahn.de), deckt die Netzwerk-Ruhe ab (siehe
   [Nachtrag](#nachtrag-netzwerk-ruhe-2026-09-25-belegt)).
4. **Der Diff braucht eine Deutungsschicht.** Rohdaten sind für Menschen
   unbrauchbar: Textboxen entstehen bei jeder Textänderung neu, ein modaler
   Dialog macht korrekt den ganzen Rest „nicht wahrnehmbar“ (75 Knoten), und
   Chrome vergibt die Rolle `dialog` auch an nicht-modale Vorschlagslisten
   (Amazon, IKEA). `respond::describe_diff` fasst das zusammen.
5. **`a11y-perception` verfolgt `checked`/`pressed` nicht.** Relief
   vergleicht zusätzlich die Zustände des Zielelements im Graph
   (`respond::target_change`). Die Korrektur ist auf barrierlab `main`
   (7ea448b); das Release 0.1.1 enthält sie nicht (`TRACKED_PROPERTIES` =
   `expanded`, `hidden`, `selected`, `invalid`, `modal`, geprüft 2026-09-25).
6. **Mehrdeutigkeit und Risiko greifen**: „Öffne den Warenkorb“ fragt zwischen
   Link und Button nach; „kaufen“, „senden“ und unbenannte Buttons verlangen
   Bestätigung. Teilstring-Treffer mitten im Wort waren zu schwach („Suche“
   traf „Besucher“ auf casoon.de) — es zählen nur noch Wortanfänge.
7. **Schließen ohne Namen des Ziels**: „Schließe den Dialog“ findet den
   Schließen-Button im offenen Dialog, sonst geht Escape an das fokussierte
   Element (APG-Menü: `expanded true → false`, Fokus zurück zum Button).
8. **iframes**: Die Bäume der Frames werden einzeln geholt und unter ihrem
   `iframe`-Knoten eingehängt; Aktionen darin funktionieren. Frames fremder
   Origins erreicht der Host nur, weil Chrome im Spike mit
   `--disable-site-isolation-trials` läuft; auch dann bleiben einzelne Frames
   unerreichbar (bild.de 2 von 4). Das Consent-Banner von bild.de liegt so im
   Graph.
9. **Grenze des AXTree bestätigt**: Ein `div` mit `onclick` und Größen-`span`s
   ohne Rolle existieren im Graph nicht. Das lässt sich nur mit DOM-Kontext
   (Quellenstufe 5) oder KI schließen — die Priorität in 03 ist richtig.
10. **Erschließung ohne KI funktioniert im Kleinen**: Icon-Links ohne Namen
    werden über die URL zu „cart“ bzw. „mein konto“ (Inferred, mit Evidence).
11. **Benannte Abschnitte fluten die Landmarks** (Wikipedia: 33 `region`s).
    Für die Seitenbeschreibung werden sie zusammengefasst.
12. **Automatisierung wird erkannt.** zeit.de blockiert Headless-Chrome,
    amazon.de lieferte dem automatisierten Chrome bei späteren Läufen eine
    leere Seite. Ein CDP-Host ist für Websites als Automatisierung sichtbar —
    ein weiteres Argument gegen den CDP-Host als Auslieferungsform. Umgangen
    wird das nicht.
13. **Consent-Dialoge sind überall** (bahn.de, ikea.com, zeit.de, bild.de) und
    ziehen teils den Fokus an sich (IKEA nach dem Füllen der Suche). Für
    Nutzertests (01b) ist das der erste Stolperstein jeder Aufgabe.

### Nachtrag: Netzwerk-Ruhe (2026-09-25) [belegt]

bahn.de ruht nach dem Laden für DOM und Netz 330–490 ms und stößt dann per
Zeitgeber weitere Anfragen an (Optimizely, `angebote/stammdaten`), deren
Antworten die Seite umbauen (54 → 65 Bedienelemente). DOM-Ruhe (150 ms) endet
davor. „Seite ruht“ verlangt deshalb zusätzlich: keine Anfrage ausstehend und,
falls seit Beginn des Wartens Netzverkehr war, 500 ms ohne Netzwerkereignis
(wie `networkIdle` in Chrome). Ausgeklammert: `EventSource`, `Ping`, `Media`,
WebSockets und Anfragen, die länger als 1 s offen sind. Die Abos starten vor
dem Laden; Ereignisse verschiedener Arten kommen über getrennte Abos, ein
`loadingFinished` kann vor seinem `requestWillBeSent` eintreffen und wird
gemerkt.

Verworfen: nur bei Ruhe an der Obergrenze vor dem nächsten Befehl neu
aufnehmen. bahn.de erreicht die Obergrenze nicht, die DOM-Ruhe endet regulär
zu früh; und vor jedem Befehl wird bei DOM-Änderung ohnehin neu aufgenommen.

Messung (M4, Google Chrome headless, `relief-cdp measure --repeat 5`, „Ruhe“ =
Warten nach dem Laden):

| Seite | vorher Ruhe / stabil | nachher Ruhe / stabil |
|---|---|---|
| bahn.de (je 8 Läufe) | 151–392 ms / 0 von 8 | 2 055–2 201 ms / 8 von 8 |
| bahn.de, Netzfenster 300 ms statt 500 | — | 551–858 ms / 0 von 5 |
| gov.uk | 151 ms / ja | 151 ms / ja |
| en.wikipedia.org (Accessibility) | 156 ms / ja | 583 ms / ja |
| de.wikipedia.org (Barrierefreiheit) | 152–193 ms / ja | 533 ms / ja |
| `spike/fixtures` (5 Seiten) | 150–152 ms / ja | 151–152 ms / ja |

Aktionen: Standardaufgaben 01–05 68/68, Median der Ruhe 152 ms vorher wie
nachher; einzig die Navigation auf eine nicht vorhandene Datei (Fehlerseite)
wartet 552 statt 200 ms. `10-real.txt` 15/15, mit `RELIEF_VERIFY=1` 6
übersprungene Aufnahmen gleich, 0 abweichend; drei Aktionen mit Netzverkehr
(gov.uk Suche, Wikipedia) warten 500–600 statt 150–250 ms. Auf bahn.de
braucht der erste Befehl nach dem Laden keine neue Aufnahme mehr (vorher:
„DOM geändert“).

Nicht belegt: Seiten, die dauerhaft kurze Anfragen senden (Polling unter 1 s);
sie warten bis zur Obergrenze von 3 s.

Nicht belegt: Aktionen über `AXActionData` (der Spike nutzt DOM/JS am Element),
Widgets, die nur auf echte Tastaturereignisse reagieren, Windows (kein
Host). Offen → Backlog 30, 31. Linux: CDP-Host in der CI belegt (→ 10,
„Linux-Nachweis“, 2026-09-30).

## Phase 0b · Machbarkeitsstudie Chromium

1. Chromium auf macOS ARM (M4) reproduzierbar bauen.
2. Wo ist der AXTree im Browser-Prozess verfügbar?
3. Wie werden Änderungen beobachtet (`AXTreeObserver`)?
4. Wie wird ein Snapshot exportiert?
5. Wie gehen AX-Aktionen zurück (`AXActionData`)?
6. Wie wird Rust angebunden (→ 02)?
7. Welche Chromium-Änderungen außerhalb `//relief/` sind nötig?
8. Wie groß wird der Fork-Wartungsaufwand?

Liefergegenstände: Architekturdiagramm · relevante
Chromium-Dateien/Klassen · Datenfluss AXTree → Rust · Datenfluss Action →
Chromium · FFI/IPC-Entscheidung · Crate-Struktur · Chromium-Verzeichnisstruktur ·
Threat Model · Privacy Model · AI Boundary · Performance-Risiken ·
Fork-/Update-Strategie · Teststrategie · Aufwand pro Phase · Blocker ·
Go/No-Go-Kriterien. Jeweils getrennt nach *belegt / Entscheidung / Annahme /
validieren*. Viele davon sind in diesen Kapiteln vorbereitet; Phase 0b füllt
die **[validieren]**-Stellen.

### Chromium-Build auf dem M4 (2026-09-24) [belegt]

Mit `scripts/chromium-setup.sh` (depot_tools, Checkout ohne Historie, Tag
flach holen, `gclient sync`, `gn gen`, `autoninja`) auf einem MacBook Pro M4
(14 Kerne, 48 GB RAM), macOS 27, Xcode 27 mit SDK 27.0:

| Schritt | Zeit | Anmerkung |
|---|---|---|
| Erstbuild `chrome` | 2 h 11 min (7 872 s) | parallel liefen vier Relief-Agenten; ruhig vermutlich kürzer |
| inkrementell, `tab_features.cc` geändert | 19,7 s | Datei des geplanten Fork-Eingriffs |
| inkrementell, `ui/accessibility/ax_tree.cc` geändert | 7,0 s | Component-Build |
| ohne Änderung | 1,6 s | |
| Platz Checkout + Build | 39 GB | davon `out/Relief` 8,4 GB |

Checkout- und Sync-Zeiten sind durch mehrere Neustarts nicht sauber messbar.
`args.gn`: `is_debug=false`, `is_component_build=true`, `symbol_level=0`,
`use_lld=false`. Der gebaute Browser meldet `Chromium 154.0.8037.58`, und
`relief-cdp` erfüllt mit ihm alle 33 Erwartungen in `spike/tasks/01`–`04`.
Das Go/No-Go-Kriterium „inkrementeller Build < 5 min“ ist damit erfüllt.

Stolpersteine, jetzt im Skript geprüft oder umgangen:

- `git-lfs` fehlt → `third_party/litert` lässt sich nicht auschecken.
- `gclient sync --revision src@refs/tags/…` hängt im Checkout ohne Historie
  (holt alle Branches) → Tag flach holen, gclient auf den Commit festlegen.
- Xcode 26+ liefert die Metal-Toolchain nicht mehr mit → ANGLE scheitert
  (`xcodebuild -downloadComponent MetalToolchain`, 839 MB).
- Eine Anwendungs-Firewall blockierte das Python von depot_tools
  („No route to host“) → Hooks konnten nichts nachladen.

Nicht geprüft: VoiceOver gegen den eigenen Build (manuell, steht aus).

→ Backlog 15–18; Integrationspunkte und Eingriffsliste (Fragen zu Klassen
und Änderungen außerhalb `//relief/`) belegt in 01.

## Phase 0c · Nutzennachweis [neu: parallel zu 0a/0b]

Der CDP-Spike liefert früh genug eine bedienbare Oberfläche, um nicht nur die
Pipeline, sondern auch die Produktannahme zu prüfen. Dafür:

1. 5–8 Gespräche mit potenziellen Nutzerinnen und Nutzern aus zunächst **einer**
   Fähigkeits-/Bedarfsgruppe; Aufgaben und vorhandene Werkzeuge dokumentieren.
2. Einen primären Job-to-be-done und eine Seitenklasse auswählen, etwa
   mehrstufige Formulare ohne präzise Zeigerbedienung. Auswahl nach Häufigkeit,
   Schwere des Problems und Stärke des Relief-Vorteils — nicht nach technischer
   Bequemlichkeit.
3. 3–5 Aufgaben zuerst mit der heutigen persönlichen Werkzeugkette und danach
   mit dem Prototyp ausführen lassen. Reihenfolge zwischen Sitzungen wechseln,
   damit Lerneffekte das Ergebnis nicht verzerren.
4. Messen: Task Completion, kritische Fehlaktionen, Abbrüche, Hilfebedarf,
   Zeit nur als Nebenmetrik sowie Vertrauen und wahrgenommene Kontrolle.
5. Beobachtete Probleme in Anforderungen übersetzen; keine Diagnose- oder
   Persona-Annahmen aus einzelnen Gesprächen verallgemeinern.

Protokoll, Aufgaben und Messbogen: → 11. Zielgruppe entschieden: ohne präzise
Zeigerbedienung.

Das ist formative Forschung, kein statistischer Wirksamkeitsnachweis. Ziel ist
eine belastbare MVP-Auswahl und das frühe Erkennen einer falschen
Produktannahme.

## Go/No-Go nach Phase 0 [Entscheidung: Meilenstein · Annahme: Schwellen]

Harter Meilenstein: **AXTree → Rust → Interaction Graph → `activate` zurück an
Chromium**, im eigenen Build, auf dem M4, mit einer echten Webseite.

Go, wenn alle erfüllt:

- Fork baut reproduzierbar aus einem Skript; inkrementeller Build nach
  Änderung in `//relief/` < 5 min.
- Patches außerhalb `//relief/` ≤ ~10 Dateien, jeder begründet.
- Rust-Runtime erhält initialen Baum und Updates einer realen Seite.
- Update-Latenz der **Relief-Strecke** (AX-Paket im Browser-Prozess → Graph
  aktualisiert) < 100 ms (p95) auf einer großen Seite (z. B.
  Nachrichten-Startseite). Festgelegt am 2026-09-24: Die Serialisierung in
  Blink (nicht-interaktive Änderungen höchstens alle 150 ms) gehört nicht
  dazu, weil sie ohne Blink-Eingriff nicht beeinflussbar ist; Ende zu Ende
  wird weiter gemessen und berichtet.
- `activate` auf einen Graph-Knoten löst die Aktion aus, der Diff zeigt das
  Ergebnis.
- VoiceOver funktioniert parallel unverändert.
- Relief-Code außerhalb der Plattform-Adapter enthält keine
  Plattformweichen (Linux-Build als Stichprobe, Windows spätestens vor Phase 2).
- *(zurückgestellt, → Backlog 90)* Eine primäre Nutzergruppe, ein konkreter
  Job-to-be-done und eine heutige Baseline sind dokumentiert.
- *(zurückgestellt, → Backlog 90)* Der kleinste Relief-Prototyp verbessert mindestens eine wichtige Aufgabe
  gegenüber dieser Baseline oder ermöglicht eine zuvor unlösbare Aufgabe, ohne
  neue kritische Fehlaktionen zu erzeugen.

### Messung im Fork (2026-09-24) [belegt]

Durchstich AXTree → Rust → Graph → `activate` → Diff im eigenen Build
(154.0.8037.58, `out/Relief`, Component-Build), M4 Pro, macOS 27. Aufbau in
01 („Umsetzung im Fork“) und 02 („Im Fork gebaut“). Gestartet mit
`--enable-relief --relief-log=<datei>`, meist `--headless=new`, spiegel.de
zusätzlich im sichtbaren Fenster; Messknoten über
`scripts/fork-measure.mjs` (60 Stück im Abstand von 250 ms). Zwei Strecken:

- **Relief-Strecke**: Eingang des AX-Pakets in
  `AccessibilityEventReceived` → eigener Baum → `cxx`-Strukturen → Sequenz
  der Runtime → Delta angewandt. Zeitstempel im Browser-Prozess.
- **Ende zu Ende**: Seite schreibt `relief-probe:<Date.now()>` in ein
  Element → Knoten mit diesem Namen im Graphen (Systemuhr auf beiden
  Seiten, ms-genau). Enthält die Serialisierung im Renderer.

| Seite (Knoten) | Ende zu Ende p50 / p95 / max | Relief-Strecke p50 / p95 / max | UI-Thread je Paket p50 / p95 / max |
|---|---|---|---|
| Testshop `spike/fixtures/shop-clean.html` (83) | 2,5 / 8,1 / 67 ms | 0,09 / 0,31 / 1,8 ms | 0,05 / 0,17 / 1,2 ms |
| en.wikipedia.org/wiki/Accessibility (7 950) | 9,8 / 12,4 / 13,1 ms | 0,11 / 0,23 / 32 ms | 0,06 / 0,14 / 26,5 ms |
| spiegel.de, Lauf 1 (8 311) | 43,2 / 184,6 / 205,6 ms | 0,08 / 0,97 / 23,2 ms | 0,05 / 0,37 / 18,5 ms |
| spiegel.de, Lauf 2 (8 354) | 42,2 / 170,5 / 237,7 ms | 0,08 / 0,60 / 23,4 ms | 0,05 / 0,16 / 18,1 ms |
| spiegel.de, sichtbares Fenster (8 431) | 45,8 / 191,9 / 314,7 ms | 0,08 / 1,08 / 23,2 ms | 0,05 / 0,35 / 18,2 ms |

- **Maxima = Vollbaum.** Das größte Paket ist der erste Baum der Seite:
  Wikipedia 7 950 Knoten in 23,1 ms `Unserialize` + 4,4 ms Füllen der
  `cxx`-Strukturen auf dem UI-Thread, danach 5,4 ms Anwenden in Rust auf der
  eigenen Sequenz; spiegel.de 14–15 + 3,3–4 ms. Die Warteschlange zur
  Runtime-Sequenz kostet p95 < 0,3 ms.
- **Ursache der spiegel.de-Werte liegt in Blink, nicht in Relief.** Blink
  serialisiert nicht-interaktive Änderungen höchstens alle 150 ms nach dem
  Laden (350 ms davor): `AXObjectCacheImpl::GetDeferredEventsDelay`
  (`third_party/blink/renderer/modules/accessibility/ax_object_cache_impl.cc:3507–3520`),
  angewandt in `:3593–3621`. Sofort serialisiert werden Aktionen
  (`EventFrom::kAction`), Änderungen am fokussierten Knoten und u. a. Fokus-,
  Klick-, Auf-/Zuklapp- und Checked-Ereignisse (`:5428–5460`). Auf
  spiegel.de ändert die Seite sich laufend selbst, ein Messknoten wartet
  deshalb im Mittel bis zum nächsten 150-ms-Fenster. Gegenprobe: 100
  Messknoten im Abstand von 40 ms auf dem ruhigen Testshop ergeben 28 Pakete,
  also eins je ~170 ms.
- **`activate`**: Testshop „In den Warenkorb“ → Knoten „Größe 42 in den
  Warenkorb gelegt.“ in der nächsten Delta nach 11,7 ms (zweiter Lauf 20,9
  ms); Consent im iframe (`spike/fixtures/with-iframe.html`, „Alle
  akzeptieren“) → „Zustimmung gespeichert.“ nach 5,6 ms; spiegel.de
  „Einwilligen und weiter“ im Sourcepoint-iframe → erste Delta nach 98 ms,
  Seiteninhalt (8 247 Knoten) nach 277 ms. Plan → gesendet < 0,2 ms.
- **`kScreenReader`** ändert die Knotenzahl auf beiden großen Seiten kaum
  (Wikipedia 7 950 = 7 950, spiegel.de 8 312 zu 8 311); CDP-Spike
  (`kScreenReader`) zum Vergleich: Wikipedia 7 039, spiegel.de 7 820
  (andere Tage, andere Seitenstände).
- **Speicher (grob)**: RSS des Browser-Prozesses auf Wikipedia nach 15 s,
  je drei Läufe: ohne `--enable-relief` 318–329 MB, mit 374–376 MB, also
  ~+50 MB für ~8 000 Knoten (AX-Modus im Browser, eigener `ui::AXTree`,
  Rust-Graph). Renderer nicht gemessen.
- **Fehler**: in allen Läufen 0 abgelehnte Deltas und 0 fehlgeschlagene
  `Unserialize`.
- **Build**: Änderung an C++ in `//relief/` → `chrome` in 9,2 s, an einer
  Rust-Datei in 7,8 s; erster Build mit Relief (inkl. `tab_features.cc`)
  24 s.

#### Nachtrag Paket 33 (2026-09-24) [belegt]

Adapter mit Positionen, Baum-Lebenszyklus und begrenztem Neuaufbau (01,
„Umsetzung im Fork“) gegen den Stand aus 17, gleicher Rechner, gleiche
Sitzung, je zwei Läufe, `--headless=new`, 60 Messknoten im Abstand von
250 ms. **Nur Offline-Kopien:** Navigationen des Builds hingen in dieser
Sitzung, auch zu 127.0.0.1; echte Seiten ließen sich nicht laden (Ursache:
unbeantwortete Anfrage an den Schlüsselbund, nicht die Firewall, → Nachtrag
Paket 35). Deshalb Wikipedia
und spiegel.de als `curl`-Kopie ohne Skripte, Stylesheets und externe
Ressourcen per `file://` (statisch, andere Knotenzahlen als oben).

| Seite (Knoten) | Relief-Strecke p50 / p95 / max, Stand 17 | dasselbe, Stand 33 |
|---|---|---|
| Testshop (83) | 0,19 / 0,95 / 1,7 · 0,21 / 0,65 / 1,5 ms | 0,22 / 0,73 / 1,4 · 0,19 / 0,37 / 0,9 ms |
| Wikipedia offline (8 905) | 0,18 / 0,26 / 14,4 · 0,19 / 0,26 / 14,5 ms | 0,20 / 0,26 / 15,2 · 0,20 / 0,31 / 16,1 ms |
| spiegel.de offline (6 888) | 0,14 / 0,20 / 16,6 · 0,13 / 0,17 / 16,5 ms | 0,14 / 0,22 / 17,0 · 0,14 / 0,20 / 14,5 ms |
| dieselbe, 40 px Scrollen je Messknoten | 0,12 / 0,17 / 16,6 · 0,12 / 0,19 / 16,7 ms | 0,12 / 0,17 / 17,4 · 0,12 / 0,17 / 16,7 ms |

- **Keine Regression der Relief-Strecke p95** (Unterschiede im Rauschen der
  Läufe). Der Vollbaum kostet mit Positionen etwas mehr: UI-Thread max auf
  Wikipedia 11,1–11,3 ms (17) zu 11,7–12,8 ms (33). Ende zu Ende
  unverändert (Wikipedia offline p50/p95 15,7–16,3 / 17,0–17,6 ms in beiden).
- **Positionen, Grenzfall:** Testseite mit 2 000 Links in einem Container,
  der sich alle 100 ms verschiebt (8 069 Knoten): 142 Positionspakete mit je
  ~8 060 `BoundsChange`, UI-Thread je Positionspaket p50 1,5 / p95 1,8 /
  max 2,1 ms, Eingang → Graph p50 2,4 / p95 2,9 ms. Baum-Pakete derselben
  Seite warten dahinter auf der Runtime-Sequenz: Relief-Strecke p95 1,07 ms
  (17: 0,66 ms). Auf gewöhnlichen Seiten waren Positionspakete selten und
  klein (Wikipedia offline: eins mit 3–5 Knoten, 0,7 ms). Scrollen der Seite
  kostet nichts (Root-Scroller).
- **Speicher** (Wikipedia offline, 8 900 Knoten, RSS nach 15 s, je drei
  Läufe, Renderer = Summe aller Renderer-Prozesse):

  | | Browser | Renderer |
  |---|---|---|
  | ohne Relief | 308–313 MB | 954–959 MB |
  | mit Relief | 343–348 MB | 1 009–1 020 MB |
  | Screenreader-Modus, ohne Relief | 339–342 MB | 997–1 010 MB |
  | Screenreader-Modus, mit Relief | 362–364 MB | 996–1 007 MB |

  Relief allein: Browser ~+35 MB (eigener `ui::AXTree`, Rust-Graph),
  Renderer ~+55 MB (Blink hält den AX-Baum). Ist schon ein Screenreader-Modus
  an, kostet Relief im Renderer nichts mehr und im Browser ~+22 MB.
  „Screenreader-Modus“ ist `--force-renderer-accessibility=complete`
  (`kNativeAPIs` samt `BrowserAccessibilityManager`), eine Näherung an
  VoiceOver; mit echtem VoiceOver gemessen → „Nachtrag Paket 19“.
- **Fehler**: 0 abgelehnte Deltas, 0 fehlgeschlagene `Unserialize`.
- **Build**: Änderung in `//relief/` → `chrome` und `relief_browsertests`
  in 13–25 s; Erstbau von `relief_browsertests` 8,6 min.

#### Nachtrag Paket 35 (2026-09-25) [belegt]

Gleicher Rechner, `--headless=new`, frisches Profil je Lauf,
`scripts/fork-measure.mjs` mit 60 Messknoten im Abstand von 250 ms.

- **Netz des Builds — Ursache: Schlüsselbund, nicht die Firewall.** Jedes
  frische Profil fragt beim Start den macOS-Schlüsselbund nach dem
  Schlüssel „Chromium Safe Storage“ (Cookie-Verschlüsselung); der
  Netzwerkkontext bekommt seine Cookie-Verschlüsselung darüber
  (`services/network/network_context.cc:3417–3419`,
  `components/os_crypt/async/browser/keychain_key_provider.mm:40–48`), und
  die erste Navigation wartet, bis die Anfrage beantwortet ist. Belegt über
  die Zeitstempel: In jedem Lauf ohne Schalter steht auf stderr
  „Keychain lookup failed … userCanceledErr (-128)“
  (`components/os_crypt/common/keychain_password_mac.mm:102`), und genau dann
  kommt die erste Seite — in drei Instanzen nach 2,2–4,0 s, in der vierten
  nach 86,6 s (Seitenwechsel im Relief-Protokoll bei 86,4 s). Danach lud
  dieselbe Instanz `http://127.0.0.1`, Wikipedia, spiegel.de und bild.de je
  in 1 s. Wer die Anfrage abbricht (Dialog, Zeitablauf), ist nicht geklärt
  [Annahme: der Dialog des Schlüsselbunds; hängt er unbeantwortet, hängt jede
  Navigation, wie in 33 und im ersten Anlauf von 35]. Die Firewall ist es
  nicht: Ihre Regel erlaubt jedes Ziel, und die Instanzen liefen
  nacheinander, nie parallel. **Abhilfe:** Messläufe mit
  `--use-mock-keychain` starten (Chromiums Testschalter „prevents blocking
  dialogs from causing timeouts“, `components/os_crypt/common/os_crypt_switches.h:16–18`;
  das Wegwerf-Profil nutzt einen Schlüssel im Speicher und fasst den echten
  Schlüsselbund nicht an). Damit luden sieben Instanzen hintereinander,
  ohne Meldung des Schlüsselbunds; wo gemessen, stand der Seitentitel nach
  4 s.
- **Wikipedia live, ein Lauf, Stand 33** (vor den Änderungen aus 35;
  Messknoten begannen noch während des Ladens), 7 856 Knoten:

  | | Stand 17 (7 950 Knoten) | Lauf 35 |
  |---|---|---|
  | Relief-Strecke p50 / p95 / max, alle Pakete | 0,11 / 0,23 / 32 ms | 0,14 / 11,8 / 46,6 ms |
  | Relief-Strecke, nur ruhige Seite (54 Pakete) | — | 0,09 bis 1,05 ms |
  | Ende zu Ende, ruhige Seite | 9,8 / 12,4 / 13,1 ms | 5,0 bis 8,4 ms (p50 ~5,7) |
  | Ende zu Ende, während des Ladens | — | 58 bis 271 ms |

  p95 und max stammen aus vier Paketen des Ladens: erster Baum (1 987
  Knoten, 11,8 ms), dann drei große Umbauten durch die Seite selbst
  (6 003 angelegt + 1 681 geändert: 46,6 ms, davon UI-Thread 35,4 ms =
  22,2 ms `Unserialize` + 13,2 ms Umwandlung; 4 929 Änderungen: 35,7 ms;
  9 347 Änderungen: 39,1 ms). Die live ausgelieferte Seite baut ihren Baum
  beim Laden also mehrfach groß um; die Offline-Kopie aus 33 (ohne
  Skripte) tat das nicht. Zwei
  Positionspakete während des Ladens (93 bzw. 101 Änderungen → 140 bzw.
  208 `BoundsChange`, UI-Thread 2,5 bzw. 3,0 ms); danach keine mehr.
  Einzellauf, daher nur Hinweis, keine Aussage zum Grenzfall aus 33.
- **Lokal nach den Änderungen aus 35** (Positionen im iframe, Zoom),
  `file://`:

  | Seite (Knoten) | Relief-Strecke p50 / p95 / max | UI-Thread je Paket | Ende zu Ende |
  |---|---|---|---|
  | Testshop (83) | 0,13 / 0,31 / 0,61 ms | 0,08 / 0,21 / 0,33 ms | 2,4 / 5,1 / 5,7 ms |
  | `with-iframe.html` (31, zwei Bäume), `--scroll 40` | 0,11 / 0,41 / 0,89 ms | 0,08 / 0,25 / 0,47 ms | 2,2 / 5,9 / 6,7 ms |

  Keine Regression gegenüber 33 (Testshop 0,19–0,22 / 0,37–0,73 /
  0,9–1,4 ms). 0 abgelehnte Deltas, 0 Fehler.
- **Positionen im iframe und mit Zoom:** belegt über
  `relief_browsertests` (01, „Umsetzung im Fork“), alle zwölf Fälle grün.

#### Nachtrag Paket 35: echte Seiten (2026-09-25) [belegt]

Stand 35 (Positionen im iframe, Zoom), Aufbau wie oben, zusätzlich
`--use-mock-keychain`; Messknoten erst 25 s nach dem Laden. Werte
p50 / p95 / max in ms. „Alle Pakete“ enthält das Laden, „Messphase“ nur die
Pakete ab dem ersten Messknoten. Auswertung aus den Protokollzeilen
`packet`/`location` (`latency_us` = Relief-Strecke,
`unserialize_us + convert_us` = UI-Thread) und `probe`.

| Lauf (Knoten) | Relief-Strecke, alle Pakete | Relief-Strecke, Messphase | UI-Thread, Messphase | Ende zu Ende |
|---|---|---|---|---|
| Wikipedia 1 (7 950 → 7 226) | 0,18 / 2,83 / 28,4 | 0,16 / 0,29 / 0,38 | 0,10 / 0,20 / 0,29 | 10,8 / 14,1 / 14,8 |
| Wikipedia 2 (7 950 → 7 226) | 0,16 / 3,34 / 32,4 | 0,15 / 0,24 / 0,32 | 0,09 / 0,17 / 0,22 | 10,4 / 12,8 / 14,0 |
| Wikipedia, `--scroll 40` | 0,13 / 0,56 / 34,7 | 0,12 / 0,38 / 0,99 | 0,08 / 0,26 / 0,83 | 12,2 / 121,7 / 135,6 |
| spiegel.de 1 (7 808 → 627) | 0,10 / 3,41 / 21,4 | 0,10 / 0,22 / 0,54 | 0,07 / 0,16 / 0,29 | 18,1 / 21,0 / 21,3 |
| spiegel.de 2 (7 808 → 627) | 0,11 / 3,15 / 21,8 | 0,11 / 0,17 / 0,22 | 0,07 / 0,12 / 0,13 | 18,3 / 20,8 / 29,1 |
| spiegel.de, `--scroll 40` | 0,10 / 6,79 / 17,9 | 0,10 / 0,16 / 0,25 | 0,07 / 0,09 / 0,19 | 17,5 / 18,4 / 19,4 |

- **Wikipedia gegen 17** (7 950 Knoten; Ende zu Ende 9,8 / 12,4 / 13,1,
  Relief-Strecke 0,11 / 0,23 / 32): ruhige Seite gleich (Messphase p95
  0,24–0,29 ms, Ende zu Ende 10,4–10,8 / 12,8–14,1 ms). p95 über alle
  Pakete steigt auf 2,8–3,3 ms, weil die live ausgelieferte Seite ihren
  Baum beim Laden umbaut (wie im Einzellauf oben): erster Baum 7 950 Knoten
  in 23,8–24,2 ms (UI-Thread 17,8–19,0), dann ein Umbau mit 2 639 angelegten,
  4 274 geänderten und 2 619 entfernten Knoten in 28,4–32,4 ms (UI-Thread
  23,6–27,4) — das Maximum aller Läufe, gleich groß wie der Vollbaum in 17
  (32 ms). Der Einzellauf oben (46,6 ms) fing während des Ladens an.
- **spiegel.de gegen 17** (8 311–8 431 Knoten, Ende zu Ende 42–46 /
  171–192): **nicht vergleichbar.** Der Consent-Dialog (Sourcepoint) liegt
  jetzt in einem Container mit `role=dialog` und `aria-modal=true`; Blink
  nimmt dann alles außerhalb aus dem Baum. Relief sieht die Seite beim Laden
  voll (bis 7 808 Knoten, größtes Paket 5 304–6 647 angelegte Knoten in
  17,9–21,8 ms, UI-Thread 14,0–17,2), ~1,4 s später entfernt ein Paket
  7 330 Knoten (14,2–15,7 ms), danach hat der Graph 627 Knoten (Hauptbaum
  plus Consent-iframe). Ein Messknoten in `body` erscheint dann nie im
  Graphen; die Läufe hängen ihn deshalb in den Dialog
  (`fork-measure.mjs --parent '[aria-modal=true]'`). Die 150-ms-Drosselung
  aus 17 zeigt sich so nicht (Ende zu Ende p95 ≤ 21 ms), weil die
  Änderungen der Seite hinter dem Dialog nicht im Baum landen. Warum Ende
  zu Ende hier bei ~18 ms statt ~10 ms wie auf Wikipedia liegt, ist nicht
  untersucht. Die volle Seite ohne Dialog lässt sich nur nach einer
  Consent-Entscheidung messen; die war hier ausgeschlossen.
- **Positionspakete:** Wikipedia ohne Scrollen zwei beim Laden (135 und 201
  `BoundsChange`, UI-Thread 1,1–1,2 ms), danach keine. Mit `--scroll 40`
  eins je Messknoten (61 in der Messphase) mit je 206 `BoundsChange`,
  UI-Thread p50 0,10 / p95 0,26 / max 1,14 ms: Das Scrollen des
  Root-Scrollers kostet nichts, bewegt aber die fest und klebend
  positionierten Teile der Seite in Seitenkoordinaten [Annahme: das sind die
  206 Knoten]. Ende zu Ende hat dabei sieben Ausreißer von 61–136 ms, sonst
  7–35 ms. spiegel.de: drei bis vier beim Laden (bis 1 366 `BoundsChange`,
  UI-Thread höchstens 1,9 ms), in der Messphase keine; die Seite scrollt
  unter dem Consent-Dialog nicht.
- **Grenzfall aus 33 nicht nötig:** Auf echten Seiten kostet ein
  Positionspaket höchstens 1,9 ms UI-Thread (Hinweis aus dem Einzellauf oben:
  2,5–3,0 ms); die synthetische Seite aus 33 (~8 000 bewegte Knoten,
  1,5–2,1 ms) bleibt der schlechteste gemessene Fall. Kein Rückwärtsindex.
- **Fehler:** in allen Läufen 0 abgelehnte Deltas, 0 `error`, 0 `skip`,
  0 `reset`.

**Consent-iframe im Relief-Graphen** auf spiegel.de und bild.de: Frame-Baum,
Buttons und Positionen stimmen mit dem DOM überein, → 01, „Umsetzung im
Fork“, Abschnitt „Live auf echten Seiten“.

Stand je Go/No-Go-Kriterium:

| Kriterium | Messung | Stand |
|---|---|---|
| Fork baut reproduzierbar aus einem Skript; inkrementell < 5 min | `scripts/chromium-setup.sh` + `scripts/fork-apply.sh`; 9,2 s (C++) / 7,8 s (Rust) | erfüllt |
| Patches außerhalb `//relief/` ≤ ~10 Dateien, begründet | 2 Dateien, 2 Patches, 3 Zeilen (`tab_features.cc`, `tabs/BUILD.gn`) | erfüllt |
| Rust-Runtime erhält initialen Baum und Updates einer realen Seite | spiegel.de bis 8 431 Knoten, Wikipedia 7 950; 0 Fehler | erfüllt |
| Update-Latenz AXTree-Änderung → Graph < 100 ms p95, große Seite | Relief-Strecke p95 ≤ 1,1 ms (max 23 ms Vollbaum); Ende zu Ende Wikipedia p95 12,4 ms, spiegel.de p95 171–192 ms | erfüllt (Kriterium gilt für die Relief-Strecke); Ende zu Ende auf spiegel.de über 100 ms wegen Blinks 150-ms-Drosselung (s. o.) |
| `activate` löst die Aktion aus, Diff zeigt das Ergebnis | Testshop +11,7 ms, iframe +5,6 ms, spiegel.de-Consent +98 ms | erfüllt |
| VoiceOver funktioniert parallel unverändert | VoiceOver per AppleScript gesteuert, mit/ohne `--enable-relief` verglichen (Nachtrag Paket 19) | erfüllt |
| Keine Plattformweichen außerhalb der Adapter | `//relief/` ohne `#if BUILDFLAG(…)`; Linux-Build nicht gemacht | Code erfüllt, Linux-Stichprobe offen → 31 |

Einschränkung zum Meilenstein: „Interaction Graph“ ist im Fork der
`SemanticGraph` der Runtime (`relief-bridge`, `relief-model`) mit der
Prüfung `plan` und einer einfachen Namenssuche (`Runtime::find`).
`relief-interaction` (Befehle, Auflösung, Risiko) arbeitet auf dem Modell
(Paket 32), ist hinter dem Fork-Adapter aber noch nicht eingebunden (→ 24).

No-Go / Umplanen, wenn: Integrationspunkt nur mit Blink-Änderungen erreichbar;
Serialisierung der Updates dauerhaft zu teuer; Rebase auf neue Chromium-Version
kostet mehr als ~1 Tag pro Monat; oder der Prototyp gegenüber vorhandenen
Werkzeugen keinen relevanten Nutzwert zeigt. Im letzten Fall nicht automatisch
das technische Projekt beenden, sondern Zielgruppe, Aufgabe oder
Auslieferungsform (CDP-Host, Component Extension, Fork) neu bewerten.

#### Nachtrag Paket 19: VoiceOver parallel (2026-09-30) [belegt]

VoiceOver (macOS 27) per AppleScript gesteuert („VoiceOver mit AppleScript
steuern erlauben“ im VoiceOver-Dienstprogramm), Tastatur über System Events,
die gesprochene Phrase über `content of last phrase` ausgelesen. Relief-Build
mit frischem Profil und `--use-mock-keychain`, je Prüfung ein frischer Start,
einmal mit und einmal ohne `--enable-relief`; die Phrasen beider Läufe
verglichen.

| Prüfung | Ergebnis |
|---|---|
| Testshop: Überschriften (VO+Cmd+H), Links (VO+Cmd+L), Tab-Reihenfolge, Vorlesen (VO+Rechts), Rotor (VO+U) | gleich mit und ohne Relief |
| „In den Warenkorb“ per VO+Leertaste | ausgelöst; VoiceOver sagt die Live-Region „Größe 42 in den Warenkorb gelegt.“ an |
| spiegel.de-Consent-iframe: hinein navigieren, Vorlesen, Tab | Phrasen Wort für Wort gleich; nichts angeklickt |
| `--relief-log` während VoiceOver | Pakete laufen weiter, keine `error`-Zeilen |

Abweichungen zwischen den Läufen hatten andere Ursachen: Startpunkt des
VoiceOver-Cursors, eine Systemmitteilung, eine „Keine Objekte“-Meldung im
Rotor kurz nach dem Laden (in beiden Varianten).

**Speicher** (Wikipedia als Offline-Kopie, RSS 15 s nach dem Laden, je fünf
Läufe, mit Fenster): Relief kostet im Browser-Prozess rund 18 MB (403–409 →
422–427 MB), im größten Renderer keinen messbaren Unterschied (284–313 MB).
Einschränkung: VoiceOver an oder aus und selbst
`--disable-renderer-accessibility` ändern im Fensterbetrieb nichts Messbares;
der Aufschlag durch VoiceOver selbst ist damit nicht belegt. Die Summe über
alle Renderer taugt nicht als Maß (vier bis sechs Renderer je Lauf,
Erweiterung und Reserveprozesse). Vom Projektinhaber als nachrangig
eingestuft.

### Entscheidung Go (2026-09-30) [Entscheidung]

Go für Phase 1, vom Projektinhaber nach dem VoiceOver-Test entschieden. Die
Linux-Stichprobe für „keine Plattformweichen“ bleibt offen (→ 10, 31); der Code
unter `//relief/` hat keine Plattformweichen. Nutzwert-Kriterien sind
zurückgestellt (→ 90). Produktumfang über Phase 1 hinaus: → 12.

## Phase 1 · Semantic Inspector

Minimale Relief-Integration, Panel im Browser.
Pipeline: Website → AXTree → Adapter → Rust → Modell → Inspector-UI.
Zeigt: Struktur, Landmarks, Headings, Controls, Namen, Rollen, Zustände,
Aktionen, Beziehungen, Certainty/Confidence, erkannte Probleme
(`a11y-rules`). Keine KI.

## Phase 2 · Interaction Proof

Interaction Graph (→ 04), Aktionen: navigate, focus, activate, set value,
select, increment/decrement, scroll. Einfache Command-Leiste:
„list actions", „focus search", „activate cart". Deterministischer Parser, kein LLM.

## Phase 3 · Speech

STT → Intent → Runtime → Antwort/Aktion → TTS (→ 08). Austauschbare Schicht.

## Phase 4 · AI Assist

Austauschbares LLM-Interface; nur strukturierte Intents und semantische
Hypothesen, JSON-Schema/Rust-Typen (→ 05, 06). Keine direkten Browseraktionen.

## Phase 5 · Semantic View

Alternative, vom Graph erzeugte Ansicht; Originalseite bleibt funktional (→ 08).

## Aufwand (grob, eine Person) [Annahme — nach Phase 1 neu schätzen]

| Phase | Größenordnung | größter Unsicherheitsfaktor |
|---|---|---|
| 0a CDP-Spike | 1–2 Wochen | wenig; Bausteine existieren |
| 0b Machbarkeit | 2–4 Wochen | Build-Setup, Einarbeitung Chromium-a11y |
| 0c Nutzennachweis | 2–4 Wochen parallel | Rekrutierung, Wahl des ersten Anwendungsfalls |
| 1 Inspector | 3–6 Wochen | Bridge + Views/WebUI |
| 2 Interaction | 4–8 Wochen | Graph-Heuristiken, Stabilität |
| 3 Speech | 2–4 Wochen | STT ohne Google-Dienste |
| 4 AI Assist | 4–8 Wochen | Kalibrierung, Privacy-Filter |
| 5 Semantic View | 6–12 Wochen | UI-Qualität, Synchronität mit Originalseite |

Phase 0 (0a und 0b) dauerte tatsächlich eine Woche (2026-09-24 bis
2026-09-30), großenteils durch parallele Agenten. Die Schätzungen für die
Phasen 1–5 bleiben stehen, bis Phase 1 eine belastbare Vergleichsgröße liefert.

Dauerkosten: Upstream-Rebase und Sicherheitsupdates, laufend.

## Technische MVP-Kriterien

1. Chromium auf macOS ARM reproduzierbar gebaut.
2. Rust-Core live mit Chromium-Accessibility-Daten versorgt.
3. Dynamische AXTree-Änderungen verarbeitet.
4. Stabiler Interaction Graph.
5. Aktionen kontrolliert von Rust an Chromium zurück.
6. Navigation und grundlegende Formulare ohne Maus bedienbar.
7. Sprache kann die semantische Struktur abfragen.
8. Mindestens eine reale, komplexere Website erfolgreich bedienbar.
9. Unsicherheit wird explizit dargestellt.
10. OS-Accessibility funktioniert weiter.

## Produktkriterien des ersten MVP

1. Der MVP hat eine primäre Nutzergruppe und ein primäres Problem; weitere
   Capability-Profile bleiben kompatibles Zukunftsziel.
2. Repräsentative Aufgaben werden gegen eine dokumentierte Baseline verglichen,
   nicht nur innerhalb von Relief getestet.
3. Kritische Fehlaktionen und unbemerkte falsche Inferenz: 0 in den
   MVP-Testaufgaben.
4. Abbruch, Unsicherheit und Rückfrage sind jederzeit möglich; Nutzer behalten
   Kontrolle und können zur Originalseite bzw. zur vorhandenen Assistenztechnik
   zurückkehren.
5. Mindestens ein wiederkehrender Anwendungsfall zeigt einen klaren Vorteil bei
   Task Completion oder notwendigem Hilfebedarf.
6. Der Nutzen ist ohne Cloud-KI erreichbar; Modellstufen dürfen ihn verbessern,
   aber nicht erst erzeugen.

## Nicht in Version 1 [Entscheidung]

Keine eigene Speech Engine · keine eigene LLM-Inferenz · keine
Braille-Treiber · keine OS-Accessibility-API ersetzen · keine Änderung an der
Rendering-Engine · keine vollständige automatische Reparatur · kein
WCAG-Konformitätsversprechen · VoiceOver/NVDA nicht ersetzen · keine
Spezialhardware direkt · **kein Fork mit hunderten Änderungen quer durch Blink**.

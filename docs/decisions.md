# Entscheidungen

Aktuell gültige Grundsatzentscheidungen. Details und Begründungen in
`plan/spezifikation/`.

**Chromium-AXTree ist die primäre semantische Quelle**
Relief liest den plattformunabhängigen Accessibility-Tree im
Browser-Prozess, statt einen eigenen DOM-basierten Baum zu bauen.
*Grund:* Chromium berechnet Rollen, Namen, Zustände, Aktionen und Bounds
bereits; ein Parallelbaum würde das duplizieren und auseinanderlaufen.
*Konsequenz:* Integration im Browser-Prozess über den AX-Datenstrom;
DOM/Layout nur als Zusatzkontext bei Bedarf.

**Eigener Baum je Frame aus dem Datenstrom (Weg B), nicht am `BrowserAccessibilityManager`**
Relief beobachtet `WebContentsObserver::AccessibilityEventReceived`, baut je
Tree-ID einen eigenen `ui::AXTree` und leitet daraus die Deltas ab; der
Modus kommt per `ScopedAccessibilityMode` (`kAXModeWebContentsOnly`).
*Grund:* Der Manager existiert nur mit `kNativeAPIs`; ein Beobachter dort
würde für jeden Relief-Tab die OS-Brücke einschalten. Weg B nutzt nur
öffentliche API und ist der Weg von Reading Mode und `chrome.automation`;
im Fork gemessen: UI-Thread je Paket p95 < 0,4 ms, 0 Fehler auf großen
Seiten.
*Konsequenz:* Läuft ein Screenreader, hält Chromium den Baum zweimal
(Speicher, ein zweites `Unserialize`). Einhängen je Tab mit einer Zeile in
`tab_features.cc`. Details in `plan/spezifikation/01` („Umsetzung im Fork“).

**Relief fordert `kScreenReader` nicht an**
*Grund:* `kScreenReader` ist ein Screenreader-Modus; die Knotenzahl ändert
sich auf den gemessenen großen Seiten damit kaum (Wikipedia gleich,
spiegel.de +1). Für Vergleiche mit dem CDP-Spike gibt es
`--relief-screen-reader-mode`.
*Konsequenz:* Display-gesperrte Inhalte (`content-visibility`) fehlen im
Fork-Graphen, anders als im CDP-Spike.

**Bridge: `cxx` im Browser-Prozess, Runtime auf eigener Sequenz; Utility-Prozess später**
Der C++-Adapter füllt im AX-Callback nur die `cxx`-Strukturen; die
Rust-Runtime läuft per `base::SequenceBound` auf einer eigenen
ThreadPool-Sequenz. Mojo-Variante (Utility-Prozess) erst, wenn Code mit
Absturz- oder Missbrauchsrisiko in die Runtime kommt.
*Grund:* Die Grenze kostet gemessen p95 < 0,3 ms je Paket; Serialisierung
entscheidet nichts, Crash-Isolation wird erst mit KI-/Heuristik-Code
wichtig.
*Konsequenz:* Eine Panik in der Runtime beendet den Browser
(`panic=abort`); die Rust-Seite ist so gebaut, dass Eingaben aus C++ sie
nicht auslösen. Details in `plan/spezifikation/02`.

**Rust-Crates im Fork: Quellen kopieren, feste Crate-Namen**
`scripts/fork-apply.sh` kopiert die Quellen der Crates, die der Fork baut,
nach `//relief/crates/`; `//relief/BUILD.gn` baut sie mit
`rust_static_library` und festem `crate_name`. `a11y-perception` bleibt im
Fork weg (Cargo-Feature `perception` in `relief-model`).
*Grund:* Der Cargo-Workspace bleibt Quelle der Wahrheit, kein Eingriff in
`//third_party/rust`; `use relief_model::…` bleibt in beiden Builds gleich.

**Aktionen im Fork über `AXActionData`, Tasten als Ersatzweg; eigene Position statt Fokus-Trick**
Der Fork führt geprüfte Pläne als `AXActionData` aus; wo das nicht reicht
(Escape, Erhöhen/Verringern), sendet er echte Tastaturereignisse an das
fokussierte Widget. Überschriften und Bereiche werden nicht fokussiert,
sondern sichtbar gemacht und als Startpunkt der Tab-Reihenfolge gesetzt;
die Sitzung merkt sich dort ihre Position, bis der Fokus sich bewegt.
*Grund:* Kein DOM-Eingriff (`tabindex`) und kein experimentelles
Blink-Feature; `Increment` über AX erreicht ARIA-Widgets nur mit
`SynthesizedKeyboardEventsForAccessibilityActions` und verfehlt damit
Zahlenfelder.
*Konsequenz:* Der Befehlsablauf (`relief_interaction::session`) ist für
CDP-Host und Fork derselbe; Tabelle und Befunde in
`plan/spezifikation/05` („Im Fork über `AXActionData`“).

**Blink bleibt unverändert, Fork minimal**
*Grund:* Wartbarkeit gegenüber einem schnelllebigen Upstream.
*Konsequenz:* eigener Code in `//relief/`, jeder Patch außerhalb ist einzeln
dokumentiert und begründet.

**Fork als Patch-Serie auf Stable-Tags**
Eingriffe außerhalb `//relief/` liegen als `git format-patch`-Dateien mit
Reihenfolgedatei in `fork/patches/`, eigener Code als Verzeichnis
`fork/relief/`; `scripts/fork-apply.sh` wendet beides mit `git am --3way` auf
einen Checkout an, `scripts/fork-export.sh` schreibt zurück. Kein
Override-Verzeichnis wie Braves `chromium_src/`.
*Grund:* Gemessen über fünf Milestones 149→154 ändern sich die gepatchten
Dateien in jedem Milestone (`tab_features.h/.cc` und `tabs/BUILD.gn`
zusammen 17–49 Commits je Milestone). Die nachgebauten Patches passen mit
stabilen Ankern trotzdem in 23 von 25 Fällen unverändert und in 2 per
3-Wege-Merge, ohne Handkonflikt (Anker am Listenende: 7 Konflikte in 35
Fällen). Bei zwei bis vier Eingriffen lohnt ein Override-Mechanismus nicht,
der selbst einen Build-Patch braucht.
*Konsequenz:* Patches folgen den Ankerregeln in
`plan/spezifikation/01-chromium-integration.md` („Fork-Strategie“); kein
DEPS-Patch, Side-Panel-Eintrag ohne eigene Action-ID; Format in
`fork/README.md`.

**Nativer Accessibility-Pfad bleibt erhalten**
*Konsequenz:* Relief zweigt vor dem `BrowserAccessibilityManager` ab und
setzt `kNativeAPIs` nicht; VoiceOver und andere Screenreader sollen
unverändert funktionieren; mit VoiceOver per AppleScript geprüft
(`plan/spezifikation/09`, „Nachtrag Paket 19“).

**Rust-Core ist browserfrei**
Kein Chromium- oder CDP-Typ in der Core-API; nur der Adapter übersetzt.
*Grund:* derselbe Core soll in WASM, CLI, CI und anderen Hosts laufen;
entspricht der Regel „browserfrei bis L3" in barrierlab.

**Eigenes typisiertes Datenmodell mit Delta-Format (`relief-model`)**
Der Core arbeitet auf einem eigenen Modell (`SemanticGraph` aus je einem
Baum pro Frame und Dokument, `SemanticNode`, `Fact<T>`, `TreeDelta`), nicht
auf dem CDP-förmigen `a11y_perception::AXNode`.
*Grund:* Chromium liefert je Frame und Dokument einen eigenen Baum mit
eigener Tree-ID, Integer-IDs nur je Baum eindeutig, iframes als Child-Trees,
inkrementelle `AXTreeUpdate`s, Enums statt Strings, Fokus in den Baumdaten
und Positionen über einen eigenen Kanal. Das CDP-Format hat einen einzigen
Baum mit String-IDs und kennt nur Vollbäume; ein Fork-Adapter dahin würde
Typisierung und Inkrementalität verwerfen.
*Konsequenz:* Identität ist (Tree-ID, Node-ID) und gilt nur innerhalb eines
Dokuments. `a11y-perception` bleibt Quelle für CDP-Host und Aufnahmen, über
einen Konverter. Details in `plan/spezifikation/03-semantisches-datenmodell.md`.

**Jede Aussage trägt ihre Herkunft: Known / Inferred / Uncertain**
*Konsequenz:* Erschlossenes wird nie als Tatsache ausgegeben; Uncertain ist nie
Grundlage einer Aktion ohne Rückfrage.

**KI ist Fallback, nicht Wahrheitsquelle**
Quellenpriorität: HTML → ARIA → AXTree → deterministische Analyse →
DOM/Layout → KI → Vision. Ist Semantik vorhanden, läuft kein Modell.

**Ein Modell führt keine Browseraktion aus**
Modelle erzeugen nur strukturierte Intents und Hypothesen; die Rust-Runtime
validiert, stuft das Risiko ein (LOW / MEDIUM / HIGH) und führt aus.
HIGH-Aktionen verlangen explizite Bestätigung; irreversible Aktionen nie
allein auf Basis einer Inferenz. Eine Bestätigung gilt einmal, kurz und nur
für den angezeigten Plan (Aktion, Wert, Ziel, Graph-Stand, Zieladresse);
eine pauschale Zustimmung gibt es nicht, auch nicht aus Cache oder Profil.

**Modelle in Stufen, Default ohne**
Stufen `none` → `os` → `local` → `api` (eigener Key), vom Nutzer gewählt,
anbieteroffen über eine Provider-Schnittstelle.
*Grund:* Ein lokales Modell, das gute Hardware braucht, wäre selbst eine
Barriere.
*Konsequenz:* Jede Funktion muss mit `none` funktionieren; KI verbessert nur
Fälle mit fehlender Semantik.

**Modelleingaben nur über den Privacy-Filter, erzwungen durch Typen**
Ein `ModelProvider` nimmt nur eine `FilteredInput` entgegen, die allein der
Filter der Runtime erzeugt; er kopiert nach Positivliste (keine Werte, kein
Feldinhalt, sensible Felder nur mit Rolle). Anbieter liefern nur Text, den
die Runtime gegen ein Schema und die Eingabe prüft.
*Grund:* Konvention reicht nicht, sobald mehrere Anbieter-Adapter existieren.
*Konsequenz:* Details in `plan/spezifikation/06` („Vertrag“) und `07`
(„Privacy-Filter“), Code in `crates/relief-ai-contract`.

**Modellaussagen werden erst mit gemessener Schwelle `Inferred`**
Eine Hypothese ist `Uncertain`, bis für genau dieses Modell eine Schwelle
aus einem Kalibrierlauf eingetragen ist (`CALIBRATED_THRESHOLDS`). Resolver-
Anfragen enthalten nur einen Ausschnitt um den Knoten, Netzcode für Anbieter
steht hinter einem Cargo-Feature.
*Grund:* Von Modellen genannte Confidence ist keine Wahrscheinlichkeit und
zwischen Modellen nicht vergleichbar; ohne Feature baut Relief ohne
Modell-Netzcode.
*Konsequenz:* Kalibrierwerkzeug und Stichprobe in `crates/relief-resolver`
und `spike/kalibrierung`, Stand in `plan/spezifikation/06`.

**Fähigkeiten statt Diagnosen**
Nutzerprofile beschreiben Fähigkeiten (Sehdetail, Farbunterscheidung,
Eingabewege …), keine Behinderungsmodi; Kombinationen sind frei.

**Erst die technische Lösung, der Nutzennachweis wird nachgeholt**
Phase 0 wird technisch entschieden; der Fork darf als Forschungsbuild entstehen.
Der Nutzennachweis mit Betroffenen (Protokoll in
`plan/spezifikation/11-nutzerstudie.md`) folgt, sobald Kontakte bestehen, und
ist Voraussetzung, bevor Relief an Dritte ausgeliefert wird.
*Grund:* Es gibt noch keine Kontakte zu Teilnehmenden; der technische Weg ist
unabhängig davon klärbar.
*Konsequenz:* Go/No-Go Phase 0 prüft nur die technischen Kriterien.

**Erste Zielgruppe: ohne präzise Zeigerbedienung**
Der erste Nutzennachweis richtet sich an Menschen, die das Web ohne Maus oder
nur mit eingeschränkter Zeigerbedienung nutzen; Aufgaben: Formulare, Menüs,
Consent-Dialoge.
*Grund:* trifft, was der Prototyp schon kann (benannte Aktionen, Rückfrage,
Bestätigung); Nutzen ist ohne Sprache und KI beobachtbar.
*Konsequenz:* Der Prototyp ist tastaturbedienbar im Browser (Befehlsleiste),
Protokoll in `plan/spezifikation/11-nutzerstudie.md`. Andere Profile bleiben
Ziel, sind aber nicht Teil des ersten MVP.

**Latenz wird für die Relief-Strecke bewertet**
Das Go/No-Go-Kriterium „< 100 ms p95“ gilt für die Strecke vom AX-Paket im
Browser-Prozess bis zum aktualisierten Graph (gemessen p95 ≤ 1,1 ms).
*Grund:* Ende zu Ende dominiert Blinks Drosselung nicht-interaktiver
Änderungen (höchstens alle 150 ms); sie ist ohne Blink-Eingriff nicht
beeinflussbar, und Nutzeraktionen sowie Fokuswechsel serialisiert Blink
ohnehin sofort.
*Konsequenz:* Ende-zu-Ende-Latenz wird weiter gemessen und berichtet, aber
nicht als Ausschlusskriterium; ein Blink-Patch dafür ist nicht vorgesehen.

**Lizenz: MIT**
Eigener Code (`//relief/`, Rust-Crates) steht unter MIT; Chromium-Code behält
BSD-3-Clause und die Drittlizenzen in `third_party/`.
*Grund:* gleiche Lizenz wie barrierlab; Crates können ohne Lizenzwechsel
zwischen den Repos wandern.

**Relief ersetzt den barrierlab-Reader; zwei Linien über einem Kern**
Relief hat eine Linie Assistenz für Endnutzer und eine Linie Prüfen im
echten Browser (Bedienablauf aus Sicht von Screenreader- und
Tastaturnutzenden). Der in barrierlab geplante Reader-Host entfällt.
Linie B ist ein Prüfmodus desselben Kerns, kein zweites Endnutzerprodukt und
keine eigenständige Browser-Roadmap.
*Grund:* zwei Ansätze für dieselbe Prüfaufgabe; Relief hat den nativen
AX-Baum im eigenen Browser und echte Screenreader-Ausgabe daneben.
*Konsequenz:* Browserfreie Bausteine, die ein weiteres Werkzeug braucht,
wandern nach barrierlab („zwei Konsumenten, dann Bibliothek“); Relief
ersetzt dort den Reader-Host als Konsument. Kein
WCAG-Konformitätsversprechen. Details in `plan/spezifikation/12`.

**Reihenfolge: erst Machbarkeit, dann KI**
Phase 0a (CDP-Spike gegen normales Chrome, ohne Fork) → Phase 0b (Machbarkeit Fork) → Inspector → Interaction Graph → Sprache → KI →
Semantic View. Harter Go/No-Go-Meilenstein: AXTree → Rust → Interaction
Graph → `activate` zurück an Chromium mit einer echten Webseite; erreicht,
Go für Phase 1 am 2026-09-30.

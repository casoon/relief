# 13 · Accessibility der Relief-Oberfläche

Relief bewertet und ergänzt Webseiten, ist aber selbst ein User Agent. Deshalb
gelten die folgenden Regeln für Inspector, Befehlsleiste, Semantic View,
Bestätigungen, Einstellungen und Update-/Fehlerdialoge. Die Regeln sind eine
gemeinsame Abnahmebasis, kein nachträglicher Audit.

## Bedienung und Fokus [Entscheidung]

- Jede Kernaufgabe ist vollständig per Tastatur und mit der jeweiligen
  Plattform-Assistenztechnik möglich; es gibt keine Tastaturfalle.
- Fokus ist immer erkennbar. Auswahl, Fokus und Aktivierung sind getrennte
  Zustände: Navigation löst keine Aktion aus.
- Öffnen und Schließen einer Relief-Oberfläche stellt den Fokus am Auslöser
  oder am zuletzt sinnvollen Ort wieder her. Ein modaler Dialog isoliert
  Hintergrundinhalte und besitzt einen eindeutigen Schließweg.
- Plattformkonventionen für Tasten, Menüs und Kurzbefehle haben Vorrang vor
  erfundenen Relief-Mustern; Konflikte mit Browser, Seite und Assistenztechnik
  werden getestet.

## Orientierung, Status und Unterbrechungen [Entscheidung]

- Oberfläche und Assistenz nennen aktuellen Ort, aktiven Modus und relevante
  Auswirkung einer Aktion. Lange Arbeit zeigt Fortschritt und lässt sich
  abbrechen.
- Dynamische Statusmeldungen kommen rechtzeitig, aber nicht störend. Häufige
  Deltas werden gebündelt; nicht kritische Ausgabe lässt sich pausieren oder
  stummschalten.
- Ein Ansichts- oder Fähigkeitswechsel erhält Aufgabe und Ort und verändert
  die Website nicht. Ist der vorige Knoten verschwunden, erklärt Relief den
  neuen Ort.

## Fehler, Bestätigung und Wiederherstellung [Entscheidung]

- Fehler erklären Wirkung und nächsten Schritt. Nach einem Formularfehler ist
  das erste fehlerhafte Feld erreichbar; zurück zum vorherigen Ort bleibt
  möglich.
- Texteingaben und reversible Einstellungen lassen sich vor einer endgültigen
  Aktion korrigieren oder zurücksetzen. Relief speichert Formulardaten nicht
  zusätzlich zur Seite, solange dafür kein eigener, bewusster Produktentscheid
  getroffen wurde.
- Eine Bestätigung zeigt die Parameter des validierten `ActionPlan` und gilt
  nur für genau diesen Plan (→ 05). Kritische Freigaben sind nie pauschal.

## Semantik und Darstellung [Entscheidung]

- Native Controls werden bevorzugt. Sonst sind Name, Rolle, Wert, Zustand,
  Beziehungen und Zustandsereignisse vollständig über VoiceOver, UI
  Automation beziehungsweise AT-SPI verfügbar.
- Relief respektiert Textskalierung/Zoom, hohen Kontrast, reduzierte Bewegung
  und weitere relevante Systemeinstellungen. Bedienung hängt nicht allein von
  Farbe, Zeigegerät oder präziser Bewegung ab.
- Fähigkeitsprofile ändern Darstellung und Ausgabe, aber entfernen weder
  Bedeutung noch Aufgaben. Jede Anpassung ist reversibel; globaler und
  websitespezifischer Geltungsbereich bleiben sichtbar.

## Verbindliche Testmatrix

Jede Kernoberfläche erhält automatisierte Semantik- und Fokusprüfungen sowie
manuelle Aufgaben. Eine Plattform gilt nicht allein durch einen erfolgreichen
Build als unterstützt.

| Plattform | Mindestens prüfen |
|---|---|
| macOS | Tastatur, VoiceOver, Voice Control, Zoom/Textgröße, Kontrast, reduzierte Bewegung; Switch Control für Kernaufgaben |
| Windows | Tastatur, UI Automation, Narrator und NVDA, Voice Access, Textskalierung und hohe Kontraste |
| Linux | Tastatur, AT-SPI/Orca und Darstellungseinstellungen, sobald ein Testhost vorhanden ist |

Ein Testfall hält Betriebssystem-, Relief-/Chromium- und AT-Version,
Einstellungen, Aufgabe, Eingabekanal und Ergebnis fest. Ausnahmen brauchen
Verantwortliche, Begründung und Ablaufdatum (→ „Ausnahmen“ unten).

## Automatisierte Prüfungen (Paket 47) [umgesetzt, belegt]

`fork/relief/testing/ui_baseline.{h,cc}` prüft eine Relief-Oberfläche, die
als WebContents vorliegt (heute: die WebUI im Side Panel), und liefert je
Prüfung die Abweichungen als Text:

| Prüfung | Was |
|---|---|
| `NamesAndRoles` | jedes nicht ausgeblendete Bedienelement im Accessibility-Tree hat einen Namen; kein Element ohne Bedienrolle bietet einen Klick an |
| `LiveRegions` | genannte Elemente sind Live-Regionen (`status`, `log`, `alert`, `aria-live`) |
| `KeyboardWalk` | echte Tab-Tasten ab Dokumentanfang: erwartete Elemente in dieser Reihenfolge, keine Tastaturfalle, Fokus verlässt die Oberfläche, jedes Element mit sichtbarem Umriss ≥ 2 px |
| `Reflow` | kein waagerechtes Scrollen, kein sichtbares Element ragt über den Viewport |
| `NoMotion` | keine Übergänge oder Animationen |

Dazu `CdpClient` und `EmulateMedia` (Medienmerkmale über CDP emulieren).
Der Snapshot wartet, bis der Baum nach dem Einschalten des AX-Modus da ist;
ein leerer Baum wäre sonst eine bestandene Prüfung.

- **Belegt** (M4, 2026-10-11): `relief_browsertests --gtest_filter=*UiBaseline*`.
  `UiBaseline` wendet die Prüfungen auf jeden Zustand des Panels an:
  Befehle und Inspector; Rückfrage in der Befehlsleiste (Zustand angesagt,
  Escape bricht ab, Fokus bleibt im Feld, auf der Seite nichts ausgelöst);
  Fähigkeiten offen; 200 % Zoom (Inspector mit Fähigkeiten, Semantic View);
  Semantic View (Fokus wandert hinein, Rückfrage bekommt den Fokus,
  Abbrechen gibt ihn an den Auslöser zurück); reduzierte Bewegung mit
  erzwungenen Farben (nichts bewegt sich, Fokus sichtbar); 30 Änderungen
  der Seite in 1,5 s ergeben höchstens zwei Ansagen in 4,5 s, mit
  ausgeschalteter Ansage keine. `UiBaselineGegenprobe` belegt auf
  `data/baseline-kaputt.html`, dass jede Prüfung anschlägt.
- **Grenzen:** Kontrast und Bewegung emuliert (`forced-colors`,
  `prefers-reduced-motion`), nicht über die Systemeinstellung; 200 % über
  den Zoom des Panels, nicht über die Textgröße des Systems. Native Views
  (Sprungmarken-Overlay) und die tatsächliche Ausgabe von VoiceOver prüfen
  die Funktionen nicht (→ manuelles Protokoll, Paket 43).

## Abnahme neuer Oberflächen [Entscheidung, Paket 47]

Eine neue Relief-Oberfläche oder ein neuer Zustand einer bestehenden ist
erst fertig, wenn `UiBaseline` (oder ein eigener Browser-Test mit
denselben Prüfungen aus `ui_baseline.h`) ihn abdeckt; das gehört in das
„Fertig, wenn“ des Pakets.

## Manuelles Testprotokoll macOS [Vorlage; Durchgang → Paket 146]

Kernaufgaben:

| Nr | Aufgabe |
|---|---|
| K1 | Panel öffnen und schließen (Strg+Umschalt+I); Fokus danach wieder auf der Seite |
| K2 | Befehl ausführen (Strg+Umschalt+Leertaste); Rückfrage bestätigen und abbrechen |
| K3 | Inspector: Liste durchgehen, Details lesen, „Im Dokument zeigen“; Ansage bei Seitenänderung, mit „Änderungen ansagen“ aus keine |
| K4 | Semantic View: wechseln, Element bedienen, Rückfrage, zurück zum Inspector am selben Ort |
| K5 | Fähigkeiten: Wert ändern, nur für diese Website, zurücksetzen |
| K6 | Sprungmarken (Strg+Umschalt+M): lesbar auf heller und dunkler Seite, bei 200 % |
| K7 | Sprechtaste (Strg+Umschalt+S): sprechen, abbrechen |

Eingabekanäle und Einstellungen je Aufgabe: nur Tastatur; VoiceOver
(Kürzelkonflikte VO-Tasten Strg+Wahl gegen Relief Strg+Umschalt; Ansage
von Zustand, Log und Rückfrage; Fokus nach Schließen); Sprachsteuerung
(„Klicke …“, Nummern einblenden); Schaltersteuerung (mindestens K1, K2);
„Größerer Text“ und Browserzoom 200 %; „Kontrast erhöhen“; „Bewegung
reduzieren“.

Ergebnis je Zeile:

| Datum | macOS | Relief/Chromium | AT und Version | Einstellungen | Aufgabe | Eingabekanal | Ergebnis | Befund |
|---|---|---|---|---|---|---|---|---|

## Ausnahmen [Entscheidung, Paket 47]

Eine Abweichung von dieser Baseline gilt nur mit Eintrag hier:
Verantwortliche Person, Begründung und Ablaufdatum; abgelaufene Einträge
sind Fehler.

| Abweichung | Verantwortlich | Begründung | Ablauf |
|---|---|---|---|
| keine | | | |

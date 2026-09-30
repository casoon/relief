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
Verantwortliche, Begründung und Ablaufdatum. Details der Umsetzung: → 47.

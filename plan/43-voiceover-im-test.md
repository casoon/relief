# 43 · Echte Screenreader-Ausgabe im Test

**Umgebung:** M4 · **Phase:** Linie B · **Abhängig von:** 42

## Ziel

Tests halten über einen plattformneutralen Treiber fest, was ein echter
Screenreader tatsächlich sagt, und vergleichen es mit der Erwartung. Der erste
Adapter steuert VoiceOver; NVDA folgt mit dem Windows-Nachweis (→ 31; siehe
[spezifikation/12](spezifikation/12-produktumfang.md#linie-b-prüfen-im-echten-browser)).

## Kontext

Verfahren im VoiceOver-Test aus 19 erprobt (→ 09, „Nachtrag Paket 19“):
AppleScript-Steuerung von VoiceOver, Tasten über System Events,
`content of last phrase`. Voraussetzung: Option „VoiceOver mit AppleScript
steuern erlauben“; die ersten Befehle nach dem Einschalten liefen ins Leere.

## Schritte

1. Browserfreies Treiber-Interface mit „starten“, „stoppen“, Konfiguration,
   OS-nativer Tasteneingabe, Ausgabe-Ereignissen und „warten bis ruhig“;
   virtueller Fokus und Modus werden erfasst, soweit der Screenreader sie
   anbietet. Adapter enthalten allein Plattform- und Screenreader-Details.
2. VoiceOver-Adapter aus dem erprobten AppleScript-Verfahren; Zustand von
   VoiceOver verlässlich erkennen.
3. Navigationsprofile: Tab-Navigation,
   sequenzielles Lesen und Schnellnavigation mindestens zu Überschriften und
   Formularfeldern. Ein Aufgabenfall nennt ausdrücklich das verwendete Profil.
4. Zusicherung `ansage:` im Aufgabenformat; Vergleich nach Bestandteilen
   (fehlend, überzählig, Reihenfolge), nicht nach exaktem Wortlaut.
5. Mit und ohne Relief laufen lassen (Regressionsschutz für den Screenreader).
6. Laufmetadaten festhalten: Betriebssystem, Browser-/Relief-Version,
   Screenreader-Version, relevante Einstellungen und Navigationsprofil.

## Fertig, wenn

- `spike/tasks/01-shop-clean.txt` und `03-form.txt` laufen mit VoiceOver und
  prüfen mindestens eine Ansage je Aufgabe; mindestens Tab- und
  Überschriftennavigation verwenden dasselbe Treiber-Interface.

## Nicht Teil

NVDA/JAWS unter Windows implementieren (→ 31); das Treiber-Interface muss sie
ohne Änderung am Aufgabenformat aufnehmen können.

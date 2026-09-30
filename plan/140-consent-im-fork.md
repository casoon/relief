# 140 · Consent: `09-consent.txt` mit Zweck-Titeln im Fork

**Umgebung:** M4 · **Phase:** Linie A · **Abhängig von:** 110 ✓

## Ziel

Die Consent-Aufgaben der Pakete 91 und 110 laufen im eigenen Build so wie
über CDP (→
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-pakete-40-80-91-110-belegt)).

## Kontext

- Über CDP belegt (2026-09-30): `09-consent.txt` 50/50, darin
  `consent-zwecke.html` (Buttons je Zweck, Rückfrage mit Zweck aus der
  Überschrift) und `consent-zweck-titel.html` (aufklappbare Zweck-Titel,
  Rückfrage mit Zweck aus dem Titel, Wahl über den Zweck).
- Zweck-Titel hängen an `Control::expandable`, also an `expanded: false`
  für zugeklappte Buttons. Der Fork setzt das aus `State::kCollapsed`
  (`fork/relief/bridge/ax_tree_mirror.cc`) [belegt im Code, im Fork nicht
  gelaufen].
- Im Fork zuletzt gelaufen: `09-consent.txt` mit Paket 80 (118/118 mit
  01–05, 07, 14); mit 91 und 110 nicht.

## Schritte

1. Relief-Build mit dem Stand von `main` (keine C++-Änderung durch 110).
2. `scripts/fork-run-tasks.sh spike/tasks/09-consent.txt` (gern mit 01–05,
   07, 14).
3. Optional, Netz: bild.de im eigenen Build, „cookie-einstellungen öffnen“,
   „welcher Dialog ist offen“ (Zweck-Titel gezählt, Zustimmen nur „Alle
   akzeptieren“), „klicke Ablehnen“ (Zweck je Zeile), „abbrechen“. Nichts
   zustimmen, keine Auswahl speichern.

## Fertig, wenn

- `09-consent.txt` im Fork ohne Ausfall; Ergebnis in spezifikation/05
  („Im Fork“).

## Nicht Teil

- Änderungen an der Erkennung; Einwilligen, Auswahl speichern, Umgehen von
  Bot-Erkennung oder Bezahlschranken.

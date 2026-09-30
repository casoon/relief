# 110 · Consent: Zweck-Titel und Rückfrage je Zweck

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 91 ✓

## Ziel

Offene Punkte aus 91 (→
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-pakete-40-80-91-belegt)).

## Kontext

Belegt (Paket 91, CDP-Host, 2026-09-30): Zustimmen und Ablehnen je Zweck
werden auf der zweiten Ebene zusammengefasst („vermutlich je Zweck
„Zustimmen“ 6-mal, „Ablehnen“ 3-mal“) und nie selbst gewählt. Offen:

- **Zweck-Titel als Buttons:** bild.de benennt die aufklappbaren Titel der
  Zwecke „… Required For Consent“; über „consent“ gelten sie als Zustimmen
  und stehen einzeln in der Ansage. faz.net: Titel wie „Verwendung
  reduzierter Daten zur Auswahl von Werbeanzeigen“ gelten über „auswahl“
  als Einstellungen; „cookie-einstellungen öffnen“ fragt dort nummeriert
  zwischen ihnen nach (nichts geklickt).
- **Rückfrage je Zweck:** „klicke Ablehnen“ auf der zweiten Ebene fragt
  nummeriert nach, aber jede Zeile heißt „[button] Ablehnen“; der Zweck
  (Überschrift davor, auf spiegel.de vorhanden, auf bild.de nicht) fehlt.
- **Fork:** `09-consent.txt` mit `consent-zwecke.html` im eigenen Build
  nicht gelaufen (M4).

## Schritte

1. Zweck-Titel erkennen (Aufklapp-Zustand, Position vor den Buttons je
   Zweck, „required for consent“/„Zustimmung erforderlich“) und aus
   Zustimmen und Einstellungen herausnehmen.
2. In der nummerierten Rückfrage zu Buttons je Zweck den Zweck nennen.
3. M4: `scripts/fork-run-tasks.sh` mit `09-consent.txt`.

## Fertig, wenn

- Die zweite Ebene auf bild.de nennt keine Zweck-Titel als Zustimmen, und
  `12-consent-real.txt` läuft ohne verfehlte Erwartung; `09-consent.txt`
  im Fork ohne Ausfall.

## Nicht Teil

- Einwilligen, Zwecke einzeln setzen ohne ausdrücklichen Befehl, Umgehen
  von Bot-Erkennung oder Bezahlschranken.

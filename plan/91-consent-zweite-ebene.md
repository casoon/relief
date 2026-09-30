# 91 · Consent: zweite Ebene auf echten Seiten, Nachweis im Fork

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 80 ✓

## Ziel

Offene Punkte aus 80 (→
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-pakete-40-80-belegt)).

## Kontext

Belegt (Paket 80, CDP-Host, 2026-09-30): „cookie-einstellungen öffnen“
öffnet auf spiegel.de, bild.de und faz.net die zweite Ebene. Dort stehen
Buttons **je Zweck**: spiegel.de dreimal „Ablehnen“ und sechsmal
„Zustimmen“, bild.de „Einwilligen“ je Zweck, faz.net Zweck-Buttons wie
„Verwendung … zur Auswahl von Werbeanzeigen“, die über „auswahl“ als
Einstellungen gelten. „cookies ablehnen“ fragt dann nummeriert nach oder
sagt „Kein Ablehnen“; geklickt wird nichts. heise.de hat zwei
„Einstellungen“-Buttons (nummerierte Rückfrage).

## Schritte

1. **Zweck-Buttons** auf der zweiten Ebene: entscheiden, ob sie als eigene
   Art gelten (nicht Ablehnen des Ganzen, nicht Einstellungen) und wie die
   Ansage sie zusammenfasst („3 Zwecke: je Ablehnen/Zustimmen“). Ein
   Ablehnen je Zweck wählt Relief weiterhin nicht von selbst.
2. **Doppelte Buttons** (heise.de: zwei „Einstellungen“ mit verschiedenen
   DOM-Knoten) prüfen: gleicher Name, gleiche Wirkung → eine Rückfrage
   weniger, oder bewusst nachfragen.
3. **Fork (M4):** `scripts/fork-run-tasks.sh` mit `09-consent.txt`
   (inklusive `consent-seite.html` und `consent-einstellungen.html`) nach
   dem nächsten Build ohne Ausfall; golem.de im eigenen Build als
   Cookie-Hinweis.

## Fertig, wenn

- Die zweite Ebene auf spiegel.de wird ohne Zweck-Liste als Einzelbuttons
  angesagt, und `12-consent-real.txt` läuft ohne verfehlte Erwartung.
- `09-consent.txt` im Fork ohne Ausfall.

## Nicht Teil

- Einwilligen, Zwecke einzeln setzen ohne ausdrücklichen Befehl, Umgehen
  von Bot-Erkennung oder Bezahlschranken.

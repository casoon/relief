# 40 · Overlay- und Consent-Dialoge

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 24

## Ziel

Consent-, Newsletter- und andere Overlays erkennen, ansagen und auf Wunsch
ablehnen oder schließen (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-a-assistenz-im-browser)).

## Regeln

- Relief stimmt nie selbst zu. Ablehnen nur auf ausdrücklichen Befehl;
  gibt es kein Ablehnen ohne Bezahlung (z. B. spiegel.de „pur“), wird das
  angesagt, nicht umgangen.
- Keine Umgehung von Bot-Erkennung oder Bezahlschranken.
- Ein modaler Dialog begrenzt weiterhin alle Aktionen auf seinen erreichbaren
  Inhalt. Eine reine Auskunft über den Hintergrund darf diese Grenze nicht in
  eine Bedienmöglichkeit verwandeln.

## Schritte

1. Erkennung auf Basis von Modalität je Frame und Schließen-Ziel (→ 04, 05),
   auch in Consent-iframes (→ 01, „Live auf echten Seiten“).
2. Ansage „Cookie-Dialog: Zustimmen, Einstellungen, Abo; kein Ablehnen“.
3. Soweit der Chromium-AX-Datenstrom ihn noch enthält, kann Seiteninhalt hinter
   dem Dialog auf ausdrückliche Anfrage beschrieben werden (offener Punkt aus
   35); Auflösen von Aktionszielen und Ausführen bleiben auf den Dialog
   beschränkt.

## Fertig, wenn

- Auf spiegel.de, bild.de und drei weiteren Seiten wird der Dialog erkannt
  und korrekt beschrieben; Ablehnen funktioniert, wo die Seite es anbietet.

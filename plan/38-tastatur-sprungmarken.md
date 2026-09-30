# 38 · Tastatur-Sprungmarken aus dem Seitenmodell

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 24, 25

## Ziel

Jedes im `SemanticGraph` exponierte Element mit ausführbarer Aktion und
Position ist mit wenigen Tasten erreichbar, auch in iframes. Marken kommen
aus dem `SemanticGraph`, nicht aus dem DOM (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-a-assistenz-im-browser)).
Elemente, die Chromium nicht in den AXTree aufnimmt, kann Relief damit nicht
erreichen; fehlende Namen darf der Resolver nur mit ausgewiesener Herkunft
ergänzen.

## Schritte

1. Marken aus Knoten mit Aktion und Position (`bounds`) im Kern berechnen,
   browserfrei, mit Tests gegen die Aufnahmen.
2. Overlay im Fork zeichnen; Auswahl löst die Aktion über `ActionPlan` aus.
3. Elemente mit geratenem Namen als unsicher kennzeichnen (`Certainty`).

## Fertig, wenn

- Alle im Graphen exponierten, ausführbaren Elemente in Testshop, Formular und
  spiegel.de-Consent sind per Sprungmarken bedienbar; VoiceOver läuft parallel
  unverändert (Verfahren aus 09, „Nachtrag Paket 19“).

# 113 · Vereinfachte Ansicht und Semantic View über die ganze Tab-Breite

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** 5 · **Abhängig von:** 29 ✓

## Ziel

Die dritte Stufe aus spezifikation/08 („Original · vereinfacht ·
semantisch“) und eine Semantic View, die statt im Side Panel die ganze
Breite des Tabs nutzt (motorische und visuelle Einschränkungen).

## Stand

Paket 29: Semantic View im Relief-Side-Panel, Wechsel Original ↔
semantisch ohne Seitenaktion, Ort bleibt (→ spezifikation/08, „Umsetzung
im Fork“). Eine vereinfachte Ansicht gibt es nicht.

## Schritte

1. Klären, was „vereinfacht“ gegenüber „semantisch“ weglässt bzw.
   zusammenfasst (z. B. nur Hauptbereich und primäre Aktion), mit
   Beispielen aus `spike/fixtures`.
2. Ort der breiten Ansicht entscheiden (eigener Tab mit WebUI, Overlay über
   dem Inhalt, Side Panel in voller Breite) und die Rückkehr zum Original.
3. Umsetzen mit denselben Regeln wie 29 (validierte Aktionen, Ort erhalten,
   Baseline aus spezifikation/13).

## Fertig, wenn

- Wechsel zwischen allen drei Stufen ohne Seitenaktion, Ort erhalten;
  Browser-Test wie `SemantischeAnsicht`.

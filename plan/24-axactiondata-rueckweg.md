# 24 · Aktionen über AXActionData

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** 2 · **Abhängig von:** 17 ✓

## Ziel

Alle `ActionKind`s laufen im Fork über `AXActionData` statt DOM/JavaScript;
Widgets, die im Spike nicht erreichbar waren (nur echte Tastaturereignisse),
werden damit geprüft.

## Fertig, wenn

- Aufgaben `spike/tasks/01`–`05` laufen im Fork über den AX-Weg.
- Liste der Aktionen, die AX nicht abdeckt, mit Ersatzweg.

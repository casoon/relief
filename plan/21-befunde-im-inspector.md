# 21 · Befunde aus a11y-rules im Inspector

**Umgebung:** Cloud schreibt, M4 prüft · **Phase:** 1 · **Abhängig von:** 20

## Ziel

„Erkannte Probleme“ im Inspector aus `a11y-rules`/`a11y-report` (barrierlab),
mit `Outcome` und Severity, ohne eigenes Befundmodell.

## Schritte

1. Klären, welche Regeln über den AXTree-Tier laufen können (`a11y-dom`-Tiers);
   was DOM braucht, als `NotRun` ausweisen.
2. Anbindung browserfrei in Rust; Anzeige im Panel.

## Fertig, wenn

- Befunde erscheinen je Knoten; nicht gelaufene Regeln sind sichtbar als solche.

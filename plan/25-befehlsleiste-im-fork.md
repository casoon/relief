# 25 · Befehlsleiste nativ im Fork

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** 2 · **Abhängig von:** 20, 24

## Ziel

Die Befehlsleiste aus dem Spike (`palette.js`) als Browser-UI statt
eingefügtem Seitenskript: kein Konflikt mit modalen Dialogen der Seite, kein
Eintrag im AXTree der Seite, Tastenkürzel auf Browser-Ebene.

## Kontext

Spike-Befunde in `docs/architecture.md` (Befehlsleiste): modale Seitendialoge
machen eingefügte UI inert; die Leiste musste vor jedem Befehl schließen.

Interaktionsregeln (→ spezifikation/05):

- Mehrdeutige Ziele erscheinen als nummerierte Kandidaten; Zahl oder Name
  wählt aus. Relief rät weiterhin nie.
- „Abbrechen“ stoppt eine laufende Ausgabe, verwirft eine offene Auswahl oder
  Bestätigung und verhindert noch nicht gesendete Aktionen. Eine bereits an
  Chromium gesendete Aktion wird nicht als ungeschehen dargestellt.
- Deiktische Ziele wie „dieses Feld“ oder „hier“ beziehen sich ausschließlich
  auf den aktuellen Fokus beziehungsweise die aktuelle Auswahl, nie auf eine
  Vermutung aus dem Seitenbild.
- Laufende Erkennung, Rückfrage, Bestätigung, Ausführung und Ergebnis sind als
  klare Zustände sichtbar und für Assistenztechnik verfügbar. Wiederholte
  Zwischenstände werden gebündelt und sind pausierbar.
- Bestätigungen zeigen die aus dem validierten `ActionPlan` abgeleiteten
  Parameter; sie bestätigen nie nur eine freie Zusammenfassung (→ 05).

## Fertig, wenn

- Alle Befehle aus dem Spike über die native Leiste; Fokus bleibt beim Ziel.
- Leiste selbst ohne Maus und mit VoiceOver bedienbar.
- Mehrdeutige Ziele lassen sich per Nummer auswählen; Auswahl, Bestätigung und
  laufende Ausgabe sind jederzeit abbrechbar.
- Öffnen, Abbrechen und Schließen stellen den Fokus deterministisch wieder her;
  die gemeinsame Baseline und Testmatrix aus spezifikation/13 und 47 ist
  erfüllt.

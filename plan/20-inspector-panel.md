# 20 · Semantic Inspector im Fork

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** 1 · **Abhängig von:** 19 (Go)

## Ziel

Side-Panel im Fork, das den Graph live zeigt: Struktur, Landmarks,
Überschriften, Bedienelemente mit Namen, Rollen, Zuständen, Aktionen,
Beziehungen und Certainty.

## Schritte

1. WebUI oder Views entscheiden (13 nennt die Vorbilder, z. B. Reading Mode) —
   WebUI bevorzugt, weil selbst barrierefrei testbar.
2. Datenweg: Rust-Graph → JSON → WebUI; Aktualisierung bei Delta.
3. Tastaturbedienung und Screenreader-Tauglichkeit des Panels selbst.
4. Rollen, Namen, Zustände, Fokus und Statusmeldungen gegen die gemeinsame
   User-Agent-Baseline prüfen (→ spezifikation/13); Live-Updates bündeln, damit
   der Inspector den Screenreader nicht mit Einzelmeldungen überflutet.

## Fertig, wenn

- Panel zeigt Graph von `spike/fixtures/*` und einer großen realen Seite live.
- Panel ist ohne Maus bedienbar (manuell geprüft, Befund im PR).
- Auswahl und Aktivierung sind getrennt, Fokus bleibt sichtbar und nach dem
  Schließen an einer nachvollziehbaren Stelle; automatisierte Semantik-Checks
  und der manuelle AT-Test aus 47 sind grün.

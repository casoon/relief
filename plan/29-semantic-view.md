# 29 · Semantic View

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** 5 · **Abhängig von:** 25

## Ziel

Alternative, aus dem Graph erzeugte Ansicht; die Originalseite bleibt geladen,
jede Bedienung läuft als validierte Aktion auf sie
([spezifikation/08](spezifikation/08-assistenz-und-capabilities.md#semantic-view)).

## Fertig, wenn

- Testshop und Formular vollständig über den Semantic View bedienbar.
- Abgrenzung zu Chromium Reading Mode dokumentiert.
- Wechsel zwischen Original-, vereinfachter und semantischer Ansicht löst
  keine Seitenaktion aus und erhält Aufgabe, aktuellen Abschnitt und Fokus,
  soweit der Zielknoten noch existiert; andernfalls wird der neue Ort erklärt.
- Zurück zur Originalseite ist jederzeit möglich; Rollen, Namen, Reihenfolge,
  Zoom/Textskalierung, Kontrast und reduzierte Bewegung erfüllen die Baseline
  aus spezifikation/13.

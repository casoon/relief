# 44 · Lauf ohne Fenster und Bericht für CI

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** Linie B · **Abhängig von:** 42

## Ziel

Aufgaben-Dateien laufen im eigenen Build kopf-los und liefern einen
JUnit-Bericht (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-b-prüfen-im-echten-browser)).

## Schritte

1. Aufgaben-Läufer an den Fork anbinden (heute nur CDP-Host gegen Chrome).
2. `--headless=new` mit `--enable-relief`; Ergebnis als JUnit-XML und als
   `a11y-report`.
3. Befunde über die Zustände einer Aufgabe mit stabilem Schlüssel
   deduplizieren; derselbe Befund nach Öffnen eines Dialogs und nach dem
   nächsten Schritt erscheint einmal mit den betroffenen Zuständen.
4. Beispiel-Workflow, der die Aufgaben in CI laufen lässt.

## Fertig, wenn

- `relief test spike/tasks/*.txt --junit out.xml` läuft ohne Fenster und
  meldet dieselben Ergebnisse wie der CDP-Host; wiederkehrende Befunde über
  mehrere Zustände werden im Bericht nicht vervielfacht.

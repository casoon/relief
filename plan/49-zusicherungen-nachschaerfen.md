# 49 · Formular-Zusicherungen nachschärfen

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 42 ✓

## Ziel

Offene Punkte aus 42 (→
[spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-paket-42)):
Statusmeldung als Änderung statt als Endzustand prüfen und die
Formular-Zusicherungen in die regelmäßigen Prüfungen aufnehmen.

## Schritte

1. `statusmeldung`: prüfen, dass die Live-Region durch die letzte Aktion
   **geändert** wurde (eine mit ihrem Text neu eingefügte Region sagen
   Screenreader oft nicht an). Dafür den Stand vor der letzten `do:`-Zeile
   übergeben und die Diff-Regeln aus `a11y-perception` (`AXTreeDiff`) bzw.
   die `TreeDelta` des Modells nutzen, nicht nachbauen.
2. `spike/tasks/06-form-assertions.txt` in die Prüfbefehle aufnehmen:
   Glob in `.github/workflows/ci.yml` (`0[1-5]` → `0[1-6]`, vorher Skill
   `ci-budget`) und Befehlszeile in `CLAUDE.md`. Entscheidung des Nutzers.
3. Optional: DOM-Fakten um Rendering (`display`, `visibility`) ergänzen und
   `accname::name_rendered` nutzen, damit per CSS verborgener Inhalt keine
   Abweichung mehr erzeugt; iframes und Shadow DOM in die DOM-Fakten.

## Fertig, wenn

- Eine Seite, die ihre Statusregion mitsamt Text neu einfügt, liefert einen
  Befund `form/status-message`; `form-clean.html` weiterhin keinen.
- Prüfbefehle aus `CLAUDE.md` grün, einschließlich 06.

## Nicht Teil

Echte Screenreader-Ausgabe (→ 43), Bericht als JUnit (→ 44).

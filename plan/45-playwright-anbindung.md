# 45 · Playwright-Anbindung

**Umgebung:** Cloud + M4 · **Phase:** Linie B · **Abhängig von:** 24, 44

## Ziel

Bestehende Playwright-Tests steuern Relief und fragen das Seitenmodell ab,
ohne das Testsystem zu wechseln (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-b-prüfen-im-echten-browser)).

## Schritte

1. Relief als `executablePath` in Playwright starten (CDP bleibt erhalten).
2. Eigene CDP-Domäne `Relief.*`: Seitentyp, Gruppen, primäre Aktion,
   Zusicherungen aus 42 abfragen.
3. Kleines npm-Paket mit Hilfsfunktionen (`expect(page).toPassReliefForm()`).

## Fertig, wenn

- Ein Playwright-Test gegen `spike/fixtures/form.html` nutzt `Relief.*` und
  schlägt beim kaputten Formular fehl.

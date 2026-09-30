# 47 · Accessibility-Baseline für Reliefs eigene Oberfläche

**Umgebung:** Cloud + M4, später Windows · **Phase:** quer · **Abhängig von:** 20, 25

## Ziel

Inspector, Befehlsleiste, Semantic View, Bestätigungen und Einstellungen
erfüllen dieselbe verbindliche User-Agent-Baseline (→ spezifikation/13).

## Schritte

1. Wiederverwendbare Prüfungen für Name, Rolle, Zustand, Fokusfolge,
   Modalität, Statusmeldungen und Fokus-Rückgabe in `//relief/testing/`.
2. Pro Kernoberfläche mindestens einen vollständigen Tastatur- und
   VoiceOver-Ablauf dokumentieren; Testdaten enthalten Versionen und
   Einstellungen.
3. Darstellung bei 200 % Text/Zoom, hohem Kontrast und reduzierter Bewegung
   prüfen; Ausgabe bei schnellen Graph-Deltas auf störende Wiederholungen
   testen.
4. Mit Paket 31 dieselben Kernaufgaben unter Windows über UI Automation,
   Narrator/NVDA und Voice Access nachweisen; Linux später ergänzen.
5. Abweichungen nur mit Verantwortlicher Person, Begründung und Ablaufdatum
   zulassen.

## Fertig, wenn

- Automatisierte Semantik-/Fokusprüfungen und die manuellen macOS-Aufgaben für
  jede vorhandene Kernoberfläche grün und reproduzierbar dokumentiert sind.
- Neue Relief-Oberflächen die Baseline als Abnahmekriterium übernehmen.
- Windows wird erst als unterstützt bezeichnet, wenn seine Matrix erfüllt ist.

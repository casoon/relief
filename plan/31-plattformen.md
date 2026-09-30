# 31 · Plattformen Linux und Windows

**Umgebung:** offen: Build-Hosts fehlen · **Phase:** quer · **Abhängig von:** 14 ✓, 17 ✓

## Ziel

Fork und Host auf Linux und Windows (Ziel laut `docs/constraints.md`).

## Stand

- CDP-Host unter Linux: CI-Job `browser` (Ubuntu 24.04, Chrome for Testing
  154.0.8037.58) liegt bei, ist aber noch nicht gelaufen (Actions-Abrechnung,
  → 10). Unter macOS erfüllt der Host alle 68 Erwartungen aus
  `spike/tasks/01`–`05`; Linux-Befund offen.
- Fork-Build Linux: braucht einen x86-64-Rechner/Runner mit > 100 GB Platte;
  Cloud reicht nicht, eine ARM-VM auf dem M4 ist kein unterstützter Host
  (Beleg in spezifikation/01).
- Windows: kein Host vorhanden. Cross-Build vom Mac ist laut
  `docs/win_cross.md` möglich (Windows-SDK einmal auf einem Windows-Rechner
  paketieren); Testen braucht Windows.

## Plattform-Nachweis

Der Nachweis umfasst nicht nur Build und Webseiten-AXTree, sondern Reliefs
eigene Oberfläche (→ spezifikation/13, 47):

- macOS: Tastatur, VoiceOver, Voice Control, Zoom/Textgröße, Kontrast und
  reduzierte Bewegung; Switch Control mindestens für die Kernaufgaben.
- Windows: UI Automation für Rollen/Namen/Zustände/Ereignisse, Tastatur,
  Narrator und NVDA, Voice Access, hohe Kontraste und Textskalierung.
- Linux folgt mit AT-SPI/Orca, sobald ein unterstützter Testhost feststeht.

Eine Plattform gilt erst als unterstützt, wenn dieselben Kernaufgaben mit den
für sie festgelegten Eingabe- und Ausgabekanälen laufen. Ein erfolgreicher
Cross-Build allein genügt nicht.

## Entscheidung nötig

Woher die Build-Hosts kommen (eigener Rechner, gemieteter Runner). Spätestens
vor Phase 2.

# 146 · Relief-Oberfläche manuell unter macOS prüfen

**Umgebung:** M4, mit Person an der Assistenztechnik · **Phase:** quer · **Abhängig von:** 47 ✓

## Ziel

Den manuellen Teil der Baseline (→ spezifikation/13, „Manuelles
Testprotokoll macOS“) einmal vollständig durchgehen: Kernaufgaben K1–K7
mit Tastatur, VoiceOver, Sprachsteuerung, Schaltersteuerung (K1, K2) und
den Darstellungseinstellungen.

## Kontext

Paket 47 hat die automatisierten Prüfungen umgesetzt (`UiBaseline`) und
das Protokoll als Vorlage angelegt. VoiceOver per AppleScript zu steuern
(wie in Paket 19) belegt Ton, Fokus und Tastatur des Rechners; Sprach- und
Schaltersteuerung brauchen ohnehin eine Person (Entscheidung Nutzer,
2026-10-11). Automatisierte Prüfung der VoiceOver-Ausgabe: → 43.

## Schritte

1. Versionen festhalten (macOS, Relief/Chromium, VoiceOver), frisches
   Profil, Testseiten aus `fork/relief/testing/data/` über einen lokalen
   Server.
2. Je Aufgabe und Eingabekanal eine Zeile in der Ergebnistabelle von
   spezifikation/13.
3. Jeder Befund wird ein eigenes Paket oder eine befristete Ausnahme
   (→ spezifikation/13, „Ausnahmen“).

## Fertig, wenn

- Die Ergebnistabelle deckt K1–K7 für alle genannten Kanäle ab, jede Zeile
  mit Ergebnis.

## Nicht Teil

- Windows und Linux (→ 31).
- Befunde beheben.

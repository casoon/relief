# 120 · Sensible Werte in Auskünften: Fork-Teil prüfen

**Umgebung:** M4 · **Phase:** quer · **Abhängig von:** 100 ✓

## Ziel

Die Regeln aus Paket 100 (→ spezifikation/07, „Auskünfte und unverstandene
Eingaben“) im eigenen Build belegen. Die Änderung liegt nur in Rust
(`relief-interaction`, `relief-bridge`), kein C++ unter `fork/`; sie kommt
mit dem nächsten Build des Forks an.

## Schritte

1. Fork mit dem Stand nach 100 bauen (`relief-bridge` neu gelinkt).
2. `relief_browsertests` laufen lassen.
3. Fork-Aufgaben 01–05, 07, 15, 16 über `--relief-run` bzw.
   `relief-cdp test --fork`; bei 16 mit `--relief-log`.
4. Inspector-Panel auf `spike/fixtures/login.html` mit ausgefülltem
   Passwort: Kurzzeile „[textbox] Passwort = (verdeckt)“, unter „Wert“
   „(verdeckt)“.
5. Befehlsleiste im Panel: „füle Passwort mit sommer123“ (unverstanden)
   erscheint im Protokoll (`command`) als „füle Passwort mit (verdeckt)“.

## Fertig, wenn

- `relief_browsertests` grün; Fork-Aufgaben 01–05, 07, 15 ohne Fehlschlag;
  16 verfehlt nur die Erwartungen zu Benutzername und Kartennummer
  (`autocomplete` kommt im Fork nicht an, → 75).
- Schritte 4 und 5 wie beschrieben; keine Protokollzeile enthält einen der
  eingegebenen Werte. Ergebnis in spezifikation/07 nachgetragen.

## Nicht Teil

- `autocomplete` im Fork (75).

# 58 · Sicherheitsgrenzen im Fork bauen und prüfen

**Umgebung:** M4 · **Phase:** quer · **Abhängig von:** 48 ✓

## Ziel

Der Fork-Teil von Paket 58 ist geschrieben, aber noch nicht gebaut: Der
Fork schreibt das Security-Log der Sitzung ins Protokoll (`--relief-log`),
und der Mirror gibt HTML-`type` eines `<input>` weiter, damit die Rückfrage
Passwortwerte verdeckt (→ spezifikation/07, „Security-Log“,
„Bestätigungstoken“). CDP-Host und Rust sind umgesetzt und belegt.

## Schritte

1. Quellen übernehmen und bauen:

   ```bash
   scripts/fork-apply.sh ~/chromium/src --continue
   autoninja -C ~/chromium/src/out/Relief chrome relief_browsertests
   ```

   Geändert: `fork/relief/bridge/runtime_host.{h,cc}` (`LogSecurity` nach
   `run_command`), `fork/relief/bridge/ax_tree_mirror.cc` (`kInputType` →
   `extra["inputType"]`), Bridge-Funktion `take_security_log`
   (`crates/relief-bridge/src/cxx_bridge.rs`).
2. Aufgaben mit Protokoll:

   ```bash
   RELIEF_LOG=/tmp/relief-58.log scripts/fork-run-tasks.sh \
     spike/tasks/0[1-5]*.txt spike/tasks/07-bestaetigung.txt
   grep -F 'security' /tmp/relief-58.log | tail -9
   ```

   Erwartet: „0 nicht erfüllt“; die letzten neun `security`-Zeilen wie im
   CDP-Host (07): `reject`/`no_prompt`, `ask_confirmation` (Plan 1, 2),
   `reject`/`no_prompt`, `ask_confirmation` (3, 4), `perform_confirmed` (4),
   `reject`/`no_prompt`, `ask_confirmation` (5). `security`-Zeilen ohne
   „kaufen“, „Erika“, „file:“ (`grep -F security … | grep -ciE
   'kaufen|erika|file:'` → 0).
3. Integrationstests ohne Regression:
   `out/Relief/relief_browsertests` (alle, besonders
   `*BestaetigungNurEinmalUndGebunden*`, `*Befehlsleiste*`).
4. Ergebnis in `spezifikation/07` („In den Hosts“, „Sensible Werte“) als
   im Fork belegt eintragen, dieses Paket löschen.

## Fertig, wenn

- Ein Fork-Lauf von `07-bestaetigung.txt` schreibt `security`-Zeilen mit
  Entscheidung, Plan-ID und Grund, ohne Feldwerte, und alle Aufgaben
  bleiben erfüllt.

## Nicht Teil

- Formularziel im Fork (→ 75).
- Werte in Antworten und Protokollzeilen außerhalb des Security-Logs (→ 76).

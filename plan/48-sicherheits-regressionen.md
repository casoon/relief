# 48 · Sicherheits-Regressionsmatrix für Modellgrenzen

**Umgebung:** M4 (Fork bauen) · **Phase:** quer · **Abhängig von:** 24 ✓; vor Modellintegration

## Ziel

Reliefs Trennung zwischen untrusted Seiten-/Modellausgabe, Rust-Validierung,
Bestätigung und Browseraktion bleibt auch unter gezielten Missbrauchsfällen
belegt (→ spezifikation/07, „Sicherheits-Regressionsmatrix“).

## Stand

Browserfrei erledigt und in spezifikation/07 beschrieben: Matrix
(`crates/relief-ai-contract/tests/missbrauch.rs`), Bestätigungstoken in
`Session`, `Budget` mit festen Grenzen, Security-Log. Die CDP-Aufgaben
01–07 laufen mit 0 nicht erfüllt.

Offen ist nur der Fork-Teil: geschrieben, **noch nicht im Fork gebaut**.

## Schritte

1. Quellen in den Checkout bringen und bauen (neu: `security.rs` in
   `relief_interaction_rs`, Browser-Test, Testseite `kasse.html`):

   ```bash
   scripts/fork-apply.sh ~/chromium/src --continue
   autoninja -C ~/chromium/src/out/Relief chrome relief_browsertests
   ```

2. Bestätigungs-Bypass im Fork prüfen:

   ```bash
   ~/chromium/src/out/Relief/relief_browsertests \
     --gtest_filter='ReliefBrowserTest.BestaetigungNurEinmalUndGebunden'
   scripts/fork-run-tasks.sh spike/tasks/07-bestaetigung.txt
   ```

3. Keine Regression: `scripts/fork-run-tasks.sh spike/tasks/0[1-5]*.txt`
   (03 hat jetzt eine Rückfrage mehr) und die übrigen
   `relief_browsertests`.

## Fertig, wenn

- `BestaetigungNurEinmalUndGebunden` grün ist und
  `fork-run-tasks.sh spike/tasks/07-bestaetigung.txt` „0 nicht erfüllt“
  meldet.
- Aufgaben 01–05 im Fork weiter ohne nicht erfüllte Erwartung.
- Ergebnis in spezifikation/07 (Matrix, Zeile Bestätigungs-Bypass) als
  belegt eingetragen, diese Datei gelöscht, Zeile in `status.md` entfernt.

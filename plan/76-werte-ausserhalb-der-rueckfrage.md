# 76 · Sensible Werte außerhalb der Rückfrage: Fork-Teil bauen

**Umgebung:** M4 · **Phase:** quer · **Abhängig von:** 58

## Ziel

Der umgesetzte Rest im Fork gebaut und geprüft (Substanz →
spezifikation/07, „Sensible Werte außerhalb der Rückfrage“). Rust-Seite,
CDP-Host und Fork-Runtime sind belegt.

## Schritte

1. `scripts/fork-apply.sh`, `autoninja -C out/Relief chrome
   relief_browsertests` (geändert: `crates/relief-interaction`,
   `crates/relief-bridge/src/cxx_bridge.rs` mit `redact_input`,
   `fork/relief/bridge/runtime_host.cc`,
   `fork/relief/inspector/relief_inspector_ui.cc`,
   `fork/relief/inspector/resources/inspector.js`).
2. `relief_browsertests` wie bisher grün.
3. `scripts/fork-run-tasks.sh spike/tasks/0[1-5]*.txt spike/tasks/07-*.txt
   spike/tasks/15-*.txt`: „0 nicht erfüllt“ (Antworten für nicht sensible
   Felder unverändert).
4. `RELIEF_LOG=<datei> scripts/fork-run-tasks.sh
   spike/tasks/16-sensible-werte.txt`: Anzeigename und Passwort erfüllt.
   Benutzername und Kartennummer (drei Erwartungen) verfehlt der Fork
   erwartungsgemäß, weil `autocomplete` nicht ankommt (→ 75); dann nennt
   die Antwort dort den Wert. Protokoll: die `command`-Zeilen lauten
   „fülle … mit (verdeckt)“, `grep -c 'geheim123\|erika\|4111\|5555'
   <datei>` ergibt 0.
5. Panel (Strg+Umschalt+I): „fülle Passwort mit geheim123“ auf
   `spike/fixtures/login.html`; das Log der Leiste zeigt „fülle Passwort mit
   (verdeckt)“ und „SetValue(verdeckt) auf …“.

## Fertig, wenn

- Schritte 2–5 belegt; Status in spezifikation/07 auf „im Fork belegt“.

## Nicht Teil

- Auskünfte und unverstandene Eingaben (→ 100).

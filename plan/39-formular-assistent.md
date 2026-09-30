# 39 · Formular-Assistent

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 24, 25; 26 für Sprache

## Ziel

Formulare verlässlich ausfüllen: Pflichtfelder, Fehlermeldungen und Fokus
nach dem Absenden ansagen, Felder per Befehl oder Sprache füllen (→
[spezifikation/12](spezifikation/12-produktumfang.md#linie-a-assistenz-im-browser)).

## Schritte

1. Formular als Gruppe im Graphen: Felder, Beschriftung, Pflicht, Fehlerzustand,
   verknüpfte Fehlermeldung (`aria-describedby`/`aria-errormessage`).
2. Befehle „Was fehlt noch?“, „fülle … mit …“, „Fehler vorlesen“.
3. Absenden ist riskant (→ 05): Bestätigung vor dem Absenden, Ergebnis aus dem
   Delta ansagen.
4. Vor HIGH-Aktionen eine prüfbare Zusammenfassung der lokalen Felder und des
   Ziels zeigen; Werte bleiben editierbar, geheime Werte werden maskiert. Die
   Bestätigung ist an genau diesen Plan gebunden (→ 05).
5. Nach Validierungsfehler zum ersten fehlerhaften Feld führen und die Rückkehr
   zum vorherigen Ort anbieten. Noch nicht abgesendete Werte werden nicht in
   Relief-Profil, Telemetrie oder Modellcache kopiert.

## Fertig, wenn

- `spike/tasks/03-form.txt` läuft im Fork über die Befehlsleiste; auf zwei
  echten Formularen werden Fehler nach dem Absenden richtig angesagt.
- Abbrechen vor dem Absenden verändert die Seite nicht zusätzlich; Änderung
  von Ziel oder Feldwert macht eine vorhandene Bestätigung ungültig.

## Nicht Teil

Persönliche Daten automatisch eintragen (Autofill bleibt bei Chromium).

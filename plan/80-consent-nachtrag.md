# 80 · Consent: Einwilligungsseiten ohne Dialog, Nachweis im Fork

**Umgebung:** Cloud + M4 · **Phase:** Linie A · **Abhängig von:** 40 ✓

## Ziel

Offene Punkte aus 40 (→
[spezifikation/05](spezifikation/05-intents-und-aktionen.md#overlay--und-consent-dialoge-paket-40-belegt)).

## Schritte

1. **Einwilligungsseite ohne Dialog-Rolle** (golem.de: ganze Seite mit
   „Zustimmen und weiter“ und „Zu Golem pur“, kein `dialog`): als
   Cookie-Hinweis erkennen, ohne gewöhnliche Seiten mit Datenschutz-Links
   falsch einzuordnen.
2. **Fork (M4):** `scripts/fork-run-tasks.sh spike/tasks/09-consent.txt`
   und die Consent-Befehle auf spiegel.de/bild.de im eigenen Build.
   Erwartung: gleiche Ergebnisse wie über CDP; „was ist hinter dem Dialog“
   meldet, dass der Baum nichts enthält, weil Blink den Inhalt hinter
   `aria-modal` herausnimmt (→ 09, Nachtrag Paket 35) — messen.
3. **Abo-Beschriftungen ohne Signalwort** (sueddeutsche.de: „Jetzt testen“):
   entscheiden, ob Buttons neben dem Zustimmen, die zu einem Angebot führen,
   als Abo gelten; heute „weitere“, die Ansage „Kein Ablehnen“ stimmt.
4. **Zweite Ebene:** Ablehnen, das erst hinter „Einstellungen“ liegt, nur
   ansagen oder auf ausdrücklichen Befehl dorthin führen — ohne dass Relief
   Einstellungen selbst wählt.

## Fertig, wenn

- golem.de wird als Cookie-Hinweis erkannt, die Korpusseiten
  (`spike/tasks/11-korpus.txt`) nicht.
- `09-consent.txt` läuft im Fork ohne Ausfall; das Verhalten hinter dem
  Dialog ist gemessen und in 05 festgehalten.

## Nicht Teil

- Einwilligen, Umgehen von Bot-Erkennung (zeit.de) oder Bezahlschranken.

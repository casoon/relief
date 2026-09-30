# 76 · Sensible Werte außerhalb der Rückfrage

**Umgebung:** Cloud (+ M4 für den Fork-Teil) · **Phase:** quer · **Abhängig von:** 58

## Ziel

Nicht nur die Rückfrage, auch Antworten und Protokolle geben Werte
sensibler Felder nicht wieder (→ spezifikation/07, „Security-Log“,
Grenzen).

## Kontext

Belegt (Paket 58): Die Antwort nach einer Aktion beginnt mit
`{:?}` des Plans, also `SetValue("…") auf …`, auch für Passwortfelder
(`Session::performed`). Das Fork-Protokoll schreibt jede Eingabe als
`command\t<Eingabe>`, das Protokoll der Befehlsleiste als `eingabe`; beide
enthalten damit eingegebene Werte, neben den wertfreien `security`-Zeilen.

## Schritte

1. `Session::performed` verdeckt den Wert bei sensiblen Feldern wie die
   Rückfrage (`is_sensitive_field`); Aufgaben, die `SetValue("…")` erwarten,
   bleiben für nicht sensible Felder gleich.
2. Entscheiden, ob Protokolle Eingaben enthalten dürfen (Nutzerstudie
   braucht sie ggf.) oder nur ihre Art; mindestens bei sensiblen Zielen
   verdecken.

## Fertig, wenn

- Ein Test füllt ein Passwortfeld aus; weder Antwort noch Protokoll
  enthalten den Wert.

## Nicht Teil

- Security-Log selbst (wertfrei, 48/58).

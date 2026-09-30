# 100 · Sensible Werte in Auskünften und unverstandenen Eingaben

**Umgebung:** Cloud (+ M4 für den Fork-Teil) · **Phase:** quer · **Abhängig von:** 76

## Ziel

Entscheiden und umsetzen, ob Auskünfte den Wert sensibler Felder nennen
und ob unverstandene Eingaben ins Protokoll dürfen (→ spezifikation/07,
„Sensible Werte außerhalb der Rückfrage“).

## Kontext

Belegt (Paket 76): Rückfrage und Antwort nach einer Aktion verdecken Werte
sensibler Felder, Protokolle den Wert jedes Ausfüll- und Auswahlbefehls.
Weiter mit Wert: `respond::control_line` in „wo bin ich“, „details zu …“
(`respond::inspect`), der Aktionsliste, Mehrdeutigkeitslisten und
Sprungmarken; der Inspector (`inspector_json`). Passwortfelder zeigt
Chromium ohnehin nur als Punkte, betroffen sind Felder mit Zahlungs- oder
Identitäts-`autocomplete` (CDP-Host; im Fork erst mit 75).

Eine Eingabe, die der Parser nicht versteht (etwa ein vertippter
Ausfüllbefehl), steht wörtlich im Protokoll, also ggf. mit einem Wert.

## Schritte

1. Entscheiden: Eine ausdrückliche Auskunft über ein Feld („details zu
   Kartennummer“) ist Vorlesen auf Wunsch, wie im Screenreader; Listen und
   „wo bin ich“ sind es nicht. Vorschlag: nur auf ausdrückliche Nachfrage
   nennen, sonst „(verdeckt)“.
2. Entscheiden, ob unverstandene Eingaben ins Protokoll dürfen (Nutzerstudie
   braucht die Formulierung) oder nur ihre Länge/Art.
3. Umsetzen mit Tests wie in 76.

## Fertig, wenn

- Entscheidung in spezifikation/07; ein Test belegt sie für ein Feld mit
  `autocomplete="cc-number"`.

## Nicht Teil

- Rückfrage und Antwort nach einer Aktion (58, 76).

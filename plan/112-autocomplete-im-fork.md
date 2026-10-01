# 112 · HTML-`autocomplete` im Fork außerhalb einer Rückfrage

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** quer · **Abhängig von:** 75 ✓

## Ziel

Felder mit `autocomplete` für Zahlungs- oder Identitätsdaten gelten im Fork
auch außerhalb einer Rückfrage als sensibel: Antwort nach „fülle …“,
Auskünfte und Inspector verdecken ihren Wert wie im CDP-Host
(→ spezifikation/07, „Sensible Werte außerhalb der Rückfrage“).

## Stand

Paket 75 fragt `autocomplete` beim Renderer nur an, solange eine Rückfrage
offen ist oder entsteht (`relief.mojom.FormFacts`, Entscheidung (b)).
`spike/tasks/16-sensible-werte.txt` verfehlt im Fork deshalb Benutzername
und Kartennummer (drei Erwartungen): Die Antwort nach dem Ausfüllen nennt
den Wert.

## Schritte

1. Weg entscheiden, mit Kosten: (a) vor jedem Ausfüll-/Auswahlbefehl für
   das Ziel anfragen (eine Mojo-Runde je Befehl, Ziel steht erst nach der
   Auflösung fest → wie `reconfirm` nachträglich, aber vor dem Senden der
   Schritte); (b) beim Laden bzw. bei neuen Feldern einmal je Formular
   anfragen (Anfrage auch ohne Rückfrage; weicht von der Entscheidung zu 75
   ab, braucht Zustimmung); (c) Blink-Patch, der `autocomplete` serialisiert.
2. Umsetzen, `16-sensible-werte.txt` im Fork ohne Fehlschlag.

## Fertig, wenn

- `scripts/fork-run-tasks.sh spike/tasks/16-sensible-werte.txt` meldet
  „0 nicht erfüllt“, `relief_browsertests` grün.

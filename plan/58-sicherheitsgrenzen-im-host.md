# 58 · Sicherheitsgrenzen in Hosts und Modellaufruf verdrahten

**Umgebung:** Cloud + M4 · **Phase:** quer · **Abhängig von:** 48; vor Modellintegration (28 im Fork, 34)

## Ziel

Was Paket 48 browserfrei gebaut hat, wirkt auch im laufenden Browser:
Security-Log wird geschrieben, Modellaufrufe laufen nur über `Budget`, die
Rückfrage bindet und zeigt alles, was sie binden soll (→ spezifikation/07,
„Sicherheits-Regressionsmatrix“, Offen).

## Schritte

1. Security-Log abholen (`Session::take_security_log`) und als JSON-Zeilen
   schreiben: CDP-Host (`RELIEF_LOG` bzw. eigene Datei), Fork
   (`--relief-log`). Prüfen, dass keine Werte darin stehen.
2. Sobald die Runtime ein Modell aufruft: nur über `Budget`, ein Budget je
   Aufgabe/Seite; der Abbruchtext geht an die Nutzerin, das Ereignis ins Log.
   Grenzwerte (`Limits::default`, Annahme) an echten Seiten und dem
   Resolver-Messlauf (28) prüfen.
3. Formularziel binden: `action` des Formulars eines Submit-Buttons (CDP:
   `DOM.describeNode`; Fork: klären, ob es im `AXNodeData` ankommt) in
   `Binding::destination` und in der Rückfrage.
4. Sensible Werte in der Rückfrage maskieren, sobald die Runtime
   Feldangaben (`type=password`, `autocomplete`) kennt (→ spezifikation/07,
   „Woher die Feldangaben kommen“).
5. CI: Aufgaben `06` und `07` aufnehmen (heute Glob `0[1-5]`), Skill
   `ci-budget` beachten.

## Fertig, wenn

- Ein Lauf von `spike/tasks/07-bestaetigung.txt` erzeugt in beiden Hosts
  Log-Zeilen mit Entscheidung, Plan-ID und Grund, ohne Feldwerte.
- Kein Modellaufruf der Runtime umgeht `Budget` (Test oder Typ).
- Eine Rückfrage zu einem Absenden-Button nennt das Formularziel, wo es
  bekannt ist, und ein anderes Ziel verlangt neue Bestätigung.

## Nicht Teil

- Die Matrix selbst und das Token (48).
- Consent-Anzeige für Cloud-Modelle (→ spezifikation/07).

# 141 · Fork: Diff meldet Knoten als „nicht mehr wahrnehmbar“

**Umgebung:** M4 · **Phase:** Linie A · **Abhängig von:** —

## Beobachtung (2026-09-30)

`spike/tasks/09-consent.txt`, `consent-zweck-titel.html`, „klicke
Ablehnen“ → „personalisierte“: Der Fork antwortet „Activate auf [button]
Ablehnen. Fokus jetzt auf button „Ablehnen“. 3 Elemente nicht mehr
wahrnehmbar. Neuer Text: …“, der CDP-Host ohne „3 Elemente nicht mehr
wahrnehmbar“. Auf der Seite verschwindet nichts.

## Schritte

1. Mit `--relief-log` und `--relief-log-nodes` festhalten, welche Knoten
   entfernt bzw. ignoriert werden (Rolle, ob `InlineTextBox`/generic,
   Grund: Neuserialisierung, Fokusring, Scrollen).
2. Ursache belegen; wenn es Serialisierungsrauschen ist, im Diff
   (`respond::describe_diff`) oder im Mirror herausnehmen, sonst die
   Antwort erklären.

## Fertig, wenn

- Fork und CDP antworten an dieser Stelle gleich, oder der Unterschied ist
  begründet und in spezifikation/05 beschrieben.

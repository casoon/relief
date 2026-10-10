# 144 · Kosten der WebMCP-Sitzung je Tab messen

**Umgebung:** M4 · **Phase:** Linie A · **Abhängig von:** 143 ✓

## Ziel

Belegen, was die DevTools-Sitzung kostet, die Relief für WebMCP an jeden
Tab hängt (`fork/relief/webmcp_client.cc`): Ladezeit, Skriptzeit, Speicher
des Renderers.

## Kontext

Die Messung in Paket 143 (→ spezifikation/12, „Werkzeuge der Website“)
war nicht belastbar: Lastdurchschnitt 6–10 durch andere Prozesse, die
Streuung innerhalb einer Variante größer als jeder Unterschied. Der
Vergleich brauchte einen Schalter, der nur im Checkout stand
(`RELIEF_OHNE_WEBMCP`, nicht im Repo).

## Schritte

1. Auf ruhigem Rechner (Lastdurchschnitt < 2, keine anderen Builds) mit
   und ohne Client messen, je mindestens 4 Starts im Wechsel, 15
   Ladevorgänge; Seite mit ~9 000 Knoten über `localhost` und eine kleine
   (`kasse.html`).
2. Zusätzlich Speicher des Renderers (`Memory.getProcessMemoryInfo` oder
   Aktivitätsanzeige) nach dem Laden.
3. Ergebnis in spezifikation/12 („Kosten“) eintragen; ist der Unterschied
   spürbar, Vorschlag für ein späteres Anhängen (etwa erst nach dem ersten
   Befehl) als neues Paket.

## Fertig, wenn

- Median und Streuung je Variante stehen in spezifikation/12, mit Last
  während der Messung.

## Nicht Teil

- Den Client umbauen.

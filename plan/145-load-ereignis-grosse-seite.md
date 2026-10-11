# 145 · Fehlendes `load`-Ereignis auf großer Seite

**Umgebung:** M4 · **Phase:** Querschnitt · **Abhängig von:** —

## Ziel

Klären, warum der eigene Build auf einer Seite mit ~9 000 Knoten bei etwa
jedem vierten Laden innerhalb von 10 s kein `Page.loadEventFired` meldet
und danach `Runtime.evaluate` nicht antwortet.

## Kontext

Nebenbefund der Messung in Paket 143 (2026-10-10/11): tritt mit und ohne
WebMCP-Client gleich oft auf (ohne: 3 von 15, 6 von 15; mit: 4 von 15),
unter hoher Last durch andere Prozesse; bei der Messung zu Paket 144
(2026-10-11, freie CPU 50–82 %) 6 bis 7 von 15, ebenfalls mit und ohne
Client. Eine Seite mit ~1 800 Knoten lud dort ohne Hänger (0 von 120). Seite: 1 500 Abschnitte mit
Überschrift, Link, Feld und Button, dazu ein Skript, das 3 000 Knoten
anhängt; Laden über `python3 -m http.server` auf `localhost`, Messskript
über CDP (`Page.navigate`, dann `Page.loadEventFired`). Nicht geprüft, ob
es auch in Chrome for Testing ohne Relief oder mit `--disable-relief`
auftritt.

## Schritte

1. Nachstellen auf ruhigem Rechner; gleiche Seite in Chrome for Testing
   154.0.8037.57 und im eigenen Build mit `--disable-relief`.
2. Tritt es nur mit Relief auf: Spur aufnehmen (`--relief-log`,
   Zeitpunkte der AX-Pakete gegen die Navigation), Ursache eingrenzen.

## Fertig, wenn

- Ursache benannt oder belegt, dass es nicht an Relief liegt.

## Nicht Teil

- Eine Behebung, bevor die Ursache feststeht.

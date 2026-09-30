# 55 · DOM-Fakten mit Rendering, iframes und Shadow DOM

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 42 ✓, 49 ✓

## Ziel

`namen-wie-accname` soll nur noch echte Abweichungen melden (→
[spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49)).
Heute tragen die DOM-Fakten kein Rendering: per CSS verborgener Inhalt
zählt für `accname` mit und erzeugt einen `review`-Befund. Felder in
iframes und Shadow DOM fehlen in den DOM-Fakten und werden `untested`.
Aus Paket 49 als optionaler Schritt ausgelagert, weil er Host (Erhebung)
und Auswertung zugleich ändert.

## Schritte

1. DOM-Fakten um Rendering ergänzen (`display`, `visibility`; im CDP-Host
   z. B. über `DOMSnapshot.captureSnapshot` mit berechneten Stilen) und
   `accname::name_rendered` statt `accname::name` nutzen; prüfen, was
   `a11y-dom` dafür erwartet, nichts nachbauen.
2. iframes (`contentDocument` aus `DOM.getDocument` mit `pierce`) und
   Shadow DOM (`shadowRoots`) in die DOM-Fakten aufnehmen, Zuordnung
   DOM-ID → Knoten dafür erhalten.
3. `form-broken.html`: Die gewollte Abweichung „PLZ (intern: Feld 7)“ muss
   dann aus einer anderen Ursache als `display: none` kommen oder als
   Erwartung angepasst werden; `form-clean.html` bleibt ohne Befund.

## Fertig, wenn

- Per CSS verborgener Label-Inhalt erzeugt keinen `form/name-accname`-Befund.
- Ein Feld in einem iframe bzw. Shadow DOM wird verglichen statt `untested`.
- Prüfbefehle aus `CLAUDE.md` grün, einschließlich 06;
  `cargo build -p relief-interaction --no-default-features` baut.

## Nicht Teil

Fork-Host (Feature `assertions` bleibt dort aus).

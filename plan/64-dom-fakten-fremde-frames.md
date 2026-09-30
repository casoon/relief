# 64 · DOM-Fakten für iframes in anderem Prozess, ID-Bereich im Shadow DOM

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 55 ✓

## Ziel

`namen-wie-accname` vergleicht auch Felder in iframes aus einem anderen
Renderer-Prozess und rechnet ID-Verweise im Shadow DOM im richtigen Bereich
(→ [spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49-55),
„Grenzen“).

## Kontext

- `DOM.getDocument` mit `pierce` liefert `contentDocument` nur für iframes im
  selben Prozess. Bei fremder Herkunft (Site Isolation) fehlt das Dokument;
  Felder dort sind heute `untested` [belegt im Code,
  `crates/relief-cdp/src/assertions.rs`, `Walk::element`]. Der Graph kennt
  sie, soweit `capture.rs` den Frame erreicht.
- Shadow DOM hängt flach unter dem Host im Dokument des Hosts; es gibt einen
  ID-Index je Dokument, nicht je Shadow-Root. `<label for>` im Shadow-Root
  mit einer ID, die es auch im Dokument gibt, kann den falschen Namen
  ergeben [Annahme, nicht gemessen].

## Schritte

1. Testseite mit iframe fremder Herkunft (z. B. `localhost` und `127.0.0.1`
   über einen lokalen Server), prüfen, ob ihr Feld im Modell steht und
   `untested` wird.
2. Für solche Frames die DOM-Fakten über eine eigene CDP-Sitzung des Ziels
   erheben (`Target.attachToTarget`, dort `DOM.getDocument` und
   `DOMSnapshot.captureSnapshot`) und als eigenes Dokument einhängen.
3. ID-Index je Shadow-Root: klären, ob `accname::IdIndex` das hergibt oder
   barrierlab eine Erweiterung braucht (dort als Issue, nicht in Relief
   nachbauen).

## Fertig, wenn

- Ein Feld in einem iframe fremder Herkunft wird verglichen statt `untested`
  (Aufgabe lokal ohne Netz).
- Gleiche ID in Shadow-Root und Dokument ergibt keinen falschen Befund, oder
  die Grenze ist mit Messung in spezifikation/12 festgehalten.
- Prüfbefehle aus `CLAUDE.md` grün.

## Nicht Teil

Fork-Host (Feature `assertions` bleibt dort aus).

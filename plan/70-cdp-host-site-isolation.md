# 70 · CDP-Host mit Site Isolation: Frames in anderem Prozess

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 64 ✓

## Ziel

Der CDP-Host läuft ohne `--disable-site-isolation-trials`, wie ein normales
Chrome: iframes fremder Herkunft in einem eigenen Renderer-Prozess stehen im
Modell, lassen sich bedienen und werden von `namen-wie-accname` verglichen.

## Kontext

- Heute hält der Schalter fremde iframes im selben Prozess; nur so erreicht
  `getFullAXTree { frameId }` sie (`crates/relief-cdp/src/main.rs`,
  `launch`; `capture.rs`, `attach_frames`) [belegt im Code].
- Gegenprobe in Paket 64: Ohne Schalter fehlt das Feld aus
  `spike/fixtures/form-fremd.html` schon im Modell (1 statt 2
  Bedienelemente), damit auch in den Befunden
  (→ [spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49-55-64))
  [belegt, gemessen].
- Aktionen (`act.rs`) arbeiten über Backend-IDs der Seiten-Sitzung; in
  einem Frame mit eigener Sitzung gelten dessen IDs.

## Schritte

1. Frames in anderem Prozess über `Target.setAutoAttach` (flach) bzw.
   `Target.attachToTarget` erreichen; je Ziel `getFullAXTree`, Knoten wie
   heute mit Präfix unter dem `iframe`-Knoten einhängen.
2. DOM-Fakten dieser Frames über dieselbe Sitzung (`DOM.getDocument`,
   `DOMSnapshot.captureSnapshot`) als eigenes Dokument; Backend-IDs sind je
   Prozess vergeben, die Zuordnung zum Modell muss das berücksichtigen.
3. Aktionen, Fokus und Tab-Folge in solchen Frames über die Sitzung des
   Frames.
4. Schalter entfernen, Aufgaben 01–07 mit `form-fremd.html` grün.

## Fertig, wenn

- `cargo run -p relief-cdp -- run spike/tasks/0[1-7]*.txt` ohne
  `--disable-site-isolation-trials` meldet „0 nicht erfüllt“, und das Feld
  aus `form-fremd.html` wird verglichen.
- Prüfbefehle aus `CLAUDE.md` grün.

## Nicht Teil

Fork-Host (hat alle Frames im AXTree des Browser-Prozesses). ID-Bereich im
Shadow DOM (barrierlab-Issue aus Paket 64).

# 85 · CDP-Host: Frames anderer Prozesse, Nachtrag (Änderungssignal, Verschachtelung, Zählung)

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 70 ✓

## Ziel

Frames in einem anderen Renderer-Prozess verhalten sich im CDP-Host wie
Frames im Prozess der Seite, auch ohne eigene Aktion und verschachtelt.

## Kontext

- Seit Paket 70 läuft Chrome mit Site Isolation; Aufnahme, DOM-Fakten,
  Aktionen, Fokus und Tab-Folge gehen über eine flache Sitzung je Frame
  (`crates/relief-cdp/src/frames.rs`,
  → [spezifikation/12](spezifikation/12-produktumfang.md#formular-zusicherungen-umgesetzt-2026-09-30-pakete-42-49-55-64))
  [belegt].
- `live.rs` hört nur die Sitzung der Seite: DOM-Mutationen und Anfragen in
  einem solchen Frame lösen keine neue Aufnahme aus und halten die Ruhe
  nicht auf. Nach eigenen Aktionen nimmt der Host ohnehin neu auf; ein
  Consent-iframe, das nach dem Laden nachlädt, erscheint erst bei der
  nächsten Aufnahme aus anderem Grund [belegt im Code, Wirkung auf echten
  Seiten nicht gemessen].
- Ein iframe im selben Prozess **innerhalb** eines Frames aus einem anderen
  Prozess wird nicht eingehängt: `attach_frames` liest nur den Frame-Baum
  der Seite, `attach_remote_frames` versucht es als eigenes Ziel und zählt
  es als nicht erreichbar (`capture.rs`) [belegt im Code].
- `measure` zählt eingehängte und nicht erreichbare Frames als Summe beider
  Wege (bild.de 2/1, spiegel.de 1/1 am 2026-09-30); warum einer nicht
  erreichbar ist, bleibt offen.

## Schritte

1. Mutations- und Netzwerkereignisse der Frame-Sitzungen (zweite
   Verbindung, `frames.rs`) an `live.rs` weiterreichen.
2. Frames im Prozess eines Frames über `getFullAXTree { frameId }` in dessen
   Sitzung einhängen.
3. `measure` getrennt nach Weg zählen und die nicht erreichbaren Frames auf
   bild.de und spiegel.de erklären.

## Fertig, wenn

- Eine Testseite (`url: server:…`), deren fremdes iframe nach dem Laden
  per Zeitgeber ein Feld einfügt, zeigt es beim nächsten Befehl ohne eigene
  Aktion.
- Prüfbefehle aus `CLAUDE.md` und Aufgaben 01–09 grün.

## Nicht Teil

Fork-Host (hat alle Frames im AXTree des Browser-Prozesses).

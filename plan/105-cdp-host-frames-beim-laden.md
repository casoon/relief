# 105 · CDP-Host: Frames anderer Prozesse schon beim Laden anhängen

**Umgebung:** Cloud · **Phase:** Linie B · **Abhängig von:** 85 ✓

## Ziel

Frames in einem anderen Renderer-Prozess melden Anfragen und Mutationen
schon beim Laden, nicht erst ab der ersten Aufnahme; die Ruhe nach dem
Laden endet dann so früh wie bei Frames im Prozess der Seite.

## Kontext

- Seit Paket 85 hängt `frames.rs` einen solchen Frame bei der ersten
  Aufnahme an und gibt ab dann seine Ereignisse an `live.rs`
  (→ [spezifikation/12](spezifikation/12-produktumfang.md)) [belegt].
- `measure` über einen lokalen Server (2026-09-30): Ruhe nach dem Laden
  `frame-nachladen.html` 1112 ms, `form-fremd.html` 1063 ms, beide mit
  einem iframe in einem anderen Prozess; `with-iframe.html` (iframe im
  Prozess der Seite) 152 ms [belegt].
- Vermutung: Die Sitzung der Seite meldet den Beginn der Anfrage für das
  Dokument des iframes, das Ende kommt nur in der Sitzung des Frames; die
  Anfrage gilt als ausstehend, bis sie nach `LONG_REQUEST` (1 s) als
  Dauerverbindung zählt [Annahme, nicht einzeln geprüft].

## Schritte

1. Prüfen, in welcher Sitzung `requestWillBeSent` und `loadingFinished`
   des iframe-Dokuments ankommen.
2. Frames beim Entstehen anhängen (`Target.setAutoAttach` mit `flatten`
   auf der zweiten Verbindung für die Seite oder `Target.setDiscoverTargets`
   und Anhängen je `iframe`-Ziel), Nummerierung für `frames::encode` beibehalten.
3. Ruhe nach dem Laden mit `measure` vorher/nachher.

## Fertig, wenn

- `measure` zeigt für `form-fremd.html` eine Ruhe nach dem Laden wie für
  `with-iframe.html` (± 100 ms), ohne verfehlte Erwartung in 01–09 und 15.

## Nicht Teil

Fork-Host.

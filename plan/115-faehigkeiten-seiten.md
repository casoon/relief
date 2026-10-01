# 115 · Fähigkeitsprofile: OS-Vorbelegung, Kontrast und Bewegung für Webseiten

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** Linie A · **Abhängig von:** 41 ✓

## Ziel

Die Wirkungen aus Paket 41 (→ spezifikation/08, „Umsetzung (Paket 41)“)
über das Panel hinaus auf Webseiten ausdehnen und einen Startwert aus den
Systemeinstellungen vorschlagen.

## Schritte

1. macOS „Bewegung reduzieren“, „Kontrast erhöhen“, Textgröße, VoiceOver
   als **vorgeschlagenen** Startwert anbieten (sichtbar, ablehnbar, nie
   automatisch als Diagnose).
2. Kontrastbedarf und Bewegung für Webseiten: `prefers-contrast`,
   `forced-colors`, `prefers-reduced-motion` je Tab setzen (Weg im Fork
   klären: WebPreferences bzw. Emulation).
3. Zoom auch im CDP-Host (`Emulation`), damit `18-faehigkeiten.txt` beide
   Hosts gleich prüft.
4. Farbunterscheidung, Spracheingabe, Schalter: Wirkung festlegen oder als
   „noch ohne Wirkung“ in der Oberfläche kennzeichnen.

## Fertig, wenn

- Eine Seite mit `@media (prefers-reduced-motion)` und `(prefers-contrast)`
  reagiert im Fork auf das Profil (Browser-Test); Vorbelegung belegt.

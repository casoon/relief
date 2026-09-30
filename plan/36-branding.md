# 36 · Name und Branding „Relief“

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** Produkt · **Abhängig von:** 19 ✓

## Ziel

Der Build heißt und erscheint als „Relief“ statt „Chromium“ (→
[spezifikation/12](spezifikation/12-produktumfang.md#grundausstattung-eines-browsers-annahme)).

## Schritte

1. Chromiums vorgesehenen Weg für Produktnamen nutzen
   (`chrome/app/theme/chromium/BRANDING`, Symbole, Bundle-ID), als Patches
   in `fork/patches/` mit Begründung.
2. Eigenes Profilverzeichnis, damit Relief neben Chrome/Chromium läuft.
3. Platzhaltersymbol; das endgültige Symbol ist eine Gestaltungsfrage.

## Fertig, wenn

- `out/Relief/Relief.app` startet, Menüleiste, Über-Dialog und Dock zeigen
  „Relief“; `relief_browsertests` laufen weiter.
- Zahl der Patches außerhalb `//relief/` in 01 nachgeführt.

## Nicht Teil

Signatur, Notarisierung, Updates (→ 37).

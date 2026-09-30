# 111 · Branding: übersetzte Texte und Symbol

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** Produkt · **Abhängig von:** 36 ✓

## Ziel

Die Reste von „Chromium“ nach Paket 36 beseitigen
([spezifikation/01](spezifikation/01-chromium-integration.md#name-und-branding-paket-36-belegt)).

## Stand

- Übersetzte Texte in `chrome/app/chromium_strings.grd` nennen „Chromium“
  wörtlich (z. B. „Über Chromium“, „Hilfe für Chromium aufrufen“, Copyright
  „Die Chromium-Autoren“). Eine Änderung in der `.grd` ändert die
  Nachrichten-ID; die Übersetzungen in `chrome/app/resources/*.xtb` passen
  dann nicht mehr und die Oberfläche fällt auf Englisch zurück.
- Symbol (`chrome/app/theme/chromium/mac/`: `app.icns`, `Assets.car`,
  `AppIcon.icon`) ist noch das von Chromium.

## Schritte

1. Weg für die Texte entscheiden: Ersetzen in `.grd` und allen `.xtb`
   per Skript beim Anwenden (`fork-apply.sh`) oder eigene Strings-Datei;
   Patch-Umfang gegen Rebase-Aufwand abwägen.
2. Platzhaltersymbol (Binärdateien als Patch oder Kopie aus `fork/`).

## Fertig, wenn

- Über-Seite und Menüs zeigen auf Deutsch und Englisch „Relief“.
- Dock zeigt ein eigenes Platzhaltersymbol; `relief_browsertests` grün.

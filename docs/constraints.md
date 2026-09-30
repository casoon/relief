# Rahmenbedingungen

## Festgelegt

- **Local-first:** Browserdaten, DOM-Inhalte, Formularwerte, Passwörter und
  Screenshots gehen nie automatisch an externe KI-Dienste. Cloud-Modelle nur
  mit eigenem API-Key des Nutzers und Zustimmung; sensible Felder werden vor
  jeder externen Übertragung ausgeschlossen.
- **Keine Hardware-Voraussetzung:** Relief ist ohne jedes Modell voll
  benutzbar. Lokale Modelle sind optional, nie Voraussetzung.
- **Plattformen:** macOS, Windows, Linux. Plattformspezifischer Code nur in
  Adaptern (Sprache, Screenreader-Koexistenz, Schlüsselspeicher).
- **OS-Accessibility bleibt funktionsfähig** (VoiceOver, UIA, AT-SPI).
- **Upstream-Kompatibilität** ist zentrale Anforderung: Änderungen außerhalb
  `//relief/` minimal, dokumentiert, durch Integrationstests nach jedem
  Upstream-Merge abgesichert.
- **Upstream-Takt:** Basis bis zum Go/No-Go der Phase 0 ist Chromium
  154.0.8037.58 (`fork/UPSTREAM`). Danach Rebase alle vier Wochen auf das
  neueste Stable-Tag (Chromium bringt seit Milestone 152 alle zwei Wochen
  einen Milestone, also jeden zweiten). Sicherheitsreleases eines Milestones
  nur vor Arbeit mit fremden Websites (Messreihen, Demos). Nach jedem Rebase
  laufen die Integrationstests der Integrationspunkte. Zahlen:
  `plan/spezifikation/01-chromium-integration.md`, „Fork-Strategie“.
- **Forschungsbuild ist kein Distributionsmodell:** Der Vier-Wochen-Takt ist
  nur für die kontrollierte Forschung zulässig. Eine Verteilung an Dritte
  setzt signierte automatische Updates, Recovery und eine kürzere verbindliche
  Sicherheitsfrist voraus (→ `plan/37-updates-und-auslieferung.md`).
- **Build-Hosts:** macOS ARM (Apple M4) für Entwicklung. Windows lässt sich
  vom Mac cross-bauen (Windows-SDK einmal auf einem Windows-Rechner
  paketieren), Testen braucht Windows. Linux braucht einen x86-64-Host.
  Belege in `plan/spezifikation/01-chromium-integration.md`.
- **Forschungsprojekt:** nicht als Alltagsbrowser gedacht; Verteilung,
  Code-Signing, DRM und Auto-Update sind zurückgestellt.

## Nicht-Ziele Version 1

Keine eigene Speech-Engine, keine eigene LLM-Inferenz, keine Braille-Treiber,
kein Ersatz von OS-Accessibility-APIs oder Screenreadern, keine Änderung an der
Rendering-Engine, keine automatische Reparatur beliebiger Seiten, kein
WCAG-Konformitätsversprechen.

## Offen (Annahmen, nicht bestätigt)

- Web-Speech-Spracherkennung ist im Fork ohne Google-Keys nicht verfügbar →
  Spracherkennung muss OS-nativ oder lokal laufen (in Phase 0 zu bestätigen).

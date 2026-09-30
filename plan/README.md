# plan/

Zwei Sorten Inhalt:

- **`spezifikation/`**: Referenzrahmen, also Zielbild, Architektur,
  Datenmodell, Regeln und Messergebnisse. Wird fortgeschrieben, nicht „erledigt".
  Einstieg: [spezifikation/00-ueberblick.md](spezifikation/00-ueberblick.md).
- **Wurzel (`NN-*.md`)**: Backlog, eine Datei je Arbeitspaket. Übersicht und
  Reihenfolge in [status.md](status.md). Erledigt und validiert: Zeile in
  `status.md` raus, fachliche Substanz in `spezifikation/` falten, Datei löschen.

Gültige Entscheidungen und Rahmenbedingungen stehen in `docs/decisions.md` und
`docs/constraints.md`. Das Ursprungskonzept ist vollständig in
`spezifikation/` aufgegangen und liegt nur noch in der Git-Historie.

## Nummern

| Bereich | Inhalt |
|---|---|
| 10–19 | Phase 0: Machbarkeit bis zum technischen Go/No-Go |
| 20–21 | Phase 1: Semantic Inspector |
| 22–25 | Phase 2: Interaction |
| 26 | Phase 3: Sprache |
| 27–28 | Phase 4: KI |
| 29 | Phase 5: Semantic View |
| 30–35 | Querschnitt |
| 36–41 | Produkt und Linie Assistenz (→ spezifikation/12) |
| 42–45 | Linie Prüfen (→ spezifikation/12) |
| 46 | barrierlab |
| 47–48 | Querschnitt: Relief-Oberfläche und Sicherheitsgrenzen |
| 58 | Querschnitt: Sicherheitsgrenzen in Hosts (Nachtrag zu 48) |
| 70 | Linie Prüfen: CDP-Host mit Site Isolation (Nachtrag zu 64) |
| 75–76 | Querschnitt: Nachträge zu 58 (Formularziel im Fork, Werte außerhalb der Rückfrage) |
| 90 | zurückgestellt |

## Jedes Paket hat

- Kopfzeile: **Umgebung** (Cloud · M4 · Cloud schreibt, M4 baut), **Phase**,
  **Abhängig von**
- **Ziel**, bei Bedarf **Kontext** und **Schritte**
- **Fertig, wenn**: prüfbar, mit Befehl oder Messung
- **Nicht Teil**: Abgrenzung zu anderen Paketen

## Arbeit in der Cloud

Regeln für Cloud-Sessions stehen in `../CLAUDE.md` (Branch je Paket, PR,
Prüfbefehle, Aufräumen). Starten, ein Paket je Session:

```bash
claude --cloud "Arbeite plan/12-snapshot-tests.md ab, nach CLAUDE.md."
```

Mehrere Sessions dürfen parallel laufen, wenn ihre Pakete nicht voneinander
abhängen (siehe Spalte „Abhängig von" in `status.md`).

Nicht in die Cloud gehören:
- **Chromium bauen und im Fork messen**: 30 GB Platte reichen nicht.
- **Echte Websites aufnehmen**: Headless-Chrome in der Cloud ist gegen fremde
  Seiten unzuverlässig.

Solche Schritte sind mit „M4" markiert.

Die Umgebung auf claude.ai braucht:
- das Setup-Skript `scripts/cloud-setup.sh` (Eintrag siehe
  `docs/project-state.md`, „Cloud-Umgebung“);
- für Pakete mit Chromium-Quellcode (17) Netzzugriff auf
  `chromium.googlesource.com` oder `github.com`;
- für Paket 28 einen API-Key als Umgebungsvariable.

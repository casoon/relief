# Relief

Chromium-basierter Forschungsbrowser, der Webseiten über den
Accessibility-Tree semantisch versteht und Bedienung an Fähigkeiten anpasst.
Ist-Zustand: `docs/project-state.md`. Zielbild und Arbeitspakete: `plan/`.

Sprache in Code-Kommentaren, Doku, Plan und Commits: **Deutsch**.

## Arbeitsweise

- Genau das tun, was das Arbeitspaket verlangt; keine spekulativen
  Verbesserungen, kein Umbau außerhalb des Pakets.
- Bestehende Muster im Code wiederverwenden, bevor neue Abstraktionen entstehen.
- Annahmen ausdrücklich nennen. Gibt es mehrere gleichwertige Wege mit
  unterschiedlichen Folgen: im PR begründet entscheiden oder als offene Frage
  markieren, nicht stillschweigend wählen.
- Keine neuen Abhängigkeiten ohne Begründung im PR.
- Keine defensiven Prüfungen für unmögliche Zustände.
- Commits ohne `Co-Authored-By`-Zeilen.

## Ein Arbeitspaket abarbeiten (Cloud-Session)

1. `plan/README.md` und `plan/status.md` lesen, dann das Paket `plan/NN-*.md`.
   Die Kopfzeile nennt Umgebung und Abhängigkeiten. Ist eine Abhängigkeit
   nicht erledigt oder verlangt das Paket den M4 (Chromium-Build), abbrechen
   und das im Ergebnis sagen.
2. Branch `plan/NN-kurzname`, ein Paket je Branch und PR.
3. „Fertig, wenn“ des Pakets erfüllen; die dort genannten Prüfbefehle laufen
   lassen und ihre Ausgabe im PR zusammenfassen.
4. Aufräumen im selben PR (Skill `plan-verzeichnis`): Zeile in
   `plan/status.md` aktualisieren; ist das Paket erledigt, seine fachliche
   Substanz in die passende `plan/spezifikation/`-Datei falten und die
   Paketdatei löschen. Neue offene Punkte als neues Paket mit freier Nummer
   anlegen, nicht in ein erledigtes schreiben.
5. `docs/` nachziehen, wenn der Ist-Zustand sich geändert hat (Skill
   `living-docs`): Ist-Zustand beschreiben, keine Historie.
6. Im PR klar trennen: belegt (mit Datei/Zeile oder Messung) · Entscheidung ·
   Annahme · offen.

## Prüfbefehle

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Browser-Tests (brauchen Chrome, `CHROME` setzt den Pfad):

```bash
cargo run -p relief-cdp -- run spike/tasks/01-shop-clean.txt spike/tasks/02-shop-broken.txt spike/tasks/03-form.txt spike/tasks/04-iframe.txt spike/tasks/05-intents.txt
```

Die Zusammenfassung am Ende muss „0 nicht erfüllt“ melden; dieselben Prüfungen
laufen in der CI (`.github/workflows/ci.yml`). In Cloud-Sessions richtet
`scripts/cloud-setup.sh` (Setup-Skript der Umgebung) Chrome ein.

`spike/tasks/10-real.txt` und `11-korpus.txt` greifen auf echte Websites zu —
in Cloud-Sessions nicht verlässlich (Netz), nur lokal.

## Grenzen der Cloud-Umgebung

Ubuntu, 4 vCPU, 16 GB RAM, ~30 GB Platte. **Kein Chromium-Checkout mit
Build** (Checkout ohne Historie plus Build: 39 GB, Erstbuild auf dem M4 gut
zwei Stunden). Chromium-Quellcode nur lesend über einen
Sparse-Checkout (Rezept in `plan/spezifikation/01-chromium-integration.md`).
C++ für den Fork darf in der Cloud geschrieben werden, gebaut und gemessen
wird auf dem M4 — das Paket sagt, welcher Teil wohin gehört. Cloud-Sessions erkennen sich an `CLAUDE_CODE_REMOTE=true`.

## Feste Architekturregeln

- `crates/relief-interaction` und alle künftigen Kern-Crates sind
  **browserfrei**: keine CDP-, chromiumoxide- oder Chromium-Typen.
- Browseraktionen entstehen nur über validierte `ActionPlan`s; ein Modell
  schlägt höchstens vor.
- Jede Aussage an Nutzende trägt ihre Herkunft (`Certainty`); Unsicheres wird
  nie als Tatsache ausgegeben.
- Gemeinsame Bausteine kommen aus `casoon/barrierlab` (crates.io); dort nichts
  duplizieren.
- Weitere gültige Entscheidungen: `docs/decisions.md`, Rahmen:
  `docs/constraints.md`.

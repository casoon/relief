# 142 · MCP-Server über `Relief.*`

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** Produkt (→ spezifikation/12) · **Abhängig von:** 45 ✓, 48 ✓, 58 ✓

## Ziel

KI-Assistenten (MCP-Clients wie Claude) bedienen Relief über das Model
Context Protocol, ohne die Sicherheitsgrenze zu umgehen: Der Assistent
schickt Befehle in Sprache und liest das Seitenmodell; Relief löst auf,
validiert, stuft das Risiko ein und fragt zurück. Bestätigt wird nur von
der Nutzerin oder dem Nutzer in Relief, nie vom Assistenten.

## Kontext

- Die Domäne `Relief.*` kennt heute nur `getPageModel` und `assert`
  (`crates/relief-bridge/src/devtools.rs`, → spezifikation/12,
  „Playwright-Anbindung“). Befehle laufen im Fork über
  `ReliefTabHelper::RunCommand` und `relief_interaction::session`, nicht
  über CDP.
- Ein MCP-Client ist für Relief wie ein Modell: Er schlägt höchstens vor
  (→ docs/decisions.md, „Ein Modell führt keine Browseraktion aus“). Seine
  Eingaben sind nicht vertrauenswürdig; was er über die Seite sagt, senkt
  kein Risiko (→ spezifikation/07, Regressionsmatrix).
- Bestätigungstoken sind einmalig, an den gezeigten Plan gebunden und
  werden mit „!“ bzw. „ja“ eingelöst (→ spezifikation/07,
  „Bestätigungstoken“). Gäbe der MCP-Weg „!“ weiter, könnte der Assistent
  seine eigenen Rückfragen beantworten.
- Seiteninhalt geht an den Assistenten und damit meist an ein
  Cloud-Modell. Sensible Werte sind in Antworten schon verdeckt (Paket 58).
- Anlass: Recherche zu Barrierefreiheit und MCP (2026-10-10). Relief ist
  dort die Schicht unter dem vorgeschlagenen „Accessibility MCP Server“,
  mit validierten Aktionen statt Playwright-Klicks.

## Schritte

1. Entscheiden und in spezifikation/07 festhalten, wo eine Rückfrage aus
   dem MCP-Weg bestätigt wird. Vorschlag: nur im Relief-Side-Panel (oder
   per Sprache), der MCP-Weg bekommt die Rückfrage als Antwort und kann
   sie nicht einlösen; „!“ und „ja“ über MCP liefern „keine Bestätigung
   über diesen Weg“. Offen: ob der Assistent danach auf das Ergebnis
   warten darf (Ereignis) oder erneut fragen muss.
2. `Relief.runCommand` (Parameter `command`, Ergebnis: Antworttext,
   Ergebnisart, offene Rückfrage ja/nein) in `devtools.rs` und im Fork über
   denselben Weg wie die Befehlsleiste; Quelle der Eingabe im Security-Log
   (`source: "mcp"`).
3. MCP-Server als eigener Prozess, der Relief über CDP anspricht
   (`--remote-debugging-port`), mit drei Werkzeugen:
   `relief_page_model` → `getPageModel`, `relief_command` → `runCommand`,
   `relief_assert` → `assert`. Keine Werkzeuge für Klicks auf Koordinaten,
   Selektoren oder JavaScript. Jede Antwort trägt die Herkunft
   (`Certainty`) so, wie Relief sie liefert.
   Offen: Sprache und Ort. Vorschlag: Node unter `examples/mcp/` neben
   `examples/playwright/`, einzige Abhängigkeit das offizielle MCP-SDK
   (Begründung im PR); Alternative: Rust-Crate mit `rmcp`.
4. Doku: Einrichtung in einem MCP-Client in `docs/project-state.md`;
   Datenfluss (was an den Assistenten geht) in spezifikation/07.

## Fertig, wenn

- `devtools.rs`-Tests: `Relief.runCommand` beantwortet „was ist hier“;
  „!“ und „ja“ über diesen Weg lösen keine offene Rückfrage ein.
- `relief_browsertests --gtest_filter=*Mcp*` (M4): Befehl über CDP auf
  `spike/fixtures/form-clean.html` → Rückfrage zum Absenden; „!“ über CDP
  → kein Absenden; „ja“ im Panel → genau ein Absenden.
- Ein MCP-Client listet die drei Werkzeuge und erledigt „was ist hier“
  und „fülle Name mit Test“ gegen den eigenen Build (Aufruf und Ausgabe
  im PR).
- Prüfbefehle aus `CLAUDE.md` grün.

## Nicht Teil

- Von Websites erklärte Werkzeuge (WebMCP, `navigator.modelContext`) als
  zusätzliche Quelle lesen: eigenes Paket, falls gewünscht.
- Ein Modell in der Runtime (→ 28, 34) und Sprachausgabe über MCP.
- Veröffentlichung als npm-Paket oder in einem MCP-Verzeichnis.
- CDP-Host (`relief-cdp`): nur der Fork bekommt `runCommand`.

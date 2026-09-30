# 00 · Überblick

> Eine normale Website ist eine Fläche.
>
> **RELIEF**
>
> macht ihre Struktur erfahrbar.

**Relief** ist ein Chromium-Fork, der Webseiten semantisch
versteht und Darstellung und Bedienung an die Fähigkeiten der Nutzerin oder
des Nutzers anpasst.

**Nicht** das Ziel: „ein Browser mit eingebautem Screenreader". VoiceOver, NVDA
usw. bleiben voll funktionsfähig — Relief ergänzt den nativen
Accessibility-Pfad, es ersetzt ihn nicht.

## Zielsatz

> Eine bereits semantisch brauchbare Website erheblich leichter zugänglich
> machen und bei unvollständiger Semantik dort assistieren, wo eine
> Rekonstruktion mit ausreichender Sicherheit möglich ist.

„Jede kaputte Website automatisch barrierefrei machen" ist ausdrücklich kein
Ziel (nicht realistisch).

## Zielbild

```
Blink / Rendering
      │
      ▼
Chromium AXTree ──── DOM/Layout bei Bedarf
      │                     (nativer Pfad → OS-Accessibility bleibt)
      ▼
Chromium-Adapter (C++)      ── einzige Stelle, die Chromium-Typen kennt
      │  Bridge (FFI oder IPC, siehe 02)
      ▼
Relief Semantic Runtime (Rust, browserfrei)
  Normalisierung · Problemerkennung · Interaction Graph ·
  Action Model · Capability Profile
      │
      ├── deterministisch sicher ──┐
      └── unklar → AI Resolver ────┤  (Fallback, siehe 06)
                                   ▼
                          Interaction Model
                 ┌─────────────┼──────────────┐
             Visual Assist  Speech Assist  Action Assist
```

## Ausgangslage: der Rust-Core existiert bereits

Die Barrierefreiheits-Bausteine liegen im Monorepo
**`casoon/barrierlab`**.
Für dieses Projekt relevant:

| Crate | Was es schon kann | Bedeutung für Relief |
|---|---|---|
| `a11y-perception` | `AXTree`/`AXNode`, `AXSnapshot`, `AXTreeDiff`, `linearize()` → Lesereihenfolge; browserfrei, nur `serde` | fast genau die Schicht „Normalisierung + Snapshot + Änderungsdiff". Aktuell CDP-förmig (String-IDs, CDP-Properties). |
| `accname` | Accessible-Name-Berechnung nach accname 1.2 / HTML-AAM | für DOM-Fallback und zum Gegenprüfen von Chromium-Namen |
| `a11y-rules` + `a11y-report` | 42 Regeln über drei Fähigkeits-Tiers, `Outcome` (Fail/Review/Pass/Untested) × `Severity` | „erkannte Probleme" im Inspector (Phase 1) |
| `a11y-dom` | DOM-förmiges Dokumentmodell mit Tiers `Semantics`/`Rendering`/`Interaction` | Regeln brauchen Attribute, die der AXTree nicht hergibt (z. B. `tabindex`) |

### Relief ersetzt den Reader [Entscheidung 2026-09-30]

`barrierlab/docs/consumers.md` führt einen **„Reader-Host — noch kein
Repository"** als künftigen Konsumenten von `a11y-perception`, gedacht als **Prüfwerkzeug** für
Entwickler/Audits: Snapshot → handeln → Snapshot → Diff, headless,
CI-tauglich.

Bis 2026-09-30 galt: Relief ist nicht dieser Host. Seitdem gilt: **Relief
ersetzt den Reader.** Relief hat zwei Linien über demselben Kern, Assistenz
für Endnutzer und Prüfen im echten Browser (→ 12).

- Gemeinsam genutzt aus barrierlab: `a11y-perception`, `accname`,
  `a11y-rules`, `a11y-report`.
- Was ein weiteres Werkzeug ebenfalls braucht (Kandidaten: Interaction Graph
  für aufgabenbasierte Journeys, VoiceOver-Treiber), wandert nach
  barrierlab-Regel „zwei Konsumenten, dann Bibliothek" nach barrierlab (→ 12,
  „barrierlab einbinden“).
- Relief ersetzt in `barrierlab/docs/consumers.md` den Reader-Host
  (casoon/barrierlab#29).

Die barrierlab-Regel „Browserfrei bis L3, Pakete nehmen Daten entgegen, sie
holen sie nicht" gilt für den Relief-Kern unverändert.

Zusätzlich: `auditmysite` holt den nativen AXTree bereits über CDP aus Chrome
und wurde an 133 Seiten / 692 Journey-Instanzen gemessen — Testkorpus und
CDP-Erhebung existieren also schon (→ 10; CDP-Spike-Ergebnis → 09).

## Erfolg in Nutzersicht (MVP)

Auf einer normalen, nicht für Relief gebauten Website kann jemand:

1. Seitenstruktur erfassen
2. Inhalte navigieren
3. Links und Buttons finden und auslösen
4. Formulare verstehen, ausfüllen, absenden
5. Dialoge, Menüs, dynamische Zustandsänderungen bedienen
6. die Seite per Sprache erkunden
7. Aktionen per Sprache ausführen
8. Darstellung an die Sehfähigkeit anpassen
9. ohne Maus navigieren
10. fehlerhafte, aber rekonstruierbare Semantik teilweise kompensiert bekommen

Und Relief **erkennt, wenn es etwas nicht zuverlässig verstanden hat** (→ 03).
Technische MVP-Kriterien: → 09.

## Erfolg als Produkt, nicht nur als Technik

Der technische Durchstich beweist, dass Relief gebaut werden kann. Er beweist
noch nicht, dass Relief eine Aufgabe für Betroffene besser löst als die heute
verfügbare Kombination aus Browser, Betriebssystem und Assistenztechnik.

Vor dem Ausbau zum Fork braucht das Projekt deshalb zusätzlich einen
**Nutzennachweis**:

1. Eine primäre Nutzergruppe und ein enges Ausgangsproblem festlegen. Relief
   kann langfristig mehrere Fähigkeitsprofile bedienen; der erste MVP darf aber
   nicht gleichzeitig Screenreader, Sprachsteuerung, Low-Vision-Werkzeug und
   kognitive Vereinfachung ersetzen wollen.
2. Für 3–5 repräsentative Aufgaben die heutige Baseline erfassen: verwendete
   Hilfsmittel, Task Completion, Abbrüche, Fehlaktionen, Hilfebedarf und
   subjektives Vertrauen.
3. Denselben Test mit dem kleinsten Relief-Prototyp wiederholen. Ein Feature
   gilt nur dann als MVP-Kandidat, wenn es die Baseline messbar verbessert oder
   eine zuvor unlösbare Aufgabe ermöglicht.
4. Betroffene nicht erst in Phase 3 testen, sondern ab dem Interaction-Proof
   bezahlt in Aufgabenwahl, Prototyping und Auswertung einbeziehen.

Die erste Positionierung sollte daher nicht „Accessibility-Browser für alle“
lauten, sondern nach der Nutzervalidierung die Form annehmen:

> Für **[primäre Nutzergruppe]** macht Relief **[konkrete Web-Aufgabe]** auf
> **[Seitenklasse]** verlässlicher als **[heutige Alternative]**.

Dieser Satz ist bis zur Nutzervalidierung bewusst offen. Er ist neben dem
technischen Go/No-Go das zweite notwendige Erfolgskriterium (→ 09, 10).

## Kapitel

| Datei | Inhalt |
|---|---|
| [01-chromium-integration.md](01-chromium-integration.md) | Pipeline, Integrationspunkt, relevante Klassen, Vorarbeiten in Chromium, Fork-Folgen |
| [02-rust-core-und-bridge.md](02-rust-core-und-bridge.md) | Crate-Schnitt, Wiederverwendung barrierlab, FFI/IPC-Optionen |
| [03-semantisches-datenmodell.md](03-semantisches-datenmodell.md) | `SemanticNode`, KNOWN/INFERRED/UNCERTAIN, Evidence, Quellenpriorität, Delta-Format (umgesetzt in `relief-model`) |
| [04-interaction-graph.md](04-interaction-graph.md) | Knoten, Kanten, Identität, Seitentypen |
| [05-intents-und-aktionen.md](05-intents-und-aktionen.md) | Intent-Katalog, Validierungskette, Risikoklassen |
| [06-ki-und-vision.md](06-ki-und-vision.md) | AI Boundary, Resolver, Vision-Kaskade |
| [07-privacy-und-sicherheit.md](07-privacy-und-sicherheit.md) | Privacy Boundary, Threat Model |
| [08-assistenz-und-capabilities.md](08-assistenz-und-capabilities.md) | Capability Profile, Visual/Speech/Action Assist, Semantic View |
| [09-phasen-und-kriterien.md](09-phasen-und-kriterien.md) | Phasen 0–5, MVP-Kriterien, Nicht-Ziele, Go/No-Go |
| [10-teststrategie.md](10-teststrategie.md) | Task-Completion-Tests, Korpus, Upstream-Regressionen |
| [11-nutzerstudie.md](11-nutzerstudie.md) | Nutzennachweis: Zielgruppe, Hypothese, Ablauf, Aufgaben, Messbogen, Einwilligung, Rekrutierung |
| [12-produktumfang.md](12-produktumfang.md) | Abgrenzung, Grundausstattung, Linie Assistenz, Linie Prüfen (ersetzt den Reader), barrierlab einbinden |
| [13-user-agent-accessibility.md](13-user-agent-accessibility.md) | Verbindliche Accessibility-Baseline für Reliefs eigene Browser-Oberfläche |

## Kennzeichnung in allen Kapiteln

Jede Aussage ist gekennzeichnet:

- **[belegt]** — im Code (barrierlab) oder in Chromium-Quellen nachgesehen
- **[Entscheidung]** — festgelegt, siehe auch `docs/decisions.md`
- **[Annahme]** — plausibel, nicht geprüft
- **[validieren]** — muss in Phase 0 experimentell/gegen Quellcode bestätigt werden

Die Chromium-Integrationspunkte in 01 und der Rust-Build in 02 sind gegen
Chromium 154.0.8037.58 belegt; übrige Chromium-Aussagen bleiben
**[validieren]**, wo so markiert.

## Geklärt (2026-09-24)

- **Plattformen:** macOS, Windows, Linux als Ziel; macOS zuerst (→ 01).
- **Charakter:** Forschungsprojekt, Ausgang offen. Verteilung, Signing,
  DRM, Auto-Update sind zurückgestellt.
- **Lizenz:** MIT (→ `docs/decisions.md`).
- **CDP-Spike vor dem Fork:** ja, erledigt (→ 09, Ergebnis).
- **Modelle:** keine Vorgaben; gute Hardware darf nicht vorausgesetzt werden,
  deshalb stufenweise bis hin zu Cloud-APIs mit eigenem Key (→ 06, 07).

- **Reader-Host:** Relief ersetzt den geplanten Reader (s. o., → 12).
- **Arbeitsname:** Relief.

## Offene Fragen an den Projektinhaber

Keine.

# 06 · KI und Vision

## Rolle [Entscheidung]

KI ist Fallback und Interpretationsschicht, nicht primäre Wahrheitsquelle.
Ist Semantik eindeutig vorhanden, darf kein Modell erforderlich sein.
`<button aria-label="Warenkorb">` braucht keine KI.

## Einsatzfelder

Fehlende Accessible Names · unklare Icons · schlecht ausgezeichnete Widgets ·
komplexe SVGs · Canvas · Diagramme · Seitentyp bei unklaren Signalen ·
Zusammenfassungen · natürliche Sprache / Intent Parsing (Phase 4).

## Kaskade

```
Semantic Engine
  ├─ Known                     → fertig
  ├─ Heuristik (Rust)          → Inferred, wenn über Schwelle
  ├─ DOM-/Layout-Kontext + LLM → Inferred / Uncertain
  └─ Screenshot-Ausschnitt + Vision-Modell → Inferred / Uncertain
```

Vision-Beispiel: „Es scheint sich um ein Balkendiagramm mit Umsätzen Januar bis
Dezember zu handeln." — immer als *Inferred/Uncertain* ausgegeben.

## Resolver-Eingabe

Eng begrenzt auf den Knoten und seine Nachbarschaft, nicht die ganze Seite,
**vorher** durch den Privacy-Filter (→ 07). Umgesetzt ist der AX-Kontext:
Vorfahren, Nachbarn, sichtbarer Text, URL, Bounds (→ „Resolver fehlender
Namen“). Nicht gebaut [Annahme, ob nötig]: DOM-Ausschnitt, SVG-Semantik
(`<title>`, Pfad-Fingerprint), Screenshot-Crop.

Aufgabe an das Modell: *ausschließlich fehlende Information rekonstruieren*,
Ausgabe als strukturierte Hypothese (Schema
`crates/relief-ai-contract/schema/hypotheses.schema.json`):

```json
{
  "hypotheses": [{
    "node": "t1:783",
    "property": "name",
    "value": "Warenkorb",
    "confidence": 0.94,
    "evidence": ["href=/cart", "icon: shopping cart", "parent: navigation"]
  }]
}
```

Modell (`local:<name>@<version>`) und Stand des Graphen trägt die Runtime
nach, nicht das Modell (→ Vertrag unten).

## Pflichtmetadaten jeder Inferenz [Entscheidung]

inferred value · confidence · evidence · source/model · Zeitpunkt/Version.
Landet als `Fact` im Modell (→ 03).

## Confidence ist nicht kalibriert [Annahme]

Von LLMs selbst genannte Zahlen sind keine Wahrscheinlichkeiten. Die Schwellen
für Inferred/Uncertain werden deshalb **empirisch** am Testkorpus bestimmt
(Anteil richtiger Namen je gemeldeter Stufe, → 10), nicht aus dem Modell
übernommen. Bis dahin: Modellausgaben höchstens als Inferred mit Hinweis.

## Performance [Annahme]

- Nie im Hot Path eines Updates. Inferenz läuft asynchron, Ergebnis wird
  nachgereicht (Knoten ist bis dahin `Uncertain`/ohne Name).
- Cache pro (Knoten-Fingerprint, Eigenschaft). Gleiches Icon auf vielen Seiten
  derselben Domain → einmal inferieren.
- Lazy: nur für Knoten im Fokus, im Viewport oder in der aktuellen Anfrage.

## Modellstrategie: keine Hardware-Voraussetzung [Entscheidung]

Ein lokales Modell, das eine starke GPU oder viel RAM braucht, wäre selbst eine
Barriere — Assistenztechnik läuft oft auf alten oder günstigen Geräten. Daher:

1. **Ohne jedes Modell voll nutzbar.** Inspector, Interaction Graph,
   Command-Leiste, Semantic View und Aktionen funktionieren rein
   deterministisch. KI verbessert nur die Fälle mit fehlender Semantik.
2. **Stufen, vom Nutzer gewählt** (Default: keine):

| Stufe | Was | Anforderung |
|---|---|---|
| `none` | nur deterministisch | keine |
| `os` | Modelle, die das Betriebssystem ohnehin mitbringt (z. B. Apple Foundation Models, Windows-On-Device-Modelle auf passender Hardware) | keine Installation, nur wo vorhanden |
| `local` | selbst installierte Runtime (llama.cpp, Ollama o. ä.) | vom Nutzer bewusst gewählt |
| `api` | Cloud-Modell mit **eigenem API-Key** des Nutzers (BYOK) | Internet, Key, Consent (→ 07) |

3. **Anbieteroffen:** eine Rust-Schnittstelle `ModelProvider` mit Adaptern je
   Anbieter; kein Anbieter fest eingebaut. `casoon/llmux` als möglicher
   Unterbau prüfen [validieren].
4. **Kleine Aufgaben, kleine Eingaben:** Resolver-Anfragen sind eng
   geschnitten (ein Knoten + Nachbarschaft), damit auch kleine Modelle und
   günstige API-Tarife reichen. Kosten pro Anfrage messen (→ 10).
5. Fällt die gewählte Stufe aus (offline, Key ungültig, Kontingent), fällt
   Relief still auf `none` zurück und sagt das einmal an.

## Vertrag [umgesetzt]

`crates/relief-ai-contract`, browserfrei; Rustdoc ist die Referenz. Kein
Anbieter und kein Netzwerkcode.

```text
filter(&SemanticGraph, &PrivacyContext) → FilteredInput        // einziger Weg (→ 07)
ModelRequest::resolve_missing(input) | ::parse_intent(utterance, input)
trait ModelProvider { tier() → Tier; complete(&ModelRequest, Permit) → Result<Option<ModelReply>, ProviderError> }
Tier = none (Default) | os | local | api;   NoModel: Stufe none, liefert nie etwas
Budget::new(Limits) → Budget                                                // einziger Aussteller einer Permit (→ 07)
budget.resolve_missing(provider, input) → Vec<Hypothesis>                         // none: leer
budget.propose_intent(provider, &UserUtterance, input) → Option<IntentProposal>   // none: None
budget.complete(provider, &ModelRequest) → Option<ModelReply>                     // ungeprüft, für Messläufe
```

- **Anbieter liefern nur Text** (`ModelReply { model, text }`). Zu
  `Hypothesis` oder `IntentProposal` wird er erst durch die Prüfung gegen die
  Eingabe, aus der die Anfrage entstand. Ein Verstoß verwirft die ganze
  Ausgabe.
- **Hypothese**: Knoten (`NodeRef`), Eigenschaft (`name` | `description`),
  Wert, Confidence, Evidence, Modell (`ModelId { tier, name, version }`, als
  `Source::Model("local:x@1")`), Stand (`GraphVersion` der Eingabe). Als
  `Fact` `Uncertain`, solange für das Modell keine gemessene Schwelle
  eingetragen ist (→ „Resolver fehlender Namen“).
- **Strenge Prüfung**: keine unbekannten oder fehlenden Felder (`null` statt
  Weglassen), Confidence 0–1, Wert 1–200 Zeichen, 1–8 Evidence-Einträge je
  höchstens 200 Zeichen, höchstens 50 Hypothesen, keine doppelten; der
  Knoten existiert in der Eingabe, ist nicht sensibel geschwärzt, und die
  Eigenschaft fehlt dort wirklich (Chromium-Wert vorhanden → abgelehnt,
  Quellenpriorität aus 03).
- **Intent-Vorschlag** (Schema `intent.schema.json`, Katalog aus 05): Die
  Äußerung im Vorschlag muss wörtlich die der Nutzerin sein; Ziel Pflicht,
  optional oder verboten je Intent; das Ziel existiert und nennt den Stand
  der Eingabe; Wert nur bei `set_value`, `select`, `scroll`; ein
  `set_value`-Wert muss in der Äußerung stehen; `scroll` nur
  `down`/`up`/`top`/`bottom`. Ein Vorschlag ist keine Aktion; die Runtime
  validiert danach wie in 05.
- **Risiko**: `assess_risk` nimmt das Maximum aus der Einstufung ohne und
  mit jeder Hypothese; eine Hypothese kann eine Rückfrage auslösen, nie
  ersparen (→ 05, 07).
- **Schema handgeschrieben** statt aus den Typen erzeugt: keine neue
  Abhängigkeit (`schemars`), und die Grenzen (Längen, Bereiche, `null` statt
  Weglassen) stehen in dem Schema, das ein Modell sieht. Ein Test prüft
  Felder, Pflichtfelder, `additionalProperties: false`, Enum-Werte und
  Grenzen gegen die Rust-Typen.
- **Synchron**: Die Runtime ruft Anbieter außerhalb des Update-Pfads auf
  (Thread o. ä.); keine Async-Runtime im Vertrag.

Offen:

- Rückfall auf `none` mit einmaliger Ansage bei `ProviderError`: Aufgabe des
  Aufrufers, noch nicht gebaut.
- Intent-Parsing per Modell sendet die Äußerung selbst (etwa „fülle Passwort
  mit …“). Vorschlag: Modell nur fragen, wenn der deterministische Parser
  scheitert, und Äußerungen mit Werten für sensible Felder nie senden.
- Ein Modell, das eine Äußerung falsch deutet, sie aber korrekt zitiert,
  erkennt der Vertrag nicht; dagegen steht nur die Validierung aus 05
  (Risiko, Bestätigung).

## Resolver fehlender Namen [umgesetzt, Messung offen]

`crates/relief-resolver`, browserfrei; Rustdoc ist die Referenz. Die Runtime
ruft ihn noch nicht auf.

```text
FilteredInput::excerpt(id, max_nodes) → Option<FilteredInput>   // relief-ai-contract, nimmt nur weg
resolve_node(&mut budget, provider, &input, id) → Option<Hypothesis>  // Ausschnitt (40) → Budget::resolve_missing → Name von id
anthropic::AnthropicProvider::from_env()                        // Stufe api, nur mit Feature `anthropic`
relief-resolver kalibrieren [--aufzeichnen D] [--wiedergeben D] // Trefferquote, Schwellenvorschlag, Tokens
```

**Ausschnitt.** Vorfahren des Knotens bis zur Wurzel, dazu ein Fenster von
höchstens 40 Knoten in Dokumentreihenfolge um ihn, innerhalb des kleinsten
Vorfahren mit mindestens 40 Knoten im Teilbaum. Knoten bleiben unverändert
bis auf `parent` (nächster behaltener Vorfahr); `focus` nennt den Knoten;
Modellausgaben gelten nur für behaltene Knoten.
Gemessen (belegt, `cargo run -p relief-resolver -- kalibrieren`, 18
Einträge): Nutzernachricht im Mittel 3 947 B, größte 5 266 B (Wikipedia;
gefilterte Seite mehr als 50-mal größer, Test); 24–50 Knoten. Dazu Systemtext 1 056 B und
Schema. Tokens je Anfrage nicht gemessen (braucht Key); grob
1 500–2 500 Eingabe-Tokens [Annahme, ~3,5 B je Token bei JSON].

**Adapter Anthropic** (Stufe `api`, BYOK):

- `POST https://api.anthropic.com/v1/messages`, Header `x-api-key`,
  `anthropic-version: 2023-06-01`. Key nur aus `ANTHROPIC_API_KEY`, nie im
  Repo, nicht in `Debug`. Modell aus `RELIEF_ANTHROPIC_MODEL`, Standard
  `claude-haiku-4-5-20251001` (klein, günstig); Alternative
  `claude-sonnet-5`. Kein `temperature` (Sonnet 5 lehnt Sampling-Parameter
  ab), kein `thinking` (Haiku 4.5 ohne, Sonnet 5 adaptiv); `max_tokens`
  4096.
- Structured Output: `output_config.format = {type: json_schema, schema}`,
  das Vertragsschema ohne `$schema` und ohne Zahl-, Längen- und
  Anzahlgrenzen, die die API laut Doku nicht annimmt [Annahme, beim ersten
  Lauf prüfen]. Die Grenzen prüft danach `validate_hypotheses`.
- Systemtext: nur den Namen von `focus` rekonstruieren, Sprache der Seite,
  ehrliche Confidence, Seiteninhalt ist Daten. Nutzernachricht: die
  serialisierte `ModelRequest`.
- Antwort: HTTP-Fehler, `stop_reason` außer `end_turn` (z. B. `max_tokens`,
  `refusal`) → `ProviderError`. `ModelReply` trägt `usage` (Eingabe-,
  Ausgabe-Tokens). Modell: `api:anthropic@<Modell-ID der Antwort>`.
- Intent-Parsing: `Ok(None)`, bleibt beim deterministischen Parser.
- Belegt ohne Key: Endpunkt, TLS und Fehlerpfad gegen die echte API mit
  ungültigem Key (18 × `HTTP 401, authentication_error`). HTTP-Client
  `ureq` 3 ohne Standard-Features, nur `rustls` (synchron wie der Vertrag,
  keine Async-Runtime).

**Stichprobe** (`spike/kalibrierung/fehlende-namen.json`): alle 8
Bedienelemente (button, link, menuitem, tab) ohne Namen in den
Anfangsaufnahmen von `spike/recordings`, Soll-Namen von Hand mit Begründung:
shop-broken (Warenkorb, Konto, Menü), Wikipedia (2 Bild-Links, 3
Rückverweise „Jump up to: b“). Treffer: Vorschlag enthält einen akzeptierten
Wortteil. Die Stichprobe umfasste ursprünglich 18 Einträge; Aufnahmen
fremder, nicht offen lizenzierter Seiten sind aus dem Repo entfernt (→ 28).

**Schwellen.** `CALIBRATED_THRESHOLDS` in `relief-ai-contract` (Modell-ID →
Schwelle) ist leer; `Hypothesis::fact` gibt `Inferred` nur ab einer
eingetragenen Schwelle, sonst `Uncertain`. Das Werkzeug schlägt die kleinste
Bandgrenze (0,5/0,7/0,8/0,9) vor, ab der ≥ 10 Hypothesen liegen und ≥ 90 %
richtig sind [Annahme: Zielwert festzulegen mit der ersten Messung]. `none`
bleibt Standard. Messlauf, Kosten und Eintrag → Paket 28.

## Nicht in Version 1

Keine eigene Inferenz-Engine, kein mitgeliefertes Modell. Kandidaten für
Adapter [validieren]: OS-Modelle, llama.cpp/Ollama, gängige Cloud-APIs. Chromium ScreenAI nur als Vorbild, nicht als
Baustein: die Bibliothek ist nicht quelloffen und wird über den Component
Updater verteilt (→ 01).

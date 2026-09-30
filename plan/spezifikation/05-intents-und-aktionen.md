# 05 · Intents und Aktionen

## Grundregel [Entscheidung]

Browseraktionen entstehen **ausschließlich** über validierte Aktionen der
Rust-Runtime. Ein LLM (oder Parser, oder Spracherkennung) schlägt nur vor.

```
Eingabe (Text | Sprache | Tastatur-Command)
  → Parser (Phase 2) oder LLM (Phase 4)
  → Structured Intent
  → Rust-Validierung
  → Risiko-/Berechtigungsprüfung (ggf. Rückfrage)
  → AXActionData an Chromium
  → Ergebnis am nächsten Diff prüfen → Antwort
```

## Intent-Katalog (Entwurf) [Annahme]

Abfragen (verändern nichts, immer LOW):

| Intent | Beispiel |
|---|---|
| `describe_page` | „Was ist auf dieser Seite?" |
| `list_actions` | „Was kann ich hier tun?" |
| `list_headings` / `list_landmarks` / `list_links` / `list_forms` | „Welche Überschriften gibt es?" |
| `read_region` | „Lies den Abschnitt technische Daten." |
| `inspect_control` | „Welche Größen gibt es?" |
| `where_am_i` | „Wo bin ich?" |

Aktionen:

| Intent | AX-Aktion [validieren] | Beispiel |
|---|---|---|
| `navigate_to` | Focus + ScrollToMakeVisible | „Gehe zu den technischen Daten." |
| `focus` | Focus | „Gehe zum nächsten Formularfeld." |
| `activate` | DoDefault | „Öffne den Warenkorb." |
| `set_value` | SetValue | „Trage Müller ein." |
| `select` | SetValue / Select auf Option | „Wähle Größe 43." |
| `increment` / `decrement` | Increment / Decrement | „Menge erhöhen." |
| `scroll` | ScrollToMakeVisible / Scroll* | „Scrolle nach unten." |
| `dismiss` | DoDefault auf Close / Escape | „Schließe den Dialog." |

Zielauflösung: `resolve("Warenkorb")` sucht im Graph nach Name, Rolle, Region,
Nähe zum Fokus. Mehrdeutig → Rückfrage mit Kandidaten, nie raten.

Interaktionsregeln [Entscheidung]:

- Mehrdeutige Kandidaten erhalten in der Oberfläche kurzlebige Nummern; eine
  Folgeeingabe darf Nummer oder Namen wählen. Die Nummer ist keine Knoten-ID
  und gilt nur für diese Rückfrage.
- „dieses Feld“, „hier“ und vergleichbare Bezüge dürfen nur auf den aktuellen
  Fokus oder eine ausdrücklich von Relief präsentierte Auswahl zeigen.
- „Abbrechen“ hat Vorrang vor normalen Intents und verwirft offene Rückfragen,
  Bestätigungen und noch nicht gesendete Pläne; laufende Ausgabe stoppt. Bereits
  ausgeführte Aktionen werden nicht stillschweigend rückgängig gemacht.
- Auch bei späteren Assistenten gilt: genau ein validierter `ActionPlan`, dann
  den nächsten Graph/Diff auswerten. Ein Modell erhält nie eine Folge direkter
  Browserbefehle zur autonomen Ausführung.

## Stand im CDP-Spike [belegt]

Browserfrei in `relief-interaction` (Parser `command.rs`, Auflösung
`resolve.rs`, Plan `validate.rs`, Antworten `respond.rs`), ausgeführt vom
CDP-Host; Unit-Tests je Satz und Intent, Browserprüfung in
`spike/tasks/05-intents.txt` auf `spike/fixtures/intents.html`.

| Intent | Befehle (Auswahl, de/en) | Umsetzung |
|---|---|---|
| `describe_page` | „was ist hier“ | `respond::describe` |
| `list_actions` | „was kann ich tun“ | `respond::list_actions` |
| `list_headings` | „überschriften“ | `respond::list_headings` |
| `where_am_i` | „wo bin ich“, „where am I“ | Fokus live abgefragt → Bedienelement/Überschrift, Bereich, Abschnitt, Seite |
| `read_region` | „lies den Abschnitt [X]“, „lies den Bereich X“, „read section X“ | Text bis zur nächsten Überschrift gleicher/höherer Ebene bzw. im Bereich; Unterüberschriften in `[H3 …]`, Bedienelemente außer Links danach; ohne Namen der Abschnitt am Fokus |
| `inspect_control` | „welche Größen gibt es“, „details zu X“, „inspect X“ | Optionen und Auswahl, Wert mit `valuemin`/`valuemax`, Zustände; Plural wird auf Singular zurückgeführt („Größen“ → „Größe“) |
| `navigate_to` | „gehe zur Überschrift X“, „zum Bereich X“, „nächster/vorheriger Abschnitt“, „next heading“ | `ActionKind::NavigateTo`, LOW |
| `focus` | „gehe zu X“, „nächstes/vorheriges Formularfeld“, „next field“ | `ActionKind::Focus`, LOW |
| `activate`, `set_value`, `select`, `dismiss` | „öffne X“, „fülle X mit Y“, „wähle Y [bei X]“, „schließe den Dialog“ | wie bisher |
| `increment` / `decrement` | „erhöhe X“, „X erhöhen“, „verringere X“, „decrease X“ | nur `slider`/`spinbutton`, MEDIUM; abgelehnt an `valuemin`/`valuemax` |
| `scroll` | „scrolle nach unten/oben/zum Anfang/zum Ende“, „scroll down“ | ohne Zielelement, LOW; Antwort aus Scrollposition |

Entscheidungen dabei:

- **Abschnitt** = Überschrift bis zur nächsten gleicher oder höherer Ebene.
  Bedienelemente und Fließtext tragen die vorausgehende Überschrift; daraus
  ergeben sich Abschnitt am Fokus, nächste/vorige Überschrift und nächstes
  Formularfeld. „Vorheriger Abschnitt“ aus einem Abschnitt heraus springt
  zuerst zu dessen eigener Überschrift (wie Screenreader).
- **Überschrift vor Bereich**: Heißen ein Bereich und eine Überschrift darin
  gleich (`<section aria-labelledby>`), gilt die Überschrift. Unbenannte
  Bereiche über Rollenwörter („Fußzeile“, „Hauptinhalt“ …).
- **Navigation auf nicht fokussierbare Ziele** setzt `tabindex="-1"` bis zum
  Verlassen (Muster der Sprunglinks); damit beginnt auch Tab dort.
- **Erhöhen/Verringern** über echte Pfeiltasten statt `stepUp()`: wirkt auf
  native Felder und auf ARIA-Widgets, die nur Tasten kennen — am nächsten an
  `Increment`/`Decrement` im Fork.
- **Scrollen ohne `ActionPlan`**, wie Escape beim Schließen: `ActionPlan` ist
  an ein Element gebunden, Scrollen verändert nichts (LOW).
- Fließtext für `read_region`: `StaticText` außerhalb von Überschriften,
  Beschriftungen (`LabelText`) und Bedienelementen; Linktext bleibt im Satz.
- **Schließen-Ziel** (`resolve::dismissal(graph, model, focus)`), in dieser
  Reihenfolge: (1) modaler Dialog — der mit dem Fokus, sonst der zuletzt
  geöffnete; (2) nicht-modaler Dialog, der den Fokus enthält; (3)
  aufgeklapptes Popup-Element (`expanded` + `hasPopup`) — von der Nutzerin
  geöffnet, anders als ein seit dem Laden stehendes Banner; (4) letzter
  Dialog in Dokumentreihenfolge. Im Ziel ein Schließen-Button → `activate`,
  sonst Escape. „Enthält den Fokus“ geht über die Vorfahren im Modell, über
  Frame-Grenzen (`TreeData::parent`); beim Popup zählt das gesteuerte
  Element (`aria-controls`) mit, weil der Fokus in Menüs auf einem Eintrag
  steht.
- **Escape sagt, wohin es geht:** Escape erreicht das fokussierte Element.
  Liegt es nicht im Ziel, trägt `Dismissal::Escape { target, reaches }` das
  tatsächlich erreichte Element (bzw. „das Dokument“ ohne Fokus), und der
  Host meldet es: „Der Fokus liegt nicht darin, Escape ging an …“.
  [belegt: IKEA nach „fülle Suche … mit Regal“: Ziel ist die aufgeklappte
  Such-Combobox, nicht der in Dokumentreihenfolge spätere Consent-Dialog;
  der Fokus steht auf dem Container des Consent-Banners, das meldet die
  Antwort. APG-Menübutton: Fokus auf `menuitem` im gesteuerten Menü, keine
  Warnung.]

Befund: `aria-valuetext` kommt über CDP als leerer `valuetext` an, bei nativen
Zahlenfeldern als Kopie des Werts; beides wird verworfen. Der Textwert eines
ARIA-Schiebereglers („fest“) ist im Spike nur über den Diff sichtbar.

Offen:

- `list_landmarks`, `list_links`, `list_forms` als eigene Abfragen.
- „Gehe zu den technischen Daten“ (gebeugter Name, ohne „Überschrift“)
  landet bei `focus` auf Bedienelementen und findet keine Überschrift.
- Scrollen im Fork über `ScrollToMakeVisible`/`Scroll*` und damit doch als
  `ActionPlan` (→ 24); `aria-valuetext` im Fork prüfen.

## Intent-Format [Annahme]

```json
{
  "intent": "activate",
  "target": { "node": "t1:783", "graph_version": 4121 },
  "value": null,
  "confidence": 0.97,
  "utterance": "öffne den Warenkorb"
}
```

Als Rust-Typ plus JSON-Schema umgesetzt (`relief-ai-contract`, → 06
„Vertrag“); Modellausgaben werden strikt validiert (Structured Output).
Umgesetzt ist dort auch „Eine KI darf die Einstufung nur erhöhen“
(`assess_risk`, → 07).

## Validierung in Rust

1. Knoten existiert (noch)?
2. `graph_version` aktuell, oder Knoten seitdem unverändert?
3. Aktion für diesen Knoten erlaubt (`ActionSet`)?
4. Knoten sichtbar, nicht `disabled`, nicht von Overlay verdeckt?
5. Wert zulässig (Option existiert, Eingabetyp passt)?
6. Risikoklasse bestimmen → ggf. Bestätigung.
7. Ziel-Name `Known`/`Inferred` über Schwelle? `Uncertain` → Rückfrage.

Eine Bestätigung autorisiert genau den angezeigten `ActionPlan`: Aktion,
Zielknoten, Graph-Version, Wert und — sofern vorhanden — Ziel-URL oder
Formularziel. Sie ist kurzlebig und nur einmal verwendbar. Ändert sich eines
dieser Felder oder der relevante Graph-Ausschnitt, wird der Plan verworfen und
neu bestätigt. Eine allgemeine Zustimmung wie „mach weiter“ gilt nie für
spätere oder veränderte Aktionen.

## Risikoklassen [Entscheidung: Prinzip]

| Klasse | Beispiele | Verhalten |
|---|---|---|
| LOW | lesen, navigieren, scrollen, Fokus setzen | direkt ausführen |
| MEDIUM | Auswahl ändern, Formular ausfüllen, Warenkorb verändern | ausführen, Ergebnis ansagen, rückgängig anbieten wo möglich |
| HIGH | Nachricht absenden, Bestellung, Vertrag bestätigen, Zahlung, Löschen | **explizite Bestätigung** mit Zusammenfassung dessen, was passiert |

Der Bestätigungstext wird aus dem validierten Plan und den lokalen
Browserdaten erzeugt, nicht aus einer freien Modellzusammenfassung. Er nennt
mindestens Verb, Ziel und Wert beziehungsweise Zieladresse; sensible Werte
werden passend maskiert.

### Die Einstufung ist selbst eine Inferenz [Annahme]

Ob ein Button „Bestellung auslösen" bedeutet, weiß Relief nur aus Name,
Kontext, Seitentyp. Deshalb konservativ:

- Jeder Submit eines Formulars ist mindestens MEDIUM.
- Submit auf Seitentyp Checkout/Login/Zahlung, oder Name trifft
  Signalwörter (kaufen, bestellen, zahlen, senden, löschen, bestätigen,
  kündigen …, mehrsprachig) → HIGH.
- Unbekannte Einstufung bei Aktivierung eines Buttons in Form → HIGH.
- Eine KI darf die Einstufung **nur erhöhen**, nie senken.
- Irreversible Aktionen werden nie durch KI-Inferenz allein ausgelöst,
  auch nicht nach vorheriger pauschaler Zustimmung.

**Seitentyp in der Einstufung [umgesetzt]:** `validate::plan_on_page(page,
control, kind)` = `plan` plus Seitentyp (→ 04): Ist er Anmeldung oder Kasse
— auch nur `Uncertain` —, wird jede Aktivierung, die sonst MEDIUM wäre, HIGH
mit Bestätigung und Hinweis „Seitentyp vermutlich/möglicherweise Kasse
(Evidence)“. Nur erhöhen: Links, Tabs, Baumeinträge (LOW), Ausfüllen,
Auswählen, Fokus und Navigation bleiben; ein HIGH aus dem Namen bleibt HIGH.
Bewusst grob: Auch „Passwort anzeigen“ auf der Anmeldeseite fragt nach —
lieber einmal zu oft. Der CDP-Host ruft `plan_on_page` mit `Graph::page`;
`plan` allein kennt keinen Seitentyp (für Aufrufer ohne Graph, z. B. die
Tests in `relief-ai-contract`). Belegt in `page.rs` (Unit-Tests Anmeldung,
Kasse, unsichere Kasse); keine Aufnahme ist eine Anmelde- oder Kassenseite.

## Ergebnis prüfen

Nach jeder Aktion den nächsten Diff abwarten (Timeout), dann:
Fokus bewegt? Dialog geöffnet? Fehlermeldung (`errorFor`, Live Region)
erschienen? Wert übernommen? → Antwort an die Nutzerin. „Geklickt, aber nichts
passiert" ist eine gültige und wichtige Antwort.

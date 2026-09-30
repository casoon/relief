# 04 · Interaction Graph

Kein zweiter Accessibility Tree, sondern die Antwort auf: **Was kann ein Mensch
auf dieser Seite tun?** Deterministisch aus dem Modell (→ 03) abgeleitet, soweit
möglich.

## Fragen, die der Graph beantworten muss

Wo bin ich? · Welche logischen Bereiche hat die Seite? · Welche Inhalte sind
relevant? · Welche Aktionen sind möglich, welche ist primär? · Was gehört
funktional zusammen? · Welche Zustände ändern sich? · Welche Navigation, welche
Formulare, Dialoge, Menüs, Tabellen, Listen, Widgets gibt es?

## Knotenarten (Entwurf) [Annahme]

| Art | Quelle | Beispiel |
|---|---|---|
| `Page` | Dokument + Seitentyp-Hypothese | Produktseite |
| `Region` | Landmarks, Überschriften-Abschnitte | Navigation, Main, Footer, „Technische Daten" |
| `Group` | funktionale Zusammengehörigkeit | Produkt: Titel, Bild, Preis, Größe, Warenkorb-Button |
| `Form` | `form`, oder Gruppe von Eingaben + Submit | Kontaktformular |
| `Control` | bedienbare Knoten mit Aktionen | Button, Link, Select, Textfeld |
| `Content` | lesbare Inhalte | Absatz, Bild, Preis, Tabelle |
| `Overlay` | Dialog, Menü, Popover, Toast | Cookie-Banner, Größentabelle |
| `LiveArea` | Live Regions, Statusmeldungen | Warenkorb-Zähler, Fehlermeldung |

## Kanten

`contains` · `labels` / `describes` · `controls` (Button → Menü) ·
`opens` (Button → Dialog, erschlossen aus `expanded`/`haspopup` + Diff) ·
`submits` (Button → Form) · `errorFor` (Meldung → Feld) · `next` (Lesereihenfolge,
aus `linearize()`).

## Seitentyp, Gruppen, primäre Aktion [umgesetzt]

Umgesetzt in `crates/relief-interaction/src/page.rs`, berechnet am Ende von
`Graph::build` als `Graph::page` (`Page { kind, groups, primary }`).
Deterministisch, aus Rollen, Namen, Überschriften, Formularen (nächster
`form`-Vorfahr im Modell, auch unbenannt), kurzen Texten und der Adresse des
Hauptbaums. Jede Aussage ist `Fact` mit `Source::Rule` und Evidence,
`Inferred` oder `Uncertain`, **nie `Known`**; `confidence` bleibt leer, weil
die Regeln nicht kalibriert sind (→ 10) — die Stufe ergibt sich aus der Zahl
unabhängiger Signale.

**Gruppen:**

- `Form`: Bedienelemente mit demselben `form`-Vorfahren, mindestens ein Feld
  (`textbox`, `combobox`, `listbox`, `checkbox`, `radio`, `switch`,
  `slider`, `spinbutton`). Formulare in `role=search` zählen nicht.
  Primär: erster Button mit Absende-Wort (senden, anmelden, bestellen,
  speichern, weiter, suchen, submit, sign in … ) → `Inferred`; sonst der
  letzte Button → `Uncertain`.
- `Product`: um den ersten Button mit Warenkorb-Wort („in den Warenkorb“,
  „zum Warenkorb“, „add to cart“ …), sonst Sofortkauf („jetzt kaufen“,
  „buy now“; nicht „kaufen“ allein, trifft „einkaufen“); dazu die
  Bedienelemente außer Links im selben Bereich und Abschnitt. Primär: dieser
  Button (`Inferred`). Ab drei Warenkorb-Buttons ist es eine Liste, keine
  Gruppe.

**Seitentyp**, Regeln in Vorrangfolge (die erste, die trifft, gilt):

| Typ | Regel | Stufe |
|---|---|---|
| Kasse | Signale: Pfadsegment `checkout`/`kasse`/`bezahlen`/`payment`/`zahlung`; Zahlungsfeld (Kartennummer, IBAN, CVC …); Bestellschritt (Liefer-/Rechnungsadresse, Zahlungsart) als Feld oder Überschrift außerhalb von Kopf, Navigation, Fuß, Randspalte; Bestell-Button („zahlungspflichtig bestellen“, „place order“ …) | ab 2 Signalen `Inferred`, 1 `Uncertain` |
| Anmeldung | `textbox` mit Passwort-Wort in einem Formular mit höchstens 3 Textfeldern; dazu Anmelde-Button oder Pfad `login`/`anmelden` … | mit Zusatz `Inferred`, sonst `Uncertain`; mehr Textfelder → eher Registrierung, kein Login |
| Suchergebnisse | Suchbegriff in der Adresse (`?q=`, `?suche=` …, Pfad `suche`/`search`), Ergebnis-Überschrift („Suchergebnisse“, „results for“); ausgefülltes Suchfeld nur als Zusatz | ab 2 `Inferred`, 1 `Uncertain` |
| Produkt | Produktgruppe, Preis (kurzer Text mit Ziffer und Währung), H1; ohne Button: Warenkorb-Wort als bloßer Text plus Preis | alle drei `Inferred`, sonst `Uncertain` |
| Formular | größtes Formular mit ≥ 3 Feldern, weniger als 300 Wörter in längeren Absätzen | `Inferred` |
| Artikel | H1, ≥ 300 Wörter in Absätzen ab 20 Wörtern, ≥ 15 solcher Wörter je Überschrift, < 25 % verlinkte Überschriften (Link um oder in der Überschrift: Anrisse auf Startseiten) | `Inferred` |
| Unbekannt | sonst; Evidence „keine Regel trifft zu“, ggf. „N Warenkorb-Buttons: Liste“ | `Uncertain` |

Primäre Aktion der Seite = primäre Aktion der Gruppe, die den Typ trägt
(Produkt: Produktgruppe; Formular/Anmeldung/Kasse: das Formular), sonst
keine. Artikel, Suchergebnisse und Unbekannt haben keine.

Entscheidungen:

- **Startseiten sind kein Typ.** Die Liste hat keinen; eine Startseite ist
  `Unknown`. Die Adresse `/` wird dafür nicht benutzt — die Inhaltsregeln
  (verlinkte Überschriften, Wörter je Überschrift) trennen Startseiten von
  Artikeln selbst.
- **Dashboard** fehlt vorerst (keine Aufnahme, kein Paket verlangt es).
- **Keine Bounds, kein Schema.org, kein `type`/`autocomplete`:** Die
  Aufnahmen haben weder Positionen noch DOM-Attribute. Visuelle Gewichtung
  für die primäre Aktion und `autocomplete` für Login/Kasse kommen dazu,
  wenn ein Host sie liefert (Fork: Bounds, → 01, „Umsetzung im Fork“; Felder: `FieldHint` in 07).
- **Stabilität:** Der Typ hängt nicht an Werten und Fokus; Eintippen ins
  Suchfeld macht keine Suchergebnisse (shop-clean nach „fülle Suche mit
  Laufschuhe“ bleibt Produkt). Ist ein nativer modaler Dialog offen, zeigt
  Chrome nur den Dialog — der Typ fällt dann auf Unbekannt
  (shop-clean `snapshot-03`).
- **`describe`** nennt den Typ nach dem Titel: „Seitentyp vermutlich
  Produktseite (erschlossen: …)“ bei `Inferred`, „Seitentyp möglicherweise
  …, unsicher (Hinweise: …)“ bei `Uncertain`; bei Unbekannt nichts.
- KI nur bei „Unbekannt“ (→ 06), noch nicht angebunden. Checkout/Login
  erhöht die Risikoklasse (→ 05) und ist der Seitentyp für den Privacy-Filter
  (`PageType::is_sensitive`, → 07).

Messung gegen die 13 Seiten in `spike/recordings`, Soll von Hand je Seite in
`tests/recordings.rs` (`SOLL`, Anfangsaufnahme; Startseiten Unbekannt,
APG-Beispielseiten Artikel) [belegt]:

| Soll | Seiten | Treffer |
|---|---|---|
| Produkt | shop-clean, shop-broken ×2 | 3/3 (shop-broken `Uncertain`: Button ist ein `div`) |
| Formular | form | 1/1 |
| Artikel | Wikipedia, 4 APG-Seiten | 5/5 |
| Unbekannt | with-iframe, gov.uk, 2 Korpus-Startseiten (casoon.de) | 4/4 |
| Anmeldung, Kasse, Suchergebnisse | keine Aufnahme | Unit-Tests in `page.rs` |

Seitentyp 13/13, primäre Aktion 13/13 (shop-clean „In den Warenkorb“, form
„Nachricht senden“, sonst keine). Über alle 38 Aufnahmen: abweichend nur
shop-clean mit offener Größentabelle (Unbekannt, s. o.) und die
Chrome-Fehlerseite nach „öffne cart“ (anderes Dokument). Während der
Entwicklung falsch und behoben: bahn.de als Kasse (`Uncertain`) über die
Fuß-Überschrift „Mögliche Zahlungsarten“. Die Regeln sind an 20 Seiten entwickelt
(darunter sieben inzwischen entfernte Aufnahmen fremder Seiten); die Trefferquote ist keine Aussage über fremde Seiten.

## Beispiel

```
PAGE (type: product, Inferred)
├── Region navigation
│   ├── Control link "Home"
│   ├── Control link "Produkte"
│   ├── Control button "Warenkorb"  (name Inferred 0.94)
│   └── Control link "Konto"
├── Region main
│   └── Group product
│       ├── Content heading "Nike Air Max"
│       ├── Content image
│       ├── Content price "129,00 €"
│       ├── Control select "Größe" [39..43, selected 42]
│       └── Control button "In den Warenkorb"  ★ primary
└── Region footer
```

Aus diesem Graph entsteht die Antwort auf „Was kann ich hier machen?" —
Produktinformationen lesen, Größe wählen, in den Warenkorb legen, zur
Navigation wechseln.

## Modalität je Frame [belegt]

Ein modaler Dialog sperrt **nur sein eigenes Dokument**, nicht das
Elterndokument seines iframes (`Graph::is_reachable`, `Graph::frames`).

- **Beleg:** HTML: Ist ein Dokument durch `showModal()` blockiert, wird
  „every node that is connected to document“ außer dem Dialog inert — nur
  Knoten dieses Dokuments. WAI-ARIA 1.2, `aria-modal`: macht selbst nichts
  inert; Autoren „SHOULD mark all other contents as inert“, soweit die
  Sprache es erlaubt, und AT soll die Navigation nicht mehr auf den Dialog
  begrenzen, sobald der Fokus ihn verlässt. Ein Frame-Dokument kann sein
  Elterndokument nicht inert machen; Tab erreicht es weiterhin.
- **Regel:** Ein Element ist erreichbar, wenn für sein Dokument und jedes
  Dokument, in dessen iframe es liegt, gilt: kein modaler Dialog darin offen,
  oder das Element liegt im Dialog (über `Region::parent`, der Bereichsstapel
  reicht über Frame-Grenzen). Frame-Grenze ist der `Iframe`-Knoten
  (`Graph::frames`: Baum des Frames → `Iframe`-Knoten im Elterndokument).
  Liegt das iframe im modalen Dialog des Elterndokuments, bleibt der Frame
  bedienbar.
- **Entscheidung:** Kein Sperren des Elterndokuments, obwohl ein Overlay im
  Frame die Seite oft sichtbar verdeckt: Relief bildet ab, was bedienbar ist,
  nicht was verdeckt aussieht. Verdeckung wäre eine eigene, unsichere Aussage
  aus Layout/Bounds.
- `Graph::active_modal` bleibt der zuletzt geöffnete modale Dialog über alle
  Frames (für Meldungen); `list_actions` meldet gesperrte Elemente nur, wenn
  es welche gibt, der Diff beim Öffnen „Rest seines Frames ist gesperrt“ für
  Dialoge außerhalb des Hauptbaums.
- 04-iframe: 3 von 3 Bedienelementen erreichbar, „Zum Artikel“ auflösbar;
  zeit.de (Dialog im Hauptdokument, Consent-iframe darin): unverändert 9 von
  599.

## Inkrementalität und Stabilität [validieren]

- Graph wird bei jedem Update nur für betroffene Teilbäume neu berechnet.
- **Stabilität** ist MVP-Kriterium: dieselbe Seite ohne Änderung ergibt
  denselben Graph; kleine Änderungen (Zähler im Warenkorb) dürfen Gruppen und
  primäre Aktion nicht umwerfen. Test in 10.
- SPAs: Routenwechsel ohne Navigation → Diff erkennt großen Strukturwechsel →
  Graph wird als neue „Seite" behandelt, Nutzerin wird informiert.

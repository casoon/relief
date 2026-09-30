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
- **Navigation auf nicht fokussierbare Ziele** setzt im CDP-Host
  `tabindex="-1"` bis zum Verlassen (Muster der Sprunglinks); damit beginnt
  auch Tab dort. Im Fork anders (→ „Im Fork über `AXActionData`“).
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
- `aria-valuetext` im Fork: der Mirror übernimmt einen von der Zahl
  abweichenden `kValue` als `valuetext`; im Lauf auf `intents.html` erscheint
  beim Schieberegler keiner — ob Chromium ihn dort liefert, ist ungeprüft.

## Im Fork über `AXActionData` (Paket 24) [belegt]

Der Befehlsablauf ist browserfrei und für beide Hosts derselbe
(`relief_interaction::session`): Eingabe → `Session::handle` → Antwort oder
geprüfter `ActionPlan`; nach der Ausführung und der Ruhe liefert
`Session::performed` bzw. `escaped` die Antwort aus den Ständen vorher und
nachher. Im Fork übersetzt `relief-bridge` (`command.rs`, `ax_steps`) den
Plan in Schritte, die der Tab-Helfer als `AXActionData` an den Frame des
Knotens schickt; `--relief-run=<aufgaben>` arbeitet Aufgabendateien damit ab
(`scripts/fork-run-tasks.sh`).

| `ActionKind` | Weg im Fork |
|---|---|
| `Activate` | `kDoDefault` (Blink: simulierter Klick, `AccessKeyAction`) |
| `Focus` | `kFocus` |
| `SetValue(v)` | `kFocus`, `kSetValue` (Blink setzt den Wert mit `input`/`change`) |
| `Select(o)` | `kDoDefault` auf der Option `o` unter dem Auswahlfeld (Blink: `HTMLOptionElement::AccessKeyAction` wählt aus) |
| `NavigateTo` | `kScrollToMakeVisible`, `kSetSequentialFocusNavigationStartingPoint` |
| `Increment`/`Decrement` | `kFocus`, dann **echte Pfeiltaste** (Ersatzweg) |
| Escape (Schließen ohne Button) | **echte Taste** an das fokussierte Widget (Ersatzweg) |
| Scrollen | `kSetScrollOffset` am Root-Scroller; Position aus dem eigenen Baum |

Was `AXActionData` nicht abdeckt, und der Ersatzweg:

- **Tasten** (Escape): `AXActionData` kennt keine Tastaturereignisse.
  Ersatz: `RenderWidgetHost::ForwardKeyboardEvent` an das Widget des
  fokussierten Frames — dieselbe Eingabe wie von der Tastatur.
- **`Increment`/`Decrement` an ARIA-Widgets:** Blink erreicht sie nur mit
  dem experimentellen Feature
  `SynthesizedKeyboardEventsForAccessibilityActions` (Pfeiltaste am
  Element); ohne es bricht `AlterSliderOrSpinButtonValue` ohne `step` ab. Mit
  dem Feature schickt Blink einem Knoten ohne vertikale Ausrichtung „Pfeil
  rechts“, und `<input type=number>` übergeht das (gemessen: Menge bleibt
  1). Ersatz: fokussieren über AX, dann Pfeil hoch/runter als echte Taste,
  wie im CDP-Host; wirkt auf native Felder und Tasten-Widgets.
- **Fokus auf nicht fokussierbare Ziele** (Überschrift, Bereich): `kFocus`
  scheitert (`CanSetFocusAttribute`). Ersatz ohne DOM-Eingriff: Startpunkt
  der Tab-Reihenfolge dort setzen (Blink nimmt dabei den Fokus weg) und die
  **Position** in der Sitzung merken, bis der Fokus sich bewegt; „wo bin
  ich“, nächster Abschnitt/nächstes Feld und „lies den Abschnitt“ gehen von
  ihr aus, die Antwort nennt sie als Fokus. Ein Screenreader-Cursor macht
  dasselbe. Escape geht weiter an den echten Fokus.

Messung (M4, Chromium 154.0.8037.58 mit `--enable-relief`):
`scripts/fork-run-tasks.sh spike/tasks/0[1-5]*.txt` → „68 erfüllt, 0 nicht
erfüllt“, zweimal hintereinander; derselbe Stand über CDP ebenfalls 68/0.
Aktionen warten auf Ruhe = 300 ms ohne AX-Paket (Baum oder Positionen),
mindestens 150 ms, höchstens 3 s; typische Antwortzeit 300–600 ms. Die
Wiederholung hängt an `--disable-backgrounding-occluded-windows`: ein
verdecktes Fenster rendert nicht, dann serialisiert Blink keinen Baum.

Dafür am Mirror nachgezogen: `kHeader`/`kFooter` (Blink vergibt sie nur
außerhalb von Sectioning-Inhalt) als `banner`/`contentinfo`, Wertebereiche
(`valuenow` als Wert, `valuemin`/`valuemax`, abweichender Text als
`valuetext`) wie im CDP-Konverter.

## Rückfragen und Befehlsleiste im Fork (Paket 25) [belegt]

Rückfragen beantwortet die Sitzung selbst, vor der Befehlszerlegung
(`Session::pending_reply`), für CDP-Host, Spike-Leiste und Fork gleich:

- **Mehrdeutig:** Kandidaten nummeriert („1. [link] Warenkorb (0) in …“);
  eine Zahl oder ein Text, der genau einen Kandidaten trifft, wählt ihn,
  und die Aktion läuft weiter (riskant → Rückfrage). Die Auswahl gilt nur
  für die nächste Eingabe; Relief rät nie.
- **Bestätigung:** „ja“ löst die gezeigte Rückfrage ein wie „!“ vor
  demselben Befehl (Einmal-Token aus 48, gebunden an Plan und Graph); ein
  zweites „ja“ ist nicht verstanden.
- **Abbrechen:** „abbrechen“/„nein“ verwirft Auswahl und Rückfrage („Nichts
  ausgeführt“); ohne offene Rückfrage sagt Relief, dass nichts offen ist.
  Eine laufende Ausführung bricht die Leiste ab: Das Warten endet sofort,
  bereits gesendete Aktionen werden nicht als ungeschehen dargestellt
  („Bereits gesendete Aktionen wirken“).

Die **Befehlsleiste im Fork** ist der obere Teil des Relief-Side-Panels
(dieselbe WebUI wie der Inspector, → 01): Strg+Umschalt+Leertaste auf der
Seite öffnet das Panel und setzt den Fokus ins Eingabefeld. Zustände stehen
sichtbar und als Statusmeldung da (bereit, führe aus, Auswahl erwartet,
Bestätigung erwartet, fertig); Antworten sammelt ein Log (`role=log`,
höchstens 20). Nach einer Aktion auf der Seite bekommt die Seite den
Tastaturfokus — dort steht das Ziel; Abfragen lassen ihn in der Leiste.
Escape bricht eine Ausführung oder Rückfrage ab, sonst schließt es das
Panel (Chromium gibt den Fokus an die Seite zurück). Während ein Befehl
läuft, bleibt eine neue Eingabe stehen („noch beschäftigt“).

Entscheidung **Side Panel statt schwebender Leiste**: Eine eigene
Bubble über der Seite braucht Views-Klassen des Browserfensters, die im
GN-Graphen über der Tab-Einbindung von `//relief` liegen (Zyklus) oder
weitere Patches; das Side Panel liegt außerhalb der Seite (kein Eintrag in
ihrem AXTree, unberührt von modalen Seitendialogen) und bringt Fokus-Rückgabe
und Schließen mit.

Ausführung (`ReliefExecutor`): Aufgaben-Runner, Leiste und Inspector teilen
denselben Weg (Schritte, Ruhe, Antwort). Belegt: `relief_browsertests
--gtest_filter=*Befehlsleiste*` (Kürzel öffnet mit Fokus im Feld, Abfrage,
Auswahl per Nummer mit Fokus auf dem Ziel, Escape bricht Bestätigung ab ohne
Wirkung, „ja“ löst genau einmal aus, Escape schließt), `crates/relief-bridge/
tests/befehle.rs`; Fork-Aufgaben 01–05, 07 weiter ohne Ausfall.

Offen: deiktische Ziele („dieses Feld“, „hier“) kennt der Parser noch
nicht; Ausgabe pausieren fehlt (die Leiste bündelt nur); manueller
Tastatur- und VoiceOver-Durchgang (→ 47).

## Overlay- und Consent-Dialoge (Paket 40) [belegt]

Browserfrei in `relief-interaction` (`overlay.rs`, Befehle in `command.rs`,
Ablauf in `session.rs`, Texte in `respond.rs`); beide Hosts nutzen es ohne
eigenen Code, weil nur `Outcome::Answer` und `Outcome::Perform` entstehen.

Regeln [Entscheidung]:

- **Relief stimmt nie selbst zu.** Es gibt keinen Befehl „zustimmen“;
  „klicke Alle akzeptieren“ bleibt ein Klick, den die Nutzerin mit Namen
  verlangt. Ein Button, dessen Name nach Zustimmung oder Abo klingt, gilt nie
  als Schließen-Button (`overlay::blocks_dismissal` in
  `resolve::dismissal`): „Akzeptieren und schließen“ führt zu Escape.
- **Ablehnen nur auf ausdrücklichen Befehl** („cookies ablehnen“, „lehne
  ab“, „reject all“ …) und nur über einen **Button**, der ablehnt und
  nichts mit Abo oder Bezahlen zu tun hat. Genau einer → gewöhnlicher
  `ActionPlan` `Activate` über `plan_on_page` (Risiko meist MEDIUM, ohne
  Rückfrage); mehrere → nummerierte Rückfrage wie bei Mehrdeutigkeit;
  keiner → Antwort „Nicht abgelehnt: … Kein Ablehnen ohne Bezahlung“ bzw.
  „Kein Ablehnen“, Hinweis auf Einstellungen, die Relief nicht selbst
  wählt. Keine Umgehung von Bezahlschranken oder Bot-Erkennung.
- **Modalität bleibt:** Erkennung und Buttons nur unter erreichbaren
  Elementen (`Graph::is_reachable`). „was ist hinter dem Dialog“ nennt
  gesperrte Überschriften und Bedienelemente (höchstens je 10), **merkt aber
  keine Auswahl**: Eine Zahl oder ein Name danach ist ein neuer Befehl, und
  „klicke …“ auf ein Element dahinter bleibt „gesperrt“ (Test
  `hintergrund_ist_nur_auskunft`).

Erkennung [Annahme: Wortlisten, nicht kalibriert]:

- **Kandidaten:** erreichbare `dialog`/`alertdialog` und benannte Bereiche,
  deren Name nach Einwilligung klingt; Buttons und Links darin, auch im
  iframe darunter (der Bereichsstapel reicht über Frame-Grenzen). Von
  verschachtelten Cookie-Dialogen gilt der innerste (spiegel.de-Aufbau:
  modaler Dialog im Hauptdokument, Dialog im Consent-iframe).
- **Art** (`Fact<OverlayKind>`, Quelle `Rule`): Cookie-Dialog, wenn Name oder
  Text ein Einwilligungswort trägt (cookie, einwilligung, datenschutz,
  privacy, tracking …), Newsletter-Dialog über „newsletter“, sonst Dialog.
  Ab zwei Hinweisen (Name, Text, Zustimmen-/Ablehnen-Button) `Inferred`
  („vermutlich“), mit einem `Uncertain` („möglicherweise … unsicher“); nie
  `Known`.
- **Buttons** (`Fact<ButtonKind>`), Vorrang: Abo (abo, abonn…, pur,
  werbefrei, bezahl…) vor Ablehnen (ablehnen, nur notwendige, ohne
  Einwilligung, reject …) vor Einstellungen (einstellung, anpassen,
  optionen, verwalten, details, auswahl …) vor Zustimmen (akzeptier…,
  zustimmen, einwilligen, einverstanden, accept, „OK“ …) vor Schließen.
  Wortanfänge, `$` für ganze Wörter („pur“ ≠ „purpose“, „consent“ ≠
  „Consenthub“). Ein **Link**, der nach Ablehnen klingt, lehnt nicht ab
  (bild.de, welt.de: „für Utiq jetzt ablehnen“ führt zu einem
  Drittanbieter).
- **Ansage** in „was ist hier“ (CDP-Host auch beim Laden) und auf „welcher
  Dialog ist offen“: „Dialog „Privacy Center“ vermutlich Cookie-Dialog
  (erschlossen: …). Modal: Bedienung nur im Dialog. Buttons nach
  Beschriftung: Zustimmen „Einwilligen und weiter“, Einstellungen
  „Einstellungen“, Abo „Jetzt abonnieren“, 9 weitere. Kein Ablehnen ohne
  Bezahlung. …“

Belege: Unit-Tests in `overlay.rs` und `session.rs`;
`spike/tasks/09-consent.txt` auf `consent-ablehnen.html` (Ablehnen
kostenlos, „Akzeptieren und schließen“ → Escape, Hintergrund nur lesend),
`consent-abo.html` (kein Ablehnen ohne Abo), `consent-iframe.html` (Buttons
im iframe eines modalen Dialogs) → 19/19 über CDP.

Echte Seiten (`spike/tasks/12-consent-real.txt`, CDP-Host, 2026-09-30,
Netz; 20/20; keine Einwilligung erteilt):

| Seite | erkannt | Beschreibung (Auszug) | Ablehnen |
|---|---|---|---|
| spiegel.de | Cookie-Dialog „Privacy Center“, vermutlich | Zustimmen „Einwilligen und weiter“, Einstellungen, Abo „Jetzt abonnieren“, 9 weitere | kein Ablehnen ohne Bezahlung, nichts geklickt |
| bild.de | „Cookie- und Einwilligungsbanner“, vermutlich | Zustimmen „Alle akzeptieren“, Einstellungen, Abo „Jetzt BILD PUR abonnieren“, 32 weitere | keins (Utiq-Link zählt nicht), nichts geklickt |
| welt.de | wie bild.de (gleicher Anbieter) | Abo „JETZT WELT PUR ABONNIEREN“ | keins, nichts geklickt |
| faz.net | „Cookiebanner“, vermutlich | Zustimmen „Einverstanden“, Einstellungen „Cookie-Manager“, Abo „F.A.Z. Pur-Abonnent? Hier anmelden“, „Abo“ | keins, nichts geklickt |
| t-online.de | „Iframe title“ (Name der Seite), vermutlich | Zustimmen „ZUSTIMMEN“, Einstellungen, Abo „Datenschutzhinweise (PUR)“ | keins, nichts geklickt |
| heise.de | „Cookie- und Datenverarbeitung“, vermutlich | Zustimmen, Einstellungen, Abo „Pur-Abo“ | keins, nichts geklickt |
| google.de | „Bevor Sie zur Google Suche weitergehen“ | Zustimmen „Alle akzeptieren“, Ablehnen „Alle ablehnen“ | ausgeführt, Dialog geschlossen |
| zdf.de | „cmp-dialog-description“ | Zustimmen, Ablehnen „Ablehnen“ | ausgeführt, Dialog geschlossen |
| ikea.com/de | nicht modaler Dialog „Hej! …“ | Ablehnen „Optionale Cookies ablehnen“ | ausgeführt, Dialog geschlossen |

Nur gelesen: stern.de (Abo „Zum PUR-Abo“, kein Ablehnen), otto.de (Ablehnen
„Einwilligung ablehnen“), sueddeutsche.de (kein Ablehnen; das Abo heißt dort
„Jetzt testen“ und bleibt „weitere“). **zeit.de** blockiert den
automatisierten Browser („Ihre Anfrage wurde blockiert“); nicht umgangen,
deshalb nicht in der Datei. **golem.de** zeigt eine Einwilligungs*seite*
ohne Dialog-Rolle — nicht erkannt (→ 80).

Befund: Über CDP enthält der Baum den Seiteninhalt hinter `aria-modal`
(spiegel.de, bild.de: „was ist hinter dem Dialog“ nennt die Überschriften
der Startseite). Im Fork nimmt Blink ihn heraus (→ 09, Nachtrag Paket 35);
dort antwortet Relief, dass der Baum nichts enthält [Annahme, im Fork nicht
gemessen → 80].

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
spätere oder veränderte Aktionen. Umgesetzt in `Session` (beide Hosts):
„!“ löst nur die unmittelbar vorher gestellte Rückfrage ein (→ 07,
„Bestätigungstoken“); das Formularziel ist noch nicht gebunden.

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

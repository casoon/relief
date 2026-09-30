# 03 · Semantisches Datenmodell

Umgesetzt in `crates/relief-model` (browserfrei, nur `serde` und
`a11y-perception`). Rustdoc ist die Referenz für Einzelheiten; hier die
Struktur und die Gründe.

## Weg 2: eigenes typisiertes Modell [Entscheidung]

Relief hat ein eigenes Modell statt des CDP-förmigen `a11y_perception::AXNode`
(Weg 1). Gründe aus den belegten Chromium-Strukturen (→ 01):

- Chromium führt **einen Baum je Frame und Dokument** mit eigener `AXTreeID`,
  Integer-Node-IDs nur je Baum eindeutig; iframes sind Child-Trees über
  `kChildTreeId` (01, Abschnitt 5). `AXNode` kennt nur einen Baum mit
  String-IDs; der CDP-Host behilft sich mit Frame-Präfixen (`f1:28`).
- Chromium liefert **inkrementelle `AXTreeUpdate`s** (01, Tabelle), `AXNode`
  nur Vollbäume. Ein Adapter nach Weg 1 müsste jedes Update in einen
  Vollsnapshot zurückverwandeln.
- Rollen, Zustände und Relationen sind in Chromium Enums und Integer-Listen,
  in `AXNode` Strings und eine Eigenschaftsliste (mit Fallen wie `hasPopup`
  vs. `haspopup`, `invalid` als Token-String).
- Fokus steht in Chromium in den Baumdaten (`AXTreeData`), Positionen kommen
  über einen eigenen Kanal (01, Abschnitt 2).

`a11y-perception` bleibt Quelle für CDP-Host und Aufnahmen und bekommt einen
Konverter (`relief_model::perception`). `relief-interaction` arbeitet auf dem
Modell (Abschnitt „Interaction auf dem Modell“).

## Aufbau [umgesetzt]

```text
SemanticGraph { version: GraphVersion, root: Option<TreeId>, trees: TreeId → SemanticTree }
SemanticTree  { id, data: TreeData, root: Option<NodeId>, nodes: NodeId → SemanticNode }
TreeData      { parent: Option<NodeRef>, url, title, focus: Option<NodeId> }   // wie AXTreeData
NodeRef       { tree: TreeId, node: NodeId }   // NodeId(i32) wie AXNodeID, TreeId(String)
```

```rust
pub struct SemanticNode {
    pub id: NodeId,
    pub role: Role,                    // eigenes Enum, ARIA-Namen, sonst Chromium-intern; Other(String)
    pub name: Fact<String>,
    pub name_from: Option<NameFrom>,   // attribute | relatedElement | contents | placeholder | title
    pub description: Fact<String>,
    pub value: Fact<String>,
    pub states: States,                // focusable, disabled, readonly, required, modal, expanded,
                                       // selected, checked/pressed (Toggle), invalid, editable,
                                       // hasPopup, orientation, busy, settable, multiline, multiselectable
    pub actions: Vec<Action>,          // was Chromium als unterstützt meldet (CDP: leer)
    pub bounds: Option<Rect>,          // Seitenkoordinaten (CDP: keine)
    pub ignored: bool,
    pub ignored_reasons: Vec<IgnoredReason>, // nur CDP meldet Gründe
    pub level: Option<u32>,
    pub url: Option<String>,
    pub parent: Option<NodeId>,        // nur innerhalb des Baums
    pub children: Vec<NodeId>,         // nur innerhalb des Baums
    pub child_tree: Option<TreeId>,    // iframe → eingebetteter Baum
    pub relations: Relations,          // labelledBy, describedBy, controls, details,
                                       // errorMessage, flowTo, activeDescendant (NodeIds im selben Baum)
    pub dom_node_id: Option<i64>,      // DOM-Kontext bei Bedarf, keine Identität
    pub extra: BTreeMap<String, String>, // gemeldet, aber (noch) nicht typisiert
}
```

Abweichungen vom Ursprungsentwurf und warum:

- `confidence` und `source` hängen nicht am Knoten, sondern **an jeder
  einzelnen Aussage** (`Fact<T>`). Ein Knoten kann eine bekannte Rolle, aber
  einen erschlossenen Namen haben.
- `focused` ist kein Zustand, sondern `TreeData::focus` (wie Chromium).
- `visibility` ist `ignored` + `ignored_reasons` + `bounds`; ein eigenes
  `Offscreen` braucht Viewport-Wissen und bleibt der Auswertung überlassen.
- `aria-owns` ist keine Relation: Chromium hängt den Knoten um.
- `Role` benennt nach ARIA (`button`, `combobox`), sonst nach Chromiums
  interner Rolle in lowerCamelCase (`staticText`, `rootWebArea`). Grund:
  Chromium hat mehrere interne Rollen je ARIA-Rolle, CDP eine dritte
  Schreibweise. Alle 67 Rollen der Aufnahmen haben eine Variante.

## Fact: Herkunft jeder Aussage [umgesetzt]

```rust
pub struct Fact<T> {
    pub value: Option<T>,
    pub certainty: Certainty,          // Known | Inferred | Uncertain
    pub source: Source,                // Chromium | Rule(id) | Model(name+version)
    pub confidence: Option<f32>,       // nur bei Inferred/Uncertain
    pub evidence: Vec<String>,
}
```

`produced_by` ist in `Source` aufgegangen. `at: GraphVersion` aus dem Entwurf
ist **nicht** umgesetzt: Die Version steht am Graphen; eine Version je
Aussage würde jede Delta alle Knoten „ändern“ lassen. Ob der Inferenz-Cache
(unten) eine Version je Aussage braucht, klärt Paket 28.

- **Known** — direkt aus Autorensemantik (HTML, ARIA) über Chromium.
- **Inferred** — rekonstruiert (Heuristik, DOM-Kontext, KI), Confidence über
  Schwelle, Evidence vorhanden.
- **Uncertain** — Hypothese unter Schwelle. Wird angezeigt/angesagt **als
  unsicher**, nie als Tatsache; nie Grundlage einer Aktion ohne Rückfrage.

Beispiel:

```yaml
role: button (Known)
name:
  value: "Warenkorb"
  status: Inferred
  confidence: 0.94
  source: heuristic/dom-context
  evidence:
    - href=/cart
    - svg resembles cart
    - parent=navigation
    - neighbours: account, search
```

### Verhältnis zu barrierlab `Outcome` [belegt + Abgrenzung]

`a11y-report` verzichtet bewusst auf eine dritte Achse „certainty": Befunde
sind `Fail` (automatisch festgestellt), `Review` (heuristisch), `Pass`,
`Untested`. Das widerspricht `Certainty` hier nicht — es sind verschiedene
Gegenstände:

- `Outcome` bewertet **Befunde über die Seite** („dieser Button hat keinen Namen" = Fail).
- `Certainty` beschreibt **Fakten, die Relief der Nutzerin mitteilt** („dieser Button heißt vermutlich Warenkorb").

Ein fehlender Name ist also gleichzeitig `Fail` in `a11y-report` und Anlass
für eine `Inferred`-Rekonstruktion in Relief. Die Achsen nicht vermischen, keinen
Score bilden (barrierlab-Regel gilt auch hier).

## Quellenpriorität [Entscheidung]

1. native HTML-Semantik
2. ARIA
3. Chromium Accessibility Tree (vereint 1+2, ist die normale Quelle)
4. deterministische Analyse des Rust-Cores
5. DOM-/Layout-Kontext
6. KI-basierte Rekonstruktion
7. optional multimodale Analyse (Screenshot-Ausschnitt)

Höhere Stufen überschreiben niedrigere nie. Ist ein Wert auf Stufe 1–3
vorhanden, läuft für diese Eigenschaft keine KI.

Offen [validieren]: Was, wenn die Autorensemantik **nachweislich falsch** ist
(`aria-label="button"`, `role="button"` auf nicht bedienbarem Element)? Vorschlag:
Known-Wert bleibt, zusätzlich ein Inferred-Gegenvorschlag mit Evidence; die UI
zeigt beides. Relief überschreibt Autorensemantik nicht stillschweigend.

## Identität und Versionen [umgesetzt]

- Knoten-ID = `NodeRef` (`TreeId`, `NodeId`) wie in Chromium; stabil
  innerhalb eines Dokuments, nicht über Navigation hinweg (neues Dokument →
  neue Tree-ID, 01 Abschnitt 5). `a11y-perception` meldet „Identität
  unklar", statt zu raten — dieselbe Regel hier: Über Tree-IDs hinweg wird
  nichts zugeordnet.
- CDP kennt keine Tree-ID. Der Konverter bekommt die ID des Hauptdokuments
  vom Aufrufer; Child-Trees heißen `<dokument>/<frame-präfix>`. In den
  Aufnahmen sind die Node-IDs innerhalb eines Dokuments zwischen zwei
  Aufnahmen stabil (gleiche ID ⇒ gleiche Backend-DOM-ID in allen 27
  verschiedenen Aufnahmepaaren) [belegt].
- Jede angewandte Delta erhöht `GraphVersion` um eins. Intents und Aktionen
  referenzieren Knoten **mit** Version (→ 05), damit veraltete Ziele erkannt
  werden.
- Inferenzen werden pro (Knoten, Eigenschaft, Eingabe-Hash) gecacht und bei
  relevanter Änderung verworfen [Entwurf, Paket 28].

## Delta-Format [umgesetzt]

```text
TreeDelta  { base: GraphVersion, root: Option<TreeId>, removed_trees: [TreeId], trees: [TreeUpdate] }
TreeUpdate { tree, data: Option<TreeData>, root: Option<NodeId>,
             removed: [NodeId], created: [SemanticNode], changed: [SemanticNode],
             bounds: [BoundsChange { node, bounds }] }
```

- Angelehnt an `AXTreeUpdate`: ein Update je Baum, geänderte Knoten
  **vollständig** (keine Feld-Diffs), Wurzel und Baumdaten nur bei Wechsel.
  Anders als dort sind entfernte, angelegte und geänderte Knoten getrennt
  aufgeführt; die Reihenfolgeregeln von `AXTreeUpdate`
  (`node_id_to_clear`, Platzhalter) entfallen.
- Wurzel-/Child-Tree-Wechsel: neuer Hauptbaum über `root`, wegfallende Bäume
  über `removed_trees` (zuerst angewandt), neue Bäume als `TreeUpdate` mit
  `data` und allen Knoten in `created`; die Verknüpfung steht im
  iframe-Knoten (`child_tree`) und in `TreeData::parent`.
- Positionen getrennt in `bounds`, weil Chromium sie über
  `AccessibilityLocationChangesReceived` meldet (01, Abschnitt 2). Der
  Fork-Adapter füllt sie seit 33: „Seitenkoordinaten“ heißt Koordinaten im
  Hauptdokument in CSS-Pixeln, unabhängig von der Scroll-Position der Seite
  und vom Browser-Zoom; auch Knoten eines iframe-Baums stehen in
  Koordinaten des Hauptdokuments (35; rückt der iframe oder scrollt sein
  Inhalt, kommen alle betroffenen Knoten als `bounds`). Ein iframe-Baum,
  dessen Host-Knoten der Adapter noch nicht kennt, hat vorerst keine
  Positionen. Angelegte und geänderte Knoten tragen ihre
  Position im Knoten, `bounds` nennt nur Knoten, die sich allein verschoben
  haben.
- `SemanticGraph::apply` prüft erst (Version, Existenz der Knoten und Bäume)
  und ändert dann; eine unpassende Delta lässt den Graphen unverändert.
  `TreeDelta::between(vorher, nachher)` leitet eine Delta aus zwei Ständen ab
  (für Hosts ohne eigene Deltas wie CDP, und für Tests).
- Rundtest `apply(vorher, between(vorher, nachher)) == nachher` auf allen
  54 Aufnahmepaaren aus 20 Seiten (27 mit verschiedenen Aufnahmen) grün,
  Delta übersteht JSON
  (`crates/relief-model/tests/recordings.rs`) [belegt]. Größenordnung
  (JSON): ruhige Schritte 10–200 Byte, typische Aktionen 2–30 KB, größter
  Schritt ikea.com 109 KB (+157 ~54 −78 Knoten bei 3 961 Knoten).

## Interaction auf dem Modell [umgesetzt]

`relief-interaction` liest nur `SemanticGraph` (Abhängigkeit
`relief-model` ohne Feature `perception`; `a11y-perception` nur noch als
Dev-Abhängigkeit für die Aufnahmen). Damit läuft derselbe Code hinter dem
CDP-Host und hinter dem Fork-Adapter.

- **Aufbau:** `Graph::build(&SemanticGraph)` geht vom Hauptbaum aus in der
  Reihenfolge von `document_order` (eigene Kinder, dann der Child-Tree eines
  iframes) — rekursiv, weil der Stapel der umschließenden Bereiche am
  Teilbaum hängt. Rollen, Zustände, Ebene und URL kommen typisiert
  (`Role`, `States`, `level`, `url`); Schieberegler-Grenzen und `valuetext`
  aus `extra`. `Control::states` bleibt eine Textliste (`hasPopup=menu`) für
  Ausgabe und Vergleich.
- **Ziele:** `Control`, `Heading`, `Region` tragen `node: NodeRef` und
  daneben `dom_node_id` für Hosts, die über das DOM handeln;
  `ActionPlan { target: NodeRef, dom_node_id, … }`. Ohne DOM-ID lehnt `plan`
  weiter ab (`NoDomNode`), weil der einzige ausführende Host (CDP) sie braucht;
  der Rückweg über `AXActionData` (24) braucht nur `target`.
- **Herkunft:** `Certainty`/`Fact` gibt es nur hier. Ein vorhandener Name
  behält die Herkunft, die das Modell für ihn führt (heute `Known` aus
  Chromium, später auch Resolver-Hypothesen, 28). Fehlt er, erzeugt
  `relief-interaction` die Aussage selbst mit `Source::Rule`:
  `name-aus-beschreibung`, `name-aus-url` (beide `Inferred`, mit Evidence),
  `kein-name` (`Uncertain`, Wert leer).
- **Fokus:** `relief_interaction::focused` = letzter Knoten in
  Dokumentreihenfolge, der Fokus seines Baums ist, ohne Baumwurzeln (Fokus auf
  dem Dokument heißt: kein Element). Liegt der Fokus in einem iframe, ist das
  das Element im Child-Tree, nicht der iframe [belegt: 04-iframe,
  `f1:32`]. „Wo bin ich“ und Schritte vom Fokus nehmen eine `NodeRef`; der
  CDP-Host fragt den Fokus weiter im DOM ab (Fokuswechsel ist keine
  Mutation) und ordnet ihn über `dom_node_id` zu.
- **Wirkung einer Aktion:** `respond::describe_diff(vorher, nachher)` auf zwei
  Modellständen, Kandidaten aus `TreeDelta::between`. Erhalten aus
  `AXTreeDiff`: „wahrnehmbar geworden“ = nachher vorhanden und nicht
  ignoriert, vorher fehlend oder ignoriert (so öffnen Menüs und native
  Dialoge); Zustandswechsel `expanded`, `invalid`, `modal`, `selected` nur an
  Knoten, die vorher und nachher wahrnehmbar sind; neue Textknoten als
  „Neuer Text“ (Live-Regionen, Zähler); Adresse und Titel aus `TreeData`
  des Hauptbaums. Anders als dort: Zuordnung über `NodeRef` statt DOM-ID
  (nach Dokumentwechsel ist alles neu), Aufzählung in Dokumentreihenfolge
  statt nach Knoten-ID als Text, Fokus aus den Baumdaten statt aus
  `document.activeElement`. `hidden` fällt weg (CDP meldet es nicht, das
  Modell führt es nicht).
- **CDP-Host:** konvertiert jede Aufnahme mit `perception::from_snapshot`.
  Tree-ID des Hauptdokuments `dokument-N`, neu bei Navigation und
  `DOM.documentUpdated`. Tests und Benchmarks nehmen die URL als Tree-ID
  (eine andere URL ist in den Aufnahmen ein anderes Dokument).
- **Sicherheitsnetz** [belegt]: Die Snapshot-Erwartungen
  (`crates/relief-interaction/tests/erwartungen/`, damals 20 Seiten) waren nach der
  Umstellung unverändert; in `aufgaben.rs` ändern sich zwei Texte (Fokus im
  iframe-Consent jetzt der Button im Frame; Texte nach dem Schließen der
  Größentabelle in Dokumentreihenfolge). `spike/tasks/01`–`05` im Browser:
  68 von 68 Erwartungen erfüllt.
- `PopUpButton` fällt aus den Rollenlisten: Chromium 154 hat die Rolle nicht
  mehr (`kComboBoxSelect` → `combobox`), keine Aufnahme enthält sie.

Verluste des CDP-Konverters [belegt, Rustdoc `perception`]: Relationen zeigen
in CDP auf DOM-Knoten und werden über die DOM-ID auf AX-Knoten desselben Baums
abgebildet (in den Aufnahmen alle 558 aufgelöst); `owns` und Knotenverweise in
`ignoredReasons` fallen weg; nicht typisierte Eigenschaften gehen als Text
nach `extra`.

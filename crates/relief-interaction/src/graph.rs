//! Interaction Graph: was auf einer Seite bedienbar und lesbar ist.
//!
//! Abgeleitet in Dokumentreihenfolge aus dem [`SemanticGraph`] (Hauptbaum,
//! iframes über ihren Child-Tree). Jedes Bedienelement gehört höchstens zu
//! einem Bereich (dem innersten Landmark oder Dialog), und sein Name trägt
//! seine Herkunft (→ `plan/spezifikation/03`).

use std::collections::BTreeMap;

use relief_model::{
    Certainty, Fact, HasPopup, Invalid, NodeRef, Role, SemanticGraph, SemanticNode, Source, Toggle,
    TreeId,
};
use serde::Serialize;

use crate::page::Page;
use crate::security::{is_sensitive_field, HTML_AUTOCOMPLETE, INPUT_TYPE};

/// Ein logischer Bereich der Seite (Landmark oder Dialog).
#[derive(Debug, Clone, Serialize)]
pub struct Region {
    pub role: Role,
    pub name: Option<String>,
    /// Umschließender Bereich (Index in [`Graph::regions`]).
    pub parent: Option<usize>,
    /// Modaler Dialog: der Rest seines Dokuments ist für die Bedienung
    /// gesperrt (→ [`Graph::is_reachable`]).
    pub modal: bool,
    pub node: NodeRef,
    /// DOM-Knoten für Hosts, die über das DOM handeln (CDP).
    pub dom_node_id: Option<i64>,
}

impl Region {
    /// Anzeigename, z. B. `navigation „Hauptmenü"`.
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) => format!("{} „{}“", self.role, n),
            None => self.role.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub node: NodeRef,
    /// DOM-Knoten für Hosts, die über das DOM handeln (CDP).
    pub dom_node_id: Option<i64>,
    /// Index in [`Graph::regions`].
    pub region: Option<usize>,
}

/// Ein Element, mit dem etwas getan werden kann.
#[derive(Debug, Clone, Serialize)]
pub struct Control {
    pub node: NodeRef,
    /// DOM-Knoten für Hosts, die über das DOM handeln (CDP).
    pub dom_node_id: Option<i64>,
    pub role: Role,
    pub name: Fact<String>,
    /// Index in [`Graph::regions`].
    pub region: Option<usize>,
    pub value: Option<String>,
    /// Sensibles Feld (Passwort, `autocomplete` für Zahlungs- und
    /// Identitätsdaten, → [`crate::security::is_sensitive_field`]): Listen
    /// und „wo bin ich“ nennen den Wert nicht (→ `respond::control_line`).
    pub sensitive: bool,
    /// Auswahlmöglichkeiten bei Combobox/Listbox.
    pub options: Vec<String>,
    pub selected_option: Option<String>,
    pub disabled: bool,
    pub focusable: bool,
    /// Chromium meldet eine Standardaktion (Klick), auch ohne Bedienrolle,
    /// etwa ein `<div>` mit Klick-Handler (→ Sprungmarken, `marks`).
    pub clickable: bool,
    /// Gemeldete Zustände als Text (`expanded`, `checked`, `invalid`,
    /// bei Schiebereglern `valuemin`/`valuemax` …); `false` fehlt.
    pub states: Vec<(String, String)>,
    /// Aufklappbar: meldet `expanded`, auch zugeklappt (in `states` fehlt
    /// `false`). Zweck-Titel im Cookie-Dialog (→ [`crate::overlay`]).
    pub expandable: bool,
    /// Vorausgehende Überschrift (Index in [`Graph::headings`]): der
    /// Abschnitt, in dem das Element liegt.
    pub heading: Option<usize>,
}

impl Control {
    /// Name für Ausgabe; bei unsicherem Namen die Rolle.
    pub fn display_name(&self) -> String {
        match (&self.name.value, self.name.certainty) {
            (Some(n), _) => n.clone(),
            (None, _) => format!("unbenannt ({})", self.role),
        }
    }
}

/// Lesbarer Text in Dokumentreihenfolge, für „lies den Abschnitt …“.
#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub text: String,
    /// Textknoten bzw. die Überschrift, deren Text es ist.
    pub node: NodeRef,
    /// Index in [`Graph::regions`].
    pub region: Option<usize>,
    /// Vorausgehende Überschrift (Index in [`Graph::headings`]).
    pub heading: Option<usize>,
    /// Ebene, wenn der Text eine Überschrift ist.
    pub level: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Graph {
    pub title: Option<String>,
    pub regions: Vec<Region>,
    pub headings: Vec<Heading>,
    pub controls: Vec<Control>,
    pub texts: Vec<Text>,
    /// Seitentyp, funktionale Gruppen, primäre Aktion (alles erschlossen).
    pub page: Page,
    /// Frame-Grenzen: Baum eines iframe-Dokuments → der `Iframe`-Knoten im
    /// Elterndokument. Welchem Dokument ein Element angehört, sagt der Baum
    /// seines Knotens (`NodeRef::tree`).
    pub frames: BTreeMap<TreeId, NodeRef>,
}

/// Wo ein fokussiertes Element in der Gliederung steht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Index in [`Graph::controls`].
    Control(usize),
    /// Index in [`Graph::headings`].
    Heading(usize),
}

/// Landmarks und Dialoge. `form` und `region` zählen nur mit Namen
/// (HTML-AAM), das prüft `Graph::visit`.
fn is_landmark(role: &Role) -> bool {
    matches!(
        role,
        Role::Banner
            | Role::Navigation
            | Role::Main
            | Role::Complementary
            | Role::ContentInfo
            | Role::Search
            | Role::Form
            | Role::Region
            | Role::Dialog
            | Role::AlertDialog
    )
}

/// Rollen, die ein Bedienelement ausmachen.
fn is_control(role: &Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::Link
            | Role::Textbox
            | Role::SearchBox
            | Role::Combobox
            | Role::Listbox
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Slider
            | Role::SpinButton
            | Role::Tab
            | Role::MenuItem
            | Role::MenuItemCheckbox
            | Role::MenuItemRadio
            | Role::TreeItem
    )
}

/// Rollen, deren Text nicht als Fließtext gelesen wird: Überschriften kommen
/// als eigener Eintrag, Bedienelemente über ihren Namen, Beschriftungen
/// (`<label>`) sind deren Namen. Links bleiben Fließtext, weil sie mitten im
/// Satz stehen.
fn owns_text(role: &Role) -> bool {
    matches!(role, Role::Heading | Role::ListMarker | Role::LabelText)
        || (is_control(role) && *role != Role::Link)
}

impl Graph {
    pub fn build(model: &SemanticGraph) -> Self {
        let root = model
            .root
            .as_ref()
            .and_then(|t| Some(NodeRef::new(t.clone(), model.trees.get(t)?.root?)));
        let mut graph = Graph {
            title: root
                .as_ref()
                .and_then(|r| model.node(r))
                .and_then(|n| non_empty(n.name.value.as_deref())),
            regions: Vec::new(),
            headings: Vec::new(),
            controls: Vec::new(),
            texts: Vec::new(),
            page: Page::default(),
            frames: BTreeMap::new(),
        };
        if let Some(root) = root {
            let mut stack = Vec::new();
            graph.visit(model, &root, &mut stack, false);
        }
        graph.page = crate::page::analyze(model, &graph);
        graph
    }

    /// Tiefensuche in der Reihenfolge von [`SemanticGraph::document_order`]
    /// (eigene Kinder, dann der Child-Tree eines iframes), mit dem Stapel der
    /// umschließenden Bereiche.
    ///
    /// `in_owner`: der Knoten liegt in einer Überschrift oder einem
    /// Bedienelement, sein Text ist kein Fließtext.
    fn visit(
        &mut self,
        model: &SemanticGraph,
        at: &NodeRef,
        stack: &mut Vec<usize>,
        in_owner: bool,
    ) {
        let Some(node) = model.node(at) else {
            return;
        };
        let role = &node.role;
        let mut pushed = false;

        if !node.ignored {
            let named = non_empty(node.name.value.as_deref());
            let landmark = is_landmark(role)
                && (!matches!(role, Role::Form | Role::Region) || named.is_some());
            if landmark {
                self.regions.push(Region {
                    role: role.clone(),
                    name: named,
                    parent: stack.last().copied(),
                    modal: matches!(role, Role::Dialog | Role::AlertDialog) && node.states.modal,
                    node: at.clone(),
                    dom_node_id: node.dom_node_id,
                });
                stack.push(self.regions.len() - 1);
                pushed = true;
            }

            if *role == Role::Heading {
                if let Some(text) = non_empty(node.name.value.as_deref()) {
                    let level = node.level.map_or(2, |l| l.clamp(1, 6) as u8);
                    self.headings.push(Heading {
                        level,
                        text: text.clone(),
                        node: at.clone(),
                        dom_node_id: node.dom_node_id,
                        region: stack.last().copied(),
                    });
                    self.push_text(text, at, stack, Some(level));
                }
            }

            if is_control(role) {
                let mut c = control(model, at, node, stack.last().copied());
                c.heading = self.headings.len().checked_sub(1);
                self.controls.push(c);
            }

            if *role == Role::StaticText && !in_owner {
                if let Some(text) = non_empty(node.name.value.as_deref()) {
                    self.push_text(text, at, stack, None);
                }
            }
        }

        let in_owner = in_owner || (!node.ignored && owns_text(role));
        for child in &node.children {
            let child = NodeRef::new(at.tree.clone(), *child);
            self.visit(model, &child, stack, in_owner);
        }
        if let Some(child_tree) = &node.child_tree {
            if let Some(root) = model.trees.get(child_tree).and_then(|t| t.root) {
                self.frames.insert(child_tree.clone(), at.clone());
                let root = NodeRef::new(child_tree.clone(), root);
                self.visit(model, &root, stack, in_owner);
            }
        }

        if pushed {
            stack.pop();
        }
    }

    fn push_text(&mut self, text: String, at: &NodeRef, stack: &[usize], level: Option<u8>) {
        self.texts.push(Text {
            text,
            node: at.clone(),
            region: stack.last().copied(),
            heading: self.headings.len().checked_sub(1),
            level,
        });
    }

    /// Signatur für den Stabilitätsvergleich zweier Aufnahmen: Rolle, Name,
    /// Bereich je Bedienelement in Dokumentreihenfolge. Knoten-IDs bleiben
    /// draußen, weil sie über Dokumente hinweg nichts bedeuten.
    pub fn signature(&self) -> Vec<String> {
        self.controls
            .iter()
            .map(|c| {
                format!(
                    "{}|{}|{}",
                    c.role,
                    c.name.value.as_deref().unwrap_or("?"),
                    c.region
                        .map(|r| self.regions[r].label())
                        .unwrap_or_default()
                )
            })
            .collect()
    }

    /// Der zuletzt geöffnete modale Dialog der Seite (über alle Frames),
    /// falls einer offen ist. Was er sperrt, sagt [`Graph::is_reachable`].
    ///
    /// Chrome blendet bei `aria-modal` den Rest der Seite nicht aus dem
    /// AXTree aus (APG-Dialogbeispiel); Relief spiegelt die Modalität selbst.
    pub fn active_modal(&self) -> Option<usize> {
        self.regions.iter().rposition(|r| r.modal)
    }

    /// Der modale Dialog, der das Dokument `tree` sperrt: der letzte in
    /// Dokumentreihenfolge, dessen Knoten in diesem Dokument liegt.
    fn modal_in(&self, tree: &TreeId) -> Option<usize> {
        self.regions
            .iter()
            .rposition(|r| r.modal && r.node.tree == *tree)
    }

    /// Liegt `region` in `ancestor` (oder ist es selbst)?
    pub fn within(&self, region: Option<usize>, ancestor: usize) -> bool {
        let mut current = region;
        while let Some(r) = current {
            if r == ancestor {
                return true;
            }
            current = self.regions[r].parent;
        }
        false
    }

    /// Ist ein Element (Knoten `node` im Bereich `region`) gerade
    /// erreichbar? Ein modaler Dialog sperrt nur sein eigenes Dokument:
    /// Für das Dokument des Elements und jedes Dokument, in dessen iframe es
    /// liegt, gilt: kein modaler Dialog offen, oder das Element liegt in ihm.
    ///
    /// HTML `showModal()` macht nur die Knoten des Dokuments inert, das den
    /// Dialog enthält; `aria-modal` macht selbst nichts inert, und das
    /// Dokument eines iframes kann sein Elterndokument nicht sperren. Tab
    /// erreicht das Elterndokument also weiterhin (→ `plan/spezifikation/04`).
    pub fn is_reachable(&self, region: Option<usize>, node: &NodeRef) -> bool {
        let mut tree = Some(&node.tree);
        while let Some(t) = tree {
            if self.modal_in(t).is_some_and(|m| !self.within(region, m)) {
                return false;
            }
            tree = self.frames.get(t).map(|iframe| &iframe.tree);
        }
        true
    }

    /// Bedienelemente, die gerade erreichbar sind: bei offenem modalem Dialog
    /// in ihrem Dokument nur dessen Inhalt.
    pub fn reachable_controls(&self) -> impl Iterator<Item = &Control> {
        self.controls
            .iter()
            .filter(move |c| self.is_reachable(c.region, &c.node))
    }

    /// Bedienelement oder Überschrift an diesem Knoten (z. B. Fokus).
    pub fn anchor(&self, node: Option<&NodeRef>) -> Option<Anchor> {
        let node = node?;
        if let Some(i) = self.controls.iter().position(|c| c.node == *node) {
            return Some(Anchor::Control(i));
        }
        self.headings
            .iter()
            .position(|h| h.node == *node)
            .map(Anchor::Heading)
    }

    /// Der Abschnitt, in dem ein Anker liegt: seine Überschrift.
    pub fn section_of(&self, anchor: Anchor) -> Option<usize> {
        match anchor {
            Anchor::Control(i) => self.controls[i].heading,
            Anchor::Heading(h) => Some(h),
        }
    }

    /// Ende des Abschnitts unter Überschrift `h` (exklusiv): die nächste
    /// Überschrift gleicher oder höherer Ebene.
    pub fn section_end(&self, h: usize) -> usize {
        let level = self.headings[h].level;
        self.headings[h + 1..]
            .iter()
            .position(|x| x.level <= level)
            .map_or(self.headings.len(), |i| h + 1 + i)
    }

    pub fn region_label(&self, idx: Option<usize>) -> String {
        idx.map(|i| self.regions[i].label())
            .unwrap_or_else(|| "außerhalb von Bereichen".to_string())
    }
}

/// Der fokussierte Knoten der Seite: der letzte Knoten in
/// Dokumentreihenfolge, der Fokus seines Baums ist. Wurzeln zählen nicht —
/// Fokus auf dem Dokument selbst heißt, kein Element ist fokussiert.
///
/// Chromium führt den Fokus je Baum. Liegt er in einem iframe, zeigt der
/// Hauptbaum auf den iframe-Knoten oder auf nichts, der Child-Tree auf das
/// Element; der Child-Tree folgt in Dokumentreihenfolge auf seinen
/// iframe-Knoten, also gewinnt das innerste Element.
pub fn focused(model: &SemanticGraph) -> Option<NodeRef> {
    model.document_order().into_iter().rev().find(|at| {
        model
            .trees
            .get(&at.tree)
            .is_some_and(|t| t.data.focus == Some(at.node) && t.root != Some(at.node))
    })
}

pub(crate) fn control(
    model: &SemanticGraph,
    at: &NodeRef,
    node: &SemanticNode,
    region: Option<usize>,
) -> Control {
    let mut options = Vec::new();
    let mut selected_option = None;
    if matches!(node.role, Role::Combobox | Role::Listbox) {
        collect_options(model, at, node, &mut options, &mut selected_option);
    }

    let value = non_empty(node.value.value.as_deref());
    Control {
        node: at.clone(),
        dom_node_id: node.dom_node_id,
        role: node.role.clone(),
        name: name_fact(node),
        region,
        states: states(node, value.as_deref()),
        expandable: node.states.expanded.is_some(),
        value,
        sensitive: is_sensitive_field(
            node.extra.get(INPUT_TYPE).map(String::as_str),
            node.extra.get(HTML_AUTOCOMPLETE).map(String::as_str),
        ),
        options,
        selected_option,
        disabled: node.states.disabled,
        focusable: node.states.focusable,
        clickable: node.actions.contains(&relief_model::Action::DoDefault),
        heading: None,
    }
}

/// Zustände als `(Name, Wert)` in fester Reihenfolge, benannt wie in
/// ARIA/CDP. `false` und Leeres fehlen.
fn states(node: &SemanticNode, value: Option<&str>) -> Vec<(String, String)> {
    let s = &node.states;
    let flag = |b: bool| b.then(|| "true".to_string());
    let toggle = |t: Toggle| match t {
        Toggle::True => Some("true".to_string()),
        Toggle::Mixed => Some("mixed".to_string()),
        Toggle::False => None,
    };
    let extra = |key: &str| node.extra.get(key).cloned();
    [
        ("expanded", s.expanded.and_then(flag)),
        ("checked", s.checked.and_then(toggle)),
        ("pressed", s.pressed.and_then(toggle)),
        ("selected", s.selected.and_then(flag)),
        (
            "invalid",
            s.invalid.as_ref().map(|i| invalid_name(i).into()),
        ),
        ("required", flag(s.required)),
        ("hasPopup", s.has_popup.map(|p| has_popup_name(p).into())),
        ("valuemin", extra("valuemin")),
        ("valuemax", extra("valuemax")),
        // Chrome meldet `valuetext` bei nativen Zahlenfeldern als Kopie des
        // Werts und bei `aria-valuetext` leer (Fixture `intents.html`).
        (
            "valuetext",
            extra("valuetext").filter(|t| Some(t.as_str()) != value),
        ),
    ]
    .into_iter()
    .filter_map(|(k, v)| Some((k.to_string(), v?)))
    .filter(|(_, v)| v != "false" && !v.is_empty())
    .collect()
}

pub(crate) fn invalid_name(invalid: &Invalid) -> &'static str {
    match invalid {
        Invalid::True => "true",
        Invalid::Spelling => "spelling",
        Invalid::Grammar => "grammar",
    }
}

fn has_popup_name(popup: HasPopup) -> &'static str {
    match popup {
        HasPopup::True => "true",
        HasPopup::Menu => "menu",
        HasPopup::Listbox => "listbox",
        HasPopup::Tree => "tree",
        HasPopup::Grid => "grid",
        HasPopup::Dialog => "dialog",
    }
}

fn collect_options(
    model: &SemanticGraph,
    at: &NodeRef,
    node: &SemanticNode,
    out: &mut Vec<String>,
    selected: &mut Option<String>,
) {
    for child in &node.children {
        let child_at = NodeRef::new(at.tree.clone(), *child);
        let Some(child) = model.node(&child_at) else {
            continue;
        };
        if matches!(child.role, Role::Option | Role::MenuListOption) {
            if let Some(name) = non_empty(child.name.value.as_deref()) {
                if child.states.selected == Some(true) {
                    *selected = Some(name.clone());
                }
                out.push(name);
            }
        }
        collect_options(model, &child_at, child, out, selected);
    }
}

/// Name mit Herkunft. Chromium hat die Autorensemantik bereits ausgewertet;
/// ein vorhandener Name behält die Herkunft, die das Modell für ihn führt.
/// Fehlt er, versucht Relief deterministische Rekonstruktion.
fn name_fact(node: &SemanticNode) -> Fact<String> {
    if let Some(name) = non_empty(node.name.value.as_deref()) {
        return Fact {
            value: Some(name),
            ..node.name.clone()
        };
    }

    if let Some(desc) = non_empty(node.description.value.as_deref()) {
        return rule(
            Some(desc),
            Certainty::Inferred,
            "name-aus-beschreibung",
            "kein Name, aber Beschreibung vorhanden".into(),
        );
    }

    if node.role == Role::Link {
        if let Some(url) = &node.url {
            if let Some(word) = url_word(url) {
                return rule(
                    Some(word),
                    Certainty::Inferred,
                    "name-aus-url",
                    format!("url={url}"),
                );
            }
        }
    }

    rule(
        None,
        Certainty::Uncertain,
        "kein-name",
        "kein Accessible Name".into(),
    )
}

/// Aussage einer deterministischen Regel dieses Crates.
fn rule(value: Option<String>, certainty: Certainty, id: &str, evidence: String) -> Fact<String> {
    Fact {
        value,
        certainty,
        source: Source::Rule(id.to_string()),
        confidence: None,
        evidence: vec![evidence],
    }
}

/// Letztes sprechendes Pfadsegment einer URL: `/de/cart?x=1` → `cart`.
/// Dateiendungen und Wörter mit Ziffern (Artikel-IDs) fallen weg.
fn url_word(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next()?;
    let path = path
        .split("://")
        .nth(1)
        .map_or(path, |rest| rest.split_once('/').map_or("", |(_, p)| p));
    path.split('/').rev().find_map(|segment| {
        let stem = segment.split('.').next().unwrap_or(segment);
        let words: Vec<&str> = stem
            .split(['-', '_'])
            .filter(|w| w.len() > 1 && !w.chars().any(|c| c.is_ascii_digit()))
            .collect();
        (!words.is_empty()).then(|| words.join(" "))
    })
}

pub(crate) fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// Knoten `id` der Testseite aus [`sample_tree`].
#[cfg(test)]
pub(crate) fn at(id: i32) -> NodeRef {
    NodeRef::new(
        relief_model::TreeId("test".into()),
        relief_model::NodeId(id),
    )
}

/// Kleine, handgebaute Seite für browserfreie Tests: Produktseite mit
/// Abschnitten, Formular und Fußzeile. CDP-förmig geschrieben und über den
/// Konverter ins Modell gebracht; DOM-ID = Knoten-ID, Baum `test`.
#[cfg(test)]
pub(crate) fn sample_tree() -> SemanticGraph {
    use a11y_perception::{AXNode, AXProperty, AXTree, AXValue};

    fn n(id: u32, role: &str, name: &str, children: &[u32]) -> AXNode {
        AXNode {
            node_id: id.to_string(),
            role: Some(role.into()),
            name: (!name.is_empty()).then(|| name.into()),
            child_ids: children.iter().map(u32::to_string).collect(),
            backend_dom_node_id: Some(id.into()),
            ..Default::default()
        }
    }
    fn prop(mut node: AXNode, name: &str, value: AXValue) -> AXNode {
        node.properties.push(AXProperty {
            name: name.into(),
            value,
        });
        node
    }
    let heading = |id, name, level, children: &[u32]| {
        prop(
            n(id, "heading", name, children),
            "level",
            AXValue::Int(level),
        )
    };
    let mut menge = n(20, "spinbutton", "Menge", &[]);
    menge.value = Some("1".into());
    let menge = prop(menge, "valuemin", AXValue::Int(1));
    let menge = prop(menge, "valuemax", AXValue::Int(5));
    // Chrome wiederholt bei nativen Zahlenfeldern den Wert als `valuetext`.
    let menge = prop(menge, "valuetext", AXValue::String("1".into()));

    let tree = AXTree::from_nodes(vec![
        n(1, "RootWebArea", "Testseite", &[2, 25]),
        n(2, "main", "", &[3, 5, 9, 14, 15, 23, 27]),
        heading(3, "Bergstiefel", 1, &[4]),
        n(4, "StaticText", "Bergstiefel", &[]),
        n(5, "paragraph", "", &[6, 7, 28]),
        n(6, "StaticText", "Robust und wasserdicht. Mehr in den", &[]),
        n(7, "link", "Pflegehinweisen", &[8]),
        n(8, "StaticText", "Pflegehinweisen", &[]),
        n(28, "StaticText", ".", &[]),
        n(9, "region", "Technische Daten", &[10, 12]),
        heading(10, "Technische Daten", 2, &[11]),
        n(11, "StaticText", "Technische Daten", &[]),
        n(12, "paragraph", "", &[13]),
        n(13, "StaticText", "Obermaterial Leder.", &[]),
        heading(14, "Bestellung", 2, &[]),
        n(15, "form", "Bestellformular", &[29, 16, 20, 21]),
        n(29, "LabelText", "", &[30]),
        n(30, "StaticText", "Größe", &[]),
        n(16, "combobox", "Größe", &[17]),
        n(17, "MenuListPopup", "", &[18, 19]),
        n(18, "MenuListOption", "40", &[]),
        prop(
            n(19, "MenuListOption", "41", &[]),
            "selected",
            AXValue::Bool(true),
        ),
        menge,
        n(21, "button", "Bestellen", &[22]),
        n(22, "StaticText", "Bestellen", &[]),
        heading(23, "Versand", 3, &[]),
        n(27, "StaticText", "Lieferung in zwei Tagen.", &[]),
        n(25, "contentinfo", "", &[26]),
        n(26, "link", "Impressum", &[]),
    ]);
    relief_model::perception::from_tree(&tree, &relief_model::TreeId("test".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_texts_and_anchors() {
        let g = Graph::build(&sample_tree());
        assert_eq!(g.headings.len(), 4);
        // „Bestellung“ (H2) endet am Seitenende, weil „Versand“ H3 ist.
        assert_eq!(g.section_end(2), 4);
        assert_eq!(g.section_end(1), 2);
        let menge = g
            .controls
            .iter()
            .position(|c| c.name.value.as_deref() == Some("Menge"));
        assert_eq!(g.anchor(Some(&at(20))), menge.map(Anchor::Control));
        assert_eq!(g.anchor(Some(&at(10))), Some(Anchor::Heading(1)));
        assert_eq!(g.anchor(Some(&at(13))), None);
        assert_eq!(g.section_of(Anchor::Control(menge.unwrap())), Some(2));
        // Linktext bleibt Fließtext, Button- und Überschriftentext nicht.
        let texts: Vec<&str> = g.texts.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "Bergstiefel",
                "Robust und wasserdicht. Mehr in den",
                "Pflegehinweisen",
                ".",
                "Technische Daten",
                "Obermaterial Leder.",
                "Bestellung",
                "Versand",
                "Lieferung in zwei Tagen."
            ]
        );
        let menge = &g.controls[menge.unwrap()];
        assert_eq!(
            menge.states,
            [
                ("valuemin".to_string(), "1".to_string()),
                ("valuemax".to_string(), "5".to_string())
            ]
        );
        assert_eq!(menge.dom_node_id, Some(20));
    }

    #[test]
    fn focus_is_innermost_element_not_document() {
        let mut model = sample_tree();
        let tree = model.trees.values_mut().next().unwrap();
        tree.data.focus = tree.root;
        assert_eq!(focused(&model), None);
        let tree = model.trees.values_mut().next().unwrap();
        tree.data.focus = Some(relief_model::NodeId(20));
        assert_eq!(focused(&model), Some(at(20)));
    }

    /// Hauptdokument mit Link „Außen“ und einem iframe; im Frame ein Link
    /// „Frame“ und ein Dialog mit Button „OK“. `modal_in_main`: Der Dialog
    /// steht stattdessen im Hauptdokument und enthält das iframe.
    fn frame_page(modal_in_main: bool) -> SemanticGraph {
        use relief_model::{NodeId, SemanticTree};
        let node = |id: i32, role: Role, name: &str, parent: Option<i32>, children: &[i32]| {
            let mut n = SemanticNode::new(NodeId(id), role);
            n.name = Fact::known(Some(name.to_string()).filter(|s| !s.is_empty()));
            n.parent = parent.map(NodeId);
            n.children = children.iter().copied().map(NodeId).collect();
            n.states.modal = matches!(n.role, Role::Dialog);
            n
        };
        let tree = |id: &str, nodes: Vec<SemanticNode>| {
            let mut t = SemanticTree::new(TreeId(id.into()));
            t.root = Some(nodes[0].id);
            t.nodes = nodes.into_iter().map(|n| (n.id, n)).collect();
            t
        };
        let iframe_parent = if modal_in_main { 3 } else { 1 };
        let mut iframe = node(4, Role::Iframe, "", Some(iframe_parent), &[]);
        iframe.child_tree = Some(TreeId("frame".into()));
        let main = if modal_in_main {
            tree(
                "main",
                vec![
                    node(1, Role::RootWebArea, "", None, &[2, 3]),
                    node(2, Role::Link, "Außen", Some(1), &[]),
                    node(3, Role::Dialog, "Hinweis", Some(1), &[4]),
                    iframe,
                ],
            )
        } else {
            tree(
                "main",
                vec![
                    node(1, Role::RootWebArea, "", None, &[2, 4]),
                    node(2, Role::Link, "Außen", Some(1), &[]),
                    iframe,
                ],
            )
        };
        let frame_dialog_role = if modal_in_main {
            Role::Generic
        } else {
            Role::Dialog
        };
        let mut frame = tree(
            "frame",
            vec![
                node(1, Role::RootWebArea, "", None, &[2, 3]),
                node(2, Role::Link, "Frame", Some(1), &[]),
                node(3, frame_dialog_role, "Consent", Some(1), &[4]),
                node(4, Role::Button, "OK", Some(3), &[]),
            ],
        );
        frame.data.parent = Some(NodeRef::new(TreeId("main".into()), relief_model::NodeId(4)));
        SemanticGraph {
            root: Some(TreeId("main".into())),
            trees: [main, frame]
                .into_iter()
                .map(|t| (t.id.clone(), t))
                .collect(),
            ..Default::default()
        }
    }

    fn reachable(g: &Graph) -> Vec<&str> {
        g.reachable_controls()
            .filter_map(|c| c.name.value.as_deref())
            .collect()
    }

    #[test]
    fn modal_dialog_blocks_only_its_own_document() {
        // Dialog im Frame: sperrt „Frame“ neben sich, nicht „Außen“.
        let g = Graph::build(&frame_page(false));
        assert_eq!(
            g.regions[g.active_modal().unwrap()].label(),
            "dialog „Consent“"
        );
        assert_eq!(g.frames.len(), 1);
        assert_eq!(reachable(&g), ["Außen", "OK"]);
        // Dialog im Hauptdokument mit dem iframe darin: Der Frame liegt im
        // Dialog und bleibt bedienbar, „Außen“ ist gesperrt.
        let g = Graph::build(&frame_page(true));
        assert_eq!(
            g.regions[g.active_modal().unwrap()].label(),
            "dialog „Hinweis“"
        );
        assert_eq!(reachable(&g), ["Frame", "OK"]);
    }

    #[test]
    fn url_word_takes_last_meaningful_segment() {
        assert_eq!(
            url_word("https://shop.example/de/cart?x=1").as_deref(),
            Some("cart")
        );
        assert_eq!(url_word("/konto/").as_deref(), Some("konto"));
        assert_eq!(url_word("https://shop.example/").as_deref(), None);
        assert_eq!(url_word("/mein-konto.html").as_deref(), Some("mein konto"));
        assert_eq!(
            url_word("https://m.bild.de/lifestyle/horoskop/thema-horoskop-alle-infos-80687724.bildMobile.html").as_deref(),
            Some("thema horoskop alle infos")
        );
        assert_eq!(url_word("/p/12345/"), None);
    }
}

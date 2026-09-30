//! Privacy-Filter (→ `plan/spezifikation/07`).
//!
//! Sitzt in der Runtime, nicht im Anbieter-Adapter: [`filter`] ist der
//! einzige Weg zu einer [`FilteredInput`], und nur daraus entsteht eine
//! Modellanfrage. Der Filter kopiert nach **Positivliste** in eigene Typen
//! ([`ModelNode`]); was er nicht kennt, kommt nicht mit. Ein neues Feld in
//! [`SemanticNode`] gelangt also nie ungeprüft zu einem Modell.
//!
//! Was der Filter entfernt:
//!
//! - **Werte immer**: `value`, `checked` und `selected` aller Knoten fehlen
//!   in [`ModelNode`] ganz (Formularwerte, Auswahlzustände).
//! - **Inhalt von Eingabefeldern**: Die Kinder eines editierbaren Feldes
//!   (Chromium legt den eingegebenen Text als `StaticText` darunter) fallen
//!   weg, ebenso ein aus dem Inhalt berechneter Name. Struktur ja, Werte nein.
//! - **Sensible Felder** ([`Redaction::Sensitive`]): nur die Rolle bleibt.
//!   Sensibel sind Passwortfelder, Felder mit `autocomplete` für Zahlungs- und
//!   Identitätsdaten, alle Felder eines Formulars, das ein solches Feld
//!   enthält (Login/Checkout-Kontext), und alle Felder auf einer Seite, die
//!   der Aufrufer als sensibel meldet.
//! - **Adressen**: Query und Fragment von URLs (Sitzungs-Token u. ä.).
//! - **Identität**: Tree-IDs (beim CDP-Host die Dokument-URL) werden durch
//!   `t0`, `t1` … ersetzt; die Zuordnung bleibt in der Runtime.
//! - **Nicht Sichtbares**: von Chromium ignorierte Knoten und Inline-Textboxen
//!   (Doppel des `StaticText`); ihre Kinder hängen am nächsten übernommenen
//!   Vorfahren.

use std::collections::{BTreeMap, BTreeSet};

use relief_model::{
    GraphVersion, NameFrom, NodeRef, Rect, Role, SemanticGraph, SemanticNode, TreeId,
};
use serde::Serialize;

/// Angaben des Hosts zu einem Formularfeld, die der Accessibility-Tree nicht
/// trägt (HTML-Attribute aus dem DOM-Kontext).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldHint {
    /// HTML-`type`, z. B. `password`.
    pub input_type: Option<String>,
    /// HTML-`autocomplete` als Token-Liste, z. B. `shipping cc-number`.
    pub autocomplete: Option<String>,
}

impl FieldHint {
    /// Passwortfeld oder `autocomplete` für Zahlungs-/Identitätsdaten
    /// (dieselbe Regel wie für die Rückfrage,
    /// `relief_interaction::security::is_sensitive_field`).
    pub fn is_sensitive(&self) -> bool {
        relief_interaction::security::is_sensitive_field(
            self.input_type.as_deref(),
            self.autocomplete.as_deref(),
        )
    }
}

/// Was der Filter über den Graphen hinaus wissen muss.
#[derive(Debug, Clone, Default)]
pub struct PrivacyContext {
    /// Seitentyp Login, Checkout oder Zahlung: alle Felder sind sensibel.
    /// Den Seitentyp bestimmt die Runtime (`relief_interaction::Graph::page`,
    /// `PageType::is_sensitive`), nicht der Filter.
    pub sensitive_page: bool,
    /// Angaben zu einzelnen Feldern, soweit der Host sie kennt.
    pub fields: BTreeMap<NodeRef, FieldHint>,
}

/// Was an einem Feld entfernt wurde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Redaction {
    /// Eingabefeld: Wert und Inhalt entfernt; Rolle, Beschriftung und
    /// Zustände bleiben.
    Value,
    /// Sensibles Feld: nur die Rolle bleibt.
    Sensitive,
}

/// Ein Knoten, wie ein Modell ihn sieht.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelNode {
    /// Lokale ID, z. B. `t0:18`; gilt nur für diese Eingabe.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    /// Ohne Query und Fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Auswahl: `disabled`, `required`, `readonly`, `focusable`, `modal`,
    /// `expanded`/`collapsed`, `invalid`, `haspopup`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<Rect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted: Option<Redaction>,
}

/// Adresse und Titel des Hauptdokuments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PageInfo {
    /// Ohne Query und Fragment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Eingabe für ein Modell, durch den Privacy-Filter gegangen.
///
/// Entsteht **nur** über [`filter`]: Die Felder sind privat, es gibt keinen
/// Konstruktor und kein `Deserialize`. Weil ein [`crate::ModelProvider`] nur
/// eine [`crate::ModelRequest`] annimmt und diese nur aus einer
/// `FilteredInput` entsteht, ist kein Modellaufruf ohne Filter möglich.
///
/// Nicht von Hand zu bauen:
///
/// ```compile_fail
/// let input = relief_ai_contract::FilteredInput {
///     graph_version: relief_model::GraphVersion(0),
///     page: relief_ai_contract::PageInfo::default(),
///     nodes: Vec::new(),
///     targets: Default::default(),
/// };
/// ```
///
/// Nicht aus JSON zu lesen:
///
/// ```compile_fail
/// let input: relief_ai_contract::FilteredInput = serde_json::from_str("{}").unwrap();
/// ```
///
/// Der vorgesehene Weg:
///
/// ```
/// use relief_ai_contract::{filter, Budget, Limits, NoModel, PrivacyContext};
/// use relief_model::SemanticGraph;
///
/// let input = filter(&SemanticGraph::default(), &PrivacyContext::default());
/// let mut budget = Budget::new(Limits::default());
/// assert!(budget.resolve_missing(&NoModel, input).unwrap().is_empty());
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct FilteredInput {
    graph_version: GraphVersion,
    page: PageInfo,
    /// Knoten, um den es geht, wenn die Eingabe ein [`Self::excerpt`] ist.
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<String>,
    nodes: Vec<ModelNode>,
    /// Lokale ID → Knoten und was die Prüfung von Modellausgaben braucht.
    #[serde(skip)]
    targets: BTreeMap<String, Target>,
}

/// Runtime-Wissen zu einem übergebenen Knoten; geht nicht ans Modell.
#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub(crate) at: NodeRef,
    pub(crate) redaction: Option<Redaction>,
    /// Chromium meldet keinen (nicht leeren) Namen.
    pub(crate) missing_name: bool,
    pub(crate) missing_description: bool,
}

impl FilteredInput {
    /// Stand des Graphen, aus dem die Eingabe entstand.
    pub fn graph_version(&self) -> GraphVersion {
        self.graph_version
    }

    pub fn page(&self) -> &PageInfo {
        &self.page
    }

    /// Knoten in Dokumentreihenfolge, iframes an ihrer Stelle.
    pub fn nodes(&self) -> &[ModelNode] {
        &self.nodes
    }

    /// Knoten zur lokalen ID, wie ein Modell sie nennt.
    pub fn node_ref(&self, id: &str) -> Option<&NodeRef> {
        self.targets.get(id).map(|t| &t.at)
    }

    pub(crate) fn target(&self, id: &str) -> Option<&Target> {
        self.targets.get(id)
    }

    /// Knoten, um den es in einem Ausschnitt geht.
    pub fn focus(&self) -> Option<&str> {
        self.focus.as_deref()
    }

    /// Ausschnitt um einen Knoten: die Resolver-Eingabe (→ `spezifikation/06`,
    /// „Resolver-Eingabe“). `None`, wenn es `id` hier nicht gibt.
    ///
    /// Behalten werden
    ///
    /// - die Vorfahren von `id` bis zur Wurzel (Rolle und Name des Kontexts:
    ///   Landmark, Formular, Liste, Abschnitt);
    /// - ein Fenster von höchstens `max_nodes` Knoten in Dokumentreihenfolge
    ///   um `id` herum, innerhalb des kleinsten Vorfahren, dessen Teilbaum
    ///   mindestens `max_nodes` Knoten hat (oder der ganzen Eingabe). Darin
    ///   liegen die eigenen Kinder, Geschwister und benachbarter Text.
    ///
    /// Der Ausschnitt entfernt nur: Er entsteht aus einer gefilterten
    /// Eingabe, übernimmt deren Knoten unverändert bis auf `parent` (der
    /// nächste behaltene Vorfahr) und prüft Modellausgaben nur noch gegen
    /// die behaltenen Knoten.
    pub fn excerpt(&self, id: &str, max_nodes: usize) -> Option<FilteredInput> {
        let index: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let target = *index.get(id)?;
        let parent = |i: usize| {
            self.nodes[i]
                .parent
                .as_deref()
                .and_then(|p| index.get(p).copied())
        };
        // Knoten stehen in Vorordnung; ein Teilbaum ist ein zusammenhängender
        // Bereich bis zum nächsten Knoten gleicher oder geringerer Tiefe.
        let mut depth = vec![0usize; self.nodes.len()];
        for i in 0..self.nodes.len() {
            depth[i] = parent(i).map_or(0, |p| depth[p] + 1);
        }
        let subtree_end = |i: usize| {
            (i + 1..self.nodes.len())
                .find(|&j| depth[j] <= depth[i])
                .unwrap_or(self.nodes.len())
        };

        let mut scope = (target, subtree_end(target));
        let mut current = target;
        while scope.1 - scope.0 < max_nodes {
            let Some(p) = parent(current) else {
                scope = (0, self.nodes.len());
                break;
            };
            current = p;
            scope = (p, subtree_end(p));
        }
        let len = max_nodes.min(scope.1 - scope.0);
        let start = target
            .saturating_sub(len / 2)
            .max(scope.0)
            .min(scope.1 - len);

        let mut keep: BTreeSet<usize> = (start..start + len).collect();
        let mut ancestor = parent(target);
        while let Some(a) = ancestor {
            keep.insert(a);
            ancestor = parent(a);
        }
        keep.insert(target);

        let mut nodes = Vec::with_capacity(keep.len());
        let mut targets = BTreeMap::new();
        for &i in &keep {
            let mut node = self.nodes[i].clone();
            let mut up = parent(i);
            while let Some(p) = up.filter(|p| !keep.contains(p)) {
                up = parent(p);
            }
            node.parent = up.map(|p| self.nodes[p].id.clone());
            if let Some(t) = self.targets.get(&node.id) {
                targets.insert(node.id.clone(), t.clone());
            }
            nodes.push(node);
        }
        Some(FilteredInput {
            graph_version: self.graph_version,
            page: self.page.clone(),
            focus: Some(id.to_string()),
            nodes,
            targets,
        })
    }
}

/// Den Graphen für ein Modell filtern.
pub fn filter(graph: &SemanticGraph, context: &PrivacyContext) -> FilteredInput {
    let mut walk = Walk {
        graph,
        context,
        sensitive_forms: sensitive_forms(graph, context),
        trees: BTreeMap::new(),
        out: FilteredInput {
            graph_version: graph.version,
            page: PageInfo::default(),
            focus: None,
            nodes: Vec::new(),
            targets: BTreeMap::new(),
        },
    };
    if let Some(main) = graph.root.as_ref().and_then(|r| graph.trees.get(r)) {
        walk.out.page = PageInfo {
            url: main.data.url.as_deref().map(without_query),
            title: main.data.title.clone(),
        };
        if let Some(root) = main.root {
            walk.node(NodeRef::new(main.id.clone(), root), None, false);
        }
    }
    walk.out
}

/// Formulare mit einem sensiblen Feld: Login- bzw. Checkout-Kontext.
fn sensitive_forms(graph: &SemanticGraph, context: &PrivacyContext) -> BTreeSet<NodeRef> {
    let mut forms = BTreeSet::new();
    for (at, hint) in &context.fields {
        if !hint.is_sensitive() {
            continue;
        }
        let Some(tree) = graph.trees.get(&at.tree) else {
            continue;
        };
        let mut current = tree.nodes.get(&at.node).and_then(|n| n.parent);
        while let Some(id) = current {
            let Some(node) = tree.nodes.get(&id) else {
                break;
            };
            if node.role == Role::Form {
                forms.insert(NodeRef::new(at.tree.clone(), id));
                break;
            }
            current = node.parent;
        }
    }
    forms
}

struct Walk<'a> {
    graph: &'a SemanticGraph,
    context: &'a PrivacyContext,
    sensitive_forms: BTreeSet<NodeRef>,
    trees: BTreeMap<TreeId, usize>,
    out: FilteredInput,
}

impl Walk<'_> {
    fn node(&mut self, at: NodeRef, parent: Option<String>, in_sensitive_form: bool) {
        let Some(node) = self.graph.node(&at) else {
            return;
        };
        let in_sensitive_form = in_sensitive_form || self.sensitive_forms.contains(&at);
        let field = is_field(node);
        let redaction = field.then(|| {
            let sensitive = self.context.sensitive_page
                || in_sensitive_form
                || self
                    .context
                    .fields
                    .get(&at)
                    .is_some_and(FieldHint::is_sensitive);
            if sensitive {
                Redaction::Sensitive
            } else {
                Redaction::Value
            }
        });

        let mut child_parent = parent.clone();
        if !node.ignored && node.role != Role::InlineTextBox {
            let id = self.local_id(&at);
            self.out
                .nodes
                .push(model_node(node, &id, parent, redaction));
            self.out.targets.insert(
                id.clone(),
                Target {
                    at: at.clone(),
                    redaction,
                    missing_name: missing(node.name.value.as_deref()),
                    missing_description: missing(node.description.value.as_deref()),
                },
            );
            child_parent = Some(id);
        }

        // Unter einem Eingabefeld steht, was eingegeben wurde; unter einem
        // sensiblen Feld gar nichts, was ein Modell sehen soll.
        if node.states.editable.is_some() || redaction == Some(Redaction::Sensitive) {
            return;
        }
        for child in &node.children {
            self.node(
                NodeRef::new(at.tree.clone(), *child),
                child_parent.clone(),
                in_sensitive_form,
            );
        }
        if let Some(tree) = node
            .child_tree
            .as_ref()
            .and_then(|t| self.graph.trees.get(t))
        {
            if let Some(root) = tree.root {
                self.node(
                    NodeRef::new(tree.id.clone(), root),
                    child_parent,
                    in_sensitive_form,
                );
            }
        }
    }

    fn local_id(&mut self, at: &NodeRef) -> String {
        let next = self.trees.len();
        let tree = *self.trees.entry(at.tree.clone()).or_insert(next);
        format!("t{tree}:{}", at.node)
    }
}

/// Ein Feld, dessen Wert Nutzereingabe ist.
fn is_field(node: &SemanticNode) -> bool {
    matches!(
        node.role,
        Role::Textbox
            | Role::SearchBox
            | Role::Combobox
            | Role::SpinButton
            | Role::Slider
            | Role::Checkbox
            | Role::Radio
            | Role::Switch
            | Role::Listbox
    ) || node.states.editable.is_some()
        || node.states.settable
}

fn model_node(
    node: &SemanticNode,
    id: &str,
    parent: Option<String>,
    redaction: Option<Redaction>,
) -> ModelNode {
    let mut m = ModelNode {
        id: id.to_string(),
        parent,
        role: node.role.as_str().to_string(),
        name: None,
        description: None,
        level: node.level,
        url: node.url.as_deref().map(without_query),
        states: states(node),
        bounds: node.bounds,
        redacted: redaction,
    };
    if redaction == Some(Redaction::Sensitive) {
        return ModelNode {
            level: None,
            url: None,
            states: Vec::new(),
            bounds: None,
            ..m
        };
    }
    // Bei editierbaren Feldern ist ein Name aus dem Inhalt die Eingabe selbst.
    let name_from_input = node.states.editable.is_some()
        && !matches!(
            node.name_from,
            Some(
                NameFrom::Attribute
                    | NameFrom::RelatedElement
                    | NameFrom::Placeholder
                    | NameFrom::Title
            )
        );
    if !name_from_input {
        m.name = present(node.name.value.as_deref());
    }
    m.description = present(node.description.value.as_deref());
    m
}

fn states(node: &SemanticNode) -> Vec<&'static str> {
    let s = &node.states;
    let mut out = Vec::new();
    for (on, name) in [
        (s.disabled, "disabled"),
        (s.required, "required"),
        (s.readonly, "readonly"),
        (s.focusable, "focusable"),
        (s.modal, "modal"),
        (s.expanded == Some(true), "expanded"),
        (s.expanded == Some(false), "collapsed"),
        (s.invalid.is_some(), "invalid"),
        (s.has_popup.is_some(), "haspopup"),
    ] {
        if on {
            out.push(name);
        }
    }
    out
}

fn missing(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}

fn present(value: Option<&str>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty()).map(str::to_string)
}

fn without_query(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autocomplete_token_entscheiden() {
        let hint = |a: &str| FieldHint {
            input_type: None,
            autocomplete: Some(a.into()),
        };
        assert!(hint("cc-number").is_sensitive());
        assert!(hint("section-x shipping CC-CSC").is_sensitive());
        assert!(hint("one-time-code").is_sensitive());
        assert!(!hint("email").is_sensitive());
        assert!(!hint("off").is_sensitive());
        let password = FieldHint {
            input_type: Some("Password".into()),
            autocomplete: None,
        };
        assert!(password.is_sensitive());
    }

    #[test]
    fn urls_ohne_query_und_fragment() {
        assert_eq!(
            without_query("https://example.org/cart?session=abc#top"),
            "https://example.org/cart"
        );
        assert_eq!(without_query("/cart#x"), "/cart");
        assert_eq!(without_query("/cart"), "/cart");
    }
}

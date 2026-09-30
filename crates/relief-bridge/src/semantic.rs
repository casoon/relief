//! Semantic View im Fork (Paket 29): eine aus dem Interaction Graph
//! erzeugte, bedienbare Ansicht der Seite. Die Originalseite bleibt geladen;
//! jede Bedienung in der Ansicht läuft als validierte Aktion über die
//! Sitzung auf die Originalseite ([`Runtime::view_act`]), mit Rückfrage bei
//! Risiko wie ein Befehl (→ `plan/spezifikation/08`, „Semantic View“).
//!
//! Die Einträge stehen in Dokumentreihenfolge: Überschriften, Texte und
//! Bedienelemente, je mit dem Bereich, in dem sie liegen. Werte sensibler
//! Felder erscheinen nie im Klartext.

use relief_interaction::{ActionKind, Control, Graph};
use relief_model::{NodeRef, Role, SemanticGraph, Toggle};
use serde::Serialize;

use crate::command::Reply;
use crate::inspector::{certainty, key, parse_key};
use crate::runtime::Runtime;

/// Ein Eintrag der Ansicht.
#[derive(Serialize)]
pub(crate) struct Entry {
    /// `heading`, `text` oder `control`.
    kind: &'static str,
    /// Schlüssel wie im Inspector; Texte haben keinen.
    key: Option<String>,
    /// Bereich, in dem der Eintrag liegt.
    region: Option<String>,
    text: String,
    level: Option<u8>,
    /// Nur Bedienelemente: Rolle, Herkunft des Namens und die Art, wie die
    /// Ansicht es darstellt (`button`, `link`, `textbox`, `checkbox`,
    /// `radio`, `select`, `number`, `other`).
    role: Option<String>,
    certainty: Option<&'static str>,
    control: Option<&'static str>,
    value: Option<String>,
    sensitive: bool,
    options: Vec<String>,
    selected: Option<String>,
    checked: Option<bool>,
    disabled: bool,
    /// Gesperrt, solange ein modaler Dialog offen ist.
    reachable: bool,
}

fn control_kind(role: &Role) -> &'static str {
    match role {
        Role::Button | Role::MenuItem | Role::Tab => "button",
        Role::Link => "link",
        Role::Textbox | Role::SearchBox => "textbox",
        Role::Checkbox | Role::Switch => "checkbox",
        Role::Radio => "radio",
        Role::Combobox | Role::Listbox => "select",
        Role::SpinButton | Role::Slider => "number",
        _ => "other",
    }
}

fn control_entry(graph: &Graph, model: &SemanticGraph, c: &Control) -> Entry {
    let checked = model
        .node(&c.node)
        .and_then(|n| n.states.checked)
        .map(|t| t == Toggle::True);
    Entry {
        kind: "control",
        key: Some(key(&c.node)),
        region: c.region.map(|r| graph.regions[r].label()),
        text: c.display_name(),
        level: None,
        role: Some(c.role.to_string()),
        certainty: Some(certainty(c.name.certainty)),
        control: Some(control_kind(&c.role)),
        value: if c.sensitive { None } else { c.value.clone() },
        sensitive: c.sensitive,
        options: c.options.clone(),
        selected: c.selected_option.clone(),
        checked,
        disabled: c.disabled,
        reachable: graph.is_reachable(c.region, &c.node),
    }
}

/// Einträge der Ansicht in Dokumentreihenfolge.
pub(crate) fn entries(graph: &Graph, model: &SemanticGraph) -> Vec<Entry> {
    let order: std::collections::HashMap<NodeRef, usize> = model
        .document_order()
        .into_iter()
        .enumerate()
        .map(|(i, at)| (at, i))
        .collect();
    let position = |at: &NodeRef| order.get(at).copied().unwrap_or(usize::MAX);
    let mut out: Vec<(usize, Entry)> = Vec::new();
    for h in &graph.headings {
        out.push((
            position(&h.node),
            Entry {
                kind: "heading",
                key: Some(key(&h.node)),
                region: h.region.map(|r| graph.regions[r].label()),
                text: h.text.clone(),
                level: Some(h.level),
                role: None,
                certainty: None,
                control: None,
                value: None,
                sensitive: false,
                options: Vec::new(),
                selected: None,
                checked: None,
                disabled: false,
                reachable: graph.is_reachable(h.region, &h.node),
            },
        ));
    }
    // Überschriften-Texte stehen schon als Überschrift da, Texte in einem
    // Bedienelement (Link, Button) als dessen Name.
    let controls: std::collections::HashSet<&NodeRef> =
        graph.controls.iter().map(|c| &c.node).collect();
    let inside_control = |at: &NodeRef| {
        let mut parent = model.node(at).and_then(|n| n.parent);
        while let Some(id) = parent {
            let p = NodeRef::new(at.tree.clone(), id);
            if controls.contains(&p) {
                return true;
            }
            parent = model.node(&p).and_then(|n| n.parent);
        }
        false
    };
    for t in graph
        .texts
        .iter()
        .filter(|t| t.level.is_none() && !inside_control(&t.node))
    {
        out.push((
            position(&t.node),
            Entry {
                kind: "text",
                key: None,
                region: t.region.map(|r| graph.regions[r].label()),
                text: t.text.clone(),
                level: None,
                role: None,
                certainty: None,
                control: None,
                value: None,
                sensitive: false,
                options: Vec::new(),
                selected: None,
                checked: None,
                disabled: false,
                reachable: graph.is_reachable(t.region, &t.node),
            },
        ));
    }
    for c in &graph.controls {
        out.push((position(&c.node), control_entry(graph, model, c)));
    }
    out.sort_by_key(|(p, _)| *p);
    out.into_iter().map(|(_, e)| e).collect()
}

impl Runtime {
    /// Bedienung aus der Semantic View: `kind` ist `activate`, `set`
    /// (`value` = Text), `select` (`value` = Option), `increment` oder
    /// `decrement`. Läuft wie ein Befehl über die Sitzung (Validierung,
    /// Rückfrage, Security-Log); „ja“/„abbrechen“ gehen danach wie gewohnt
    /// über [`Runtime::command`].
    pub fn view_act(&mut self, key: &str, kind: &str, value: &str) -> Reply {
        self.pending = None;
        let graph = Graph::build(&self.graph);
        let Some(control) = parse_key(key)
            .and_then(|at| graph.controls.iter().find(|c| c.node == at))
            .cloned()
        else {
            return Reply::Answer("Das Element ist nicht mehr auf der Seite.".into());
        };
        if !graph.is_reachable(control.region, &control.node) {
            return Reply::Answer(format!(
                "Gesperrt, solange ein modaler Dialog offen ist: {}",
                control.display_name()
            ));
        }
        let kind = match kind {
            "activate" => ActionKind::Activate,
            "set" => ActionKind::SetValue(value.to_string()),
            "select" => ActionKind::Select(value.to_string()),
            "increment" => ActionKind::Increment,
            "decrement" => ActionKind::Decrement,
            other => return Reply::Answer(format!("Unbekannte Bedienung „{other}“.")),
        };
        let outcome = self.session.request(&graph, &self.graph, control, kind);
        self.reply(graph, outcome)
    }
}

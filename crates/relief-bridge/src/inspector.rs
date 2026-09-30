//! Daten für den Semantic Inspector im Fork (Paket 20): Bereiche,
//! Überschriften und Bedienelemente des Interaction Graph mit Rolle, Namen
//! samt Herkunft (`Certainty`), Zuständen, Aktionen und Beziehungen, als
//! JSON für die WebUI.
//!
//! Auswahl und Aktivierung sind getrennt: Die WebUI wählt über den
//! Schlüssel eines Eintrags aus; „im Dokument zeigen“ ([`Runtime::show`])
//! bewegt Relief dorthin, ohne etwas auszulösen (Fokus bzw. Hinbewegen,
//! Risiko LOW).

use relief_interaction::{
    plan_navigation, plan_on_page, respond, rules, ActionKind, Control, Graph, Place,
};
use relief_model::{Action, Certainty, Fact, NodeId, NodeRef, SemanticGraph, Source, TreeId};
use serde::Serialize;

use crate::command::{ax_steps, Pending, Reply};
use crate::runtime::Runtime;

#[derive(Serialize)]
struct View {
    version: u64,
    title: Option<String>,
    url: Option<String>,
    /// Seitentyp mit Herkunft, z. B. „Seitentyp vermutlich Produktseite
    /// (erschlossen: …)“; fehlt, wenn unbekannt.
    page: Option<String>,
    /// Schlüssel des Eintrags am Fokus bzw. an der Position.
    focus: Option<String>,
    regions: Vec<Item>,
    headings: Vec<Item>,
    controls: Vec<Item>,
    /// Prüfung mit `a11y-rules` (Paket 21).
    checks: Checks,
}

#[derive(Serialize)]
struct Checks {
    /// Regelkennungen, die gelaufen sind.
    ran: usize,
    /// Befunde, die keinem Eintrag der Listen gehören (z. B. Text).
    page: Vec<FindingView>,
    /// Nicht gelaufene Regeln mit Grund (nicht geprüft ist nicht bestanden).
    not_run: Vec<[String; 2]>,
}

#[derive(Serialize, Clone)]
struct FindingView {
    rule: String,
    /// `fail`, `review`, `untested`.
    outcome: &'static str,
    severity: &'static str,
    message: String,
}

#[derive(Serialize)]
struct Item {
    /// `<Baum>#<Knoten>`; bleibt über Deltas gleich, solange der Knoten
    /// besteht.
    key: String,
    /// Kurzzeile für die Liste.
    label: String,
    role: String,
    name: Option<String>,
    /// Herkunft des Namens: `known`, `inferred`, `uncertain`.
    certainty: &'static str,
    /// Wer den Namen erzeugt hat und woraus.
    origin: String,
    /// Bereich, in dem der Eintrag liegt.
    region: Option<String>,
    level: Option<u8>,
    value: Option<String>,
    states: Vec<[String; 2]>,
    actions: Vec<String>,
    relations: Vec<[String; 2]>,
    /// Gesperrt, solange ein modaler Dialog offen ist.
    reachable: bool,
    findings: Vec<FindingView>,
}

fn key(at: &NodeRef) -> String {
    format!("{}#{}", at.tree.0, at.node.0)
}

fn parse_key(text: &str) -> Option<NodeRef> {
    let (tree, node) = text.rsplit_once('#')?;
    Some(NodeRef::new(
        TreeId(tree.to_string()),
        NodeId(node.parse().ok()?),
    ))
}

fn certainty(c: Certainty) -> &'static str {
    match c {
        Certainty::Known => "known",
        Certainty::Inferred => "inferred",
        Certainty::Uncertain => "uncertain",
    }
}

fn origin<T>(fact: &Fact<T>) -> String {
    let source = match &fact.source {
        Source::Chromium => "Chromium".to_string(),
        Source::Rule(id) => format!("Regel {id}"),
        Source::Model(id) => format!("Modell {id}"),
    };
    if fact.evidence.is_empty() {
        source
    } else {
        format!("{source}: {}", fact.evidence.join(", "))
    }
}

fn action_name(action: Action) -> &'static str {
    match action {
        Action::DoDefault => "auslösen",
        Action::Focus => "fokussieren",
        Action::Blur => "Fokus abgeben",
        Action::SetValue => "Wert setzen",
        Action::Increment => "erhöhen",
        Action::Decrement => "verringern",
        Action::Expand => "aufklappen",
        Action::Collapse => "zuklappen",
        Action::ScrollToMakeVisible => "sichtbar machen",
        Action::ShowContextMenu => "Kontextmenü",
        Action::SetSequentialFocusNavigationStartingPoint => "Tab-Startpunkt",
    }
}

/// Aktionen und Beziehungen eines Knotens aus dem Modell, Beziehungen mit
/// den Namen der Zielknoten.
fn node_details(model: &SemanticGraph, at: &NodeRef) -> (Vec<String>, Vec<[String; 2]>) {
    let Some(node) = model.node(at) else {
        return (Vec::new(), Vec::new());
    };
    let actions = node
        .actions
        .iter()
        .map(|a| action_name(*a).to_string())
        .collect();
    let name_of = |id: &NodeId| {
        model
            .node(&NodeRef::new(at.tree.clone(), *id))
            .map(|n| match &n.name.value {
                Some(name) if !name.is_empty() => format!("{} „{name}“", n.role),
                _ => n.role.to_string(),
            })
            .unwrap_or_else(|| format!("Knoten {}", id.0))
    };
    let r = &node.relations;
    let mut relations = Vec::new();
    for (label, ids) in [
        ("beschriftet von", &r.labelled_by),
        ("beschrieben von", &r.described_by),
        ("steuert", &r.controls),
        ("Details", &r.details),
        ("Fehlermeldung", &r.error_message),
        ("weiter zu", &r.flow_to),
    ] {
        if !ids.is_empty() {
            relations.push([
                label.to_string(),
                ids.iter().map(name_of).collect::<Vec<_>>().join(", "),
            ]);
        }
    }
    if let Some(id) = &r.active_descendant {
        relations.push(["aktives Element".to_string(), name_of(id)]);
    }
    (actions, relations)
}

fn control_item(graph: &Graph, model: &SemanticGraph, c: &Control) -> Item {
    let (actions, relations) = node_details(model, &c.node);
    Item {
        key: key(&c.node),
        label: respond::control_line(c),
        role: c.role.to_string(),
        name: c.name.value.clone(),
        certainty: certainty(c.name.certainty),
        origin: origin(&c.name),
        region: c.region.map(|r| graph.regions[r].label()),
        level: None,
        value: c.value.clone(),
        states: c
            .states
            .iter()
            .map(|(k, v)| [k.clone(), v.clone()])
            .collect(),
        actions,
        relations,
        reachable: graph.is_reachable(c.region, &c.node),
        findings: Vec::new(),
    }
}

/// Der aktuelle Graph als JSON für die WebUI.
pub fn inspector_json(runtime: &Runtime) -> String {
    let model = runtime.graph();
    let graph = Graph::build(model);
    // Befunde je Knoten (Schlüssel wie die Einträge).
    let doc = rules::AxDocument::new(model);
    let report = rules::check(&doc);
    let mut by_key: std::collections::HashMap<String, Vec<FindingView>> =
        std::collections::HashMap::new();
    for f in report
        .findings
        .iter()
        .filter(|f| f.outcome.is_visible_by_default())
    {
        let view = FindingView {
            rule: f.rule_id.clone(),
            outcome: f.outcome.as_str(),
            severity: f.severity.as_str(),
            message: f.message.clone(),
        };
        let at = f.location.node.as_deref().and_then(|n| doc.node_ref(n));
        by_key
            .entry(at.map(key).unwrap_or_default())
            .or_default()
            .push(view);
    }
    let main = model.root.as_ref().and_then(|t| model.trees.get(t));
    let focus = runtime
        .session
        .position(relief_interaction::focused(model).as_ref());

    let regions = graph
        .regions
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (actions, relations) = node_details(model, &r.node);
            let name = Fact::known(r.name.clone());
            Item {
                key: key(&r.node),
                label: r.label(),
                role: r.role.to_string(),
                name: r.name.clone(),
                certainty: certainty(name.certainty),
                origin: origin(&name),
                region: r.parent.map(|p| graph.regions[p].label()),
                level: None,
                value: None,
                states: if r.modal {
                    vec![["modal".into(), "true".into()]]
                } else {
                    Vec::new()
                },
                actions,
                relations,
                reachable: graph.is_reachable(Some(i), &r.node),
                findings: Vec::new(),
            }
        })
        .collect();
    let headings = graph
        .headings
        .iter()
        .map(|h| {
            let (actions, relations) = node_details(model, &h.node);
            let name = Fact::known(Some(h.text.clone()));
            Item {
                key: key(&h.node),
                label: format!("H{} {}", h.level, h.text),
                role: "heading".into(),
                name: Some(h.text.clone()),
                certainty: certainty(name.certainty),
                origin: origin(&name),
                region: h.region.map(|r| graph.regions[r].label()),
                level: Some(h.level),
                value: None,
                states: Vec::new(),
                actions,
                relations,
                reachable: graph.is_reachable(h.region, &h.node),
                findings: Vec::new(),
            }
        })
        .collect();
    let controls = graph
        .controls
        .iter()
        .map(|c| control_item(&graph, model, c))
        .collect();

    let mut take = |k: &str| by_key.remove(k).unwrap_or_default();
    let mut regions: Vec<Item> = regions;
    let mut headings: Vec<Item> = headings;
    let mut controls: Vec<Item> = controls;
    for item in regions.iter_mut().chain(&mut headings).chain(&mut controls) {
        item.findings = take(&item.key);
    }
    let checks = Checks {
        ran: report.rule_runs.iter().filter(|r| r.did_run()).count(),
        page: by_key.into_values().flatten().collect(),
        not_run: report
            .rule_runs
            .iter()
            .filter(|r| !r.did_run())
            .map(|r| [r.rule_id.clone(), r.reason.clone().unwrap_or_default()])
            .collect(),
    };
    let view = View {
        version: model.version.0,
        title: main.and_then(|t| t.data.title.clone()),
        url: main.and_then(|t| t.data.url.clone()),
        page: respond::page_type(&graph),
        focus: focus.as_ref().map(key),
        regions,
        headings,
        controls,
        checks,
    };
    serde_json::to_string(&view).unwrap_or_else(|_| "{}".into())
}

impl Runtime {
    /// „Im Dokument zeigen“: Relief bewegt sich zum Eintrag, ohne ihn
    /// auszulösen. Bedienelemente werden fokussiert, Überschriften und
    /// Bereiche angesteuert; gesperrte Einträge (modaler Dialog offen)
    /// werden abgelehnt.
    pub fn show(&mut self, key: &str) -> Reply {
        self.pending = None;
        let Some(at) = parse_key(key) else {
            return Reply::Answer("Unbekannter Eintrag.".into());
        };
        let graph = Graph::build(&self.graph);
        let (plan, label) = if let Some(c) = graph.controls.iter().find(|c| c.node == at) {
            let label = respond::control_line(c);
            if !graph.is_reachable(c.region, &c.node) {
                return Reply::Answer(format!(
                    "Gesperrt, solange ein modaler Dialog offen ist: {label}"
                ));
            }
            let kind = if c.focusable {
                ActionKind::Focus
            } else {
                ActionKind::NavigateTo
            };
            let plan = if kind == ActionKind::Focus {
                plan_on_page(&graph.page, c, kind)
            } else {
                plan_navigation(&c.node, c.dom_node_id)
            };
            (plan, label)
        } else if let Some(h) = graph.headings.iter().position(|h| h.node == at) {
            let heading = &graph.headings[h];
            (
                plan_navigation(&heading.node, heading.dom_node_id),
                respond::place_label(&graph, Place::Heading(h)),
            )
        } else if let Some(r) = graph.regions.iter().position(|r| r.node == at) {
            let region = &graph.regions[r];
            (
                plan_navigation(&region.node, region.dom_node_id),
                respond::place_label(&graph, Place::Region(r)),
            )
        } else {
            return Reply::Answer("Eintrag ist nicht mehr auf der Seite.".into());
        };
        let plan = match plan {
            Ok(p) => p,
            Err(rejection) => return Reply::Answer(format!("Abgelehnt: {rejection} ({label})")),
        };
        match ax_steps(&self.graph, &plan) {
            Ok(steps) => {
                self.pending = Some(Pending::Plan {
                    plan,
                    label,
                    before: self.graph.clone(),
                    before_graph: Box::new(graph),
                });
                Reply::Perform(steps)
            }
            Err(msg) => Reply::Answer(format!("Aktion fehlgeschlagen: {msg} ({label})")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schluessel_hin_und_zurueck() {
        let at = NodeRef::new(TreeId("7a-b#c".into()), NodeId(42));
        assert_eq!(parse_key(&key(&at)), Some(at));
        assert_eq!(parse_key("ohne"), None);
    }
}

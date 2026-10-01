//! Die Runtime hinter der Grenze: hält den Graphen, nimmt Deltas an,
//! beantwortet Anfragen und prüft Aktionen.

use std::fmt;

use relief_model::{
    Action, ApplyError, Fact, GraphVersion, NodeRef, Role, SemanticGraph, TreeDelta,
};

/// Zustand einer Seite (eines Tabs) auf der Rust-Seite der Grenze. Nicht
/// kopierbar, weil die Sitzung eine offene Rückfrage hält.
#[derive(Debug, Default)]
pub struct Runtime {
    pub(crate) graph: SemanticGraph,
    /// Befehle in Sprache (→ `command`).
    pub(crate) session: relief_interaction::Session,
    pub(crate) pending: Option<crate::command::Pending>,
    /// HTML-`autocomplete` je Feld, vom Renderer erfragt (Pakete 75, 112).
    /// Haftet am Knoten über Deltas, die ihn ersetzen; der AXTree trägt es
    /// nicht.
    pub(crate) autocomplete: std::collections::HashMap<relief_model::NodeRef, String>,
}

/// Auskunft über einen Knoten, mit Herkunft des Namens.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// Stand des Graphen, auf den sich die Auskunft bezieht.
    pub version: GraphVersion,
    pub node: NodeRef,
    pub role: Role,
    pub name: Fact<String>,
}

/// Wunsch nach einer Browseraktion, z. B. aus einem Befehl.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRequest {
    pub target: NodeRef,
    pub action: Action,
    /// Nur bei [`Action::SetValue`].
    pub value: Option<String>,
    /// Stand des Graphen, auf dem der Wunsch entstand.
    pub version: GraphVersion,
}

/// Geprüfte Aktion: das Einzige, was zurück an den Browser geht
/// (Chromium: `AXActionData`, → `plan/spezifikation/02`, „Datenfluss zurück“).
#[derive(Debug, Clone, PartialEq)]
pub struct ActionPlan {
    pub target: NodeRef,
    pub action: Action,
    pub value: Option<String>,
    /// Stand, gegen den geprüft wurde. Der Adapter prüft vor dem Senden, ob
    /// der Knoten noch existiert.
    pub version: GraphVersion,
}

/// Warum aus einem [`ActionRequest`] kein [`ActionPlan`] wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    /// Der Graph hat sich seit dem Wunsch geändert.
    Stale {
        requested: GraphVersion,
        current: GraphVersion,
    },
    UnknownNode,
    /// Chromium nimmt den Knoten aus der Barrierefreiheits-Sicht heraus.
    Ignored,
    Disabled,
    /// Chromium meldet die Aktion für den Knoten nicht.
    Unsupported,
    /// `SetValue` ohne Wert oder ein Wert bei einer anderen Aktion.
    ValueMismatch,
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale { requested, current } => write!(
                f,
                "veraltet: Wunsch auf Version {}, Graph auf {}",
                requested.0, current.0
            ),
            Self::UnknownNode => f.write_str("unbekannter Knoten"),
            Self::Ignored => f.write_str("Knoten ist ignoriert"),
            Self::Disabled => f.write_str("Knoten ist deaktiviert"),
            Self::Unsupported => f.write_str("Aktion wird für den Knoten nicht gemeldet"),
            Self::ValueMismatch => f.write_str("Wert passt nicht zur Aktion"),
        }
    }
}

impl std::error::Error for Rejection {}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn graph(&self) -> &SemanticGraph {
        &self.graph
    }

    /// Wendet eine Delta an; bei Fehler bleibt der Graph unverändert.
    pub fn apply(&mut self, delta: &TreeDelta) -> Result<GraphVersion, ApplyError> {
        self.graph.apply(delta)?;
        // Erfragtes `autocomplete` wieder anheften; verschwundene Knoten
        // vergessen (IDs können wiederkommen).
        let graph = &mut self.graph;
        self.autocomplete.retain(|at, value| {
            let Some(node) = graph
                .trees
                .get_mut(&at.tree)
                .and_then(|t| t.nodes.get_mut(&at.node))
            else {
                return false;
            };
            node.extra
                .entry(relief_interaction::security::HTML_AUTOCOMPLETE.into())
                .or_insert_with(|| value.clone());
            true
        });
        Ok(self.graph.version)
    }

    /// Der fokussierte Knoten des Hauptdokuments.
    pub fn focus(&self) -> Option<NodeRef> {
        let root = self.graph.root.as_ref()?;
        let focus = self.graph.trees.get(root)?.data.focus?;
        Some(NodeRef::new(root.clone(), focus))
    }

    /// Rolle und Name eines Knotens.
    pub fn describe(&self, at: &NodeRef) -> Option<Answer> {
        let node = self.graph.node(at)?;
        Some(Answer {
            version: self.graph.version,
            node: at.clone(),
            role: node.role.clone(),
            name: node.name.clone(),
        })
    }

    /// Erster Knoten in Dokumentreihenfolge, der genau `name` trägt, nicht
    /// ignoriert ist und `action` meldet.
    ///
    /// Einfachste Auflösung eines Ziels für den Durchstich im Fork
    /// (`--relief-activate`); Befehle in Sprache kommen mit Paket 32.
    pub fn find(&self, name: &str, action: Action) -> Option<NodeRef> {
        self.graph.document_order().into_iter().find(|at| {
            self.graph.node(at).is_some_and(|node| {
                !node.ignored
                    && node.name.value.as_deref() == Some(name)
                    && node.actions.contains(&action)
            })
        })
    }

    /// Prüft einen Aktionswunsch gegen den aktuellen Graphen.
    ///
    /// Maßgeblich sind die Aktionen, die Chromium am Knoten meldet; der
    /// CDP-Konverter meldet keine, auf CDP-Aufnahmen wird also jede Aktion
    /// abgelehnt.
    pub fn plan(&self, request: &ActionRequest) -> Result<ActionPlan, Rejection> {
        if request.version != self.graph.version {
            return Err(Rejection::Stale {
                requested: request.version,
                current: self.graph.version,
            });
        }
        let node = self
            .graph
            .node(&request.target)
            .ok_or(Rejection::UnknownNode)?;
        if node.ignored {
            return Err(Rejection::Ignored);
        }
        if node.states.disabled {
            return Err(Rejection::Disabled);
        }
        if !node.actions.contains(&request.action) {
            return Err(Rejection::Unsupported);
        }
        if (request.action == Action::SetValue) != request.value.is_some() {
            return Err(Rejection::ValueMismatch);
        }
        Ok(ActionPlan {
            target: request.target.clone(),
            action: request.action,
            value: request.value.clone(),
            version: self.graph.version,
        })
    }
}

#[cfg(test)]
mod tests {
    use relief_model::{NodeId, SemanticNode, TreeData, TreeId, TreeUpdate};

    use super::*;

    fn runtime_mit_button() -> (Runtime, NodeRef) {
        let tree = TreeId("t".into());
        let mut root = SemanticNode::new(NodeId(1), Role::from_name("rootWebArea"));
        root.children = vec![NodeId(2)];
        let mut button = SemanticNode::new(NodeId(2), Role::from_name("button"));
        button.parent = Some(NodeId(1));
        button.name = Fact::known(Some("Senden".into()));
        button.actions = vec![Action::DoDefault, Action::Focus];
        let delta = TreeDelta {
            base: GraphVersion(0),
            root: Some(tree.clone()),
            removed_trees: Vec::new(),
            trees: vec![TreeUpdate {
                tree: tree.clone(),
                data: Some(TreeData {
                    focus: Some(NodeId(2)),
                    ..TreeData::default()
                }),
                root: Some(NodeId(1)),
                removed: Vec::new(),
                created: vec![root, button],
                changed: Vec::new(),
                bounds: Vec::new(),
            }],
        };
        let mut rt = Runtime::new();
        assert_eq!(rt.apply(&delta), Ok(GraphVersion(1)));
        (rt, NodeRef::new(tree, NodeId(2)))
    }

    fn wunsch(target: &NodeRef, action: Action, version: u64) -> ActionRequest {
        ActionRequest {
            target: target.clone(),
            action,
            value: None,
            version: GraphVersion(version),
        }
    }

    #[test]
    fn fokus_und_auskunft() {
        let (rt, button) = runtime_mit_button();
        assert_eq!(rt.focus(), Some(button.clone()));
        let answer = rt.describe(&button).unwrap();
        assert_eq!(answer.name.value.as_deref(), Some("Senden"));
        assert_eq!(answer.version, GraphVersion(1));
    }

    #[test]
    fn gemeldete_aktion_wird_plan() {
        let (rt, button) = runtime_mit_button();
        let plan = rt.plan(&wunsch(&button, Action::DoDefault, 1)).unwrap();
        assert_eq!(plan.target, button);
        assert_eq!(plan.version, GraphVersion(1));
    }

    #[test]
    fn findet_benannten_knoten_mit_aktion() {
        let (rt, button) = runtime_mit_button();
        assert_eq!(rt.find("Senden", Action::DoDefault), Some(button));
        assert_eq!(rt.find("Senden", Action::Expand), None);
        assert_eq!(rt.find("Abbrechen", Action::DoDefault), None);
    }

    #[test]
    fn ablehnungen() {
        let (rt, button) = runtime_mit_button();
        assert_eq!(
            rt.plan(&wunsch(&button, Action::DoDefault, 0)),
            Err(Rejection::Stale {
                requested: GraphVersion(0),
                current: GraphVersion(1)
            })
        );
        assert_eq!(
            rt.plan(&wunsch(&button, Action::Expand, 1)),
            Err(Rejection::Unsupported)
        );
        let fremd = NodeRef::new(TreeId("t".into()), NodeId(9));
        assert_eq!(
            rt.plan(&wunsch(&fremd, Action::DoDefault, 1)),
            Err(Rejection::UnknownNode)
        );
        let mut mit_wert = wunsch(&button, Action::DoDefault, 1);
        mit_wert.value = Some("x".into());
        assert_eq!(rt.plan(&mit_wert), Err(Rejection::ValueMismatch));
    }
}

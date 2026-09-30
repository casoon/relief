//! Konverter `a11y_perception::AXTree` → [`SemanticGraph`].
//!
//! Für den CDP-Host und die Aufnahmen in `spike/recordings`. Der CDP-Baum ist
//! ein einziger Baum mit Frame-Präfixen in den Knoten-IDs (`f1:28`, vom
//! CDP-Host vergeben, → `relief-cdp/src/capture.rs`); hier wird er in einen
//! Baum je Frame zerlegt, wie Chromium ihn intern führt.
//!
//! Tree-IDs kennt CDP nicht. Der Aufrufer gibt die ID des Hauptdokuments vor
//! (`document`); Child-Trees heißen `<document>/<präfix>`. Wer sie bei jeder
//! Navigation wechselt, bekommt dieselbe Identitätsregel wie in Chromium.
//!
//! Verlustbehaftet an drei Stellen:
//! - Relationen zeigen in CDP auf DOM-Knoten (`backendDOMNodeId`); sie werden
//!   auf den AX-Knoten desselben Baums mit dieser DOM-ID abgebildet. Ziele
//!   ohne AX-Knoten fallen weg.
//! - `owns` fällt weg (in Chromium keine Relation, sondern Umhängen im Baum),
//!   ebenso Knotenverweise in `ignoredReasons`.
//! - Nicht typisierte Eigenschaften landen als Text in
//!   [`SemanticNode::extra`]; Listen- und Knotenwerte dort fallen weg.

use std::collections::HashMap;

use a11y_perception::{AXNode, AXSnapshot, AXTree, AXValue, NameSource};

use crate::{
    Editable, Fact, HasPopup, IgnoredReason, Invalid, NameFrom, NodeId, NodeRef, Orientation, Role,
    SemanticGraph, SemanticNode, SemanticTree, Toggle, TreeId,
};

/// Eine Aufnahme als Modell; URL und Titel gehen an den Hauptbaum.
pub fn from_snapshot(snapshot: &AXSnapshot, document: &TreeId) -> SemanticGraph {
    let mut graph = from_tree(&snapshot.tree, document);
    if let Some(main) = graph.trees.get_mut(document) {
        main.data.url = Some(snapshot.url.clone());
        main.data.title = Some(snapshot.document_title.clone());
    }
    graph
}

/// Ein CDP-förmiger Baum als Modell (Version 0).
///
/// # Panics
///
/// Wenn eine Knoten-ID keine Ganzzahl (mit optionalem Frame-Präfix) ist. CDP
/// vergibt Blink-AX-IDs, die immer Ganzzahlen sind.
pub fn from_tree(tree: &AXTree, document: &TreeId) -> SemanticGraph {
    let mut graph = SemanticGraph::default();
    if tree.is_empty() {
        return graph;
    }

    let refs: HashMap<&str, NodeRef> = tree
        .iter_all()
        .map(|n| (n.node_id.as_str(), node_ref(&n.node_id, document)))
        .collect();
    // Relationen: (Baum, DOM-ID) → AX-Knoten.
    let mut by_dom: HashMap<(&TreeId, i64), NodeId> = HashMap::new();
    for n in tree.iter_all() {
        if let Some(dom) = n.backend_dom_node_id {
            let at = &refs[n.node_id.as_str()];
            by_dom.entry((&at.tree, dom)).or_insert(at.node);
        }
    }

    for n in tree.iter_all() {
        let at = &refs[n.node_id.as_str()];
        let t = graph
            .trees
            .entry(at.tree.clone())
            .or_insert_with(|| SemanticTree::new(at.tree.clone()));
        let mut node = convert(n, at, &refs, &by_dom);

        match n.parent_id.as_deref().and_then(|p| refs.get(p)) {
            Some(parent) if parent.tree == at.tree => node.parent = Some(parent.node),
            Some(parent) => {
                t.data.parent = Some(parent.clone());
                t.root = Some(at.node);
            }
            None => {}
        }
        if n.get_property_bool("focused") == Some(true) {
            t.data.focus = Some(at.node);
        }
        t.nodes.insert(at.node, node);
    }

    if let Some(root) = tree.root_id.as_deref().and_then(|r| refs.get(r)) {
        graph.root = Some(root.tree.clone());
        if let Some(t) = graph.trees.get_mut(&root.tree) {
            t.root = Some(root.node);
        }
    }
    // Ein Child-Tree ohne eingehängte Wurzel: erster Knoten in
    // Dokumentreihenfolge.
    for n in tree.iter_all() {
        let at = &refs[n.node_id.as_str()];
        let t = graph.trees.get_mut(&at.tree).expect("oben angelegt");
        if t.root.is_none() {
            t.root = Some(at.node);
        }
    }
    graph
}

fn node_ref(cdp_id: &str, document: &TreeId) -> NodeRef {
    let (tree, id) = match cdp_id.rsplit_once(':') {
        Some((prefix, id)) => (TreeId(format!("{document}/{prefix}")), id),
        None => (document.clone(), cdp_id),
    };
    let id = id
        .parse()
        .unwrap_or_else(|_| panic!("CDP-Knoten-ID ist keine Ganzzahl: {cdp_id}"));
    NodeRef::new(tree, NodeId(id))
}

fn convert(
    n: &AXNode,
    at: &NodeRef,
    refs: &HashMap<&str, NodeRef>,
    by_dom: &HashMap<(&TreeId, i64), NodeId>,
) -> SemanticNode {
    let mut node = SemanticNode::new(at.node, Role::from_cdp(n.role.as_deref().unwrap_or("")));
    node.name = Fact::known(n.name.clone());
    node.name_from = n.name_source.map(|s| match s {
        NameSource::Attribute => NameFrom::Attribute,
        NameSource::RelatedElement => NameFrom::RelatedElement,
        NameSource::Contents => NameFrom::Contents,
        NameSource::Placeholder => NameFrom::Placeholder,
        NameSource::Title => NameFrom::Title,
    });
    node.description = Fact::known(n.description.clone());
    node.value = Fact::known(n.value.clone());
    node.ignored = n.ignored;
    node.ignored_reasons = n
        .ignored_reasons
        .iter()
        .map(|r| IgnoredReason::from_cdp(&r.name))
        .collect();
    node.dom_node_id = n.backend_dom_node_id;

    for child in &n.child_ids {
        let Some(child) = refs.get(child.as_str()) else {
            continue;
        };
        if child.tree == at.tree {
            node.children.push(child.node);
        } else {
            node.child_tree = Some(child.tree.clone());
        }
    }

    let targets = |value: &AXValue| -> Vec<NodeId> {
        match value {
            AXValue::Node { related_nodes } => related_nodes
                .iter()
                .filter_map(|r| by_dom.get(&(&at.tree, r.backend_dom_node_id?)).copied())
                .collect(),
            _ => Vec::new(),
        }
    };

    let s = &mut node.states;
    for p in &n.properties {
        let text = n.property_value_str(&p.name);
        let flag = text.as_deref() == Some("true");
        let r = &mut node.relations;
        match p.name.as_str() {
            "focused" => {} // Baumdaten, siehe from_tree
            "focusable" => s.focusable = flag,
            "disabled" => s.disabled = flag,
            "readonly" => s.readonly = flag,
            "required" => s.required = flag,
            "modal" => s.modal = flag,
            "multiline" => s.multiline = flag,
            "multiselectable" => s.multiselectable = flag,
            "settable" => s.settable = flag,
            "busy" => s.busy = flag,
            "expanded" => s.expanded = Some(flag),
            "selected" => s.selected = Some(flag),
            "checked" => s.checked = text.as_deref().map(toggle),
            "pressed" => s.pressed = text.as_deref().map(toggle),
            "invalid" => {
                s.invalid = match text.as_deref() {
                    Some("false") | None => None,
                    Some("spelling") => Some(Invalid::Spelling),
                    Some("grammar") => Some(Invalid::Grammar),
                    Some(_) => Some(Invalid::True),
                }
            }
            "editable" => {
                s.editable = match text.as_deref() {
                    Some("richtext") => Some(Editable::Richtext),
                    Some(_) => Some(Editable::Plaintext),
                    None => None,
                }
            }
            "hasPopup" => {
                s.has_popup = match text.as_deref() {
                    Some("menu") => Some(HasPopup::Menu),
                    Some("listbox") => Some(HasPopup::Listbox),
                    Some("tree") => Some(HasPopup::Tree),
                    Some("grid") => Some(HasPopup::Grid),
                    Some("dialog") => Some(HasPopup::Dialog),
                    Some("false") | None => None,
                    Some(_) => Some(HasPopup::True),
                }
            }
            "orientation" => {
                s.orientation = match text.as_deref() {
                    Some("horizontal") => Some(Orientation::Horizontal),
                    Some("vertical") => Some(Orientation::Vertical),
                    _ => None,
                }
            }
            "level" => node.level = p.value.as_int().and_then(|l| u32::try_from(l).ok()),
            "url" => node.url = text,
            "labelledby" => r.labelled_by = targets(&p.value),
            "describedby" => r.described_by = targets(&p.value),
            "controls" => r.controls = targets(&p.value),
            "details" => r.details = targets(&p.value),
            "errormessage" => r.error_message = targets(&p.value),
            "flowto" => r.flow_to = targets(&p.value),
            "activedescendant" => r.active_descendant = targets(&p.value).first().copied(),
            "owns" => {}
            other => {
                if let Some(text) = text {
                    node.extra.insert(other.to_string(), text);
                }
            }
        }
    }
    node
}

fn toggle(text: &str) -> Toggle {
    match text {
        "true" => Toggle::True,
        "mixed" => Toggle::Mixed,
        _ => Toggle::False,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a11y_perception::{AXProperty, RelatedNode};

    fn ax(id: &str, role: &str, parent: Option<&str>, children: &[&str]) -> AXNode {
        AXNode {
            node_id: id.into(),
            role: Some(role.into()),
            parent_id: parent.map(String::from),
            child_ids: children.iter().map(|c| c.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn iframe_wird_eigener_baum() {
        let mut iframe = ax("3", "Iframe", Some("1"), &["f1:1"]);
        iframe.backend_dom_node_id = Some(30);
        let mut button = ax("f1:2", "button", Some("f1:1"), &[]);
        button.properties.push(AXProperty {
            name: "focused".into(),
            value: AXValue::Bool(true),
        });
        let tree = AXTree::from_nodes(vec![
            ax("1", "RootWebArea", None, &["3"]),
            iframe,
            ax("f1:1", "RootWebArea", Some("3"), &["f1:2"]),
            button,
        ]);
        let doc = TreeId("doc".into());
        let g = from_tree(&tree, &doc);

        let child = TreeId("doc/f1".into());
        assert_eq!(g.root, Some(doc.clone()));
        assert_eq!(g.trees.len(), 2);
        let host = &g.trees[&doc].nodes[&NodeId(3)];
        assert_eq!(host.child_tree, Some(child.clone()));
        assert!(host.children.is_empty());
        let t = &g.trees[&child];
        assert_eq!(t.root, Some(NodeId(1)));
        assert_eq!(t.data.parent, Some(NodeRef::new(doc, NodeId(3))));
        assert_eq!(t.data.focus, Some(NodeId(2)));
        assert_eq!(t.nodes[&NodeId(1)].parent, None);
        assert_eq!(g.document_order().len(), 4);
    }

    #[test]
    fn relationen_zeigen_auf_ax_knoten() {
        let mut label = ax("2", "LabelText", Some("1"), &[]);
        label.backend_dom_node_id = Some(20);
        let mut field = ax("3", "textbox", Some("1"), &[]);
        field.properties = vec![
            AXProperty {
                name: "labelledby".into(),
                value: AXValue::Node {
                    related_nodes: vec![
                        RelatedNode {
                            backend_dom_node_id: Some(20),
                            idref: None,
                            text: None,
                        },
                        RelatedNode {
                            backend_dom_node_id: Some(99),
                            idref: None,
                            text: None,
                        },
                    ],
                },
            },
            AXProperty {
                name: "invalid".into(),
                value: AXValue::String("false".into()),
            },
            AXProperty {
                name: "checked".into(),
                value: AXValue::String("mixed".into()),
            },
            AXProperty {
                name: "roledescription".into(),
                value: AXValue::String("Eingabe".into()),
            },
        ];
        let tree = AXTree::from_nodes(vec![
            ax("1", "RootWebArea", None, &["2", "3"]),
            label,
            field,
        ]);
        let g = from_tree(&tree, &TreeId("d".into()));
        let n = &g.trees[&TreeId("d".into())].nodes[&NodeId(3)];
        assert_eq!(n.relations.labelled_by, vec![NodeId(2)]);
        assert_eq!(n.states.invalid, None);
        assert_eq!(n.states.checked, Some(Toggle::Mixed));
        assert_eq!(n.extra["roledescription"], "Eingabe");
        assert_eq!(n.role, Role::Textbox);
    }
}

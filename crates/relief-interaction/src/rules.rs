//! Befunde aus `a11y-rules` (barrierlab) über dem Accessibility-Tree
//! (Paket 21), für den Inspector.
//!
//! Der Fork kennt kein DOM, nur den AXTree. Daraus entsteht hier ein
//! Dokument im Sinne von `a11y-dom`: Elemente mit dem Tag-Namen, den
//! Chromium mitschickt (`htmlTag`, im Modell unter [`HTML_TAG`]), Text aus
//! `StaticText`, als Attribut nur `href` aus der Link-Adresse; Rolle, Name
//! und „ignoriert“ aus dem Modell ([`Semantics`]). Darauf laufen die
//! Regeln der Stufe `Semantics` (Namen von Links, Buttons, SVG, mehrdeutige
//! und generische Linktexte). Regeln der Stufen `Structure` und `Rendering`
//! brauchen Markup bzw. berechnete Stile und stehen als `NotRun` mit Grund
//! im Bericht, nicht als bestanden (→ „nicht geprüft ist nicht bestanden“).

use a11y_dom::{Document, Node, NodeId as DomId, NodeKind, Semantics};
use a11y_report::{NotRun, Report, RuleRun};
use a11y_rules::Locale;
use relief_model::{NodeRef, Role, SemanticGraph};

/// Schlüssel im Modell (`SemanticNode::extra`), unter dem der Host den
/// HTML-Tag ablegt (Fork: `kHtmlTag`).
pub const HTML_TAG: &str = "htmlTag";

/// Grund für Regeln, die nicht laufen können.
const NUR_AXTREE: &str =
    "Nur der Accessibility-Tree liegt vor, kein Markup und keine berechneten Stile";

struct Entry {
    at: NodeRef,
    kind: NodeKind,
    tag: String,
    attributes: Vec<(String, String)>,
    text: String,
    role: String,
    name: Option<String>,
    ignored: bool,
    parent: Option<u32>,
    children: Vec<u32>,
}

/// Der AXTree als Dokument, in Dokumentreihenfolge über alle Bäume (iframes
/// unter ihrem Host-Knoten).
pub struct AxDocument {
    entries: Vec<Entry>,
}

#[derive(Clone, Copy)]
pub struct AxNode<'a> {
    doc: &'a AxDocument,
    index: u32,
}

impl PartialEq for AxNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.doc, other.doc) && self.index == other.index
    }
}

impl Eq for AxNode<'_> {}

impl AxDocument {
    pub fn new(model: &SemanticGraph) -> Self {
        let mut entries: Vec<Entry> = Vec::new();
        let mut stack: Vec<(NodeRef, Option<u32>)> = Vec::new();
        if let Some(root) = model
            .root
            .as_ref()
            .and_then(|t| model.trees.get(t))
            .and_then(|t| t.root.map(|r| NodeRef::new(t.id.clone(), r)))
        {
            stack.push((root, None));
        }
        while let Some((at, parent)) = stack.pop() {
            let Some(node) = model.node(&at) else {
                continue;
            };
            if node.role == Role::InlineTextBox {
                continue;
            }
            let index = entries.len() as u32;
            let text_node = node.role == Role::StaticText;
            let tag = node.extra.get(HTML_TAG).cloned().unwrap_or_default();
            let mut attributes = Vec::new();
            if let Some(url) = &node.url {
                if tag == "a" {
                    attributes.push(("href".to_string(), url.clone()));
                }
            }
            entries.push(Entry {
                at: at.clone(),
                kind: if text_node {
                    NodeKind::Text
                } else {
                    NodeKind::Element
                },
                tag,
                attributes,
                text: if text_node {
                    node.name.value.clone().unwrap_or_default()
                } else {
                    String::new()
                },
                role: node.role.to_string(),
                name: node.name.value.clone(),
                ignored: node.ignored,
                parent,
                children: Vec::new(),
            });
            if let Some(p) = parent {
                entries[p as usize].children.push(index);
            }
            // Kinder in umgekehrter Folge auf den Stapel: Dokumentreihenfolge.
            let mut next: Vec<NodeRef> = node
                .children
                .iter()
                .map(|id| NodeRef::new(at.tree.clone(), *id))
                .collect();
            // Ein iframe-Knoten trägt seinen Child-Tree als Kind.
            if let Some(child) = node
                .child_tree
                .as_ref()
                .and_then(|t| model.trees.get(t))
                .and_then(|t| t.root.map(|r| NodeRef::new(t.id.clone(), r)))
            {
                next.push(child);
            }
            for child in next.into_iter().rev() {
                stack.push((child, Some(index)));
            }
        }
        AxDocument { entries }
    }

    /// Knoten des Modells zu einer Kennung aus einem Befund.
    pub fn node_ref(&self, id: &str) -> Option<&NodeRef> {
        let i: usize = id.parse().ok()?;
        self.entries.get(i).map(|e| &e.at)
    }

    fn node(&self, index: u32) -> AxNode<'_> {
        AxNode { doc: self, index }
    }

    fn entry(&self, index: u32) -> &Entry {
        &self.entries[index as usize]
    }
}

impl<'a> Node<'a> for AxNode<'a> {
    fn id(self) -> DomId {
        DomId(self.index)
    }

    fn kind(self) -> NodeKind {
        self.doc.entry(self.index).kind
    }

    fn parent(self) -> Option<Self> {
        self.doc.entry(self.index).parent.map(|p| self.doc.node(p))
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let doc = self.doc;
        doc.entry(self.index)
            .children
            .iter()
            .map(move |&c| doc.node(c))
    }

    fn local_name(self) -> &'a str {
        &self.doc.entry(self.index).tag
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        self.doc
            .entry(self.index)
            .attributes
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
    }

    fn text(self) -> &'a str {
        &self.doc.entry(self.index).text
    }
}

impl Document for AxDocument {
    type N<'a> = AxNode<'a>;

    fn root(&self) -> AxNode<'_> {
        self.node(0)
    }

    fn node_count(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}

impl Semantics for AxDocument {
    fn role<'n>(&'n self, node: AxNode<'n>) -> Option<String> {
        Some(self.entry(node.index).role.clone())
    }

    fn accessible_name<'n>(&'n self, node: AxNode<'n>) -> Option<String> {
        self.entry(node.index).name.clone()
    }

    fn is_ignored<'n>(&'n self, node: AxNode<'n>) -> bool {
        self.entry(node.index).ignored
    }
}

/// Befunde der Seite: Regeln der Stufe `Semantics` auf dem AXTree, die
/// übrigen als nicht gelaufen. Befundorte (`Location::node`) sind Indizes
/// in `doc` ([`AxDocument::node_ref`]).
pub fn check(doc: &AxDocument) -> Report {
    let mut report = Report::new();
    if doc.entries.is_empty() {
        return report.finish();
    }
    for rule in a11y_rules::semantics_rules::<AxDocument>() {
        let mut out = Vec::new();
        (rule.run)(doc, Locale::De, &mut out);
        for id in rule.meta.ids {
            let count = out.iter().filter(|f| f.rule_id == *id).count();
            report.record(RuleRun::ran(*id, count));
        }
        report.extend(out);
    }
    for meta in a11y_rules::structure_metas()
        .iter()
        .chain(a11y_rules::rendering_metas())
    {
        for id in meta.ids {
            report.record(RuleRun::not_run(*id, NotRun::CapabilityMissing).with_reason(NUR_AXTREE));
        }
    }
    report.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use relief_model::{Fact, NodeId, SemanticNode, TreeData, TreeDelta, TreeId, TreeUpdate};

    fn node(id: i32, role: &str, tag: &str, name: Option<&str>, children: &[i32]) -> SemanticNode {
        let mut n = SemanticNode::new(NodeId(id), Role::from_name(role));
        n.name = Fact::known(name.map(String::from));
        n.children = children.iter().map(|c| NodeId(*c)).collect();
        if !tag.is_empty() {
            n.extra.insert(HTML_TAG.into(), tag.into());
        }
        n
    }

    fn page() -> SemanticGraph {
        let tree = TreeId("t".into());
        let mut link = node(3, "link", "a", None, &[]);
        link.url = Some("https://example.org/".into());
        let mut nodes = vec![
            node(1, "rootWebArea", "#document", Some("Seite"), &[2]),
            node(2, "main", "main", None, &[3, 4, 5]),
            link,
            node(4, "button", "button", Some("Senden"), &[]),
            node(5, "button", "button", None, &[]),
        ];
        for n in nodes.iter_mut().skip(1) {
            n.parent = Some(NodeId(if n.id.0 == 2 { 1 } else { 2 }));
        }
        let mut g = SemanticGraph::default();
        g.apply(&TreeDelta {
            base: g.version,
            root: Some(tree.clone()),
            removed_trees: vec![],
            trees: vec![TreeUpdate {
                tree,
                data: Some(TreeData::default()),
                root: Some(NodeId(1)),
                removed: vec![],
                created: nodes,
                changed: vec![],
                bounds: vec![],
            }],
        })
        .unwrap();
        g
    }

    #[test]
    fn namenlose_links_und_buttons_nicht_gelaufene_regeln_vermerkt() {
        let model = page();
        let doc = AxDocument::new(&model);
        let report = check(&doc);
        let ids: Vec<&str> = report.findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert!(ids.contains(&"links/name-missing"), "{ids:?}");
        assert!(ids.contains(&"buttons/name-missing"), "{ids:?}");
        assert_eq!(report.findings.len(), 2, "{ids:?}");
        // Befundorte führen zu den Knoten des Modells.
        let orte: Vec<i32> = report
            .findings
            .iter()
            .filter_map(|f| doc.node_ref(f.location.node.as_deref()?))
            .map(|at| at.node.0)
            .collect();
        assert_eq!(orte, vec![3, 5]);
        assert!(report.summary.rules_not_run > 0);
        assert!(report
            .rule_runs
            .iter()
            .any(|r| !r.did_run() && r.reason.as_deref() == Some(NUR_AXTREE)));
    }
}

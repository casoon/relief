//! Alle Bäume einer Seite.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{GraphVersion, NodeId, NodeRef, SemanticNode, TreeId};

/// Die Accessibility-Bäume eines Tabs: ein Hauptbaum und je iframe ein
/// Child-Tree, verknüpft über [`SemanticNode::child_tree`] und
/// [`TreeData::parent`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticGraph {
    pub version: GraphVersion,
    /// Baum des Hauptdokuments.
    pub root: Option<TreeId>,
    pub trees: BTreeMap<TreeId, SemanticTree>,
}

/// Ein Baum: ein Dokument in einem Frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTree {
    pub id: TreeId,
    pub data: TreeData,
    pub root: Option<NodeId>,
    pub nodes: BTreeMap<NodeId, SemanticNode>,
}

/// Angaben zum Baum als Ganzem (Chromium: `AXTreeData`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TreeData {
    /// Der iframe-Knoten im Elternbaum; `None` beim Hauptdokument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Fokussierter Knoten dieses Baums.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<NodeId>,
}

impl SemanticTree {
    pub fn new(id: TreeId) -> Self {
        Self {
            id,
            data: TreeData::default(),
            root: None,
            nodes: BTreeMap::new(),
        }
    }
}

impl SemanticGraph {
    pub fn node(&self, at: &NodeRef) -> Option<&SemanticNode> {
        self.trees.get(&at.tree)?.nodes.get(&at.node)
    }

    /// Anzahl aller Knoten über alle Bäume.
    pub fn len(&self) -> usize {
        self.trees.values().map(|t| t.nodes.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Alle vom Hauptbaum aus erreichbaren Knoten in Dokumentreihenfolge
    /// (Tiefensuche; ein iframe-Knoten wird vor seinem Child-Tree besucht).
    pub fn document_order(&self) -> Vec<NodeRef> {
        let mut out = Vec::with_capacity(self.len());
        let Some(root) = self.root.as_ref() else {
            return out;
        };
        let mut stack: Vec<NodeRef> = Vec::new();
        if let Some(r) = self.trees.get(root).and_then(|t| t.root) {
            stack.push(NodeRef::new(root.clone(), r));
        }
        while let Some(at) = stack.pop() {
            let Some(node) = self.node(&at) else {
                continue;
            };
            // Umgekehrt auf den Stapel, damit das erste Kind zuerst kommt;
            // der Child-Tree folgt nach den eigenen Kindern.
            if let Some(child_tree) = &node.child_tree {
                if let Some(r) = self.trees.get(child_tree).and_then(|t| t.root) {
                    stack.push(NodeRef::new(child_tree.clone(), r));
                }
            }
            for child in node.children.iter().rev() {
                stack.push(NodeRef::new(at.tree.clone(), *child));
            }
            out.push(at);
        }
        out
    }
}

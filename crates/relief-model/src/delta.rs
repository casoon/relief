//! Delta-Format: inkrementelle Änderung eines [`SemanticGraph`].
//!
//! Angelehnt an Chromiums `AXTreeUpdate` (ein Update je Baum, geänderte
//! Knoten vollständig statt feldweise, neue Wurzel nur bei Wechsel), aber
//! ohne dessen Reihenfolgeregeln: Ein [`TreeUpdate`] nennt entfernte,
//! angelegte und geänderte Knoten getrennt. Positionen sind ein eigener Teil
//! ([`BoundsChange`]), weil Chromium sie über einen eigenen Kanal meldet
//! (`AccessibilityLocationChangesReceived`).

use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    GraphVersion, NodeId, NodeRef, Rect, SemanticGraph, SemanticNode, SemanticTree, TreeData,
    TreeId,
};

/// Änderung eines Graphen von Version `base` zur nächsten.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TreeDelta {
    /// Version, auf die die Delta passt.
    pub base: GraphVersion,
    /// Neuer Hauptbaum, falls er wechselt (Navigation im Hauptframe).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<TreeId>,
    /// Bäume, die wegfallen (Navigation, entferntes iframe). Wird zuerst
    /// angewandt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_trees: Vec<TreeId>,
    /// Änderungen je Baum; ein unbekannter Baum wird angelegt.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trees: Vec<TreeUpdate>,
}

/// Änderung eines Baums.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TreeUpdate {
    pub tree: TreeId,
    /// Neue Baumdaten (Eltern-iframe, URL, Titel, Fokus), falls geändert;
    /// bei einem neuen Baum immer gesetzt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<TreeData>,
    /// Neue Wurzel, falls sie wechselt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created: Vec<SemanticNode>,
    /// Geänderte Knoten, vollständig (ersetzen den bisherigen Knoten).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<SemanticNode>,
    /// Knoten, bei denen sich nur die Position geändert hat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bounds: Vec<BoundsChange>,
}

/// Neue Position eines Knotens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundsChange {
    pub node: NodeId,
    pub bounds: Option<Rect>,
}

impl TreeUpdate {
    fn new(tree: TreeId) -> Self {
        Self {
            tree,
            data: None,
            root: None,
            removed: Vec::new(),
            created: Vec::new(),
            changed: Vec::new(),
            bounds: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.data.is_none()
            && self.root.is_none()
            && self.removed.is_empty()
            && self.created.is_empty()
            && self.changed.is_empty()
            && self.bounds.is_empty()
    }
}

impl TreeDelta {
    /// Die Änderung von `before` nach `after`.
    ///
    /// Knoten werden über ihre [`NodeRef`] zugeordnet; ein Baum mit neuer
    /// Tree-ID ist ein neuer Baum, auch wenn er dasselbe Frame füllt.
    pub fn between(before: &SemanticGraph, after: &SemanticGraph) -> Self {
        let mut delta = TreeDelta {
            base: before.version,
            root: (before.root != after.root)
                .then(|| after.root.clone())
                .flatten(),
            removed_trees: before
                .trees
                .keys()
                .filter(|id| !after.trees.contains_key(*id))
                .cloned()
                .collect(),
            trees: Vec::new(),
        };
        for (id, new) in &after.trees {
            let update = match before.trees.get(id) {
                Some(old) => tree_update(old, new),
                None => TreeUpdate {
                    data: Some(new.data.clone()),
                    root: new.root,
                    created: new.nodes.values().cloned().collect(),
                    ..TreeUpdate::new(id.clone())
                },
            };
            if !update.is_empty() {
                delta.trees.push(update);
            }
        }
        delta
    }

    /// Keine Änderung.
    pub fn is_empty(&self) -> bool {
        self.root.is_none() && self.removed_trees.is_empty() && self.trees.is_empty()
    }
}

fn tree_update(old: &SemanticTree, new: &SemanticTree) -> TreeUpdate {
    let mut update = TreeUpdate::new(new.id.clone());
    update.data = (old.data != new.data).then(|| new.data.clone());
    update.root = (old.root != new.root).then_some(new.root).flatten();
    update.removed = old
        .nodes
        .keys()
        .filter(|id| !new.nodes.contains_key(*id))
        .copied()
        .collect();
    for (id, node) in &new.nodes {
        match old.nodes.get(id) {
            None => update.created.push(node.clone()),
            Some(prev) if prev == node => {}
            Some(prev) if only_bounds_differ(prev, node) => update.bounds.push(BoundsChange {
                node: *id,
                bounds: node.bounds,
            }),
            Some(_) => update.changed.push(node.clone()),
        }
    }
    update
}

fn only_bounds_differ(a: &SemanticNode, b: &SemanticNode) -> bool {
    a.bounds != b.bounds
        && SemanticNode {
            bounds: b.bounds,
            ..a.clone()
        } == *b
}

/// Warum eine Delta nicht passt. Der Graph bleibt dann unverändert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// Die Delta setzt eine andere Version voraus.
    VersionMismatch {
        graph: GraphVersion,
        delta: GraphVersion,
    },
    /// Ein zu entfernender Baum existiert nicht.
    UnknownTree(TreeId),
    /// Ein zu entfernender, zu ändernder oder zu verschiebender Knoten
    /// existiert nicht.
    UnknownNode(NodeRef),
    /// Ein anzulegender Knoten existiert schon.
    DuplicateNode(NodeRef),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionMismatch { graph, delta } => write!(
                f,
                "Delta passt zu Version {}, Graph steht auf {}",
                delta.0, graph.0
            ),
            Self::UnknownTree(id) => write!(f, "unbekannter Baum {id}"),
            Self::UnknownNode(at) => write!(f, "unbekannter Knoten {at}"),
            Self::DuplicateNode(at) => write!(f, "Knoten {at} existiert schon"),
        }
    }
}

impl std::error::Error for ApplyError {}

impl SemanticGraph {
    /// Wendet `delta` an und erhöht die Version um eins.
    ///
    /// Erst wird geprüft, dann geändert: Passt die Delta nicht, bleibt der
    /// Graph unverändert.
    pub fn apply(&mut self, delta: &TreeDelta) -> Result<(), ApplyError> {
        self.check(delta)?;

        for id in &delta.removed_trees {
            self.trees.remove(id);
            if self.root.as_ref() == Some(id) {
                self.root = None;
            }
        }
        for update in &delta.trees {
            let tree = self
                .trees
                .entry(update.tree.clone())
                .or_insert_with(|| SemanticTree::new(update.tree.clone()));
            for id in &update.removed {
                tree.nodes.remove(id);
                if tree.root == Some(*id) {
                    tree.root = None;
                }
            }
            for node in update.created.iter().chain(&update.changed) {
                tree.nodes.insert(node.id, node.clone());
            }
            for change in &update.bounds {
                if let Some(node) = tree.nodes.get_mut(&change.node) {
                    node.bounds = change.bounds;
                }
            }
            if let Some(root) = update.root {
                tree.root = Some(root);
            }
            if let Some(data) = &update.data {
                tree.data = data.clone();
            }
        }
        if let Some(root) = &delta.root {
            self.root = Some(root.clone());
        }
        self.version = delta.base.next();
        Ok(())
    }

    fn check(&self, delta: &TreeDelta) -> Result<(), ApplyError> {
        if delta.base != self.version {
            return Err(ApplyError::VersionMismatch {
                graph: self.version,
                delta: delta.base,
            });
        }
        for id in &delta.removed_trees {
            if !self.trees.contains_key(id) {
                return Err(ApplyError::UnknownTree(id.clone()));
            }
        }
        for update in &delta.trees {
            // Ein im selben Schritt entfernter Baum wird neu angelegt.
            let existing = self
                .trees
                .get(&update.tree)
                .filter(|_| !delta.removed_trees.contains(&update.tree));
            let exists = |id: &NodeId| existing.is_some_and(|t| t.nodes.contains_key(id));
            let at = |id: &NodeId| NodeRef::new(update.tree.clone(), *id);
            let removed: HashSet<NodeId> = update.removed.iter().copied().collect();

            for id in update
                .removed
                .iter()
                .chain(update.changed.iter().map(|n| &n.id))
            {
                if !exists(id) {
                    return Err(ApplyError::UnknownNode(at(id)));
                }
            }
            for node in &update.created {
                if exists(&node.id) && !removed.contains(&node.id) {
                    return Err(ApplyError::DuplicateNode(at(&node.id)));
                }
            }
            for change in &update.bounds {
                let created = update.created.iter().any(|n| n.id == change.node);
                if !created && (!exists(&change.node) || removed.contains(&change.node)) {
                    return Err(ApplyError::UnknownNode(at(&change.node)));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fact, Role};

    fn node(id: i32, role: Role, children: &[i32]) -> SemanticNode {
        let mut n = SemanticNode::new(NodeId(id), role);
        n.children = children.iter().copied().map(NodeId).collect();
        n
    }

    fn graph(tree: &str, nodes: Vec<SemanticNode>) -> SemanticGraph {
        let id = TreeId(tree.into());
        let mut t = SemanticTree::new(id.clone());
        t.root = nodes.first().map(|n| n.id);
        for n in nodes {
            t.nodes.insert(n.id, n);
        }
        SemanticGraph {
            version: GraphVersion(3),
            root: Some(id.clone()),
            trees: [(id, t)].into(),
        }
    }

    fn roundtrip(before: &SemanticGraph, after: &SemanticGraph) -> TreeDelta {
        let delta = TreeDelta::between(before, after);
        let mut applied = before.clone();
        applied.apply(&delta).unwrap();
        let mut expected = after.clone();
        expected.version = before.version.next();
        assert_eq!(applied, expected);
        delta
    }

    #[test]
    fn gleiche_graphen_ergeben_leere_delta() {
        let g = graph("t", vec![node(1, Role::RootWebArea, &[])]);
        assert!(TreeDelta::between(&g, &g).is_empty());
    }

    #[test]
    fn angelegt_geaendert_entfernt() {
        let before = graph(
            "t",
            vec![
                node(1, Role::RootWebArea, &[2, 3]),
                node(2, Role::Button, &[]),
                node(3, Role::Link, &[]),
            ],
        );
        let mut button = node(2, Role::Button, &[]);
        button.name = Fact::known(Some("Kaufen".into()));
        let after = graph(
            "t",
            vec![
                node(1, Role::RootWebArea, &[2, 4]),
                button,
                node(4, Role::Dialog, &[]),
            ],
        );
        let delta = roundtrip(&before, &after);
        let update = &delta.trees[0];
        assert_eq!(update.removed, vec![NodeId(3)]);
        assert_eq!(update.created.len(), 1);
        assert_eq!(update.changed.len(), 2, "Wurzel (Kinder) und Button (Name)");
        assert_eq!(update.root, None);
    }

    #[test]
    fn navigation_ersetzt_den_hauptbaum() {
        let before = graph("a", vec![node(1, Role::RootWebArea, &[])]);
        let after = graph("b", vec![node(1, Role::RootWebArea, &[])]);
        let delta = roundtrip(&before, &after);
        assert_eq!(delta.root, Some(TreeId("b".into())));
        assert_eq!(delta.removed_trees, vec![TreeId("a".into())]);
    }

    #[test]
    fn nur_position_geht_in_bounds() {
        let before = graph("t", vec![node(1, Role::RootWebArea, &[])]);
        let mut after = before.clone();
        after
            .trees
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .get_mut(&NodeId(1))
            .unwrap()
            .bounds = Some(Rect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        });
        let delta = roundtrip(&before, &after);
        assert_eq!(delta.trees[0].bounds.len(), 1);
        assert!(delta.trees[0].changed.is_empty());
    }

    #[test]
    fn veraltete_delta_wird_abgelehnt_und_aendert_nichts() {
        let before = graph("t", vec![node(1, Role::RootWebArea, &[])]);
        let after = graph(
            "t",
            vec![node(1, Role::RootWebArea, &[2]), node(2, Role::Button, &[])],
        );
        let delta = TreeDelta::between(&before, &after);
        let mut g = before.clone();
        g.apply(&delta).unwrap();
        let snapshot = g.clone();
        assert_eq!(
            g.apply(&delta),
            Err(ApplyError::VersionMismatch {
                graph: GraphVersion(4),
                delta: GraphVersion(3)
            })
        );
        assert_eq!(g, snapshot);
    }

    #[test]
    fn unbekannter_knoten_wird_abgelehnt() {
        let g = graph("t", vec![node(1, Role::RootWebArea, &[])]);
        let mut update = TreeUpdate::new(TreeId("t".into()));
        update.changed.push(node(9, Role::Button, &[]));
        let delta = TreeDelta {
            base: g.version,
            trees: vec![update],
            ..Default::default()
        };
        let mut applied = g.clone();
        assert_eq!(
            applied.apply(&delta),
            Err(ApplyError::UnknownNode(NodeRef::new(
                TreeId("t".into()),
                NodeId(9)
            )))
        );
        assert_eq!(applied, g);
    }
}

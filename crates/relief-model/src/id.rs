//! Identität von Bäumen und Knoten, Version des Graphen.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Ein Accessibility-Baum: je Frame und Dokument einer.
///
/// In Chromium ist das die `AXTreeID` (das Embedding-Token des Frames), die mit
/// jedem neuen Dokument wechselt. Der CDP-Host kennt sie nicht und vergibt
/// eine eigene (→ [`crate::perception`]).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TreeId(pub String);

impl fmt::Display for TreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Ein Knoten innerhalb **eines** Baums.
///
/// Wie Chromiums `AXNodeID` (`int32_t`): nur zusammen mit der [`TreeId`]
/// eindeutig, negative Werte kommen vor (z. B. bei Inline-Textboxen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub i32);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Ein Knoten über Baumgrenzen hinweg: (Baum, Knoten).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeRef {
    pub tree: TreeId,
    pub node: NodeId,
}

impl NodeRef {
    pub fn new(tree: TreeId, node: NodeId) -> Self {
        Self { tree, node }
    }
}

impl fmt::Display for NodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.tree, self.node)
    }
}

/// Stand des Graphen. Jede angewandte [`crate::TreeDelta`] erhöht ihn um eins.
///
/// Intents und Aktionen verweisen auf Knoten **mit** Version, damit ein
/// veraltetes Ziel erkannt wird (→ `plan/spezifikation/05`).
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct GraphVersion(pub u64);

impl GraphVersion {
    /// Die Version nach einer weiteren Änderung.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

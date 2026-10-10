//! Werkzeuge, die eine Website über WebMCP erklärt (Paket 143).
//!
//! Sie kommen nicht aus dem Accessibility-Tree, sondern über einen eigenen
//! Kanal des Hosts, und hängen am Baum des Dokuments, das sie angemeldet hat
//! ([`SemanticGraph::tools`](crate::SemanticGraph::tools)). Alles darin
//! behauptet die Website selbst: Name, Beschreibung, Eingabeschema und
//! Anmerkungen sind höchstens [`Certainty::Inferred`](crate::Certainty) und
//! senken nie ein Risiko (→ `plan/spezifikation/07`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredTool {
    pub name: String,
    pub description: String,
    /// JSON-Schema der Eingabe als Text, wie die Website es angibt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<String>,
    /// Anmerkungen der Website (WebMCP `annotations`).
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub untrusted_content: bool,
    #[serde(default)]
    pub consequential: bool,
    /// DOM-Knoten eines deklarativen Werkzeugs (`<form toolname>`), wie
    /// [`SemanticNode::dom_node_id`](crate::SemanticNode::dom_node_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dom_node_id: Option<i64>,
}

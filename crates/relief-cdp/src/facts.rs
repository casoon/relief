//! Angaben aus dem DOM, die der Accessibility-Tree nicht trägt, als
//! `SemanticNode::extra` (→ `relief_interaction::security`):
//!
//! - **Formularziel** eines Absenden-Buttons ([`FORM_ACTION`]): `formaction`
//!   des Buttons, sonst `action` des Formulars, sonst die Dokumentadresse;
//!   aufgelöst gegen die Basisadresse des Dokuments. Formulare mit
//!   `method=dialog` schicken nichts ab und haben kein Ziel.
//! - **Feldangaben** ([`INPUT_TYPE`], [`HTML_AUTOCOMPLETE`]): HTML-`type`
//!   eines `<input>` und HTML-`autocomplete`, damit die Rückfrage sensible
//!   Werte verdeckt.
//!
//! Eine Anfrage (`DOM.getDocument`, Tiefe -1, `pierce`) je Aufnahme, wie
//! `live.rs`; iframes im selben Renderer-Prozess und Shadow DOM sind dabei,
//! Frames in anderen Prozessen über ihre Sitzung (`Frames::document`).

use std::collections::HashMap;

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::dom::Node;
use chromiumoxide::Page;
use relief_interaction::security::{FORM_ACTION, HTML_AUTOCOMPLETE, INPUT_TYPE};
use relief_model::SemanticGraph;
use url::Url;

use crate::frames::Frames;

const ELEMENT_NODE: i64 = 1;

/// Angaben je Backend-ID.
type Facts = HashMap<i64, Vec<(&'static str, String)>>;

/// Angaben aus dem DOM holen und an die Knoten des Modells hängen (über die
/// DOM-ID).
pub async fn annotate(page: &Page, frames: &Frames, model: &mut SemanticGraph) -> Result<()> {
    let (root, _) = frames.document(page).await?;
    let mut facts = Facts::new();
    scope(&root, &root, &mut facts);
    for tree in model.trees.values_mut() {
        for node in tree.nodes.values_mut() {
            if let Some(found) = node.dom_node_id.and_then(|id| facts.get(&id)) {
                for (key, value) in found {
                    node.extra.insert((*key).to_string(), value.clone());
                }
            }
        }
    }
    Ok(())
}

/// Ein Dokument oder Shadow-Root: eigener Baum für Formular-IDs und
/// Vorfahren. `document`: der Dokumentknoten, dessen Adressen gelten.
fn scope<'n>(root: &'n Node, document: &'n Node, facts: &mut Facts) {
    let mut forms = HashMap::new();
    index_forms(root, &mut forms);
    let walk = Walk { document, forms };
    for child in root.children.iter().flatten() {
        walk.node(child, None, facts);
    }
}

/// Formulare mit `id` in diesem Baum (ohne Shadow-Roots und iframes).
fn index_forms<'n>(node: &'n Node, forms: &mut HashMap<String, &'n Node>) {
    for child in node.children.iter().flatten() {
        if child.node_type == ELEMENT_NODE && child.local_name == "form" {
            if let Some(id) = attr(child, "id") {
                forms.entry(id.to_string()).or_insert(child);
            }
        }
        index_forms(child, forms);
    }
}

struct Walk<'n> {
    document: &'n Node,
    forms: HashMap<String, &'n Node>,
}

impl<'n> Walk<'n> {
    fn node(&self, node: &'n Node, form: Option<&'n Node>, facts: &mut Facts) {
        if node.node_type != ELEMENT_NODE {
            return;
        }
        let found = self.facts(node, form);
        if !found.is_empty() {
            facts.insert(*node.backend_node_id.inner(), found);
        }
        if let Some(doc) = node.content_document.as_deref() {
            scope(doc, doc, facts);
        }
        for shadow in node.shadow_roots.iter().flatten() {
            scope(shadow, self.document, facts);
        }
        let form = if node.local_name == "form" {
            Some(node)
        } else {
            form
        };
        for child in node.children.iter().flatten() {
            self.node(child, form, facts);
        }
    }

    fn facts(&self, node: &Node, ancestor: Option<&Node>) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        let tag = node.local_name.as_str();
        if tag == "input" {
            out.push((INPUT_TYPE, attr(node, "type").unwrap_or("text").to_string()));
        }
        if matches!(tag, "input" | "textarea" | "select") {
            if let Some(a) = attr(node, "autocomplete") {
                out.push((HTML_AUTOCOMPLETE, a.to_string()));
            }
        }
        if let Some(action) = self.form_action(node, ancestor) {
            out.push((FORM_ACTION, action));
        }
        out
    }

    /// Wohin ein Absenden-Button das Formular schickt.
    fn form_action(&self, node: &Node, ancestor: Option<&Node>) -> Option<String> {
        let kind = attr(node, "type").map(str::to_ascii_lowercase);
        let submit = match node.local_name.as_str() {
            // Fehlender oder ungültiger `type` heißt `submit`.
            "button" => !matches!(kind.as_deref(), Some("button" | "reset")),
            "input" => matches!(kind.as_deref(), Some("submit" | "image")),
            _ => false,
        };
        if !submit {
            return None;
        }
        let form = match attr(node, "form") {
            Some(id) => *self.forms.get(id)?,
            None => ancestor?,
        };
        let method = non_empty(attr(node, "formmethod")).or(non_empty(attr(form, "method")));
        if method.is_some_and(|m| m.eq_ignore_ascii_case("dialog")) {
            return None;
        }
        let document = self.document.document_url.as_deref()?;
        let base = self.document.base_url.as_deref().unwrap_or(document);
        let action = non_empty(attr(node, "formaction"))
            .or(non_empty(attr(form, "action")))
            .unwrap_or(document);
        Url::parse(base)
            .and_then(|b| b.join(action.trim()))
            .ok()
            .map(String::from)
    }
}

/// Attributwert; CDP liefert Attribute als flache Liste [Name, Wert, …].
fn attr<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attributes
        .as_deref()?
        .chunks(2)
        .find(|pair| pair[0].eq_ignore_ascii_case(name))
        .and_then(|pair| pair.get(1))
        .map(String::as_str)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.trim().is_empty())
}

//! Tastatur-Sprungmarken (Paket 38): jedes im Modell exponierte Element mit
//! ausführbarer Aktion und Position bekommt eine kurze Buchstabenfolge; wer
//! sie tippt, bedient das Element über den üblichen geprüften Plan.
//!
//! Kandidaten kommen aus dem [`SemanticGraph`], nicht aus dem DOM:
//! Bedienelemente des Interaction Graph und Knoten, für die Chromium eine
//! Standardaktion meldet (etwa ein `<div>` mit Klick-Handler). Was Chromium
//! nicht in den AXTree aufnimmt, erreicht Relief auch so nicht. Ohne Position
//! (`bounds`) keine Marke. Bei offenem modalem Dialog nur dessen Inhalt.
//! Eine Marke trägt die Herkunft des Namens; ein nicht gesicherter Name
//! wird als solcher gezeigt.

use relief_model::{Action, Certainty, Fact, NodeRef, Rect, Role, SemanticGraph};

use crate::graph::{control, Control, Graph};
use crate::validate::ActionKind;

/// Buchstaben der Marken: Grundreihe, damit die Hände liegen bleiben.
const ALPHABET: &[char] = &['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l'];

/// Eine Sprungmarke.
#[derive(Debug, Clone)]
pub struct Mark {
    /// Buchstabenfolge; alle Marken eines Stands sind gleich lang, also
    /// ist keine Präfix einer anderen.
    pub label: String,
    /// Das Element als Bedienelement (auch für Knoten ohne Bedienrolle).
    pub control: Control,
    /// Seitenkoordinaten in CSS-Pixeln.
    pub bounds: Rect,
    /// Was die Auswahl tut: Felder fokussieren, sonst auslösen.
    pub kind: ActionKind,
}

impl Mark {
    /// Name ist nicht gesichert (erschlossen, unsicher oder fehlt).
    pub fn uncertain(&self) -> bool {
        self.control.name.certainty != Certainty::Known || self.control.name.value.is_none()
    }

    pub fn name(&self) -> &Fact<String> {
        &self.control.name
    }
}

/// Rollen, deren Auswahl in das Feld führt statt es auszulösen.
fn is_field(role: &Role) -> bool {
    matches!(
        role,
        Role::Textbox
            | Role::SearchBox
            | Role::Combobox
            | Role::Listbox
            | Role::SpinButton
            | Role::Slider
    )
}

/// Knoten, deren Standardaktion nur die eines anderen ist (Text in einem
/// Link, Beschriftung eines Felds): keine eigene Marke.
fn is_part(role: &Role) -> bool {
    matches!(
        role,
        Role::StaticText | Role::InlineTextBox | Role::LabelText | Role::ListMarker
    )
}

fn has_area(r: &Rect) -> bool {
    r.width > 0.0 && r.height > 0.0
}

/// Container, die als Ganzes nichts auslösen, auch wenn Chromium dort einen
/// Klick meldet (Klick-Handler am Wrapper).
fn is_container(role: &Role) -> bool {
    matches!(
        role,
        Role::Dialog
            | Role::AlertDialog
            | Role::Banner
            | Role::Navigation
            | Role::Main
            | Role::Complementary
            | Role::ContentInfo
            | Role::Search
            | Role::Form
            | Role::Region
            | Role::RootWebArea
            | Role::Iframe
    )
}

/// Marken für den aktuellen Stand, in Leserichtung (oben nach unten, links
/// nach rechts).
///
/// Bedienelemente des Interaction Graph bekommen immer eine Marke. Ein
/// Knoten ohne Bedienrolle, für den Chromium einen Klick meldet, nur wenn er
/// kein Bedienelement enthält (sonst ist er ein Wrapper, oft die ganze
/// Seite) und nicht in einem markierten Knoten liegt (Chromium meldet den
/// Klick auch an Kindern eines Links).
pub fn marks(model: &SemanticGraph, graph: &Graph) -> Vec<Mark> {
    // Vorfahren von Bedienelementen: Wrapper, keine eigene Marke.
    let mut wrappers: std::collections::HashSet<NodeRef> = std::collections::HashSet::new();
    for c in &graph.controls {
        let mut current = model.node(&c.node).and_then(|n| n.parent);
        while let Some(id) = current {
            let parent = NodeRef::new(c.node.tree.clone(), id);
            if !wrappers.insert(parent.clone()) {
                break;
            }
            current = model.node(&parent).and_then(|n| n.parent);
        }
    }
    let mut found: Vec<(Control, Rect)> = Vec::new();
    let mut taken: Vec<NodeRef> = Vec::new();
    for at in model.document_order() {
        let Some(node) = model.node(&at) else {
            continue;
        };
        let Some(bounds) = node.bounds.filter(has_area) else {
            continue;
        };
        if node.ignored || node.states.disabled || is_part(&node.role) {
            continue;
        }
        let known = graph.controls.iter().find(|c| c.node == at);
        if known.is_none()
            && (!node.actions.contains(&Action::DoDefault)
                || is_container(&node.role)
                || wrappers.contains(&at)
                || inside_taken(model, &at, &taken))
        {
            continue;
        }
        let region = known
            .and_then(|c| c.region)
            .or_else(|| region_of(model, graph, &at));
        if !graph.is_reachable(region, &at) {
            continue;
        }
        let c = match known {
            Some(c) => c.clone(),
            None => control(model, &at, node, region),
        };
        taken.push(at);
        found.push((c, bounds));
    }
    found.sort_by(|a, b| {
        (a.1.y, a.1.x)
            .partial_cmp(&(b.1.y, b.1.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let labels = labels(found.len());
    found
        .into_iter()
        .zip(labels)
        .map(|((control, bounds), label)| Mark {
            label,
            kind: if is_field(&control.role) {
                ActionKind::Focus
            } else {
                ActionKind::Activate
            },
            control,
            bounds,
        })
        .collect()
}

/// Liegt der Knoten in einem, der schon eine Marke trägt?
fn inside_taken(model: &SemanticGraph, at: &NodeRef, taken: &[NodeRef]) -> bool {
    let mut current = model.node(at).and_then(|n| n.parent);
    while let Some(id) = current {
        let parent = NodeRef::new(at.tree.clone(), id);
        if taken.contains(&parent) {
            return true;
        }
        current = model.node(&parent).and_then(|n| n.parent);
    }
    false
}

/// Innerster Bereich (Landmark oder Dialog) über einem Knoten desselben
/// Baums.
fn region_of(model: &SemanticGraph, graph: &Graph, at: &NodeRef) -> Option<usize> {
    let mut current = Some(at.node);
    while let Some(id) = current {
        let here = NodeRef::new(at.tree.clone(), id);
        if let Some(i) = graph.regions.iter().position(|r| r.node == here) {
            return Some(i);
        }
        current = model.node(&here).and_then(|n| n.parent);
    }
    None
}

/// `n` gleich lange Buchstabenfolgen aus [`ALPHABET`].
fn labels(n: usize) -> Vec<String> {
    let mut length = 1;
    while ALPHABET.len().pow(length) < n {
        length += 1;
    }
    (0..n)
        .map(|mut i| {
            let mut out = vec![' '; length as usize];
            for slot in out.iter_mut().rev() {
                *slot = ALPHABET[i % ALPHABET.len()];
                i /= ALPHABET.len();
            }
            out.into_iter().collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marken_gleich_lang_und_eindeutig() {
        assert_eq!(labels(3), vec!["a", "s", "d"]);
        let many = labels(10);
        assert_eq!(many[0], "aa");
        assert_eq!(many[9], "sa");
        assert!(many.iter().all(|l| l.len() == 2));
        let mut unique = many.clone();
        unique.dedup();
        assert_eq!(unique.len(), 10);
        assert_eq!(labels(82).last().unwrap().len(), 3);
    }
}
